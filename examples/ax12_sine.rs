//! Small sine-wave motion on every AX-12A of a Protocol 1 bus, through the public `Bus` API.
//!
//! ```sh
//! DXL_PORT=/dev/serial/by-id/usb-FTDI_FT232R_USB_UART_AL02L1KL-if00-port0 \
//!   cargo run --example ax12_sine
//! ```
//!
//! - `DXL_PORT` (required) and `DXL_BAUD` (default 1000000).
//! - `DXL_TIMEOUT_MS` (default 20): reply timeout, also per ID in the scan.
//!   Above the FTDI latency timer, 16 ms unless lowered in
//!   `/sys/bus/usb-serial/devices/ttyUSB0/latency_timer`.
//! - `DXL_AMPLITUDE` ticks around each actuator's start position (default 30, at most 100;
//!   0.29° per tick on the AX-12A), `DXL_PERIOD_MS` (default 2000), `DXL_CYCLES` (default 3).
//!
//! Torque goes off on every actuator at the end, also after an error.
//!
//! A long daisy chain sags below the AX-12A's Min Voltage Limit (10.5 V by default) on the far
//! actuators, and braking spikes past its Max Voltage Limit (13.0 V). Setup holds an actuator
//! reporting an Input Voltage error `Faulted` instead of failing, so after the scan each
//! AX-12A has its voltage limits saved (`voltage_limits()`, or the limits its voltage fault
//! carries), torque turned off, the limits widened to 5.0-16.0 V, the e-Manual range, with
//! `set_voltage_limits()`, and its fault cleared. At the end, also after an error, each gets
//! the limits it held back (NFR-11: the caller restores what it changed). A table of limits
//! and present voltage follows each change.

use std::time::{Duration, Instant};

use dxl_rs_v2::{
    Actuator, ActuatorRef, Bus, Clock, Config, Error, FaultError, Id, IdSet, P1, SerialBus,
    SerialTransport, SetupError, StdClock, TemperatureError, Transport, Voltage, VoltageError,
};

/// Highest Goal Position in joint mode on the AX-12A; goals near either end are clamped.
const MAX_POSITION: i32 = 1023;
/// Model Number of the AX-12A.
const AX12A: u16 = 12;
/// 5.0-16.0 V while the example runs, the e-Manual range of both limits: under load the far
/// actuators sag below 9 V, and braking pushes short spikes above 13 V.
const LIMITS_RUN: (f32, f32) = (5.0, 16.0);
/// Extra attempts after a timeout or a corrupted reply.
const RETRIES: u8 = 3;
/// Failed goal writes in a row on one actuator before the run stops.
const MAX_STREAK: u32 = 50;

/// One line on what to do about a measured hardware fault, read from the error's fields.
fn fault_advice(err: &Error) -> Option<String> {
    match err {
        Error::Fault(FaultError::Voltage(VoltageError::BelowMin {
            id, measured, min, ..
        })) => Some(format!(
            "id {}: supply sags to {measured}, under the {min} minimum: thicker or shorter \
             wiring, or feed power at both ends of the chain",
            id.get()
        )),
        Error::Fault(FaultError::Voltage(VoltageError::AboveMax {
            id, measured, max, ..
        })) => Some(format!(
            "id {}: supply at {measured}, over the {max} maximum: lower the supply voltage",
            id.get()
        )),
        Error::Fault(FaultError::Temperature(TemperatureError::AboveLimit {
            id,
            measured,
            limit,
            ..
        })) => Some(format!(
            "id {}: {measured}, over its {limit} limit: let it cool",
            id.get()
        )),
        _ => None,
    }
}

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name).map_or(default, |v| v.parse().expect(name))
}

/// The voltage limits and the measured voltage a voltage fault carries, in volts.
fn voltage_fault(err: &Error) -> Option<((Voltage, Voltage), Voltage)> {
    let volts = |d: dxl_rs_v2::Decivolts| Voltage::from_volts(f32::from(d.0) / 10.0);
    match err {
        Error::Fault(FaultError::Voltage(
            VoltageError::BelowMin {
                measured, min, max, ..
            }
            | VoltageError::AboveMax {
                measured, min, max, ..
            }
            | VoltageError::InsideLimits {
                measured, min, max, ..
            },
        )) => Some(((volts(*min), volts(*max)), volts(*measured))),
        _ => None,
    }
}

