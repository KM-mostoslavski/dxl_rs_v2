//! Temperature, voltage, load and speed of every actuator on a Protocol 1 bus, read with the
//! typed FR-9 fields. Torque stays off: nothing moves and nothing is written.
//!
//! ```sh
//! DXL_PORT=/dev/serial/by-id/usb-FTDI_FT232R_USB_UART_AL02L1KL-if00-port0 \
//!   cargo run --example rx28_telemetry
//! ```
//!
//! - `DXL_PORT` (required) and `DXL_BAUD` (default 1000000).
//! - `DXL_SAMPLES` lines to print (default 10), `DXL_PERIOD_MS` between them (default 500).
//!
//! Each field is one Read per actuator on Protocol 1 (Protocol 2 would send one Sync Read).
//! In wheel mode the RX-28 and AX-12A give their speed in % of output, not as a velocity
//! (e-Manual), so the speed of an actuator in wheel mode is printed raw.

use std::time::Duration;

use dxl_rs_v2::field::{PresentLoad, PresentTemperature, PresentVelocity, PresentVoltage, Raw};
use dxl_rs_v2::{Actuator, Id, LimitMode, Load, P1, SerialBus, Temperature, Velocity, Voltage};

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name).map_or(default, |v| v.parse().expect(name))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("DXL_PORT").map_err(|_| "set DXL_PORT to the serial adapter")?;
    let samples = env_u32("DXL_SAMPLES", 10);
    let period = Duration::from_millis(env_u32("DXL_PERIOD_MS", 500).into());

    let mut bus = SerialBus::<P1>::open(&path, env_u32("DXL_BAUD", 1_000_000))?;
    let held = bus.autoscan()?;
    let ids: Vec<Id> = held.iter().map(Actuator::id).collect();
    let names: Vec<&str> = held.iter().map(Actuator::name).collect();
    let in_wheel = |wheel: bool| -> Vec<Id> {
        held.iter()
            .filter(|a| (a.limit_mode() == LimitMode::Wheel) == wheel)
            .map(Actuator::id)
            .collect()
    };
    let (wheel, joint) = (in_wheel(true), in_wheel(false));
    println!(
        "found {} actuators, {} in wheel mode",
        ids.len(),
        wheel.len()
    );

    let n = ids.len();
    let mut celsius = vec![Temperature::default(); n];
    let mut volts = vec![Voltage::default(); n];
    let mut load = vec![Load::default(); n];
    let mut velocity = vec![Velocity::ZERO; joint.len()];
    let mut raw_speed = vec![0; wheel.len()];
    for _ in 0..samples {
        bus.read_field(PresentTemperature, ids.iter().copied(), &mut celsius)?;
        bus.read_field(PresentVoltage, ids.iter().copied(), &mut volts)?;
        bus.read_field(PresentLoad, ids.iter().copied(), &mut load)?;
        bus.read_field(PresentVelocity, joint.iter().copied(), &mut velocity)?;
        bus.read_field(Raw(PresentVelocity), wheel.iter().copied(), &mut raw_speed)?;
        for (i, id) in ids.iter().enumerate() {
            let speed = match joint.iter().position(|j| j == id) {
                Some(k) => format!("{:+.3} rad/s", velocity[k].rad_per_s()),
                None => {
                    let k = wheel.iter().position(|w| w == id).unwrap_or_default();
                    format!("raw {}", raw_speed[k])
                }
            };
            println!(
                "  {} id {:>3}: {:>5.1} °C  {:>5.1} V  load {:+.3}  speed {speed}",
                names[i],
                id.get(),
                celsius[i].celsius(),
                volts[i].volts(),
                load[i].ratio(),
            );
        }
        std::thread::sleep(period);
    }
    Ok(())
}
