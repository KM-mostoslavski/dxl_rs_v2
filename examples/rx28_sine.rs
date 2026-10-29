//! Small sine-wave motion on every actuator of a Protocol 1 bus, through the public `Bus` API.
//!
//! ```sh
//! DXL_PORT=/dev/serial/by-id/usb-FTDI_FT232R_USB_UART_AL02L1KL-if00-port0 \
//!   cargo run --example rx28_sine
//! ```
//!
//! - `DXL_PORT` (required) and `DXL_BAUD` (default 1000000).
//! - `DXL_AMPLITUDE` ticks around each actuator's start position (default 30, at most 100;
//!   0.29° per tick on the RX-28), `DXL_PERIOD_MS` (default 2000), `DXL_CYCLES` (default 3).
//!
//! Torque goes off on every actuator at the end, also after an error.

use std::time::{Duration, Instant};

use dxl_rs_v2::{Actuator, Id, P1, SerialBus};

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name).map_or(default, |v| v.parse().expect(name))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("DXL_PORT").map_err(|_| "set DXL_PORT to the serial adapter")?;
    let amplitude = env_u32("DXL_AMPLITUDE", 30);
    if amplitude > 100 {
        return Err("DXL_AMPLITUDE above 100 ticks".into());
    }
    let period = Duration::from_millis(env_u32("DXL_PERIOD_MS", 2000).into());
    let cycles = env_u32("DXL_CYCLES", 3);

    let mut bus = SerialBus::<P1>::open(&path, env_u32("DXL_BAUD", 1_000_000))?;
    let ids: Vec<Id> = bus.autoscan()?.iter().map(|a| a.id()).collect();
    println!("found {} actuators", ids.len());

    let mut centers = Vec::new();
    for &id in &ids {
        let mut a = bus.actuator(id).ok_or("not on the bus")?;
        let center = a.present_position_raw()?;
        println!("  {} id {:>3}: at {center}", a.name(), id.get());
        centers.push(center);
    }

    let result = run(&mut bus, &ids, &centers, amplitude, period, cycles);

    for &id in &ids {
        if let Some(mut a) = bus.actuator(id)
            && let Err(e) = a.disable_torque()
        {
            println!("  id {}: torque off failed: {e}", id.get());
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
        bus.actuator(id).ok_or("not on the bus")?.enable_torque()?;
    }
    let start = Instant::now();
    let total = period * cycles;
    loop {
        let elapsed = start.elapsed().min(total);
        let phase = elapsed.as_secs_f64() / period.as_secs_f64() * std::f64::consts::TAU;
        let offset = (f64::from(amplitude) * phase.sin()).round() as i32;
        for (&id, &center) in ids.iter().zip(centers) {
            bus.actuator(id)
                .ok_or("not on the bus")?
                .set_goal_position_raw(center + offset)?;
        }
        if elapsed >= total {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(300));
    for (&id, &center) in ids.iter().zip(centers) {
        let now = bus
            .actuator(id)
            .ok_or("not on the bus")?
            .present_position_raw()?;
        println!("  id {:>3}: back at {now} (start {center})", id.get());
    }
    Ok(())
}
