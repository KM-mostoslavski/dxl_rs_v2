//! Wheel mode on AX-12A and RX-28: spin every actuator in wheel mode one way, then the other,
//! then stop, through the typed goal velocity field.
//!
//! ```sh
//! DXL_PORT=/dev/serial/by-id/usb-FTDI_FT232R_USB_UART_AL02L1KL-if00-port0 \
//!   cargo run --example ax_wheel
//! ```
//!
//! - `DXL_PORT` (required) and `DXL_BAUD` (default 1000000).
//! - `DXL_OUTPUT` share of output per direction, in e-Manual steps of about 0.1 % (default
//!   200, at most 400), `DXL_SPIN_MS` per direction (default 1500).
//!
//! Wheel mode is CW Angle Limit = CCW Angle Limit = 0 in EEPROM. This crate does not write
//! EEPROM through typed calls, and Setup reads the mode once: set both limits to 0 (e.g. with
//! the `raw-registers` feature or the ROBOTIS tools), power-cycle, then run this. Actuators in
//! joint mode are listed and left alone.
//!
//! In wheel mode the e-Manual gives AX and RX Moving Speed as a share of output ("only the
//! output control is possible, not speed"), not as a velocity: bit 10 is the direction (set =
//! CW) and bits 0..=9 the magnitude. So the goal is written raw, `Raw(GoalVelocity)`; the bus
//! still checks torque, wheel mode and the register size before sending. Torque goes off on
//! every actuator at the end, also after an error.

use std::time::Duration;

use dxl_rs_v2::field::{GoalVelocity, PresentVelocity, Raw};
use dxl_rs_v2::{Actuator, Id, LimitMode, P1, SerialBus};

/// Bit 10 of AX / RX wheel-mode Moving Speed: set turns CW.
const CW: i32 = 1 << 10;

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name).map_or(default, |v| v.parse().expect(name))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("DXL_PORT").map_err(|_| "set DXL_PORT to the serial adapter")?;
    let output = env_u32("DXL_OUTPUT", 200);
    if output > 400 {
        return Err("DXL_OUTPUT above 400 steps".into());
    }
    let output = i32::try_from(output)?;
    let spin = Duration::from_millis(env_u32("DXL_SPIN_MS", 1500).into());

    let mut bus = SerialBus::<P1>::open(&path, env_u32("DXL_BAUD", 1_000_000))?;
    let mut wheels = Vec::new();
    for a in bus.autoscan()? {
        if a.limit_mode() == LimitMode::Wheel {
            wheels.push(a.id());
        } else {
            println!(
                "  {} id {:>3}: {:?} mode, left alone",
                a.name(),
                a.id().get(),
                a.limit_mode()
            );
        }
    }
    println!("{} actuators in wheel mode", wheels.len());

    let result = run(&mut bus, &wheels, output, spin);

    let stop = vec![0; wheels.len()];
    if let Err(e) = bus.sync_write(Raw(GoalVelocity), wheels.iter().copied(), &stop) {
        println!("  stop failed: {e}");
    }
    if let Err(e) = bus.disable_torque(wheels.iter().copied()) {
        println!("  torque off failed: {e}");
    }
    result
}

fn run(
    bus: &mut SerialBus<P1>,
    wheels: &[Id],
    output: i32,
    spin: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    bus.enable_torque(wheels.iter().copied())?;
    let mut speed = vec![0; wheels.len()];
    for (label, goal) in [("CCW", output), ("CW", CW | output), ("stop", 0)] {
        let goals = vec![goal; wheels.len()];
        bus.sync_write(Raw(GoalVelocity), wheels.iter().copied(), &goals)?;
        std::thread::sleep(spin);
        bus.read_field(Raw(PresentVelocity), wheels.iter().copied(), &mut speed)?;
        for (id, raw) in wheels.iter().zip(&speed) {
            let direction = if raw & CW != 0 { "CW" } else { "CCW" };
            println!(
                "  {label:>4}: id {:>3} turns {direction} at {} steps",
                id.get(),
                raw & (CW - 1)
            );
        }
    }
    Ok(())
}
