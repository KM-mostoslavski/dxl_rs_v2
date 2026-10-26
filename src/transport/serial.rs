//! [`Transport`](super::Transport) over an OS serial port, via `serialport`.

extern crate std;

use core::time::Duration;
use std::io;

use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};

#[cfg(unix)]
type NativePort = serialport::TTYPort;
#[cfg(windows)]
type NativePort = serialport::COMPort;

/// How long a read may wait when no byte is pending. The bus only reads
/// after the port reports pending input, so this only bounds a race.
const POLL: Duration = Duration::from_millis(1);

/// A serial port (a U2D2 or any USB-TTL / RS-485 adapter) used as a
/// [`Transport`](crate::Transport).
///
/// ```no_run
/// use dxl_rs_v2::SerialTransport;
///
/// let mut port = SerialTransport::open("/dev/ttyUSB0", 1_000_000)?;
/// # Ok::<(), serialport::Error>(())
/// ```
///
/// # Latency
///
/// FTDI-based adapters, the U2D2 included, hold incoming bytes for up to
/// their latency timer before passing them on: 16 ms by default on Linux,
/// longer than the bus's default status timeout. Set it to 1 ms:
///
/// ```sh
/// echo 1 | sudo tee /sys/bus/usb-serial/devices/ttyUSB0/latency_timer
/// ```
///
/// This crate never changes OS driver settings.
#[derive(Debug)]
pub struct SerialTransport {
    port: NativePort,
}

impl SerialTransport {
    /// Opens `path` (e.g. `/dev/ttyUSB0`, `COM3`) at `baud_rate`: 8 data
    /// bits, no parity, 1 stop bit, no flow control.
    ///
    /// For other settings, open the port with [`serialport::new`] and
    /// convert it with [`From`].
    ///
    /// # Errors
    ///
    /// A typed [`serialport::Error`]: on Unix, a missing port is
    /// [`Io(NotFound)`](serialport::ErrorKind::Io), a port another program
    /// holds is [`NoDevice`](serialport::ErrorKind::NoDevice), and a baud
    /// rate the OS rejects is [`InvalidInput`](serialport::ErrorKind::InvalidInput).
    pub fn open(path: &str, baud_rate: u32) -> serialport::Result<Self> {
        serialport::new(path, baud_rate)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(POLL)
            .open_native()
            .map(Self::from)
    }

    /// The underlying OS port, for settings this type does not cover.
    #[cfg(unix)]
    pub fn port_mut(&mut self) -> &mut serialport::TTYPort {
        &mut self.port
    }

    /// The underlying OS port, for settings this type does not cover.
    #[cfg(windows)]
    pub fn port_mut(&mut self) -> &mut serialport::COMPort {
        &mut self.port
    }

    /// Gives the port back.
    #[cfg(unix)]
    pub fn into_inner(self) -> serialport::TTYPort {
        self.port
    }

    /// Gives the port back.
    #[cfg(windows)]
    pub fn into_inner(self) -> serialport::COMPort {
        self.port
    }
}

/// Wraps a port the caller already opened, keeping its settings. Its timeout
/// only matters when a read races an empty buffer, so keep it short.
#[cfg(unix)]
impl From<serialport::TTYPort> for SerialTransport {
    fn from(port: serialport::TTYPort) -> Self {
        Self { port }
    }
}

/// Wraps a port the caller already opened, keeping its settings. Its timeout
/// only matters when a read races an empty buffer, so keep it short.
#[cfg(windows)]
impl From<serialport::COMPort> for SerialTransport {
    fn from(port: serialport::COMPort) -> Self {
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

impl embedded_io::ReadReady for SerialTransport {
    fn read_ready(&mut self) -> io::Result<bool> {
        Ok(self.port.bytes_to_read().map_err(io::Error::from)? > 0)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::time::Instant;

    use embedded_io::{ErrorKind, Read, ReadReady, Write};
    use serialport::TTYPort;

    use super::*;

    fn pair() -> (SerialTransport, SerialTransport) {
        let (mut a, mut b) = TTYPort::pair().unwrap();
        a.set_timeout(POLL).unwrap();
        b.set_timeout(POLL).unwrap();
        (a.into(), b.into())
    }

    fn wait_ready(port: &mut SerialTransport) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(50) {
            if port.read_ready().unwrap() {
                return true;
            }
        }
        false
    }

    #[test]
    fn bytes_cross_the_link() {
        let (mut a, mut b) = pair();
        a.write_all(&[0xFF, 0xFF, 0xFD, 0x00]).unwrap();
        a.flush().unwrap();
        assert!(wait_ready(&mut b));
        let mut buf = [0; 4];
        b.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0xFF, 0xFF, 0xFD, 0x00]);
    }

    #[test]
    fn idle_line_is_not_ready() {
        let (_a, mut b) = pair();
        assert!(!b.read_ready().unwrap());
    }

    #[test]
    fn ready_after_peer_writes() {
        let (mut a, mut b) = pair();
        a.write_all(&[7]).unwrap();
        a.flush().unwrap();
        assert!(wait_ready(&mut b));
    }

    #[test]
    fn from_keeps_port_settings() {
        let (mut raw, _peer) = TTYPort::pair().unwrap();
        raw.set_timeout(Duration::from_millis(37)).unwrap();
        let mut port = SerialTransport::from(raw);
        assert_eq!(port.port_mut().timeout(), Duration::from_millis(37));
    }

    #[test]
    fn into_inner_returns_the_port() {
        let (a, _b) = pair();
        assert_eq!(a.into_inner().timeout(), POLL);
    }

    #[test]
    fn open_missing_port_is_not_found() {
        let err = SerialTransport::open("/dev/does-not-exist", 1_000_000).unwrap_err();
        assert_eq!(
            err.kind(),
            serialport::ErrorKind::Io(std::io::ErrorKind::NotFound)
        );
    }

    #[test]
    fn idle_read_times_out_fast() {
        let (_a, mut b) = pair();
        let start = Instant::now();
        let err = b.read(&mut [0; 1]).unwrap_err();
        assert!(start.elapsed() < Duration::from_millis(2));
        assert_eq!(embedded_io::Error::kind(&err), ErrorKind::TimedOut);
    }

    #[test]
    fn read_ready_is_fast() {
        let (_a, mut b) = pair();
        for _ in 0..100 {
            let start = Instant::now();
            b.read_ready().unwrap();
            assert!(start.elapsed() < Duration::from_millis(1));
        }
    }

    #[test]
    fn closed_peer_is_an_error_not_a_hang() {
        let (a, mut b) = pair();
        drop(a);
        let start = Instant::now();
        let outcome = match b.read_ready() {
            Err(_) => None,
            Ok(_) => Some(b.read(&mut [0; 1])),
        };
        assert!(start.elapsed() < Duration::from_millis(50));
        if let Some(read) = outcome {
            assert!(matches!(read, Err(_) | Ok(0)));
        }
    }
}
