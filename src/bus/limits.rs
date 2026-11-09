//! EEPROM limit writes (plan 2026-11-09, D2 to D5): the checks a limit
//! write passes before a byte is sent, the packets it sends, and the cache
//! that follows it.

use crate::Transport;
use crate::actuator::{Actuator, Describe, LimitMode, ModeSource, TorqueState};
use crate::error::{Error, Instruction, ValidationError};
use crate::field::{LimitField, ReadField};
use crate::models::ActuatorKind;
use crate::protocol::{Clock, Protocol};
use crate::register::{Access, Register};
use crate::units::Limits;

use super::actuator_ref::ActuatorRef;
use super::diagnose::diagnosed;
use super::field::{self as fields, sealed::Decode, sealed::Site};
use super::observe::observe;
use super::slots::Bound;

/// One limit register write, checked: where it goes and its raw value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Checked {
    /// The register's site.
    site: Site,
    /// The register value.
    raw: i32,
}

impl Checked {
    /// Write `raw` to `site`, as if checked, for tests of the sending
    /// order on a table no setter reaches yet.
    #[cfg(all(test, feature = "y-series"))]
    pub(super) const fn new(site: Site, raw: i32) -> Self {
        Self { site, raw }
    }

    /// The register value as the register's bytes, in `out`.
    ///
    /// # Errors
    ///
    /// `DoesNotFit` when the value does not fit the register.
    fn encode<'o>(&self, out: &'o mut [u8; 4]) -> Result<&'o [u8], ValidationError> {
        self.site
            .reg
            .encode(self.raw, out)
            .map_err(|err| err.within(self.site.request(Instruction::Write)))
    }
}

/// Refuses an EEPROM write while the torque state the bus holds is `On`:
/// the bus writes EEPROM only with torque off, on every model. X-series,
/// MX(2.0) and XL-320 firmware also writes EEPROM only with Torque Enable 0
/// (XM430-W350: "Data in the EEPROM Area can only be written to if Torque
/// Enable(64) is cleared to '0'",
/// <https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>, 2026-11-09);
/// the AX, RX and MX Protocol 1 pages document no such lock (Lock(47) locks
/// their EEPROM), so there this check is the only guard. `Off` and
/// `Faulted` pass, so an actuator held faulted by Setup can have its
/// limits fixed.
fn check_off(
    id: crate::Id,
    state: TorqueState,
    register: &'static str,
) -> Result<(), ValidationError> {
    if state.is_on() {
        Err(ValidationError::ActuatorIsOn { id, register })
    } else {
        Ok(())
    }
}

/// The position limits Setup cached, for the order check of a single
/// position-limit write.
fn cached_limits(kind: &ActuatorKind) -> Result<Limits, ValidationError> {
    kind.cache()
        .limits()
        .ok_or(ValidationError::LimitsUnknown { id: kind.id() })
}

/// Checks that a single write of `raw` to `site` keeps the position limits
/// ordered against the other cached limit. Any other register passes.
fn check_order(kind: &ActuatorKind, site: &Site, raw: i32) -> Result<(), ValidationError> {
    let registers = kind.data().limits;
    let (min, max) = if site.reg.addr == registers.min().addr {
        (raw, cached_limits(kind)?.max_raw())
    } else if site.reg.addr == registers.max().addr {
        (cached_limits(kind)?.min_raw(), raw)
    } else {
        return Ok(());
    };
    // DECISION: a single position-limit write whose other limit is unknown (a failed write since Setup) is refused with LimitsUnknown: its order cannot be checked; the pair setter checks its own two values.
    Limits::new(kind.id(), min, max).map(drop)
}

