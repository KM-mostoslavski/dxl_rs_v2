//! Raw byte I/O between the protocol layer and the wire.

#[cfg(test)]
pub(crate) mod mock;
#[cfg(feature = "std")]
mod serial;

#[cfg(feature = "std")]
pub use serial::SerialTransport;

use core::time::Duration;

use embedded_io::{ErrorKind, ReadExactError};

/// A byte link to a Dynamixel bus: blocking `embedded-io` I/O plus the two
/// controls the protocol layer needs around every transaction.
///
/// [`SerialTransport`] implements it for a serial port under the `std`
/// feature. On embedded targets, implement it for your UART.
///
/// # Half-duplex direction control
///
/// Dynamixel TTL and RS-485 buses are half-duplex. If your hardware has a
/// direction pin (DE/RE), drive it inside your implementation: switch to
/// transmit in [`write`](embedded_io::Write::write) and back to receive at
/// the end of [`flush`](embedded_io::Write::flush), which must block until
/// the last byte is on the wire. USB adapters such as the U2D2 do this in
/// hardware.
pub trait Transport: embedded_io::Read + embedded_io::Write {
    /// Discards every byte waiting in the receive buffer.
    ///
    /// Called before every transmission, so a late or stray reply can't be
    /// mistaken for the answer to the next instruction.
    fn clear_rx(&mut self) -> Result<(), Self::Error>;

    /// Sets how long a [`read`](embedded_io::Read::read) may block before it
    /// fails with [`ErrorKind::TimedOut`].
    ///
    /// The bus owns the timeout values and calls this before each
    /// transaction; the transport only enforces them, since it is the layer
    /// with access to a clock.
    fn set_timeout(&mut self, timeout: Duration) -> Result<(), Self::Error>;
}

/// An I/O failure on the transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TransportError {
    /// No reply, or only part of one, arrived before the timeout.
    #[error("timed out waiting for a reply")]
    Timeout,

    /// The transport closed while a packet was still expected.
    #[error("transport closed mid-packet")]
    UnexpectedEof,

    /// Any other I/O error, by kind.
    #[error("transport I/O error: {0}")]
    Io(ErrorKind),
}

impl TransportError {
    /// Converts a transport's own error, keeping only its kind.
    ///
    /// [`ErrorKind::TimedOut`] becomes [`TransportError::Timeout`].
    pub fn from_io(err: impl embedded_io::Error) -> Self {
        match err.kind() {
            ErrorKind::TimedOut => Self::Timeout,
            kind => Self::Io(kind),
        }
    }

    /// Converts the error of [`read_exact`](embedded_io::Read::read_exact).
    pub fn from_read_exact<E: embedded_io::Error>(err: ReadExactError<E>) -> Self {
        match err {
            ReadExactError::UnexpectedEof => Self::UnexpectedEof,
            ReadExactError::Other(err) => Self::from_io(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn timed_out_becomes_timeout() {
        assert_eq!(
            TransportError::from_io(ErrorKind::TimedOut),
            TransportError::Timeout
        );
    }

    #[test]
    fn other_kinds_are_kept() {
        for kind in [
            ErrorKind::Other,
            ErrorKind::Interrupted,
            ErrorKind::BrokenPipe,
        ] {
            assert_eq!(TransportError::from_io(kind), TransportError::Io(kind));
        }
    }

    #[test]
    fn read_exact_errors_convert() {
        assert_eq!(
            TransportError::from_read_exact::<ErrorKind>(ReadExactError::UnexpectedEof),
            TransportError::UnexpectedEof
        );
        assert_eq!(
            TransportError::from_read_exact(ReadExactError::Other(ErrorKind::TimedOut)),
            TransportError::Timeout
        );
    }

    #[test]
    fn wraps_into_crate_error() {
        let err: Error = TransportError::Timeout.into();
        assert_eq!(err, Error::Transport(TransportError::Timeout));
        assert_eq!(std::format!("{err}"), "timed out waiting for a reply");
        assert_eq!(
            std::format!("{}", TransportError::Io(ErrorKind::Other)),
            std::format!("transport I/O error: {}", ErrorKind::Other)
        );
    }
}
