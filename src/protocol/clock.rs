//! Time source that bounds every wait for a reply.

/// A monotonic clock in microseconds.
///
/// The bus reads it to stop waiting for a reply once its timeout has passed,
/// so a silent actuator never hangs the caller. Any closure returning
/// microseconds is a `Clock`, which keeps embedded setup to one line.
///
/// ```
/// use dxl_rs_v2::Clock;
///
/// let mut ticks = 0u64;
/// let mut clock = move || {
///     ticks += 10;
///     ticks
/// };
/// assert_eq!(clock.now_us(), 10);
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a dxl_rs_v2 clock",
    note = "pass a type implementing `Clock` (or a `FnMut() -> u64` closure) to the bus; under `std`, `StdClock` is provided"
)]
pub trait Clock {
    /// Microseconds since an arbitrary, fixed origin. Must never go
    /// backwards.
    fn now_us(&mut self) -> u64;
}

impl<F: FnMut() -> u64> Clock for F {
    fn now_us(&mut self) -> u64 {
        self()
    }
}

/// [`Clock`] backed by [`std::time::Instant`].
///
/// ```
/// use dxl_rs_v2::{Clock, StdClock};
///
/// let mut clock = StdClock::default();
/// let start = clock.now_us();
/// assert!(clock.now_us() >= start);
/// ```
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy)]
pub struct StdClock {
    origin: std::time::Instant,
}

#[cfg(feature = "std")]
impl Default for StdClock {
    fn default() -> Self {
        Self {
            origin: std::time::Instant::now(),
        }
    }
}

#[cfg(feature = "std")]
impl Clock for StdClock {
    fn now_us(&mut self) -> u64 {
        u64::try_from(self.origin.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_are_clocks() {
        let mut t = 0;
        let mut clock = || {
            t += 5;
            t
        };
        assert_eq!(Clock::now_us(&mut clock), 5);
        assert_eq!(Clock::now_us(&mut clock), 10);
    }

    #[test]
    #[cfg(feature = "std")]
    fn std_clock_is_monotonic() {
        let mut clock = StdClock::default();
        let a = clock.now_us();
        let b = clock.now_us();
        assert!(b >= a);
    }
}
