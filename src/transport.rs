//! Raw byte I/O between the protocol layer and the wire.
//!
//! A transport moves bytes and answers one question without blocking: "is
//! input waiting?". It knows nothing about Dynamixel packets and keeps no
//! time. Timeouts belong to the bus, which bounds every wait with its own
//! clock.
//!
//! # Bring your own UART
//!
//! Any type implementing [`embedded_io::Read`], [`embedded_io::Write`] and
//! [`embedded_io::ReadReady`] is a [`Transport`], with no wrapper from this
//! crate. Most embedded HALs implement these traits for their UARTs, so a
//! microcontroller UART plugs straight into the bus in a `no_std`, heap-free
//! build.
//!
//! Ready-made transports:
//! - `SerialTransport` (feature `std`): an OS serial port, e.g. a U2D2.
//! - `MockTransport` (feature `mock`): a scripted line for tests without
//!   hardware.
//!
//! # Half-duplex
//!
//! Dynamixel TTL and RS-485 buses are half-duplex. The U2D2 and RS-485
//! transceivers with automatic direction control switch direction in
//! hardware. If your hardware has a direction pin (DE/RE), drive it inside
//! your transport: switch to transmit in `write`, and back to receive before
//! `flush` returns.
//!
//! The transport must not echo sent bytes back on receive. Single-wire UARTs
//! without automatic direction control are not supported.

#[cfg(any(test, feature = "mock"))]
#[cfg_attr(
    all(test, not(feature = "mock")),
    expect(dead_code, reason = "public only with the mock feature")
)]
mod mock;
#[cfg(feature = "std")]
mod serial;

#[cfg(feature = "mock")]
pub use mock::{Mismatch, MockTime, MockTransport};
#[cfg(feature = "std")]
pub use serial::SerialTransport;

use embedded_io::{Read, ReadReady, Write};

/// A byte channel to a Dynamixel bus: blocking read and write, plus a
/// non-blocking check for pending input.
///
/// Implemented for every type that has the three `embedded-io` traits; there
/// is nothing to implement by hand. The compiler cannot check the rules
/// below, so an implementor must follow them:
///
/// - [`read_ready`](ReadReady::read_ready) never blocks.
/// - [`read`](Read::read) called after `read_ready()` returned `Ok(true)`
///   returns at least one byte without blocking. `Ok(0)` means end of
///   stream: the port is gone.
/// - [`flush`](Write::flush) returns only after every written byte has left
///   the implementor's buffers. Direction pins (DE/RE), if any, switch back
///   to receive before `flush` returns.
/// - The transport does not echo sent bytes back on receive.
/// - No internal retry. A timeout, if the transport has one, is reported as
///   [`ErrorKind::TimedOut`](embedded_io::ErrorKind::TimedOut).
///
/// # Example
///
/// A minimal UART driver becomes a transport by implementing the three
/// traits:
///
/// ```
/// use dxl_rs_v2::Transport;
/// use embedded_io::{ErrorKind, ErrorType, Read, ReadReady, Write};
///
/// struct Uart;
///
/// impl ErrorType for Uart {
///     type Error = ErrorKind;
/// }
/// impl Read for Uart {
///     fn read(&mut self, _buf: &mut [u8]) -> Result<usize, ErrorKind> {
///         Ok(0)
///     }
/// }
/// impl Write for Uart {
///     fn write(&mut self, buf: &[u8]) -> Result<usize, ErrorKind> {
///         Ok(buf.len())
///     }
///     fn flush(&mut self) -> Result<(), ErrorKind> {
///         Ok(())
///     }
/// }
/// impl ReadReady for Uart {
///     fn read_ready(&mut self) -> Result<bool, ErrorKind> {
///         Ok(false)
///     }
/// }
///
/// fn needs_transport<T: Transport>(_: &mut T) {}
/// needs_transport(&mut Uart);
/// ```
///
/// Without [`ReadReady`] the bus could not bound its waits, so such a type is
/// rejected at compile time:
///
/// ```compile_fail
/// use dxl_rs_v2::Transport;
/// use embedded_io::{ErrorKind, ErrorType, Read, Write};
///
/// struct Uart;
///
/// impl ErrorType for Uart {
///     type Error = ErrorKind;
/// }
/// impl Read for Uart {
///     fn read(&mut self, _buf: &mut [u8]) -> Result<usize, ErrorKind> {
///         Ok(0)
///     }
/// }
/// impl Write for Uart {
///     fn write(&mut self, buf: &[u8]) -> Result<usize, ErrorKind> {
///         Ok(buf.len())
///     }
///     fn flush(&mut self) -> Result<(), ErrorKind> {
///         Ok(())
///     }
/// }
///
/// fn needs_transport<T: Transport>(_: &mut T) {}
/// needs_transport(&mut Uart);
/// ```
pub trait Transport: Read + Write + ReadReady {}

impl<T: Read + Write + ReadReady> Transport for T {}

#[cfg(test)]
mod tests {
    use embedded_io::{ErrorKind, ErrorType};

    use super::*;

    struct Loop;

    impl ErrorType for Loop {
        type Error = ErrorKind;
    }

    impl Read for Loop {
        fn read(&mut self, _buf: &mut [u8]) -> Result<usize, ErrorKind> {
            Ok(0)
        }
    }

    impl Write for Loop {
        fn write(&mut self, buf: &[u8]) -> Result<usize, ErrorKind> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> Result<(), ErrorKind> {
            Ok(())
        }
    }

    impl ReadReady for Loop {
        fn read_ready(&mut self) -> Result<bool, ErrorKind> {
            Ok(false)
        }
    }

    fn takes<T: Transport>(transport: &mut T) -> Result<bool, T::Error> {
        transport.write_all(&[1])?;
        transport.flush()?;
        transport.read_ready()
    }

    #[test]
    fn any_read_write_read_ready_type_is_a_transport() {
        assert_eq!(takes(&mut Loop), Ok(false));
    }
}
