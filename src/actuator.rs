//! What every actuator model has in common.

use crate::Id;
use crate::error::ValidationError;
use crate::units::Limits;

/// Common interface of every actuator model.
///
/// Implemented by each generated model type and by [`ActuatorKind`], so code
/// that only needs the common features works with either.
///
/// [`ActuatorKind`]: crate::ActuatorKind
pub trait Actuator {
    /// Firmware ID of this actuator.
    fn id(&self) -> Id;

    /// Model number reported by the actuator (register 0).
    fn model_number(&self) -> u16;

    /// Display name, e.g. `"XM430-W350"`.
    fn name(&self) -> &'static str;
}

/// How a goal position is checked, decided at Setup from the actuator's
/// operating mode (architect, 2026-10-28).
///
/// The firmware applies the EEPROM position limits only in position mode.
/// The XM430-W350 e-Manual says of Min/Max Position Limit: "These values are
/// not used in Extended Position Control Mode and Current-based Position
/// Control Mode" (<https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>).
/// The goal check follows the firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
#[allow(
    dead_code,
    reason = "Unbounded and NoPosition are recorded by bus-1 Setup"
)]
pub(crate) enum LimitMode {
    /// Position (joint) mode: the goal must lie in the EEPROM limits read at
    /// Setup.
    #[default]
    Bounded = 0,
    /// Extended or current-based position, MX multi-turn: the firmware does
    /// not use the EEPROM limits; only the register size check applies.
    Unbounded = 1,
    /// Velocity, current, PWM, AX / XL-320 wheel mode: the firmware ignores
    /// Goal Position; a goal is refused.
    NoPosition = 2,
}

/// What core keeps per actuator: its ID, its limit mode and the position
/// limits read from its EEPROM at Setup.
///
/// It holds no torque state, no fault state and no fault report: the bus
/// keeps a [`TorqueState`](crate::TorqueState) per actuator beside it.
///
/// [`Cache::check_goal`] is the one limit check, for typed and raw goals
/// alike. Limits follow the operating mode: in [`LimitMode::Unbounded`]
/// only `Register::encode` stops a value that does not fit the register. No
/// constructor fills limits from a control table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cache {
    id: Id,
    #[allow(dead_code, reason = "read by check_goal, called by bus-1")]
    mode: LimitMode,
    limits: Option<Limits>,
}

impl Cache {
    /// The cache of actuator `id`: limit mode [`LimitMode::Bounded`], limits
    /// unknown.
    #[allow(dead_code, reason = "used by X1 / bus-1")]
    pub(crate) const fn new(id: Id) -> Self {
        Self {
            id,
            mode: LimitMode::Bounded,
            limits: None,
        }
    }

    /// The actuator's ID.
    #[allow(dead_code, reason = "used by X1 / bus-1")]
    pub(crate) const fn id(&self) -> Id {
        self.id
    }

    /// The limits read at Setup, or `None` before Setup read them.
    #[allow(dead_code, reason = "used by X1 / bus-1")]
    pub(crate) const fn limits(&self) -> Option<Limits> {
        self.limits
    }

    /// Records the limits `min..=max` read from the actuator's EEPROM.
    ///
    /// # Errors
    ///
    /// [`ValidationError::InvertedLimits`] if `min > max`; the recorded
    /// limits do not change.
    #[allow(dead_code, reason = "used by X1 / bus-1")]
    pub(crate) fn record_limits(&mut self, min: i32, max: i32) -> Result<Limits, ValidationError> {
        let limits = Limits::new(self.id, min, max)?;
        self.limits = Some(limits);
        Ok(limits)
    }

    /// Records the limit mode Setup found. The limits do not change.
    #[allow(dead_code, reason = "used by X1 / bus-1")]
    pub(crate) fn record_mode(&mut self, mode: LimitMode) {
        self.mode = mode;
    }

