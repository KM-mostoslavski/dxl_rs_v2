//! Angles in radians and their mapping to raw position ticks.

use core::ops::{Add, AddAssign, Sub, SubAssign};

use crate::Id;
use crate::error::ValidationError;

/// An angle in radians.
///
/// Zero is the model's zero-radian tick: 2048 on an XM430, 512 on an
/// AX-12A. A model's [`PositionScale`] converts it to and from raw ticks.
///
/// The angle is an `f32`, which holds every tick exactly up to an offset of
/// 2²⁴ ticks from zero: more than 4000 turns on an X-series, about 33 on an
/// H54. Beyond that, use raw ticks.
///
/// Any `f32` can be stored. NaN and infinity are refused when converted to
/// ticks.
///
/// ```
/// use dxl_rs_v2::Position;
///
/// let goal = Position::from_radians(0.5);
/// let moved = goal + Position::from_radians(0.1);
/// assert!((moved.radians() - 0.6).abs() < 1e-6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Position(f32);

impl Position {
    /// The model's zero-radian tick.
    pub const ZERO: Self = Self(0.0);

    /// An angle of `radians`.
    pub const fn from_radians(radians: f32) -> Self {
        Self(radians)
    }

    /// The angle in radians.
    pub const fn radians(self) -> f32 {
        self.0
    }
}

impl Add for Position {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Position {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Position {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

/// How one actuator model maps raw position ticks to radians.
///
/// Built from the `type_info` of the model's control table: the zero-radian
/// tick and the slope between the minimum and maximum ticks and radians.
/// There is no public constructor, so every scale comes from a table:
///
/// ```compile_fail,E0624
/// let scale = dxl_rs_v2::PositionScale::new(2048, 0, 4095, -3.1415927, 3.1415927);
/// ```
///
/// There is no range clamp: ticks outside the table's range convert
/// linearly, as multi-turn positions do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionScale {
    /// `type_info.value_of_zero_radian_position`.
    zero_raw: i32,
    /// `type_info.value_of_min_radian_position`.
    min_raw: i32,
    /// `type_info.value_of_max_radian_position`.
    max_raw: i32,
    /// `type_info.min_radian`.
    min_rad: f32,
    /// `type_info.max_radian`.
    max_rad: f32,
    /// `(max_rad - min_rad) / (max_raw - min_raw)`, computed once.
    radians_per_tick: f32,
    /// The inverse slope, so `to_raw` needs no division.
    ticks_per_radian: f32,
}

impl PositionScale {
    /// The scale of a table whose `type_info` gives `zero_raw` as the
    /// zero-radian tick, and `min_raw..=max_raw` as `min_rad..=max_rad`.
    #[allow(dead_code, reason = "called by generated ModelData from X1")]
    pub(crate) const fn new(
        zero_raw: i32,
        min_raw: i32,
        max_raw: i32,
        min_rad: f32,
        max_rad: f32,
    ) -> Self {
        let ticks = (max_raw as i64 - min_raw as i64) as f32;
        let radians = max_rad - min_rad;
        Self {
            zero_raw,
            min_raw,
            max_raw,
            min_rad,
            max_rad,
            radians_per_tick: radians / ticks,
            ticks_per_radian: ticks / radians,
        }
    }

    /// The tick of zero radians.
    pub const fn zero_raw(&self) -> i32 {
        self.zero_raw
    }

    /// The angle of one tick, in radians.
    pub fn radians_per_tick(&self) -> f32 {
        self.radians_per_tick
    }

    /// The angle of tick `raw`. Any `i32` converts.
    pub fn to_position(&self, raw: i32) -> Position {
        let offset = i64::from(raw) - i64::from(self.zero_raw);
        Position(offset as f32 * self.radians_per_tick)
    }

