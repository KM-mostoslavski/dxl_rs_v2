//! One-off: show, and with `DXL_MIN` set, write Min Voltage Limit on every AX-12A, through the
//! public `Bus` API.
//!
//! Setup holds an AX-12A that reports an Input Voltage error `Faulted` instead of failing, and
//! a read of it returns the measured fault, e.g.
//! `Fault(Voltage(BelowMin { id: 5, measured: 9.7 V, min: 10.5 V, max: 13.0 V }))`, which
//! tells whether to lower `DXL_MIN` here or to fix the supply. `set_voltage_limits` then writes
//! the new minimum beside the maximum the actuator holds, after checking it against the
//! e-Manual range (5.0-16.0 V) and that maximum, and `clear_fault` asks the actuator whether
//! its fault is gone.
//!
//! `DXL_MIN` is in 0.1 V: 90 lowers the limit to 9.0 V (the bottom of the AX-12A operating
//! range), 105 puts it back to 10.5 V. The value stays in EEPROM until written again.
//! `ax12_sine` saves, widens and restores the limits for its own run and does not need this.
//!
//! ```sh
//! DXL_PORT=/dev/ttyUSB0 cargo run --example ax12_voltage_fix              # report only
//! DXL_PORT=/dev/ttyUSB0 DXL_MIN=90 cargo run --example ax12_voltage_fix   # 9.0 V
//! DXL_PORT=/dev/ttyUSB0 DXL_MIN=105 cargo run --example ax12_voltage_fix  # back to 10.5 V
//! ```

use std::time::Duration;

use dxl_rs_v2::{
    Actuator, Bus, Config, Decivolts, Error, FaultError, P1, SerialBus, SerialTransport, StdClock,
    Voltage, VoltageError,
};

const AX12A: u16 = 12;

/// `d` in volts.
fn volts(d: Decivolts) -> Voltage {
    Voltage::from_volts(f32::from(d.0) / 10.0)
}

/// The voltage limits and the measured voltage a voltage fault carries.
fn voltage_fault(err: &Error) -> Option<((Voltage, Voltage), Voltage)> {
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

/// `result`, except that a hardware fault from a faulted actuator is no error: every reply
/// it sends carries the fault until it is cleared.
fn tolerate(result: Result<(), Error>, faulted: bool) -> Result<(), Error> {
    match result {
        Err(e) if faulted && e.is_hardware_fault() => Ok(()),
        other => other,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("DXL_PORT").map_err(|_| "set DXL_PORT to the serial adapter")?;
    let baud = std::env::var("DXL_BAUD").map_or(1_000_000, |v| v.parse().expect("DXL_BAUD"));
    let target: Option<u8> = std::env::var("DXL_MIN")
        .ok()
        .map(|v| v.parse().expect("DXL_MIN"));
    let timeout = Duration::from_millis(20);
    let config = Config::new().timeout(timeout).scan_timeout(timeout);
    let mut bus: SerialBus<P1> = Bus::new(
        SerialTransport::open(&path, baud)?,
        StdClock::default(),
        config,
    );
    let report = bus.autoscan()?;
    let faulted = report.faulted;
    let found: Vec<_> = report
        .actuators
        .iter()
        .map(|a| (a.id(), a.model_number()))
        .collect();

    for (id, model) in found {
        if model != AX12A {
            println!("id {:>3}: model {model}, skipped", id.get());
            continue;
        }
        let Some(mut a) = bus.actuator(id) else {
            continue;
        };
        let held_faulted = faulted.contains(id);
        let ((lo, hi), fault) = match a.voltage_limits() {
            Ok(limits) => (limits, None),
            Err(e) => match voltage_fault(&e) {
                Some((limits, _)) => (limits, Some(e)),
                None => return Err(e.into()),
            },
        };
        let now = match &fault {
            Some(e) => voltage_fault(e).map(|(_, measured)| measured),
            None => Some(a.present_voltage()?),
        };
        println!(
            "id {:>3}: AX-12A, limits {:.1}-{:.1} V, present {}{}",
            id.get(),
            lo.volts(),
            hi.volts(),
            now.map_or_else(|| "?".into(), |v| format!("{:.1} V", v.volts())),
            fault.map_or_else(String::new, |e| format!(", fault: {e}")),
        );
        let Some(target) = target else { continue };
        let min = Voltage::from_volts(f32::from(target) / 10.0);
        if (lo.volts() - min.volts()).abs() < 0.05 {
            continue;
        }
        // Setup writes no torque: the bus holds `Off`, the actuator may still be on.
        tolerate(a.disable_torque(), held_faulted)?;
        tolerate(a.set_voltage_limits(min, hi), held_faulted)?;
        println!("         min limit {:.1} V written", min.volts());
        if held_faulted {
            match a.clear_fault() {
                Ok(()) => println!("         fault cleared"),
                Err(e) => println!("         still faulted: {e}"),
            }
        }
    }
    if target.is_none() {
        println!("report only; DXL_MIN=<0.1 V> writes Min Voltage Limit where it differs");
    }
    Ok(())
}
