//! [`Acceleration`].

use crate::error::ValidationError;

use super::scale::RegisterScale;

/// An angular acceleration in radians per second squared, held as an
/// `f32`. Positive is CCW, as for [`Velocity`](crate::Velocity).
///
/// The step of one raw value comes from the model's control table, e.g.
/// MX-64(2.0) Acceleration Limit(40): "214.577 Rev/min²"
/// (<https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/>), so a model whose
/// table gives no unit has no typed form, only raw. Acceleration Limit is a
/// magnitude: a value that rounds below raw 0 (below minus half a raw step)
/// does not fit it; a smaller negative value rounds to 0.
///
/// Any `f32` can be stored. NaN and infinity are refused when converted to
/// a raw value, and a value outside the register's e-Manual range is
/// refused before anything is sent.
///
/// ```
/// use dxl_rs_v2::Acceleration;
///
/// let a = Acceleration::from_rad_per_s2(37.5);
/// assert_eq!(a.rad_per_s2(), 37.5);
/// assert!(Acceleration::ZERO < a);
/// ```
///
/// [`Default`] is zero, to fill a caller-owned buffer before a
/// multi-actuator read.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Acceleration(f32);

impl Acceleration {
    /// No acceleration.
    pub const ZERO: Self = Self(0.0);

    /// The acceleration of `rad_per_s2` radians per second squared.
    pub const fn from_rad_per_s2(rad_per_s2: f32) -> Self {
        Self(rad_per_s2)
    }

    /// The acceleration in radians per second squared.
    pub const fn rad_per_s2(self) -> f32 {
        self.0
    }
}

impl RegisterScale {
    /// The acceleration of raw value `raw`.
    pub const fn acceleration(&self, raw: u32) -> Acceleration {
        Acceleration(self.si_of(raw))
    }

    /// The raw value nearest to `acceleration`.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::NotFinite`] if the acceleration is NaN or
    ///   infinite.
    /// - [`ValidationError::DoesNotFit`] if it does not fit the register.
    pub fn acceleration_to_raw(&self, acceleration: Acceleration) -> Result<u32, ValidationError> {
        self.raw_of_si(acceleration.0)
    }
}

#[cfg(test)]
mod tests {
    use super::super::scale::Sign;
    use super::*;
    use crate::error::Request;

    /// `rev_per_min2` revolutions per minute squared in rad/s².
    const fn rev_per_min2(rev_per_min2: f32) -> f32 {
        rev_per_min2 * core::f32::consts::TAU / 3600.0
    }

    /// MX-64(2.0) Acceleration Limit: 214.577 rev/min², 4 bytes, 0 to
    /// 32,767.
    const MX64_ACCELERATION_LIMIT: RegisterScale =
        RegisterScale::new(rev_per_min2(214.577), Sign::Unsigned, 4, false);

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5 * b.abs().max(1.0)
    }

    #[test]
    fn rad_per_s2_round_trips() {
        assert_eq!(Acceleration::from_rad_per_s2(-2.5).rad_per_s2(), -2.5);
        assert_eq!(Acceleration::default(), Acceleration::ZERO);
        assert!(Acceleration::from_rad_per_s2(1.0) < Acceleration::from_rad_per_s2(2.0));
    }

    #[test]
    fn mx64_raw_100_is_21457_7_rev_per_min2() {
        let a = MX64_ACCELERATION_LIMIT.acceleration(100);
        assert!(close(a.rad_per_s2(), rev_per_min2(21_457.7)), "{a:?}");
        assert!((a.rad_per_s2() - 37.450_5).abs() < 1e-3, "{a:?}");
        assert_eq!(MX64_ACCELERATION_LIMIT.acceleration_to_raw(a), Ok(100));
        assert_eq!(
            MX64_ACCELERATION_LIMIT.acceleration_to_raw(Acceleration::ZERO),
            Ok(0)
        );
    }

    #[test]
    fn every_raw_value_in_the_range_round_trips() {
        for raw in 0..=32_767 {
            let a = MX64_ACCELERATION_LIMIT.acceleration(raw);
            assert_eq!(
                MX64_ACCELERATION_LIMIT.acceleration_to_raw(a),
                Ok(raw),
                "raw {raw}"
            );
        }
    }

    #[test]
    fn non_finite_and_negative_are_refused() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                MX64_ACCELERATION_LIMIT.acceleration_to_raw(Acceleration::from_rad_per_s2(value)),
                Err(ValidationError::NOT_FINITE)
            );
        }
        assert_eq!(
            MX64_ACCELERATION_LIMIT
                .acceleration_to_raw(Acceleration::from_rad_per_s2(rev_per_min2(-214.577 * 3.0))),
            Err(ValidationError::DoesNotFit {
                raw: -3,
                size: 4,
                request: Request::NONE,
            })
        );
        assert!(
            MX64_ACCELERATION_LIMIT
                .acceleration_to_raw(Acceleration::from_rad_per_s2(1e12))
                .is_err()
        );
    }
}