    /// The tick nearest to `position`, half a tick rounding away from zero.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::NotFinite`] if the angle is NaN or infinite.
    /// - [`ValidationError::DoesNotFit`] with `size` 0 if the tick does not
    ///   fit an `i32`.
    pub fn to_raw(&self, position: Position) -> Result<i32, ValidationError> {
        if !position.0.is_finite() {
            return Err(ValidationError::NotFinite);
        }
        let raw = round_half_away(position.0 * self.ticks_per_radian)
            .saturating_add(i64::from(self.zero_raw));
        i32::try_from(raw).map_err(|_| ValidationError::DoesNotFit { raw, size: 0 })
    }
}

/// `x` rounded to the nearest whole number, half away from zero.
///
/// `x - whole` is exact, so no `f32` addition can round the result up a
/// tick. `as` saturates, so no input panics.
fn round_half_away(x: f32) -> i64 {
    let whole = x as i64;
    let rest = x - whole as f32;
    if rest >= 0.5 {
        whole.saturating_add(1)
    } else if rest <= -0.5 {
        whole.saturating_sub(1)
    } else {
        whole
    }
}

/// The position limits of one actuator, in raw ticks, as read from its
/// EEPROM: Min/Max Position Limit, or CW/CCW Angle Limit.
///
/// Built only from values read from the actuator, never from a control
/// table's defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Min Position Limit, or CW Angle Limit.
    min: i32,
    /// Max Position Limit, or CCW Angle Limit.
    max: i32,
}

impl Limits {
    /// The limits `min..=max` read from actuator `id`.
    ///
    /// # Errors
    ///
    /// [`ValidationError::InvertedLimits`] if `min > max`. Equal limits are
    /// accepted.
    pub(crate) const fn new(id: Id, min: i32, max: i32) -> Result<Self, ValidationError> {
        if min > max {
            Err(ValidationError::InvertedLimits { id, min, max })
        } else {
            Ok(Self { min, max })
        }
    }

    /// The minimum limit (Min Position Limit, or CW Angle Limit).
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn min_raw(&self) -> i32 {
        self.min
    }

    /// The maximum limit (Max Position Limit, or CCW Angle Limit).
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn max_raw(&self) -> i32 {
        self.max
    }
}

#[cfg(test)]
mod tests {
    use core::f32::consts::{FRAC_PI_2, PI};

    use proptest::prelude::*;

    use super::*;

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    fn xm() -> PositionScale {
        PositionScale::new(2048, 0, 4095, -PI, PI)
    }

    fn ax() -> PositionScale {
        PositionScale::new(512, 0, 1023, 0.0, 5.2359877)
    }

    /// A scale whose slope is a power of two, so tick fractions are exact.
    fn exact() -> PositionScale {
        PositionScale::new(0, -1024, 1024, -1.0, 1.0)
    }

    /// The distinct `type_info` scales of today's tables (research R-6).
    const TABLE_SCALES: [(i32, i32, i32, f32, f32); 8] = [
        (2048, 0, 4095, -PI, PI),
        (512, 0, 1023, 0.0, 5.2359877),
        (0, -303_750, 303_750, -PI, PI),
        (0, -501_923, 501_923, -PI, PI),
        (0, -263_187, 263_187, -PI, PI),
        (0, -251_417, 251_417, -PI, PI),
        (0, -740, 740, -1.099, 1.099),
        (0, -262_144, 262_144, -PI, PI),
    ];

    /// The angle `ticks` ticks away from the zero tick of `scale`.
    fn ticks_from_zero(scale: PositionScale, ticks: f32) -> Position {
        Position::from_radians(ticks * scale.radians_per_tick())
    }

    #[test]
    fn zero_radians_is_the_zero_tick() {
        assert_eq!(xm().to_raw(Position::ZERO), Ok(2048));
        assert_eq!(ax().to_raw(Position::ZERO), Ok(512));
        assert_eq!(xm().zero_raw(), 2048);
        assert_eq!(xm().to_position(2048), Position::ZERO);
    }

    #[test]
    fn every_tick_of_every_table_scale_round_trips() {
        for (zero, min, max, min_rad, max_rad) in TABLE_SCALES {
            let scale = PositionScale::new(zero, min, max, min_rad, max_rad);
            for tick in min..=max {
                assert_eq!(scale.to_raw(scale.to_position(tick)), Ok(tick), "{scale:?}");
            }
        }
    }

    #[test]
    fn multi_turn_ticks_round_trip() {
        let scale = xm();
        for tick in (-(1 << 20)..=1 << 20).step_by(7) {
            assert_eq!(scale.to_raw(scale.to_position(tick)), Ok(tick));
        }
    }

    #[test]
    fn rounds_half_away_from_zero() {
        let scale = xm();
        assert_eq!(scale.to_raw(ticks_from_zero(scale, 0.4)), Ok(2048));
        assert_eq!(scale.to_raw(ticks_from_zero(scale, 0.6)), Ok(2049));
        assert_eq!(scale.to_raw(ticks_from_zero(scale, -0.4)), Ok(2048));
        assert_eq!(scale.to_raw(ticks_from_zero(scale, -0.6)), Ok(2047));
        let exact = exact();
        assert_eq!(exact.to_raw(ticks_from_zero(exact, 0.5)), Ok(1));
        assert_eq!(exact.to_raw(ticks_from_zero(exact, -0.5)), Ok(-1));
        assert_eq!(exact.to_raw(ticks_from_zero(exact, 2.5)), Ok(3));
        assert_eq!(exact.to_raw(ticks_from_zero(exact, -2.5)), Ok(-3));
        assert_eq!(exact.to_raw(ticks_from_zero(exact, 0.49999997)), Ok(0));
        assert_eq!(exact.to_raw(ticks_from_zero(exact, -0.49999997)), Ok(0));
        assert_eq!(
            exact.to_raw(ticks_from_zero(exact, 8_388_609.0)),
            Ok(8_388_609)
        );
        assert_eq!(
            exact.to_raw(ticks_from_zero(exact, -8_388_609.0)),
            Ok(-8_388_609)
        );
    }

