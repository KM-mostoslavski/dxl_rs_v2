//! [`Velocity`] and [`SpeedLimit`].

use crate::error::{Request, ValidationError};

use super::scale::RegisterScale;

/// An angular velocity in radians per second, held as an `f32`.
/// Positive is CCW, as for [`Position`](crate::Position).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Velocity(f32);

impl Velocity {
    /// No motion.
    pub const ZERO: Self = Self(0.0);

    /// The velocity of `rad_per_s` radians per second.
    pub const fn from_rad_per_s(rad_per_s: f32) -> Self {
        Self(rad_per_s)
    }

    /// The velocity in radians per second.
    pub const fn rad_per_s(self) -> f32 {
        self.0
    }
}

/// A speed limit: Protocol 1 Moving Speed in joint mode, where raw 0 means
/// no limit, not zero. AX-12A Moving Speed(32): "If it is set to 0, it means
/// the maximum rpm of the motor is used without controlling the speed"
/// (<https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpeedLimit {
    /// No speed control: the actuator moves at its maximum speed.
    Unlimited,
    /// Move at most this fast.
    Limited(Velocity),
}

impl RegisterScale {
    /// The velocity of raw value `raw`.
    pub const fn velocity(&self, raw: u32) -> Velocity {
        Velocity(self.si_of(raw))
    }

    /// The raw value nearest to `velocity`.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::NotFinite`] if the velocity is NaN or infinite.
    /// - [`ValidationError::DoesNotFit`] if it does not fit the register.
    pub fn velocity_to_raw(&self, velocity: Velocity) -> Result<u32, ValidationError> {
        self.raw_of_si(velocity.0)
    }

    /// The speed limit of raw value `raw`: [`SpeedLimit::Unlimited`] for
    /// raw 0 when this scale reads 0 as no limit.
    pub const fn speed_limit(&self, raw: u32) -> SpeedLimit {
        if self.zero_is_unlimited() && self.signed_of(raw) == 0 {
            SpeedLimit::Unlimited
        } else {
            SpeedLimit::Limited(self.velocity(raw))
        }
    }

    /// The raw value of `limit`.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::DoesNotFit`] with `raw` 0 if the scale has no
    ///   "no limit" value and `limit` is [`SpeedLimit::Unlimited`], or if
    ///   it has one and a [`SpeedLimit::Limited`] value rounds to raw 0.
    /// - [`ValidationError::NotFinite`] and [`ValidationError::DoesNotFit`]
    ///   as [`velocity_to_raw`](Self::velocity_to_raw).
    pub fn speed_limit_to_raw(&self, limit: SpeedLimit) -> Result<u32, ValidationError> {
        let reserved = ValidationError::DoesNotFit {
            raw: 0,
            size: self.size(),
            request: Request::NONE,
        };
        match (limit, self.zero_is_unlimited()) {
            (SpeedLimit::Unlimited, true) => Ok(0),
            (SpeedLimit::Unlimited, false) => Err(reserved),
            (SpeedLimit::Limited(velocity), unlimited) => {
                let raw = self.velocity_to_raw(velocity)?;
                if unlimited && raw == 0 {
                    Err(reserved)
                } else {
                    Ok(raw)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::scale::tests::{AX_MOVING_SPEED, AX_SPEED, MX64_VELOCITY, rpm};
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5 * b.abs().max(1.0)
    }

    #[test]
    fn mx64_raw_100_is_22_9_rpm() {
        let v = MX64_VELOCITY.velocity(100);
        assert!(close(v.rad_per_s(), rpm(22.9)), "{v:?}");
        assert!((v.rad_per_s() - 2.398).abs() < 1e-3, "{v:?}");
        assert_eq!(MX64_VELOCITY.velocity_to_raw(v), Ok(100));
    }

    #[test]
    fn negative_velocity_is_twos_complement_on_p2() {
        let v = MX64_VELOCITY.velocity(0xFFFF_FF9C);
        assert!(close(v.rad_per_s(), -rpm(22.9)));
        assert_eq!(MX64_VELOCITY.velocity_to_raw(v), Ok(0xFFFF_FF9C));
    }

    #[test]
    fn direction_bit_gives_cw_on_p1() {
        let ccw = AX_SPEED.velocity(100);
        let cw = AX_SPEED.velocity(1124);
        assert_eq!(cw.rad_per_s(), -ccw.rad_per_s());
        assert_eq!(AX_SPEED.velocity_to_raw(cw), Ok(1124));
    }

    #[test]
    fn non_finite_and_too_fast_are_refused() {
        assert_eq!(
            AX_SPEED.velocity_to_raw(Velocity::from_rad_per_s(f32::NAN)),
            Err(ValidationError::NOT_FINITE)
        );
        assert_eq!(
            AX_SPEED.velocity_to_raw(Velocity::from_rad_per_s(rpm(0.111) * 1024.0)),
            Err(ValidationError::DoesNotFit {
                raw: 1024,
                size: 2,
                request: Request::NONE,
            })
        );
    }

    #[test]
    fn moving_speed_zero_is_unlimited() {
        assert_eq!(AX_MOVING_SPEED.speed_limit(0), SpeedLimit::Unlimited);
        assert_eq!(
            AX_MOVING_SPEED.speed_limit_to_raw(SpeedLimit::Unlimited),
            Ok(0)
        );
        let limited = AX_MOVING_SPEED.speed_limit(100);
        assert_eq!(limited, SpeedLimit::Limited(AX_MOVING_SPEED.velocity(100)));
        assert_eq!(AX_MOVING_SPEED.speed_limit_to_raw(limited), Ok(100));
    }

    #[test]
    fn a_limit_that_rounds_to_zero_is_refused() {
        let tiny = SpeedLimit::Limited(Velocity::from_rad_per_s(rpm(0.01)));
        assert_eq!(
            AX_MOVING_SPEED.speed_limit_to_raw(tiny),
            Err(ValidationError::DoesNotFit {
                raw: 0,
                size: 2,
                request: Request::NONE,
            })
        );
        assert_eq!(
            AX_MOVING_SPEED.speed_limit_to_raw(SpeedLimit::Limited(Velocity::ZERO)),
            Err(ValidationError::DoesNotFit {
                raw: 0,
                size: 2,
                request: Request::NONE,
            })
        );
    }

    #[test]
    fn without_the_flag_zero_is_zero() {
        assert_eq!(AX_SPEED.speed_limit(0), SpeedLimit::Limited(Velocity::ZERO));
        assert_eq!(
            AX_SPEED.speed_limit_to_raw(SpeedLimit::Limited(Velocity::ZERO)),
            Ok(0)
        );
        assert_eq!(
            AX_SPEED.speed_limit_to_raw(SpeedLimit::Unlimited),
            Err(ValidationError::DoesNotFit {
                raw: 0,
                size: 2,
                request: Request::NONE,
            })
        );
    }
}
