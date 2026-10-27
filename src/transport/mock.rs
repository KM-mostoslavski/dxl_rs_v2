//! A scripted, allocation-free [`Transport`](super::Transport) for tests
//! without hardware.

use core::cell::Cell;
use core::fmt;

use embedded_io::{ErrorKind, ErrorType, Read, ReadReady, Write};

/// Shared test time, in microseconds, for a [`MockTransport`] and the clock
/// the bus reads.
///
/// Time only moves forward. [`ticker`](Self::ticker) hands out a closure the
/// bus can use as its clock:
///
/// ```
/// use dxl_rs_v2::MockTime;
///
/// let time = MockTime::new();
/// let mut clock = time.ticker(10);
/// assert_eq!(clock(), 10);
/// assert_eq!(clock(), 20);
/// assert_eq!(time.now(), 20);
/// ```
#[derive(Debug, Default)]
pub struct MockTime {
    now: Cell<u64>,
}

impl MockTime {
    /// Time zero.
    pub const fn new() -> Self {
        Self { now: Cell::new(0) }
    }

    /// The current time.
    pub fn now(&self) -> u64 {
        self.now.get()
    }

    /// Moves time to `us`. Setting it backwards is ignored, to keep it
    /// monotonic.
    pub fn set(&self, us: u64) {
        self.now.set(self.now.get().max(us));
    }

    /// Moves time forward by `us`.
    pub fn advance(&self, us: u64) {
        self.now.set(self.now.get().saturating_add(us));
    }

    /// A clock that moves time forward by `step_us` on every call and
    /// returns the new time.
    pub fn ticker(&self, step_us: u64) -> impl FnMut() -> u64 + '_ {
        move || {
            self.advance(step_us);
            self.now()
        }
    }
}

/// When a scripted chunk of incoming bytes becomes readable.
#[derive(Debug, Clone, Copy)]
enum Gate {
    /// From the start: bytes left on the line by earlier traffic.
    Always,
    /// Once the `flushes`-th packet overall has been sent and the time
    /// reaches `at`.
    AfterTx { flushes: usize, at: u64 },
}

/// A serial line that replays scripted incoming bytes and records what is
/// sent. Nothing allocates; capacities are const parameters. Writing past
/// `TX` fails with [`ErrorKind::WriteZero`].
///
/// Incoming chunks open strictly in script order: a chunk that is not
/// readable yet hides every chunk after it. A line with nothing scripted
/// stays silent. [`reply`](Self::reply) and [`reply_at`](Self::reply_at)
/// wait for the next packet sent after they are scripted, so a test can
/// script each reply right before the exchange that should get it.
///
/// ```
/// use dxl_rs_v2::{MockTime, MockTransport};
/// use dxl_rs_v2::embedded_io::{Read, ReadReady, Write};
///
/// let time = MockTime::new();
/// let mut line = MockTransport::new(&time);
/// line.reply(&[0xAA, 0xBB]);
///
/// assert!(!line.read_ready().unwrap());
/// line.write_all(&[1, 2, 3]).unwrap();
/// line.flush().unwrap();
///
/// let mut reply = [0; 2];
/// line.read_exact(&mut reply).unwrap();
/// assert_eq!(reply, [0xAA, 0xBB]);
/// line.assert_sent(&[1, 2, 3]);
/// line.assert_done();
/// ```
pub struct MockTransport<
    't,
    const RX: usize = 4096,
    const TX: usize = 4096,
    const CHUNKS: usize = 32,
> {
    time: &'t MockTime,
    rx: [u8; RX],
    rx_len: usize,
    rx_pos: usize,
    chunks: [(usize, Gate); CHUNKS],
    n_chunks: usize,
    tx: [u8; TX],
    tx_len: usize,
    reads: usize,
    writes: usize,
    flushes: usize,
    fail: Option<ErrorKind>,
    eof: bool,
}

impl<'t> MockTransport<'t> {
    /// A silent line with the default capacities (4096 bytes in, 4096 out,
    /// 32 scripted chunks), reading time from `time`.
    pub fn new(time: &'t MockTime) -> Self {
        Self::with_capacity(time)
    }
}

