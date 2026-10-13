//! [`Transport`] over a host serial port, via the `serialport` crate.

use core::time::Duration;
use std::boxed::Box;
use std::io;

use serialport::{ClearBuffer, SerialPort};

use super::Transport;

/// A serial port (e.g. a U2D2 or any USB–TTL/RS-485 adapter) used as a
/// [`Transport`].
///
/// # Latency
///
/// FTDI-based adapters, the U2D2 included, buffer incoming bytes for up to
/// their latency timer before passing them on: 16 ms by default on Linux,
/// which dominates every round-trip. Set it to 1 ms, e.g.
/// `echo 1 > /sys/bus/usb-serial/devices/ttyUSB0/latency_timer`.
pub struct SerialTransport {
    port: Box<dyn SerialPort>,
}

impl SerialTransport {
    /// Opens `path` (e.g. `/dev/ttyUSB0`, `COM3`) at `baud_rate`, 8N1.
    ///
    /// For other port settings, build the port with [`serialport::new`] and
    /// convert it with [`From`].
    pub fn open(path: &str, baud_rate: u32) -> serialport::Result<Self> {
        serialport::new(path, baud_rate).open().map(Self::from)
    }

    /// Returns the underlying port.
    pub fn into_inner(self) -> Box<dyn SerialPort> {
        self.port
    }
}

impl From<Box<dyn SerialPort>> for SerialTransport {
    /// Wraps an already opened port.
    fn from(port: Box<dyn SerialPort>) -> Self {
        Self { port }
    }
}

impl embedded_io::ErrorType for SerialTransport {
    type Error = io::Error;
}

impl embedded_io::Read for SerialTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        io::Read::read(&mut self.port, buf)
    }
}

impl embedded_io::Write for SerialTransport {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::Write::write(&mut self.port, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.port)
    }
}

impl Transport for SerialTransport {
    fn clear_rx(&mut self) -> io::Result<()> {
        Ok(self.port.clear(ClearBuffer::Input)?)
    }

    fn set_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        Ok(self.port.set_timeout(timeout)?)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use embedded_io::{Read, Write};
    use serialport::TTYPort;

    use super::*;
    use crate::TransportError;

    const SHORT: Duration = Duration::from_millis(50);

    fn pair() -> (SerialTransport, SerialTransport) {
        let (a, b) = TTYPort::pair().expect("pseudo-terminal pair");
        let mut a = SerialTransport::from(Box::new(a) as Box<dyn SerialPort>);
        let mut b = SerialTransport::from(Box::new(b) as Box<dyn SerialPort>);
        a.set_timeout(SHORT).unwrap();
        b.set_timeout(SHORT).unwrap();
        (a, b)
    }

    #[test]
    fn bytes_cross_the_link() {
        let (mut a, mut b) = pair();
        a.write_all(&[0xFF, 0xFF, 0xFD, 0x00]).unwrap();
        a.flush().unwrap();

        let mut buf = [0; 4];
        b.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0xFF, 0xFF, 0xFD, 0x00]);
    }

    #[test]
    fn empty_link_times_out() {
        let (_a, mut b) = pair();
        let err = b.read(&mut [0; 1]).unwrap_err();
        assert_eq!(TransportError::from_io(err), TransportError::Timeout);
    }

    #[test]
    fn clear_rx_drops_pending_bytes() {
        let (mut a, mut b) = pair();
        a.write_all(&[1, 2, 3]).unwrap();
        a.flush().unwrap();
        std::thread::sleep(SHORT);

        b.clear_rx().unwrap();
        a.write_all(&[4]).unwrap();
        a.flush().unwrap();

        let mut buf = [0; 1];
        b.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [4]);
    }

    #[test]
    fn into_inner_returns_the_port() {
        let (a, _b) = pair();
        assert_eq!(a.into_inner().timeout(), SHORT);
    }

    #[test]
    fn open_reports_missing_port() {
        assert!(SerialTransport::open("/dev/does-not-exist", 1_000_000).is_err());
    }
}
