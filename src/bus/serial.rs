//! A bus on a desktop serial port (`std`).

use crate::protocol::{Protocol, StdClock};
use crate::transport::SerialTransport;

use super::{Bus, Config};

/// A [`Bus`] on a serial port, timed by [`StdClock`].
///
/// ```no_run
/// use dxl_rs_v2::{Error, P2, SerialBus};
///
/// let mut bus = SerialBus::<P2>::open("/dev/ttyUSB0", 1_000_000)?;
/// for actuator in bus.autoscan()? {
///     println!("{actuator:?}");
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub type SerialBus<P, const N: usize = 32> = Bus<P, SerialTransport, StdClock, N>;

impl<P: Protocol, const N: usize> Bus<P, SerialTransport, StdClock, N> {
    /// Opens the serial port at `path` (e.g. `/dev/ttyUSB0`, `COM3`) at
    /// `baud` and builds a bus on it with [`StdClock`] and [`Config::new`].
    /// Nothing is sent: call [`autoscan`](Bus::autoscan) or
    /// [`declare`](Bus::declare) next.
    ///
    /// For another config, build the bus with [`Bus::new`] from a
    /// [`SerialTransport`], a [`StdClock`] and the config.
    ///
    /// # Errors
    ///
    /// The [`serialport::Error`] of [`SerialTransport::open`].
    pub fn open(path: &str, baud: u32) -> Result<Self, serialport::Error> {
        let transport = SerialTransport::open(path, baud)?;
        Ok(Self::new(transport, StdClock::default(), Config::new()))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use serialport::{SerialPort, TTYPort};

    use super::*;
    use crate::protocol::P2;

    #[test]
    #[cfg_attr(
        target_os = "macos",
        ignore = "serialport sets the baud with the IOSSIOSPEED ioctl, which macOS ptys reject (ENOTTY)"
    )]
    fn open_sends_nothing() {
        let (peer, port) = TTYPort::pair().unwrap();
        let path = port.name().unwrap();
        drop(port);
        let bus = SerialBus::<P2>::open(&path, 1_000_000).unwrap();
        assert_eq!(bus.config(), Config::new());
        assert!(bus.actuators().is_empty());
        assert_eq!(peer.bytes_to_read().unwrap(), 0);
    }

    #[test]
    fn open_missing_port_fails() {
        let err = SerialBus::<P2>::open("/dev/does-not-exist", 1_000_000)
            .err()
            .unwrap();
        assert_eq!(
            err.kind(),
            serialport::ErrorKind::Io(std::io::ErrorKind::NotFound)
        );
    }
}