impl<'t, const RX: usize, const TX: usize, const CHUNKS: usize> MockTransport<'t, RX, TX, CHUNKS> {
    /// A silent line with the capacities of the type parameters.
    pub fn with_capacity(time: &'t MockTime) -> Self {
        Self {
            time,
            rx: [0; RX],
            rx_len: 0,
            rx_pos: 0,
            chunks: [(0, Gate::Always); CHUNKS],
            n_chunks: 0,
            tx: [0; TX],
            tx_len: 0,
            reads: 0,
            writes: 0,
            flushes: 0,
            fail: None,
            eof: false,
        }
    }

    #[track_caller]
    fn push(&mut self, bytes: &[u8], gate: Gate) -> &mut Self {
        let end = self.rx_len + bytes.len();
        assert!(
            end <= RX,
            "MockTransport: scripted bytes exceed RX capacity {RX}"
        );
        assert!(
            self.n_chunks < CHUNKS,
            "MockTransport: scripted chunks exceed CHUNKS capacity {CHUNKS}"
        );
        self.rx[self.rx_len..end].copy_from_slice(bytes);
        self.rx_len = end;
        self.chunks[self.n_chunks] = (end, gate);
        self.n_chunks += 1;
        self
    }

    /// Bytes already waiting before anything is sent.
    ///
    /// # Panics
    ///
    /// When the script would exceed `RX` bytes or `CHUNKS` chunks.
    #[track_caller]
    pub fn stale(&mut self, bytes: &[u8]) -> &mut Self {
        self.push(bytes, Gate::Always)
    }

    /// Bytes that arrive once the next packet has been sent (flushed).
    ///
    /// # Panics
    ///
    /// When the script would exceed `RX` bytes or `CHUNKS` chunks.
    #[track_caller]
    pub fn reply(&mut self, bytes: &[u8]) -> &mut Self {
        self.reply_at(0, bytes)
    }

    /// Bytes that arrive once the next packet has been sent and the time
    /// reaches `at_us`.
    ///
    /// # Panics
    ///
    /// When the script would exceed `RX` bytes or `CHUNKS` chunks.
    #[track_caller]
    pub fn reply_at(&mut self, at_us: u64, bytes: &[u8]) -> &mut Self {
        self.push(
            bytes,
            Gate::AfterTx {
                flushes: self.flushes + 1,
                at: at_us,
            },
        )
    }

    /// Bytes that arrive once the `nth_flush`-th packet since the mock was
    /// created has been sent, to script several exchanges up front.
    ///
    /// # Panics
    ///
    /// When the script would exceed `RX` bytes or `CHUNKS` chunks.
    #[track_caller]
    pub fn reply_to(&mut self, nth_flush: usize, bytes: &[u8]) -> &mut Self {
        self.push(
            bytes,
            Gate::AfterTx {
                flushes: nth_flush,
                at: 0,
            },
        )
    }

    /// Makes every later call fail with `kind`.
    pub fn fail_with(&mut self, kind: ErrorKind) -> &mut Self {
        self.fail = Some(kind);
        self
    }

    /// Makes the line report input ready but read zero bytes, as a
    /// disconnected port does.
    pub fn end_of_stream(&mut self) -> &mut Self {
        self.eof = true;
        self
    }

    /// Every byte written so far.
    pub fn sent(&self) -> &[u8] {
        &self.tx[..self.tx_len]
    }

    /// Number of `read` calls so far.
    pub fn reads(&self) -> usize {
        self.reads
    }

    /// Number of `write` calls so far.
    pub fn writes(&self) -> usize {
        self.writes
    }

    /// Number of `flush` calls so far.
    pub fn flushes(&self) -> usize {
        self.flushes
    }

    /// Scripted bytes not read yet.
    pub fn unread(&self) -> usize {
        self.rx_len - self.rx_pos
    }

    /// Compares the bytes sent so far with `expected`.
    ///
    /// # Errors
    ///
    /// The first [`Mismatch`] when they differ.
    pub fn check_sent(&self, expected: &[u8]) -> Result<(), Mismatch> {
        let sent = self.sent();
        if sent == expected {
            return Ok(());
        }
        let offset = sent
            .iter()
            .zip(expected)
            .position(|(a, e)| a != e)
            .unwrap_or(sent.len().min(expected.len()));
        Err(Mismatch {
            offset,
            expected: expected.get(offset).copied(),
            actual: sent.get(offset).copied(),
        })
    }

