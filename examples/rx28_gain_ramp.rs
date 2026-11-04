//! Sine sweep of every actuator on a Protocol 1 bus while ramping Kp/Kd (RX-28 test).
//!
//! ```sh
//! DXL_PORT=/dev/ttyUSB0 cargo run --example rx28_gain_ramp --features raw-registers
//! ```
//!
//! Each actuator follows `center + amplitude * sin(2π t / period)` around the
//! position it holds at start, for `DURATION` seconds. Present temperature
//! and position are read every cycle. P and D gain ramp linearly from 0 to
//! their max over the run (P from 1: the RX-28 refuses slope 0), written raw at the MX-28 Protocol 1 addresses
//! (D Gain 26, P Gain 28): on an RX-28 these are CW Compliance Margin and
//! CW Compliance Slope, which is what this run observes. Torque is disabled
//! at the end, also on error.
//!
//! - `DXL_PORT` (required) and `DXL_BAUD` (default 1000000).
//! - `DXL_TIMEOUT_MS` (optional): reply and scan timeout, for an FTDI latency timer left at 16 ms.
//! - `DXL_AMP_DEG` (default 30), `DXL_PERIOD_S` (default 2): the sine.
//! - `DXL_KP_MAX` (default 254), `DXL_KD_MAX` (default 254): gain ramp end values.

use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use dxl_rs_v2::{
    Actuator, Bus, Config, Error, Id, P1, Position, SerialBus, SerialTransport, SetupError,
    StdClock,
};

const DURATION: Duration = Duration::from_secs(20);
const CYCLE: Duration = Duration::from_millis(1);
const D_GAIN_ADDR: u16 = 26;
const P_GAIN_ADDR: u16 = 28;

fn env_or<V: std::str::FromStr>(name: &str, default: V) -> V {
    std::env::var(name).map_or(default, |v| v.parse().ok().expect(name))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("DXL_PORT").map_err(|_| "set DXL_PORT to the serial adapter")?;
    let baud = env_or("DXL_BAUD", 1_000_000);
    let amplitude = Position::from_radians(env_or("DXL_AMP_DEG", 30.0_f32).to_radians());
    let period: f32 = env_or("DXL_PERIOD_S", 2.0);
    let kp_max: u8 = env_or("DXL_KP_MAX", 254);
    let kd_max: u8 = env_or("DXL_KD_MAX", 254);

    let mut bus: SerialBus<P1> = match std::env::var("DXL_TIMEOUT_MS") {
        Ok(ms) => {
            let t = Duration::from_millis(ms.parse().expect("DXL_TIMEOUT_MS"));
            let config = Config::new().timeout(t).scan_timeout(t);
            Bus::new(
                SerialTransport::open(&path, baud)?,
                StdClock::default(),
                config,
            )
        }
        Err(_) => SerialBus::open(&path, baud)?,
    };
    let held = bus.autoscan()?;
    let ids: Vec<Id> = held.iter().map(Actuator::id).collect();
    let names: Vec<&str> = held.iter().map(Actuator::name).collect();
    println!("found {} actuators", ids.len());

    let result = sweep(&mut bus, &ids, &names, amplitude, period, kp_max, kd_max);

    for &id in &ids {
        if let Some(mut a) = bus.actuator(id)
            && let Err(e) = a.disable_torque()
        {
            println!("  disable torque failed: {e}");
        }
    }
    result.map_err(Into::into)
}

fn sweep(
    bus: &mut SerialBus<P1>,
    ids: &[Id],
    names: &[&str],
    amplitude: Position,
    period: f32,
    kp_max: u8,
    kd_max: u8,
) -> Result<(), Error> {
    let mut centers = Vec::with_capacity(ids.len());
    for &id in ids {
        let mut a = bus.actuator(id).ok_or(SetupError::NotOnBus { id })?;
        centers.push(a.present_position()?);
        a.enable_torque()?;
    }

    let start = Instant::now();
    let mut gains = None;
    let mut next = start;
    while start.elapsed() < DURATION {
        let t = start.elapsed().as_secs_f32();
        let ramp = t / DURATION.as_secs_f32();
        let kp = (1.0 + ramp * f32::from(kp_max.saturating_sub(1))).round() as u8;
        let kd = (ramp * f32::from(kd_max)).round() as u8;
        let offset = Position::from_radians(amplitude.radians() * (TAU * t / period).sin());

        for (i, &id) in ids.iter().enumerate() {
            let mut a = bus.actuator(id).ok_or(SetupError::NotOnBus { id })?;
            if gains != Some((kp, kd)) {
                a.write_raw(P_GAIN_ADDR, &[kp])?;
                a.write_raw(D_GAIN_ADDR, &[kd])?;
            }
            a.set_goal_position(centers[i] + offset)?;
            let temp = a.present_temperature()?;
            let pos = a.present_position()?;
            println!(
                "t {t:5.2} s  {} id {:>3}  kp {kp:>3} kd {kd:>3}  goal {:7.2}°  pos {:7.2}°  {:.1} °C",
                names[i],
                id.get(),
                (centers[i] + offset).radians().to_degrees(),
                pos.radians().to_degrees(),
                temp.celsius(),
            );
        }
        gains = Some((kp, kd));

        next += CYCLE;
        if let Some(wait) = next.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
    }
    Ok(())
}
