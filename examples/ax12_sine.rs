//! Small sine-wave motion on every actuator of a Protocol 1 bus, through the public `Bus` API.
//!
//! ```sh
//! DXL_PORT=/dev/serial/by-id/usb-FTDI_FT232R_USB_UART_AL02L1KL-if00-port0 \
//!   cargo run --example ax12_sine
//! ```
//!
//! - `DXL_PORT` (required) and `DXL_BAUD` (default 1000000).
//! - `DXL_TIMEOUT_MS` (default 20): reply timeout, also per ID in the voltage table sweep.
//!   Above the FTDI latency timer, 16 ms unless lowered in
//!   `/sys/bus/usb-serial/devices/ttyUSB0/latency_timer`.
//! - `DXL_AMPLITUDE` ticks around each actuator's start position (default 30, at most 100;
//!   0.29° per tick on the AX-12A), `DXL_PERIOD_MS` (default 2000), `DXL_CYCLES` (default 3).
//!
//! Torque goes off on every actuator at the end, also after an error.
//!
//! A long daisy chain sags below the AX-12A's Min Voltage Limit (10.5 V) on the far actuators,
//! braking spikes past the Max Voltage Limit (13.0 V), and the bus refuses an actuator
//! reporting an Input Voltage error. So the limits (addresses 12 and 13) are widened to
//! 5.0-25.0 V, the widest they accept, before Setup, and set back to 10.5-13.0 V at the
//! end, also after an error. Both are broadcast writes: every actuator on the port gets them. A table of limits, present voltage and error byte follows
//! each write, read back raw.

use std::time::{Duration, Instant};

use dxl_rs_v2::serialport::{self, SerialPort};
use dxl_rs_v2::{
    Actuator, Bus, Config, Error, FaultError, Id, ModelNumber, P1, SerialBus, SerialTransport,
    SetupError, StdClock, TemperatureError, VoltageError,
};

/// Highest Goal Position in joint mode on the AX-12A; goals near either end are clamped.
const MAX_POSITION: i32 = 1023;
/// Model Number of the AX-12A.
const AX12A: u16 = 12;
/// Min Voltage Limit, in 0.1 V; Max Voltage Limit follows at 13.
const VOLTAGE_LIMITS: u8 = 12;
/// Present Voltage, in 0.1 V.
const PRESENT_VOLTAGE: u8 = 42;
/// 5.0-25.0 V while the example runs, the widest the AX-12A accepts: under load the far
/// actuators sag below 9 V, and braking pushes short spikes above 13 V.
const LIMITS_RUN: [u8; 2] = [50, 250];
/// 10.5-13.0 V, the values these actuators held before.
const LIMITS_RESTORE: [u8; 2] = [105, 130];
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

fn checksum(body: &[u8]) -> u8 {
    !body.iter().fold(0u8, |acc, b| acc.wrapping_add(*b))
}

fn open_raw(path: &str, baud: u32, timeout: Duration) -> serialport::Result<Box<dyn SerialPort>> {
    serialport::new(path, baud).timeout(timeout).open()
}

/// One raw Protocol 1 instruction packet, input drained first.
fn send(port: &mut dyn SerialPort, id: u8, instr: u8, params: &[u8]) -> std::io::Result<()> {
    let mut body = vec![id, params.len() as u8 + 2, instr];
    body.extend_from_slice(params);
    let mut packet = vec![0xFF, 0xFF];
    packet.extend_from_slice(&body);
    packet.push(checksum(&body));
    port.clear(serialport::ClearBuffer::Input)?;
    port.write_all(&packet)?;
    port.flush()
}

/// Status packet: (error byte, parameters), or `None` on timeout or a bad packet.
fn receive(port: &mut dyn SerialPort, id: u8) -> Option<(u8, Vec<u8>)> {
    let mut head = [0u8; 4];
    port.read_exact(&mut head).ok()?;
    if head[..2] != [0xFF, 0xFF] || head[2] != id {
        return None;
    }
    let mut rest = vec![0u8; usize::from(head[3])];
    port.read_exact(&mut rest).ok()?;
    let (chk, data) = rest.split_last()?;
    let mut body = vec![head[2], head[3]];
    body.extend_from_slice(data);
    (checksum(&body) == *chk).then(|| (data[0], data[1..].to_vec()))
}

fn read(port: &mut dyn SerialPort, id: u8, addr: u8, len: u8) -> Option<(u8, Vec<u8>)> {
    send(port, id, 0x02, &[addr, len]).ok()?;
    receive(port, id).filter(|(_, p)| p.len() == usize::from(len))
}

