//! Scripted [`Transport`] for unit tests.

use core::time::Duration;
use std::collections::VecDeque;
use std::fmt;
use std::vec::Vec;

use embedded_io::ErrorKind;

use super::Transport;

/// A [`Transport`] that plays back a script of request/reply exchanges.
///
/// Each reply only becomes readable once its expected request has been
/// written in full, so [`Transport::clear_rx`] before a write can't discard
/// it. Bytes written that don't match the script panic.
#[derive(Debug, Default)]
pub(crate) struct MockTransport {
    exchanges: VecDeque<(Vec<u8>, Vec<u8>)>,
    tx: Vec<u8>,
    rx: VecDeque<u8>,
    fail_next: Option<ErrorKind>,
    clear_count: usize,
    timeout: Option<Duration>,
}

/// Error type of [`MockTransport`]: just a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MockError(pub(crate) ErrorKind);

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mock error: {}", self.0)
    }
}

impl core::error::Error for MockError {}

impl embedded_io::Error for MockError {
    fn kind(&self) -> ErrorKind {
        self.0
    }
}

impl MockTransport {
    /// Creates a transport with an empty script.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Expects `request` to be written next, then makes `reply` readable.
    pub(crate) fn exchange(mut self, request: &[u8], reply: &[u8]) -> Self {
        self.exchanges.push_back((request.to_vec(), reply.to_vec()));
        self
    }

    /// Makes `bytes` readable right away, as noise left on the line.
    pub(crate) fn stray_rx(mut self, bytes: &[u8]) -> Self {
        self.rx.extend(bytes);
        self
    }

    /// Makes the next transport call fail with `kind`.
    pub(crate) fn fail_next(&mut self, kind: ErrorKind) {
        self.fail_next = Some(kind);
    }

    /// Number of [`Transport::clear_rx`] calls so far.
    pub(crate) fn clear_count(&self) -> usize {
        self.clear_count
    }

    /// Last value passed to [`Transport::set_timeout`].
    pub(crate) fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Panics unless every scripted exchange was written and every reply
    /// byte read.
    pub(crate) fn assert_done(&self) {
        assert!(
            self.exchanges.is_empty(),
            "unsent requests: {:02X?}",
            self.exchanges
        );
        assert!(self.tx.is_empty(), "partial request: {:02X?}", self.tx);
        assert!(self.rx.is_empty(), "unread reply bytes: {:02X?}", self.rx);
    }

    fn check_fail(&mut self) -> Result<(), MockError> {
        self.fail_next
            .take()
            .map_or(Ok(()), |kind| Err(MockError(kind)))
    }
}

impl embedded_io::ErrorType for MockTransport {
    type Error = MockError;
}

impl embedded_io::Read for MockTransport {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, MockError> {
        self.check_fail()?;
        if buf.is_empty() {
            return Ok(0);
        }
        if self.rx.is_empty() {
            return Err(MockError(ErrorKind::TimedOut));
        }
        let n = buf.len().min(self.rx.len());
        for (slot, byte) in buf.iter_mut().zip(self.rx.drain(..n)) {
            *slot = byte;
        }
        Ok(n)
    }
}

impl embedded_io::Write for MockTransport {
    fn write(&mut self, buf: &[u8]) -> Result<usize, MockError> {
        self.check_fail()?;
        self.tx.extend_from_slice(buf);
        while let Some((request, _)) = self.exchanges.front() {
            let n = request.len().min(self.tx.len());
            assert_eq!(self.tx[..n], request[..n], "unexpected bytes written");
            if n < request.len() {
                break;
            }
            self.tx.drain(..n);
            let (_, reply) = self.exchanges.pop_front().unwrap();
            self.rx.extend(reply);
        }
        assert!(
            self.tx.is_empty() || !self.exchanges.is_empty(),
            "write with no exchange scripted: {:02X?}",
            self.tx
        );
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), MockError> {
        self.check_fail()
    }
}

impl Transport for MockTransport {
    fn clear_rx(&mut self) -> Result<(), MockError> {
        self.check_fail()?;
        self.clear_count += 1;
        self.rx.clear();
        Ok(())
    }

    fn set_timeout(&mut self, timeout: Duration) -> Result<(), MockError> {
        self.check_fail()?;
        self.timeout = Some(timeout);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use embedded_io::{Read, Write};

    use super::*;

    #[test]
    fn reply_follows_its_request() {
        let mut t = MockTransport::new().exchange(&[1, 2], &[3, 4]);
        t.clear_rx().unwrap();
        t.write_all(&[1]).unwrap();
        assert_eq!(t.read(&mut [0; 2]), Err(MockError(ErrorKind::TimedOut)));
        t.write_all(&[2]).unwrap();
        t.flush().unwrap();

        let mut buf = [0; 2];
        t.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [3, 4]);
        t.assert_done();
    }

    #[test]
    fn clear_rx_drops_stray_bytes() {
        let mut t = MockTransport::new().stray_rx(&[0xAA]);
        t.clear_rx().unwrap();
        assert_eq!(t.clear_count(), 1);
        assert_eq!(t.read(&mut [0; 1]), Err(MockError(ErrorKind::TimedOut)));
        assert_eq!(t.read(&mut []), Ok(0));
        t.assert_done();
    }

    #[test]
    fn records_timeout() {
        let mut t = MockTransport::new();
        assert_eq!(t.timeout(), None);
        t.set_timeout(Duration::from_millis(3)).unwrap();
        assert_eq!(t.timeout(), Some(Duration::from_millis(3)));
    }

    #[test]
    fn fail_next_fails_once() {
        let mut t = MockTransport::new();
        t.fail_next(ErrorKind::BrokenPipe);
        let err = t.set_timeout(Duration::ZERO).unwrap_err();
        assert_eq!(embedded_io::Error::kind(&err), ErrorKind::BrokenPipe);
        assert_eq!(
            std::format!("{err}"),
            std::format!("mock error: {}", ErrorKind::BrokenPipe)
        );
        t.set_timeout(Duration::ZERO).unwrap();
    }

    #[test]
    #[should_panic(expected = "unexpected bytes written")]
    fn wrong_request_panics() {
        let mut t = MockTransport::new().exchange(&[1], &[]);
        t.write_all(&[2]).unwrap();
    }

    #[test]
    #[should_panic(expected = "write with no exchange scripted")]
    fn unscripted_write_panics() {
        MockTransport::new().write_all(&[1]).unwrap();
    }

    #[test]
    #[should_panic(expected = "unread reply bytes")]
    fn assert_done_catches_unread_reply() {
        let mut t = MockTransport::new().exchange(&[1], &[2]);
        t.write_all(&[1]).unwrap();
        t.assert_done();
    }
}