/// The voltage limits `a` holds: read, or, while it reports a voltage fault (every reply
/// carries it, so the read returns the fault instead of the data), the limits that fault
/// carries.
fn held_limits<P, T, C>(a: &mut ActuatorRef<'_, P, T, C>) -> Result<(Voltage, Voltage), Error>
where
    P: dxl_rs_v2::Protocol,
    T: Transport,
    C: Clock,
{
    a.voltage_limits()
        .or_else(|e| voltage_fault(&e).map(|(limits, _)| limits).ok_or(e))
}

/// `result`, except that a hardware fault from an actuator held faulted is no error: every
/// reply it sends carries the fault until it is cleared, so the write's outcome is in the
/// table that follows.
fn tolerate(result: Result<(), Error>, faulted: bool) -> Result<(), Error> {
    match result {
        Err(e) if faulted && e.is_hardware_fault() => Ok(()),
        other => other,
    }
}

/// Prints limits and present voltage of every actuator among `ids`.
fn voltage_table(bus: &mut SerialBus<P1>, ids: &[Id]) {
    for &id in ids {
        let Some(mut a) = bus.actuator(id) else {
            continue;
        };
        let row = match a.voltage_limits() {
            Ok((min, max)) => match a.present_voltage() {
                Ok(now) => format!(
                    "limits {:.1}-{:.1} V, present {:.1} V",
                    min.volts(),
                    max.volts(),
                    now.volts()
                ),
                Err(e) => format!(
                    "limits {:.1}-{:.1} V, voltage read failed: {e}",
                    min.volts(),
                    max.volts()
                ),
            },
            Err(e) => match voltage_fault(&e) {
                Some(((min, max), now)) => format!(
                    "limits {:.1}-{:.1} V, present {:.1} V, fault: {e}",
                    min.volts(),
                    max.volts(),
                    now.volts()
                ),
                None => format!("voltage read failed: {e}"),
            },
        };
        println!("id {:>3}: {}, {row}", id.get(), a.name());
    }
}

/// Saves the voltage limits of each actuator among `ids` into `saved`, turns its torque off,
/// widens its limits to [`LIMITS_RUN`] and clears its fault if Setup held it `Faulted`.
/// An actuator is saved before its limits change, so whatever this did can be restored.
fn widen(
    bus: &mut SerialBus<P1>,
    ids: &[Id],
    faulted: IdSet,
    saved: &mut Vec<(Id, (Voltage, Voltage))>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (min, max) = LIMITS_RUN;
    for &id in ids {
        let held_faulted = faulted.contains(id);
        let mut a = bus.actuator(id).ok_or(SetupError::NotOnBus { id })?;
        saved.push((id, held_limits(&mut a)?));
        // Setup writes no torque: the bus holds `Off`, the actuator may still be on.
        tolerate(a.disable_torque(), held_faulted)?;
        tolerate(
            a.set_voltage_limits(Voltage::from_volts(min), Voltage::from_volts(max)),
            held_faulted,
        )?;
        if held_faulted {
            a.clear_fault()?;
        }
    }
    println!("voltage limits {min:.1}-{max:.1} V written");
    Ok(())
}

/// Turns torque off and writes back the voltage limits each actuator of `saved` held.
/// Returns the first error after trying them all.
fn restore(
    bus: &mut SerialBus<P1>,
    saved: &[(Id, (Voltage, Voltage))],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut first: Option<Error> = None;
    for &(id, (min, max)) in saved {
        let Some(mut a) = bus.actuator(id) else {
            continue;
        };
        let faulted = a.torque_state() == dxl_rs_v2::TorqueState::Faulted;
        let result = tolerate(a.disable_torque(), faulted)
            .and_then(|()| tolerate(a.set_voltage_limits(min, max), faulted));
        match result {
            Ok(()) => println!(
                "id {:>3}: voltage limits {:.1}-{:.1} V restored",
                id.get(),
                min.volts(),
                max.volts()
            ),
            Err(e) => {
                println!("id {:>3}: restoring voltage limits failed: {e}", id.get());
                first.get_or_insert(e);
            }
        }
    }
    first.map_or(Ok(()), |e| Err(e.into()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("DXL_PORT").map_err(|_| "set DXL_PORT to the serial adapter")?;
    let baud = env_u32("DXL_BAUD", 1_000_000);
    let timeout = Duration::from_millis(env_u32("DXL_TIMEOUT_MS", 20).into());
    let amplitude = env_u32("DXL_AMPLITUDE", 30);
    if amplitude > 100 {
        return Err("DXL_AMPLITUDE above 100 ticks".into());
    }
    let period = Duration::from_millis(env_u32("DXL_PERIOD_MS", 2000).into());
    let cycles = env_u32("DXL_CYCLES", 3);

    let config = Config::new()
        .timeout(timeout)
        .scan_timeout(timeout)
        .retries(RETRIES);
    let mut bus: SerialBus<P1> = Bus::new(
        SerialTransport::open(&path, baud)?,
        StdClock::default(),
        config,
    );
    let report = bus.autoscan()?;
    let faulted = report.faulted;
    let ids: Vec<Id> = report
        .actuators
        .iter()
        .filter(|a| a.model_number() == AX12A)
        .map(|a| a.id())
        .collect();
    println!("found {} AX-12A, {} held faulted", ids.len(), faulted.len());
    voltage_table(&mut bus, &ids);

    let mut saved = Vec::new();
    let result = widen(&mut bus, &ids, faulted, &mut saved).and_then(|()| {
        voltage_table(&mut bus, &ids);
        drive(&mut bus, &ids, amplitude, period, cycles)
    });

    let restored = restore(&mut bus, &saved);
    voltage_table(&mut bus, &ids);
    result.and(restored)
}

/// Sine, torque off. `ids` are the AX-12A the scan found, their voltage limits widened.
fn drive(
    bus: &mut SerialBus<P1>,
    ids: &[Id],
    amplitude: u32,
    period: Duration,
    cycles: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut centers = Vec::new();
    for &id in ids {
        let mut a = bus.actuator(id).ok_or(SetupError::NotOnBus { id })?;
        let center = a.present_position_raw()?;
        println!("  {} id {:>3}: at {center}", a.name(), id.get());
        centers.push(center);
    }

    let result = run(bus, ids, &centers, amplitude, period, cycles);

    for &id in ids {
        if let Some(mut a) = bus.actuator(id)
            && let Err(e) = a.disable_torque()
        {
            println!("  torque off failed: {e}");
        }
    }
    result
}

fn run(
    bus: &mut SerialBus<P1>,
    ids: &[Id],
    centers: &[i32],
    amplitude: u32,
    period: Duration,
    cycles: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    for &id in ids {
        bus.actuator(id)
            .ok_or(SetupError::NotOnBus { id })?
            .enable_torque()?;
    }
    let start = Instant::now();
    let total = period * cycles;
    // Per actuator: failed goal writes in a row, and in total.
    let mut streak = vec![0u32; ids.len()];
    let mut failed = vec![0u32; ids.len()];
    loop {
        let elapsed = start.elapsed().min(total);
        let phase = elapsed.as_secs_f64() / period.as_secs_f64() * std::f64::consts::TAU;
        let offset = (f64::from(amplitude) * phase.sin()).round() as i32;
        for (i, (&id, &center)) in ids.iter().zip(centers).enumerate() {
            let at = elapsed.as_secs_f64();
            match bus
                .actuator(id)
                .ok_or(SetupError::NotOnBus { id })?
                .set_goal_position_raw((center + offset).clamp(0, MAX_POSITION))
            {
                Ok(()) => streak[i] = 0,
                // A fault the actuator reports about itself stops the run. The bus has
                // already measured it: `e` says the voltage or temperature and its limit.
                Err(e) if e.is_hardware_fault() => {
                    println!("  id {:>3} at {at:.2} s: hardware fault", id.get());
                    if let Some(advice) = fault_advice(&e) {
                        println!("  {advice}");
                    }
                    return Err(e.into());
                }
                // A lost or corrupted packet: the next round sends a fresh goal.
                Err(e) => {
                    streak[i] += 1;
                    failed[i] += 1;
                    println!("  at {at:.2} s: {e}");
                    if streak[i] >= MAX_STREAK {
                        return Err(
                            format!("id {}: {MAX_STREAK} goals in a row failed", id.get()).into(),
                        );
                    }
                }
            }
        }
        if elapsed >= total {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    std::thread::sleep(Duration::from_millis(300));
    for ((&id, &center), failed) in ids.iter().zip(centers).zip(failed) {
        match bus
            .actuator(id)
            .ok_or(SetupError::NotOnBus { id })?
            .present_position_raw()
        {
            Ok(now) => println!(
                "  id {:>3}: back at {now} (start {center}), {failed} goals lost",
                id.get()
            ),
            Err(e) => println!("  position read failed: {e}, {failed} goals lost"),
        }
    }
    Ok(())
}