    /// Asserts that the bytes sent so far equal `expected`.
    ///
    /// # Panics
    ///
    /// On the first [`Mismatch`], showing both byte strings.
    #[track_caller]
    pub fn assert_sent(&self, expected: &[u8]) {
        if let Err(mismatch) = self.check_sent(expected) {
            panic!(
                "{mismatch}\n    sent: {:02X?}\nexpected: {expected:02X?}",
                self.sent()
            );
        }
    }

    /// Asserts that every scripted byte was read.
    ///
    /// # Panics
    ///
    /// When scripted bytes are left unread.
    #[track_caller]
    pub fn assert_done(&self) {
        assert!(
            self.unread() == 0,
            "MockTransport: {} scripted bytes never read",
            self.unread()
        );
    }

    fn available_end(&self) -> usize {
        let mut end = self.rx_pos;
        for &(chunk_end, gate) in &self.chunks[..self.n_chunks] {
            if chunk_end <= self.rx_pos {
                continue;
            }
            let open = match gate {
                Gate::Always => true,
                Gate::AfterTx { flushes, at } => self.flushes >= flushes && self.time.now() >= at,
            };
            if !open {
                break;
            }
            end = chunk_end;
        }
        end
    }

    fn check(&self) -> Result<(), ErrorKind> {
        self.fail.map_or(Ok(()), Err)
    }
}

/// Shows counters and the bytes sent, not the raw buffers, so a failing
/// assertion on a value holding the mock stays readable.
impl<const RX: usize, const TX: usize, const CHUNKS: usize> fmt::Debug
    for MockTransport<'_, RX, TX, CHUNKS>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockTransport")
            .field("now", &self.time.now())
            .field("sent", &self.sent())
            .field("unread", &self.unread())
            .field("chunks", &self.n_chunks)
            .field("reads", &self.reads)
            .field("writes", &self.writes)
            .field("flushes", &self.flushes)
            .field("fail", &self.fail)
            .field("eof", &self.eof)
            .finish()
    }
}

impl<const RX: usize, const TX: usize, const CHUNKS: usize> ErrorType
    for MockTransport<'_, RX, TX, CHUNKS>
{
    type Error = ErrorKind;
}

impl<const RX: usize, const TX: usize, const CHUNKS: usize> ReadReady
    for MockTransport<'_, RX, TX, CHUNKS>
{
    fn read_ready(&mut self) -> Result<bool, ErrorKind> {
        self.check()?;
        Ok(self.eof || self.available_end() > self.rx_pos)
    }
}

impl<const RX: usize, const TX: usize, const CHUNKS: usize> Read
    for MockTransport<'_, RX, TX, CHUNKS>
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ErrorKind> {
        self.check()?;
        self.reads += 1;
        let start = self.rx_pos;
        let n = buf.len().min(self.available_end() - start);
        buf[..n].copy_from_slice(&self.rx[start..start + n]);
        self.rx_pos += n;
        Ok(n)
    }
}

impl<const RX: usize, const TX: usize, const CHUNKS: usize> Write
    for MockTransport<'_, RX, TX, CHUNKS>
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, ErrorKind> {
        self.check()?;
        self.writes += 1;
        let start = self.tx_len;
        let n = buf.len().min(TX - start);
        if n == 0 && !buf.is_empty() {
            return Err(ErrorKind::WriteZero);
        }
        self.tx[start..start + n].copy_from_slice(&buf[..n]);
        self.tx_len += n;
        Ok(n)
    }

    fn flush(&mut self) -> Result<(), ErrorKind> {
        self.check()?;
        self.flushes += 1;
        Ok(())
    }
}

