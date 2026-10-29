//! [`Transport`](super::Transport) over an OS serial port, via `serialport`.

extern crate std;

use core::time::Duration;
use std::io;

use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};

#[cfg(unix)]
type NativePort = serialport::TTYPort;
#[cfg(windows)]
type NativePort = serialport::COMPort;

/// The port timeout `open` sets. It bounds a read that races an empty
/// buffer, the wait for the OS to accept written bytes (the whole write on
/// Windows), and on Unix the retry window of a `flush` interrupted by a
/// signal. Reply timeouts are the bus's job: the bus reads only after
/// `read_ready`, so this never delays a reply.
const PORT_TIMEOUT: Duration = Duration::from_millis(50);

/// A serial port (a U2D2 or any USB-TTL / RS-485 adapter) used as a
/// [`Transport`](crate::Transport).
///
/// ```no_run
/// use dxl_rs_v2::SerialTransport;
///
/// let mut port = SerialTransport::open("/dev/ttyUSB0", 1_000_000)?;
/// # Ok::<(), dxl_rs_v2::serialport::Error>(())
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
    /// bits, no parity, 1 stop bit, no flow control, a 50 ms port timeout.
    ///
    /// For other settings, open the port with [`serialport::new`] and
    /// convert it with [`From`].
    ///
    /// # Errors
    ///
    /// A typed [`serialport::Error`]. On Unix, a missing port is
    /// [`Io(NotFound)`](serialport::ErrorKind::Io) and a port another program
    /// holds is [`NoDevice`](serialport::ErrorKind::NoDevice). Linux drivers
    /// usually round an unsupported baud rate rather than reject it.
    pub fn open(path: &str, baud_rate: u32) -> serialport::Result<Self> {
        serialport::new(path, baud_rate)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(PORT_TIMEOUT)
            .open_native()
            .map(Self::from)
    }

    /// The underlying OS port (`serialport::TTYPort` on Unix,
    /// `serialport::COMPort` on Windows), for settings this type does not
    /// cover.
    pub fn port_mut(&mut self) -> &mut NativePort {
        &mut self.port
    }

    /// Gives the port back.
    pub fn into_inner(self) -> NativePort {
        self.port
    }
}

/// Wraps a port the caller already opened, keeping its settings. Its timeout
/// bounds a read that races an empty buffer, the wait for the OS to accept
/// written bytes and, on Unix, the retry window of an interrupted flush, so a
/// few tens of milliseconds is a good value.
impl From<NativePort> for SerialTransport {
    fn from(port: NativePort) -> Self {
        Self { port }
    }
}

/// Maps a write that accepted nothing to [`io::ErrorKind::TimedOut`]: a
/// Windows port reports a write timeout as `Ok(0)`, which `embedded-io`
/// forbids for a non-empty buffer (its `write_all` panics on it), and the
/// transport contract reports timeouts as `TimedOut`.
fn accepted(written: usize, buf: &[u8]) -> io::Result<usize> {
    if written == 0 && !buf.is_empty() {
        Err(io::ErrorKind::TimedOut.into())
    } else {
        Ok(written)
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
        accepted(io::Write::write(&mut self.port, buf)?, buf)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepting_nothing_is_a_timeout() {
        assert_eq!(
            accepted(0, &[1]).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        assert_eq!(accepted(0, &[]).unwrap(), 0);
        assert_eq!(accepted(2, &[1, 2, 3]).unwrap(), 2);
    }
}

#[cfg(all(test, unix))]
mod pty_tests {
    use std::time::Instant;

    use embedded_io::{ErrorKind, Read, ReadReady, Write};
    use serialport::TTYPort;

    use super::*;

    /// Generous bounds: they catch a hang, not a slow CI runner.
    const HANG: Duration = Duration::from_millis(500);

    fn pair() -> (SerialTransport, SerialTransport) {
        let (mut a, mut b) = TTYPort::pair().unwrap();
        a.set_timeout(PORT_TIMEOUT).unwrap();
        b.set_timeout(PORT_TIMEOUT).unwrap();
        (a.into(), b.into())
    }

    fn wait_ready(port: &mut SerialTransport) -> bool {
        let start = Instant::now();
        while start.elapsed() < HANG {
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
        assert_eq!(a.into_inner().timeout(), PORT_TIMEOUT);
    }

    #[test]
    #[cfg_attr(
        target_os = "macos",
        ignore = "serialport sets the baud with the IOSSIOSPEED ioctl, which macOS ptys reject (ENOTTY)"
    )]
    fn open_configures_8n1_without_flow_control() {
        let (_master, slave) = TTYPort::pair().unwrap();
        let path = slave.name().unwrap();
        drop(slave);
        let mut port = SerialTransport::open(&path, 57_600).unwrap();
        let port = port.port_mut();
        assert_eq!(port.data_bits().unwrap(), DataBits::Eight);
        assert_eq!(port.parity().unwrap(), Parity::None);
        assert_eq!(port.stop_bits().unwrap(), StopBits::One);
        assert_eq!(port.flow_control().unwrap(), FlowControl::None);
        assert_eq!(port.timeout(), PORT_TIMEOUT);
    }

    #[test]
    #[cfg_attr(
        target_os = "macos",
        ignore = "serialport sets the baud with the IOSSIOSPEED ioctl, which macOS ptys reject (ENOTTY)"
    )]
    fn open_busy_port_is_no_device() {
        let (_master, slave) = TTYPort::pair().unwrap();
        let path = slave.name().unwrap();
        drop(slave);
        let _held = SerialTransport::open(&path, 57_600).unwrap();
        let err = SerialTransport::open(&path, 57_600).unwrap_err();
        assert_eq!(err.kind(), serialport::ErrorKind::NoDevice);
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
    fn idle_read_times_out_within_the_port_timeout() {
        let (_a, mut b) = pair();
        let start = Instant::now();
        let err = b.read(&mut [0; 1]).unwrap_err();
        assert!(start.elapsed() < PORT_TIMEOUT + HANG);
        assert_eq!(embedded_io::Error::kind(&err), ErrorKind::TimedOut);
    }

    #[test]
    fn read_ready_never_waits() {
        let (_a, mut b) = pair();
        let start = Instant::now();
        for _ in 0..100 {
            b.read_ready().unwrap();
        }
        assert!(start.elapsed() < HANG);
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
        assert!(start.elapsed() < PORT_TIMEOUT + HANG);
        if let Some(read) = outcome {
            assert!(matches!(read, Err(_) | Ok(0)));
        }
    }
}
