//! One-off: show, and with `DXL_MIN` set, write Min Voltage Limit (12) on every AX-12A.
//!
//! Raw Protocol 1 over `serialport`, bypassing `Bus`: Setup refuses an actuator that reports
//! an Input Voltage error, so the crate cannot reach the register that causes it. The error
//! Setup returns says which side of the limits it is, e.g.
//! `Fault(Voltage(BelowMin { id: 5, measured: 9.7 V, min: 10.5 V, max: 13.0 V }))`, which
//! tells whether to lower `DXL_MIN` here or to fix the supply.
//!
//! `DXL_MIN` is in 0.1 V: 90 lowers the limit to 9.0 V (the bottom of the AX-12A operating
//! range), 105 puts it back to 10.5 V. Lower it for one run and restore it right after:
//!
//! ```sh
//! DXL_PORT=/dev/ttyUSB0 cargo run --example ax12_voltage_fix                # report only
//! export DXL_PORT=/dev/ttyUSB0
//! DXL_MIN=90 cargo run --example ax12_voltage_fix \
//!   && cargo run --example ax12_sine --features raw-registers; \
//!   DXL_MIN=105 cargo run --example ax12_voltage_fix
//! ```

use std::time::Duration;

use dxl_rs_v2::serialport::{self, SerialPort};

const AX12A: u16 = 12;
const MIN_VOLTAGE_LIMIT: u8 = 12;
/// Min Voltage Limit range accepted by the AX-12A, in 0.1 V.
const MIN_RANGE: std::ops::RangeInclusive<u8> = 50..=160;

fn checksum(body: &[u8]) -> u8 {
    !body.iter().fold(0u8, |acc, b| acc.wrapping_add(*b))
}

fn send(port: &mut dyn SerialPort, id: u8, instr: u8, params: &[u8]) -> std::io::Result<()> {
    let mut body = vec![id, params.len() as u8 + 2, instr];
    body.extend_from_slice(params);
    let mut packet = vec![0xFF, 0xFF];
    packet.extend_from_slice(&body);
    packet.push(checksum(&body));
    port.clear(serialport::ClearBuffer::Input)?;
    port.write_all(&packet)
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("DXL_PORT").map_err(|_| "set DXL_PORT to the serial adapter")?;
    let baud = std::env::var("DXL_BAUD").map_or(1_000_000, |v| v.parse().expect("DXL_BAUD"));
    let target: Option<u8> = std::env::var("DXL_MIN")
        .ok()
        .map(|v| v.parse().expect("DXL_MIN"));
    if target.is_some_and(|t| !MIN_RANGE.contains(&t)) {
        return Err("DXL_MIN outside 50..=160 (5.0-16.0 V)".into());
    }
    let mut port = serialport::new(&path, baud)
        .timeout(Duration::from_millis(20))
        .open()?;
    let port = port.as_mut();

    for id in 0..=252u8 {
        let Some((_, model)) = read(port, id, 0, 2) else {
            continue;
        };
        let model = u16::from_le_bytes([model[0], model[1]]);
        if model != AX12A {
            println!("id {id:>3}: model {model}, skipped");
            continue;
        }
        let limits = read(port, id, 12, 2).ok_or("limits read failed")?;
        let present = read(port, id, 42, 1).ok_or("voltage read failed")?;
        let (lo, hi, now) = (limits.1[0], limits.1[1], present.1[0]);
        println!(
            "id {id:>3}: AX-12A, limits {:.1}-{:.1} V, present {:.1} V, error 0x{:02X}",
            f32::from(lo) / 10.0,
            f32::from(hi) / 10.0,
            f32::from(now) / 10.0,
            present.0,
        );
        let Some(target) = target else { continue };
        if lo == target {
            continue;
        }
        if target >= hi {
            return Err(format!("id {id}: DXL_MIN must stay below the max limit").into());
        }
        send(port, id, 0x03, &[MIN_VOLTAGE_LIMIT, target])?;
        receive(port, id).ok_or("write not acknowledged")?;
        std::thread::sleep(Duration::from_millis(10));
        let after = read(port, id, MIN_VOLTAGE_LIMIT, 1).ok_or("verify read failed")?;
        println!(
            "         min limit now {:.1} V",
            f32::from(after.1[0]) / 10.0
        );
    }
    if target.is_none() {
        println!("report only; DXL_MIN=<0.1 V> writes address 12 where it differs");
    }
    Ok(())
}