/// A single write of `value` to limit field `F` of `kind`, whose torque
/// state is `state`, checked in the order of plan D4: the register, the
/// torque, the value's conversion, the e-Manual range, the position-limit
/// order.
///
/// # Errors
///
/// `Unsupported`, `ActuatorIsOn`, `NoUnit`, `NotFinite`, `DoesNotFit`,
/// `OutOfLimits`, `LimitsUnknown`, `InvertedLimits`.
pub(super) fn check_single<F: LimitField>(
    kind: &ActuatorKind,
    state: TorqueState,
    value: F::Value,
) -> Result<Checked, ValidationError> {
    let site = fields::limit_site::<F>(kind)?;
    check_off(kind.id(), state, site.name)?;
    let raw = fields::limit_raw(&site, value)?;
    check_order(kind, &site, raw)?;
    Ok(Checked { site, raw })
}

/// A write of the pair `min` to `Lo` and `max` to `Hi`, checked in the
/// order of plan D4: both registers, the torque, both values' conversions
/// and ranges, then `min <= max`.
///
/// # Errors
///
/// `Unsupported`, `ActuatorIsOn` (naming `Lo`), `NoUnit`, `NotFinite`,
/// `DoesNotFit`, `OutOfLimits`, `InvertedLimits`.
pub(super) fn check_pair<Lo: LimitField, Hi: LimitField>(
    kind: &ActuatorKind,
    state: TorqueState,
    min: Lo::Value,
    max: Hi::Value,
) -> Result<(Checked, Checked), ValidationError> {
    let lo = fields::limit_site::<Lo>(kind)?;
    let hi = fields::limit_site::<Hi>(kind)?;
    // DECISION: a pair refused for torque names its minimum register, the first the caller passes.
    check_off(kind.id(), state, lo.name)?;
    let min = fields::limit_raw(&lo, min)?;
    let max = fields::limit_raw(&hi, max)?;
    Limits::new(kind.id(), min, max)?;
    Ok((
        Checked { site: lo, raw: min },
        Checked { site: hi, raw: max },
    ))
}

/// The pair `lo`, `hi` in the order to send it when the registers are
/// apart: the maximum first when the new minimum lies above the maximum
/// the bus holds, so the actuator never holds an inverted pair between
/// the two writes.
pub(super) fn ordered(kind: &ActuatorKind, lo: Checked, hi: Checked) -> (Checked, Checked) {
    // DECISION: two writes of an apart pair (Y series) go maximum first only when the new minimum lies above the held maximum, minimum first otherwise; with the held limits unknown, minimum first.
    if kind
        .cache()
        .limits()
        .is_some_and(|old| lo.raw > old.max_raw())
    {
        (hi, lo)
    } else {
        (lo, hi)
    }
}

/// The register spanning `a` and `b` when they are adjacent, named after
/// the lower one, for one Write or Read of both.
fn span(a: Register, b: Register) -> Option<Register> {
    let (lower, upper) = if a.addr <= b.addr { (a, b) } else { (b, a) };
    let end = lower.addr.checked_add(u16::from(lower.size))?;
    (end == upper.addr).then_some(Register {
        size: lower.size.checked_add(upper.size)?,
        access: Access::RW,
        ..lower
    })
}

/// `a` then `b`, ordered by address, as one buffer of `size` bytes.
fn joined<'o>(
    (a, a_bytes): (Register, &[u8]),
    (b, b_bytes): (Register, &[u8]),
    size: u8,
    out: &'o mut [u8; 8],
) -> &'o [u8] {
    let (first, second) = if a.addr <= b.addr {
        (a_bytes, b_bytes)
    } else {
        (b_bytes, a_bytes)
    };
    for (slot, byte) in out.iter_mut().zip(first.iter().chain(second)) {
        *slot = *byte;
    }
    out.get(..usize::from(size)).unwrap_or_default()
}

/// The EEPROM register that bounds the goal `kind`'s recorded mode uses
/// (Velocity Limit in velocity mode, Current Limit in a current mode), if
/// its table records one.
fn bound_register(kind: &ActuatorKind) -> Option<Register> {
    fields::bounded_goal(kind.cache().mode())
        .and_then(|role| kind.data().role(role))
        .and_then(|goal| goal.bounded_by())
}