    #[test]
    fn non_finite_angles_are_refused() {
        for radians in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                xm().to_raw(Position::from_radians(radians)),
                Err(ValidationError::NotFinite)
            );
        }
    }

    #[test]
    fn angles_beyond_i32_do_not_fit() {
        for radians in [1e30, -1e30, f32::MAX, f32::MIN] {
            assert!(matches!(
                xm().to_raw(Position::from_radians(radians)),
                Err(ValidationError::DoesNotFit { size: 0, .. })
            ));
        }
        let exact = exact();
        assert_eq!(
            exact.to_raw(ticks_from_zero(exact, 2_147_483_904.0)),
            Err(ValidationError::DoesNotFit {
                raw: 2_147_483_904,
                size: 0
            })
        );
        assert_eq!(
            exact.to_raw(ticks_from_zero(exact, -2_147_483_648.0)),
            Ok(i32::MIN)
        );
    }

    #[test]
    fn to_position_takes_any_i32() {
        for scale in [xm(), ax()] {
            for raw in [i32::MIN, i32::MAX] {
                assert!(scale.to_position(raw).radians().is_finite());
            }
        }
        assert!(xm().to_position(i32::MIN) < Position::ZERO);
        assert!(xm().to_position(i32::MAX) > Position::ZERO);
    }

    #[test]
    fn ax_angle_below_zero_gives_negative_tick() {
        let below_center = ax().to_raw(Position::from_radians(-FRAC_PI_2)).unwrap();
        assert!((0..512).contains(&below_center));
        let below_tick_zero = ax().to_raw(Position::from_radians(-PI)).unwrap();
        assert!(below_tick_zero < 0);
    }

    #[test]
    fn position_arithmetic() {
        let goal = Position::from_radians(0.5);
        let step = Position::from_radians(0.1);
        assert!(((goal + step).radians() - 0.6).abs() < 1e-6);
        assert!(((goal - step).radians() - 0.4).abs() < 1e-6);
        let mut moved = goal;
        moved += step;
        assert_eq!(moved, goal + step);
        moved -= step;
        assert_eq!(moved, goal);
        assert_eq!(Position::default(), Position::ZERO);
        assert_eq!(Position::ZERO.radians(), 0.0);
        assert!(step < goal);
        assert!(Position::from_radians(-0.1) < Position::ZERO);
    }

    #[test]
    fn scale_exposes_its_slope() {
        assert!((xm().radians_per_tick() - 2.0 * PI / 4095.0).abs() < 1e-9);
        assert_eq!(exact().radians_per_tick(), 1.0 / 1024.0);
        assert_eq!(xm(), xm());
        assert_ne!(xm(), ax());
    }

    #[test]
    fn inverted_limits_are_refused() {
        assert_eq!(
            Limits::new(id(3), 3000, 1000),
            Err(ValidationError::InvertedLimits {
                id: id(3),
                min: 3000,
                max: 1000
            })
        );
    }

    #[test]
    fn equal_limits_are_accepted() {
        let limits = Limits::new(id(3), 2048, 2048).unwrap();
        assert_eq!((limits.min_raw(), limits.max_raw()), (2048, 2048));
    }

    #[test]
    fn limits_keep_raw_bounds() {
        let limits = Limits::new(id(3), 1000, 3000).unwrap();
        assert_eq!(limits.min_raw(), 1000);
        assert_eq!(limits.max_raw(), 3000);
        let wide = Limits::new(id(1), i32::MIN, i32::MAX).unwrap();
        assert_eq!((wide.min_raw(), wide.max_raw()), (i32::MIN, i32::MAX));
    }

    proptest! {
        #[test]
        fn to_raw_never_panics(radians in any::<f32>()) {
            for scale in [xm(), ax()] {
                let _ = scale.to_raw(Position::from_radians(radians));
            }
        }

        #[test]
        fn to_position_never_panics(raw in any::<i32>()) {
            for scale in [xm(), ax()] {
                prop_assert!(scale.to_position(raw).radians().is_finite());
            }
        }
    }
}
