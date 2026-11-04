//! Sine-wave sweep from 10 to 4085 ticks on every actuator of a Protocol 2 bus, through the
//! public `Bus` API.
//!
//! ```sh
//! DXL_PORT=/dev/serial/by-id/usb-FTDI_FT232R_USB_UART_AL02L1KL-if00-port0 \
//!   cargo run --example mx64_sine
//! ```
//!
//! - `DXL_PORT` (required) and `DXL_BAUD` (default 1000000).
//! - `DXL_PERIOD_MS` (default 8000), `DXL_CYCLES` (default 3).
//!
//! Each actuator's sine starts at the phase matching its start position, so the first goal
//! does not jump across the range. Every tick logs each actuator's goal, present position
//! and present temperature.
//!
//! Last comes a step test: move to 4085, then a single goal of 10, logged each tick until
//! the actuator is within 10 ticks of it (5 s timeout per move).
//!
//! Torque goes off on every actuator at the end, also after an error.

use std::f64::consts::TAU;
use std::time::{Duration, Instant};

use dxl_rs_v2::{Actuator, Id, P2, SerialBus, SetupError};

/// Lowest goal of the sweep, in ticks (0.088° per tick on the MX-64).
const LOW: f64 = 10.0;
/// Highest goal of the sweep, in ticks.
const HIGH: f64 = 4085.0;
const MID: f64 = (LOW + HIGH) / 2.0;
const HALF: f64 = (HIGH - LOW) / 2.0;

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name).map_or(default, |v| v.parse().expect(name))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("DXL_PORT").map_err(|_| "set DXL_PORT to the serial adapter")?;
    let period = Duration::from_millis(env_u32("DXL_PERIOD_MS", 8000).into());
    let cycles = env_u32("DXL_CYCLES", 3);

    let mut bus = SerialBus::<P2>::open(&path, env_u32("DXL_BAUD", 1_000_000))?;
    let ids: Vec<Id> = bus.autoscan()?.iter().map(|a| a.id()).collect();
    println!("found {} actuators", ids.len());

    let mut starts = Vec::new();
    for &id in &ids {
        let mut a = bus.actuator(id).ok_or(SetupError::NotOnBus { id })?;
        let start = a.present_position_raw()?;
        let temp = a.present_temperature()?.celsius();
        println!("  {} id {:>3}: at {start}, {temp} °C", a.name(), id.get());
        starts.push(start);
    }

    let result = run(&mut bus, &ids, &starts, period, cycles);

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
    bus: &mut SerialBus<P2>,
    ids: &[Id],
    starts: &[i32],
    period: Duration,
    cycles: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Phase where MID + HALF * sin(phase) equals the start position, clamped into the sweep.
    let phases: Vec<f64> = starts
        .iter()
        .map(|&s| ((f64::from(s) - MID) / HALF).clamp(-1.0, 1.0).asin())
        .collect();
    for &id in ids {
        bus.actuator(id)
            .ok_or(SetupError::NotOnBus { id })?
            .enable_torque()?;
    }
    let start = Instant::now();
    let total = period * cycles;
    loop {
        let elapsed = start.elapsed().min(total);
        let phase = elapsed.as_secs_f64() / period.as_secs_f64() * TAU;
        for (&id, &phase0) in ids.iter().zip(&phases) {
            let mut a = bus.actuator(id).ok_or(SetupError::NotOnBus { id })?;
            let goal = (MID + HALF * (phase0 + phase).sin()).round() as i32;
            a.set_goal_position_raw(goal)?;
            let now = a.present_position_raw()?;
            let temp = a.present_temperature()?.celsius();
            println!(
                "{:>6} ms  id {:>3}: goal {goal:>4}  at {now:>4}  {temp} °C",
                elapsed.as_millis(),
                id.get()
            );
        }
        if elapsed >= total {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    std::thread::sleep(Duration::from_millis(300));
    for (&id, &start) in ids.iter().zip(starts) {
        let mut a = bus.actuator(id).ok_or(SetupError::NotOnBus { id })?;
        let now = a.present_position_raw()?;
        let temp = a.present_temperature()?.celsius();
        println!(
            "  id {:>3}: ended at {now} (start {start}), {temp} °C",
            id.get()
        );
    }

    println!("step test: {HIGH} -> {LOW}");
    move_to(bus, ids, HIGH as i32)?;
    move_to(bus, ids, LOW as i32)?;
    Ok(())
}

/// Writes `goal` to every actuator once, then logs position and temperature each tick until
/// all are within `SETTLED` ticks of it or `MOVE_TIMEOUT` passes.
fn move_to(
    bus: &mut SerialBus<P2>,
    ids: &[Id],
    goal: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    const SETTLED: i32 = 10;
    const MOVE_TIMEOUT: Duration = Duration::from_secs(5);
    for &id in ids {
        bus.actuator(id)
            .ok_or(SetupError::NotOnBus { id })?
            .set_goal_position_raw(goal)?;
    }
    let start = Instant::now();
    loop {
        let elapsed = start.elapsed();
        let mut settled = true;
        for &id in ids {
            let mut a = bus.actuator(id).ok_or(SetupError::NotOnBus { id })?;
            let now = a.present_position_raw()?;
            let temp = a.present_temperature()?.celsius();
            println!(
                "{:>6} ms  id {:>3}: goal {goal:>4}  at {now:>4}  {temp} °C",
                elapsed.as_millis(),
                id.get()
            );
            settled &= (now - goal).abs() <= SETTLED;
        }
        if settled {
            println!("  reached {goal} in {} ms", elapsed.as_millis());
            return Ok(());
        }
        if elapsed >= MOVE_TIMEOUT {
            return Err(format!("not within {SETTLED} ticks of {goal} after 5 s").into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