/// Broadcast Write of `data` at `addr`: no status reply, so the table after it is the check.
fn broadcast_write(
    path: &str,
    baud: u32,
    timeout: Duration,
    addr: u8,
    data: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut port = open_raw(path, baud, timeout)?;
    let params: Vec<u8> = std::iter::once(addr).chain(data.iter().copied()).collect();
    send(port.as_mut(), 0xFE, 0x03, &params)?;
    // EEPROM write time before the next packet.
    std::thread::sleep(Duration::from_millis(20));
    Ok(())
}

/// Prints limits, present voltage and error byte of every AX-12A among `ids`, or among all IDs
/// when `ids` is `None`; returns the AX-12A IDs that answered.
fn voltage_table(
    path: &str,
    baud: u32,
    timeout: Duration,
    ids: Option<&[u8]>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut port = open_raw(path, baud, timeout)?;
    let port = port.as_mut();
    let all: Vec<u8> = (0..=252).collect();
    let mut found = Vec::new();
    for &id in ids.unwrap_or(&all) {
        let Some((_, model)) = read(port, id, 0, 2) else {
            if ids.is_some() {
                println!("id {id:>3}: no reply");
            }
            continue;
        };
        if u16::from_le_bytes([model[0], model[1]]) != AX12A {
            continue;
        }
        found.push(id);
        let (Some(limits), Some(present)) = (
            read(port, id, VOLTAGE_LIMITS, 2),
            read(port, id, PRESENT_VOLTAGE, 1),
        ) else {
            println!("id {id:>3}: AX-12A, voltage read failed");
            continue;
        };
        println!(
            "id {id:>3}: AX-12A, limits {:.1}-{:.1} V, present {:.1} V, error 0x{:02X}",
            f32::from(limits.1[0]) / 10.0,
            f32::from(limits.1[1]) / 10.0,
            f32::from(present.1[0]) / 10.0,
            present.0,
        );
    }
    Ok(found)
}

fn print_sent([lo, hi]: [u8; 2]) {
    println!(
        "voltage limits {:.1}-{:.1} V sent",
        f32::from(lo) / 10.0,
        f32::from(hi) / 10.0
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("DXL_PORT").map_err(|_| "set DXL_PORT to the serial adapter")?;
    let baud = env_u32("DXL_BAUD", 1_000_000);
    let timeout = Duration::from_millis(env_u32("DXL_TIMEOUT_MS", 20).into());

    broadcast_write(&path, baud, timeout, VOLTAGE_LIMITS, &LIMITS_RUN)?;
    print_sent(LIMITS_RUN);
    let ids = voltage_table(&path, baud, timeout, None);
    let result = ids.and_then(|ids| drive(&path, baud, timeout, &ids).map(|()| ids));

    let restored = broadcast_write(&path, baud, timeout, VOLTAGE_LIMITS, &LIMITS_RESTORE);
    match &restored {
        Ok(()) => print_sent(LIMITS_RESTORE),
        Err(e) => println!("restoring voltage limits failed: {e}"),
    }
    let ids = result.as_ref().ok().map(Vec::as_slice);
    if let Err(e) = voltage_table(&path, baud, timeout, ids) {
        println!("voltage table failed: {e}");
    }
    result.and(restored).map(|_| ())
}

/// Scan, sine, torque off. The bus, and with it the port, closes on return.
/// `found` are the AX-12A IDs the voltage table saw: declared, not scanned again.
fn drive(
    path: &str,
    baud: u32,
    timeout: Duration,
    found: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
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
        SerialTransport::open(path, baud)?,
        StdClock::default(),
        config,
    );
    let list = found
        .iter()
        .map(|&id| Ok((Id::new(id)?, ModelNumber(AX12A))))
        .collect::<Result<Vec<_>, Error>>()?;
    let ids: Vec<Id> = bus.declare(list)?.iter().map(|a| a.id()).collect();
    println!("found {} actuators", ids.len());

    let mut centers = Vec::new();
    for &id in &ids {
        let mut a = bus.actuator(id).ok_or(SetupError::NotOnBus { id })?;
        let center = a.present_position_raw()?;
        println!("  {} id {:>3}: at {center}", a.name(), id.get());
        centers.push(center);
    }

    let result = run(&mut bus, &ids, &centers, amplitude, period, cycles);

    for &id in &ids {
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