/// Where the bytes a [`MockTransport`] sent first differ from the expected
/// ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mismatch {
    /// Index of the first differing byte, or the shorter length.
    pub offset: usize,
    /// The expected byte there; `None` when more bytes were sent than
    /// expected.
    pub expected: Option<u8>,
    /// The sent byte there; `None` when fewer bytes were sent than expected.
    pub actual: Option<u8>,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let at = self.offset;
        match (self.actual, self.expected) {
            (Some(a), Some(e)) => write!(f, "sent byte {at} is {a:#04X}, expected {e:#04X}"),
            (Some(a), None) => write!(f, "sent byte {at} is {a:#04X}, expected nothing more"),
            (None, Some(e)) => write!(f, "sent only {at} bytes, expected byte {at} {e:#04X}"),
            (None, None) => write!(f, "sent bytes match"),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::string::ToString;

    use super::*;

    fn send(line: &mut MockTransport<'_>, bytes: &[u8]) {
        line.write_all(bytes).unwrap();
        line.flush().unwrap();
    }

    fn read_all(line: &mut MockTransport<'_>) -> ([u8; 16], usize) {
        let mut buf = [0; 16];
        let n = line.read(&mut buf).unwrap();
        (buf, n)
    }

    #[test]
    fn stale_is_readable_before_any_send() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.stale(&[9, 9]);
        assert_eq!(line.read_ready(), Ok(true));
        let (buf, n) = read_all(&mut line);
        assert_eq!(&buf[..n], &[9, 9]);
    }

    #[test]
    fn reply_waits_for_first_flush() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.reply(&[1]);
        line.write_all(&[0]).unwrap();
        assert_eq!(line.read_ready(), Ok(false));
        line.flush().unwrap();
        assert_eq!(line.read_ready(), Ok(true));
    }

    #[test]
    fn reply_at_waits_for_time() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.reply_at(50, &[1]);
        send(&mut line, &[0]);
        time.set(49);
        assert_eq!(line.read_ready(), Ok(false));
        time.set(50);
        assert_eq!(line.read_ready(), Ok(true));
    }

    #[test]
    fn reply_waits_for_the_send_after_it_was_scripted() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.reply(&[1]);
        send(&mut line, &[0]);
        assert_eq!(read_all(&mut line).1, 1);
        line.reply(&[2]);
        assert_eq!(line.read_ready(), Ok(false));
        send(&mut line, &[0]);
        let (buf, n) = read_all(&mut line);
        assert_eq!(&buf[..n], &[2]);
    }

    #[test]
    fn reply_at_needs_both_send_and_time() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.reply_at(50, &[1]);
        time.set(50);
        assert_eq!(line.read_ready(), Ok(false));
        send(&mut line, &[0]);
        assert_eq!(line.read_ready(), Ok(true));
    }

    #[test]
    fn debug_is_compact() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.write_all(&[0xAB]).unwrap();
        let text = std::format!("{line:?}");
        assert!(text.starts_with("MockTransport { now: 0, sent: [171]"));
        assert!(text.len() < 200);
    }

    #[test]
    fn reply_to_nth_flush() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.reply_to(1, &[1]).reply_to(2, &[2]);
        send(&mut line, &[0]);
        let (buf, n) = read_all(&mut line);
        assert_eq!(&buf[..n], &[1]);
        assert_eq!(line.read_ready(), Ok(false));
        send(&mut line, &[0]);
        let (buf, n) = read_all(&mut line);
        assert_eq!(&buf[..n], &[2]);
    }

    #[test]
    fn closed_chunk_hides_later_chunks() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.reply_at(100, &[1]).reply(&[2]);
        send(&mut line, &[0]);
        assert_eq!(line.read_ready(), Ok(false));
        time.set(100);
        let (buf, n) = read_all(&mut line);
        assert_eq!(&buf[..n], &[1, 2]);
    }

    #[test]
    fn silent_line_is_never_ready() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        send(&mut line, &[0]);
        time.set(u64::MAX);
        assert_eq!(line.read_ready(), Ok(false));
        assert_eq!(read_all(&mut line).1, 0);
    }

    #[test]
    fn read_copies_what_fits() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.stale(&[1, 2, 3]);
        let mut one = [0; 1];
        assert_eq!(line.read(&mut one), Ok(1));
        assert_eq!(one, [1]);
        assert_eq!(line.unread(), 2);
    }

    #[test]
    fn fail_with_reaches_every_call() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.stale(&[1]).fail_with(ErrorKind::BrokenPipe);
        assert_eq!(line.read_ready(), Err(ErrorKind::BrokenPipe));
        assert_eq!(line.read(&mut [0; 1]), Err(ErrorKind::BrokenPipe));
        assert_eq!(line.write(&[0]), Err(ErrorKind::BrokenPipe));
        assert_eq!(line.flush(), Err(ErrorKind::BrokenPipe));
    }

    #[test]
    fn end_of_stream_is_ready_but_reads_zero() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.end_of_stream();
        assert_eq!(line.read_ready(), Ok(true));
        assert_eq!(line.read(&mut [0; 4]), Ok(0));
    }

    #[test]
    fn write_past_tx_capacity_is_write_zero() {
        let time = MockTime::new();
        let mut line = MockTransport::<4, 4, 2>::with_capacity(&time);
        assert_eq!(line.write(&[1, 2, 3]), Ok(3));
        assert_eq!(line.write(&[4, 5]), Ok(1));
        assert_eq!(line.write_all(&[6]), Err(ErrorKind::WriteZero));
        assert_eq!(line.write(&[]), Ok(0));
        assert_eq!(line.sent(), &[1, 2, 3, 4]);
    }

    #[test]
    fn counters_and_sent_bytes() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.stale(&[1]);
        line.write_all(&[1, 2]).unwrap();
        line.write_all(&[3]).unwrap();
        line.flush().unwrap();
        line.read(&mut [0; 1]).unwrap();
        assert_eq!(line.sent(), &[1, 2, 3]);
        assert_eq!((line.reads(), line.writes(), line.flushes()), (1, 2, 1));
    }

    #[test]
    fn check_sent_reports_first_difference() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.write_all(&[0, 1, 2, 3, 4, 5, 6, 3]).unwrap();
        assert_eq!(line.check_sent(&[0, 1, 2, 3, 4, 5, 6, 3]), Ok(()));
        let mismatch = line.check_sent(&[0, 1, 2, 3, 4, 5, 6, 2]).unwrap_err();
        assert_eq!(
            mismatch,
            Mismatch {
                offset: 7,
                expected: Some(2),
                actual: Some(3)
            }
        );
        assert_eq!(mismatch.to_string(), "sent byte 7 is 0x03, expected 0x02");
    }

    #[test]
    fn check_sent_reports_length_differences() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.write_all(&[1, 2]).unwrap();
        let more = line.check_sent(&[1]).unwrap_err();
        assert_eq!(
            (more.offset, more.expected, more.actual),
            (1, None, Some(2))
        );
        assert_eq!(
            more.to_string(),
            "sent byte 1 is 0x02, expected nothing more"
        );
        let fewer = line.check_sent(&[1, 2, 3]).unwrap_err();
        assert_eq!(
            (fewer.offset, fewer.expected, fewer.actual),
            (2, Some(3), None)
        );
        assert_eq!(fewer.to_string(), "sent only 2 bytes, expected byte 2 0x03");
        let none = Mismatch {
            offset: 0,
            expected: None,
            actual: None,
        };
        assert_eq!(none.to_string(), "sent bytes match");
    }

    #[test]
    #[should_panic(expected = "sent byte")]
    fn assert_sent_panics_on_mismatch() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.write_all(&[1]).unwrap();
        line.assert_sent(&[2]);
    }

    #[test]
    #[should_panic(expected = "never read")]
    fn assert_done_panics_on_unread_bytes() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.stale(&[1]);
        line.assert_done();
    }

    #[test]
    #[should_panic(expected = "MockTransport: scripted bytes exceed RX")]
    fn scripting_past_rx_panics() {
        let time = MockTime::new();
        let mut line = MockTransport::<2, 4, 4>::with_capacity(&time);
        line.stale(&[1, 2, 3]);
    }

    #[test]
    #[should_panic(expected = "MockTransport: scripted chunks exceed CHUNKS")]
    fn scripting_past_chunks_panics() {
        let time = MockTime::new();
        let mut line = MockTransport::<8, 4, 1>::with_capacity(&time);
        line.reply(&[1]).reply(&[2]);
    }

    #[test]
    fn time_is_monotonic() {
        let time = MockTime::default();
        time.advance(10);
        time.set(5);
        assert_eq!(time.now(), 10);
        let mut tick = time.ticker(7);
        assert_eq!((tick(), tick()), (17, 24));
        assert_eq!(time.now(), 24);
        time.set(u64::MAX);
        time.advance(1);
        assert_eq!(time.now(), u64::MAX);
    }
}