    /// Checks goal tick `raw` against the limit mode and returns it.
    ///
    /// - [`LimitMode::Bounded`]: `raw` must lie in the recorded limits, both
    ///   bounds included.
    /// - [`LimitMode::Unbounded`]: any `raw` passes.
    /// - [`LimitMode::NoPosition`]: every goal is refused.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::LimitsUnknown`]: bounded, and no limits were
    ///   recorded.
    /// - [`ValidationError::OutOfLimits`]: bounded, and `raw` is outside
    ///   them.
    /// - [`ValidationError::NotPositionMode`]: the mode ignores goal
    ///   positions, whether limits are known or not.
    #[allow(dead_code, reason = "used by X1 / bus-1")]
    pub(crate) fn check_goal(&self, raw: i32) -> Result<i32, ValidationError> {
        match (self.mode, self.limits) {
            (LimitMode::Bounded, None) => Err(ValidationError::LimitsUnknown { id: self.id }),
            (LimitMode::Bounded, Some(limits))
                if raw < limits.min_raw() || raw > limits.max_raw() =>
            {
                Err(ValidationError::OutOfLimits {
                    id: self.id,
                    raw,
                    min: limits.min_raw(),
                    max: limits.max_raw(),
                })
            }
            (LimitMode::Bounded | LimitMode::Unbounded, _) => Ok(raw),
            (LimitMode::NoPosition, _) => Err(ValidationError::NotPositionMode { id: self.id }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActuatorKind, Error};

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    #[test]
    #[cfg(feature = "x-series")]
    fn builds_model_from_number() {
        let kind = ActuatorKind::new(1020, 45, id(3)).unwrap();
        assert!(matches!(kind, ActuatorKind::Xm430W350(_)));
        assert_eq!(kind.id(), id(3));
        assert_eq!(kind.model_number(), 1020);
        assert_eq!(kind.name(), "XM430-W350");
    }

    #[test]
    #[cfg(feature = "x-series")]
    fn picks_table_by_firmware() {
        let old = ActuatorKind::new(1190, 52, id(1)).unwrap();
        let new = ActuatorKind::new(1190, 53, id(1)).unwrap();
        assert!(matches!(old, ActuatorKind::Xl330M077Fw52(_)));
        assert!(matches!(new, ActuatorKind::Xl330M077(_)));
        assert_eq!(old.name(), new.name());
    }

    #[test]
    #[cfg(feature = "ax")]
    fn concrete_types_implement_actuator() {
        let ActuatorKind::Ax12a(ax) = ActuatorKind::new(12, 0, id(9)).unwrap() else {
            panic!("model 12 is an AX-12A");
        };
        assert_eq!(
            (ax.id(), ax.model_number(), ax.name()),
            (id(9), 12, "AX-12A")
        );
    }

    #[test]
    fn unknown_model_is_unsupported() {
        assert_eq!(
            ActuatorKind::new(9999, 0, id(1)),
            Err(Error::UnsupportedModel {
                model_number: 9999,
                feature: None
            })
        );
    }

    #[test]
    fn unsupported_model_message_names_the_feature() {
        extern crate std;
        use std::string::ToString;
        let with = Error::UnsupportedModel {
            model_number: 1020,
            feature: Some("x-series"),
        };
        let without = Error::UnsupportedModel {
            model_number: 9999,
            feature: None,
        };
        assert_eq!(
            with.to_string(),
            "unsupported model number 1020 (enable the `x-series` feature)"
        );
        assert_eq!(without.to_string(), "unsupported model number 9999");
    }

    mod cache {
        use core::f32::consts::PI;

        use super::*;
        use crate::units::{Limits, Position, PositionScale};

        fn limited(id: Id) -> Cache {
            let mut cache = Cache::new(id);
            cache.record_limits(1000, 3000).unwrap();
            cache
        }

        #[test]
        fn goal_outside_limits_names_id_tick_and_limits() {
            let cache = limited(id(3));
            for raw in [3001, 999, i32::MIN, i32::MAX] {
                assert_eq!(
                    cache.check_goal(raw),
                    Err(ValidationError::OutOfLimits {
                        id: id(3),
                        raw,
                        min: 1000,
                        max: 3000
                    })
                );
            }
        }

        #[test]
        fn goals_on_the_bounds_pass() {
            let cache = limited(id(3));
            for raw in [1000, 2048, 3000] {
                assert_eq!(cache.check_goal(raw), Ok(raw));
            }
        }

        #[test]
        fn unknown_limits_refuse_every_goal() {
            let cache = Cache::new(id(3));
            assert_eq!(cache.id(), id(3));
            assert_eq!(cache.limits(), None);
            for raw in [2048, i32::MIN, i32::MAX] {
                assert_eq!(
                    cache.check_goal(raw),
                    Err(ValidationError::LimitsUnknown { id: id(3) })
                );
            }
        }

        #[test]
        fn recorded_limits_are_kept() {
            let mut cache = Cache::new(id(3));
            let limits = Limits::new(id(3), 1000, 3000).unwrap();
            assert_eq!(cache.record_limits(1000, 3000), Ok(limits));
            assert_eq!(cache.limits(), Some(limits));
        }

        #[test]
        fn inverted_record_leaves_limits_unknown() {
            let inverted = Err(ValidationError::InvertedLimits {
                id: id(3),
                min: 3000,
                max: 1000,
            });
            let mut cache = Cache::new(id(3));
            assert_eq!(cache.record_limits(3000, 1000), inverted);
            assert_eq!(cache.limits(), None);
            let mut cache = limited(id(3));
            assert_eq!(cache.record_limits(3000, 1000), inverted);
            assert_eq!(cache.limits(), Limits::new(id(3), 1000, 3000).ok());
        }

        #[test]
        fn typed_goal_is_checked_on_the_rounded_tick() {
            let scale = PositionScale::new(2048, 0, 4095, -PI, PI);
            let at = |tick: f32| Position::from_radians((tick - 2048.0) * scale.radians_per_tick());
            let cache = limited(id(3));
            let over = scale.to_raw(at(3000.6)).unwrap();
            assert_eq!(over, 3001);
            assert_eq!(
                cache.check_goal(over),
                Err(ValidationError::OutOfLimits {
                    id: id(3),
                    raw: 3001,
                    min: 1000,
                    max: 3000
                })
            );
            let inside = scale.to_raw(at(3000.4)).unwrap();
            assert_eq!(cache.check_goal(inside), Ok(3000));
        }
    }

    mod limit_mode {
        use super::*;

        fn limited(id: Id) -> Cache {
            let mut cache = Cache::new(id);
            cache.record_limits(1000, 3000).unwrap();
            cache
        }

        #[test]
        fn new_cache_is_bounded() {
            assert_eq!(LimitMode::default(), LimitMode::Bounded);
            assert_eq!(Cache::new(id(3)).mode, LimitMode::Bounded);
            assert_eq!(
                Cache::new(id(3)).check_goal(2048),
                Err(ValidationError::LimitsUnknown { id: id(3) })
            );
        }

        #[test]
        fn mode_fits_the_padding() {
            assert_eq!(size_of::<LimitMode>(), 1);
            assert_eq!(size_of::<Cache>(), 16);
            assert_eq!(LimitMode::Bounded as u8, 0);
            assert_eq!(LimitMode::Unbounded as u8, 1);
            assert_eq!(LimitMode::NoPosition as u8, 2);
        }

        #[test]
        fn bounded_keeps_the_limit_check() {
            let mut cache = limited(id(3));
            cache.record_mode(LimitMode::Bounded);
            assert_eq!(
                cache.check_goal(3001),
                Err(ValidationError::OutOfLimits {
                    id: id(3),
                    raw: 3001,
                    min: 1000,
                    max: 3000
                })
            );
            assert_eq!(cache.check_goal(3000), Ok(3000));
        }

        #[test]
        fn unbounded_skips_the_limits() {
            let mut cache = limited(id(3));
            cache.record_mode(LimitMode::Unbounded);
            for raw in [5000, 999, i32::MIN, i32::MAX] {
                assert_eq!(cache.check_goal(raw), Ok(raw));
            }
            let mut unknown = Cache::new(id(3));
            unknown.record_mode(LimitMode::Unbounded);
            assert_eq!(unknown.check_goal(2048), Ok(2048));
        }

        #[test]
        fn no_position_refuses_every_goal() {
            let refused = Err(ValidationError::NotPositionMode { id: id(4) });
            let mut known = limited(id(4));
            known.record_mode(LimitMode::NoPosition);
            let mut unknown = Cache::new(id(4));
            unknown.record_mode(LimitMode::NoPosition);
            for cache in [known, unknown] {
                for raw in [2000, 1000, i32::MIN, i32::MAX] {
                    assert_eq!(cache.check_goal(raw), refused);
                }
            }
        }

        #[test]
        fn record_mode_keeps_limits() {
            let mut cache = limited(id(3));
            let limits = cache.limits();
            for mode in [
                LimitMode::Unbounded,
                LimitMode::NoPosition,
                LimitMode::Bounded,
            ] {
                cache.record_mode(mode);
                assert_eq!(cache.mode, mode);
                assert_eq!(cache.limits(), limits);
            }
            assert!(matches!(
                cache.check_goal(3001),
                Err(ValidationError::OutOfLimits { .. })
            ));
        }
    }
}