/// Whether `kind`'s limit mode follows its angle limits: AX, RX and MX on
/// Protocol 1, unless the MX-64's torque control switch is on.
fn mode_follows_limits(kind: &ActuatorKind) -> bool {
    match kind.data().mode {
        ModeSource::AngleLimits => true,
        ModeSource::TorqueControl { .. } => kind.cache().mode() != LimitMode::Current,
        ModeSource::Register { .. } => false,
    }
}

/// Records the position limits an acknowledged write left: `min` and `max`
/// where written, the cached limit where not, and on a model whose mode
/// follows them the mode `LimitMode::from_angle_limits` gives, as Setup
/// would have recorded it. Both written, they determine the limits and
/// that mode even when a failed write had left them unknown (plan D5).
fn record_written(kind: &mut ActuatorKind, bound: &mut Bound, min: Option<i32>, max: Option<i32>) {
    // DECISION: one written limit with the other unknown leaves the limits unknown; check_order refuses that write before a byte, so this only guards the cache.
    let (min, max) = match (min, max, kind.cache().limits()) {
        (Some(min), Some(max), _) => (min, max),
        (min, max, Some(old)) => (min.unwrap_or(old.min_raw()), max.unwrap_or(old.max_raw())),
        (_, _, None) => return,
    };
    let Ok(limits) = kind.cache_mut().record_limits(min, max) else {
        kind.cache_mut().forget_limits();
        return;
    };
    if !mode_follows_limits(kind) {
        return;
    }
    let mode = LimitMode::from_angle_limits(limits);
    let was_known = kind.cache().check_mode_known().is_ok();
    if mode != kind.cache().mode() || !was_known {
        kind.cache_mut().know_mode(mode);
        *bound = None;
        if bound_register(kind).is_some() {
            kind.cache_mut().forget_bound();
        }
    }
}

/// Makes the cache follow limit writes `writes` (plan D5). Acknowledged
/// (`ok`): the position limits, the limit mode on the models whose mode
/// follows them, and the goal bound take the written values, and a write
/// that determines one makes it known again (both limits, or the bounding
/// register). Failed: the cache they touch is forgotten, the goal bound
/// the bus holds included, until the next Setup or an acknowledged write
/// that determines it. Voltage, temperature, acceleration, PWM and Max
/// Torque limits are not cached.
fn follow(kind: &mut ActuatorKind, bound: &mut Bound, writes: &[Checked], ok: bool) {
    let registers = kind.data().limits;
    let bounding = bound_register(kind);
    let (mut min, mut max) = (None, None);
    for write in writes {
        let addr = write.site.reg.addr;
        if addr == registers.min().addr {
            min = Some(write.raw);
        } else if addr == registers.max().addr {
            max = Some(write.raw);
        } else if bounding.is_some_and(|reg| reg.addr == addr) {
            if ok {
                *bound = Some(write.raw.cast_unsigned());
                kind.cache_mut().know_bound();
            } else {
                *bound = None;
                kind.cache_mut().forget_bound();
            }
        }
    }
    if min.is_none() && max.is_none() {
        return;
    }
    if ok {
        record_written(kind, bound, min, max);
    } else {
        if mode_follows_limits(kind) {
            kind.cache_mut().forget_mode();
            *bound = None;
        }
        kind.cache_mut().forget_limits();
    }
}

impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> {
    /// Writes `bytes` to `reg`: one acknowledged write, its reply through
    /// the fault check.
    fn exchange(&mut self, reg: Register, bytes: &[u8]) -> Result<(), Error> {
        let result = self.wire.write(self.kind.id(), reg, bytes);
        let result = diagnosed(&mut self.wire, self.kind, result);
        observe(self.state, &result);
        result
    }

    /// Sends checked limit write `first`, then `second`: one Write of both
    /// when their registers are adjacent, else one each, the second only
    /// after the first was acknowledged. Every value is encoded before a
    /// byte is sent. The cache then follows the result (plan D5).
    ///
    /// # Errors
    ///
    /// `DoesNotFit` with nothing sent, else any error of a write.
    pub(super) fn send_limits(
        &mut self,
        first: Checked,
        second: Option<Checked>,
    ) -> Result<(), Error> {
        let mut a = [0; 4];
        let mut b = [0; 4];
        let mut both = [0; 8];
        let first_bytes = first.encode(&mut a)?;
        let result = match second {
            None => self.exchange(first.site.reg, first_bytes),
            Some(second) => {
                let second_bytes = second.encode(&mut b)?;
                // DECISION: a pair whose registers are adjacent goes out as one Write, so the actuator never holds half of it; the request names the lower register with both sizes.
                match span(first.site.reg, second.site.reg) {
                    Some(reg) => {
                        let bytes = joined(
                            (first.site.reg, first_bytes),
                            (second.site.reg, second_bytes),
                            reg.size,
                            &mut both,
                        );
                        self.exchange(reg, bytes)
                    }
                    None => self
                        .exchange(first.site.reg, first_bytes)
                        .and_then(|()| self.exchange(second.site.reg, second_bytes)),
                }
            }
        };
        // DECISION: every error of the exchange counts as sent: the bus cannot tell whether the actuator applied the write, so the cache it touches is forgotten.
        let ok = result.is_ok();
        match second {
            Some(second) => follow(self.kind, self.bound, &[first, second], ok),
            None => follow(self.kind, self.bound, &[first], ok),
        }
        result
    }

    /// Reads fields `Lo` and `Hi`: one Read of both when their registers
    /// are adjacent, else one each.
    ///
    /// # Errors
    ///
    /// As [`read`](ActuatorRef::read).
    pub(super) fn read_pair<Lo: ReadField, Hi: ReadField>(
        &mut self,
    ) -> Result<(Lo::Value, Hi::Value), Error> {
        let lo = fields::read_site::<Lo>(self.kind)?;
        let hi = fields::read_site::<Hi>(self.kind)?;
        // DECISION: a pair getter reads both adjacent registers in one Read ("one unicast read each", plan D3), else one Read each.
        let Some(reg) = span(lo.reg, hi.reg) else {
            return Ok((self.read_one::<Lo>(&lo)?, self.read_one::<Hi>(&hi)?));
        };
        let mut buf = [0; 8];
        let bytes = buf.get_mut(..usize::from(reg.size)).unwrap_or_default();
        let result = self.wire.read(self.kind.id(), reg, bytes);
        let result = diagnosed(&mut self.wire, self.kind, result);
        observe(self.state, &result);
        result?;
        let (lower, upper) = if lo.reg.addr <= hi.reg.addr {
            (lo.reg, hi.reg)
        } else {
            (hi.reg, lo.reg)
        };
        let (lower_bytes, upper_bytes) = bytes.split_at(usize::from(lower.size).min(bytes.len()));
        let lower_raw = lower.decode(lower_bytes)?;
        let upper_raw = upper.decode(upper_bytes)?;
        let (lo_raw, hi_raw) = if lower == lo.reg {
            (lower_raw, upper_raw)
        } else {
            (upper_raw, lower_raw)
        };
        Ok((
            Lo::Value::from_raw(lo_raw, &lo)?,
            Hi::Value::from_raw(hi_raw, &hi)?,
        ))
    }

    /// Reads field `F` at its checked `site`.
    fn read_one<F: ReadField>(&mut self, site: &Site) -> Result<F::Value, Error> {
        let result = self.wire.read_value(self.kind.id(), site.reg);
        let result = diagnosed(&mut self.wire, self.kind, result);
        observe(self.state, &result);
        Ok(F::Value::from_raw(result?, site)?)
    }
}
