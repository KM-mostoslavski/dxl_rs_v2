//! [`Gain`], [`ComplianceMargin`] and [`ComplianceSlope`]: tuning values,
//! model-specific (Tier 2).

use crate::error::{Request, ValidationError};

use super::position::{Position, PositionScale};

/// A position P or D gain, in the register's own units. Gain scales differ
/// between families and are not in the tables' units, so no SI form is
/// offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Gain(u16);

impl Gain {
    /// Gain `value` for a `size`-byte gain register.
    ///
    /// # Errors
    ///
    /// [`ValidationError::DoesNotFit`] if `value` does not fit `size` bytes.
    pub const fn new(value: u16, size: u8) -> Result<Self, ValidationError> {
        if size == 1 && value > 0xFF {
            Err(ValidationError::DoesNotFit {
                raw: value as i64,
                size,
                request: Request::NONE,
            })
        } else {
            Ok(Self(value))
        }
    }

    /// The gain read from a register.
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    /// The raw register value.
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// AX / RX CW or CCW Compliance Margin: the position error the actuator
/// allows before it pushes back, in position ticks. AX-12A: "The range of
/// the value is 0~255, and the unit is the same as Goal Position"
/// (<https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ComplianceMargin(u8);

impl ComplianceMargin {
    /// The margin of raw register value `raw`.
    pub const fn from_raw(raw: u8) -> Self {
        Self(raw)
    }

    /// The raw register value, in position ticks.
    pub const fn raw(self) -> u8 {
        self.0
    }

    /// The margin as an angle, with the model's position scale.
    pub fn to_position(self, scale: &PositionScale) -> Position {
        Position::from_radians(f32::from(self.0) * scale.radians_per_tick())
    }
}

/// AX / RX CW or CCW Compliance Slope, as one of the seven e-Manual steps.
///
/// AX-12A Compliance Slope (<https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>):
///
/// | Step | Data value | Representative value |
/// |---|---|---|
/// | 1 | 0–3 | 2 |
/// | 2 | 4–7 | 4 |
/// | 3 | 8–15 | 8 |
/// | 4 | 16–31 | 16 |
/// | 5 | 32–63 | 32 |
/// | 6 | 64–127 | 64 |
/// | 7 | 128–254 | 128 |
///
/// Reading never fails, since the firmware may hold any byte; 255, which
/// the table does not list, reads as step 7. Writing sends only the step's
/// representative value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComplianceSlope(u8);

impl ComplianceSlope {
    /// Step `step`, `1..=7`, written as its representative value
    /// `2^step`.
    ///
    /// # Errors
    ///
    /// [`ValidationError::InvalidSlopeStep`] if `step` is outside `1..=7`.
    pub const fn new(step: u8) -> Result<Self, ValidationError> {
        if step >= 1 && step <= 7 {
            Ok(Self(1 << step))
        } else {
            Err(ValidationError::InvalidSlopeStep {
                step,
                request: Request::NONE,
            })
        }
    }

    /// The slope of raw register value `raw`.
    pub const fn from_raw(raw: u8) -> Self {
        Self(raw)
    }

    /// The raw register value.
    pub const fn raw(self) -> u8 {
        self.0
    }

    /// The e-Manual step the raw value falls in, `1..=7`.
    pub const fn step(self) -> u8 {
        if self.0 < 4 {
            1
        } else {
            (7 - self.0.leading_zeros()) as u8
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_byte_gain_stops_at_255() {
        assert_eq!(Gain::new(255, 1).map(Gain::raw), Ok(255));
        assert_eq!(
            Gain::new(256, 1),
            Err(ValidationError::DoesNotFit {
                raw: 256,
                size: 1,
                request: Request::NONE,
            })
        );
        assert_eq!(Gain::new(16_383, 2).map(Gain::raw), Ok(16_383));
        assert_eq!(Gain::from_raw(7).raw(), 7);
    }

    #[test]
    fn margin_is_in_position_ticks() {
        let ax = PositionScale::new(512, 0, 1023, -150f32.to_radians(), 150f32.to_radians());
        let margin = ComplianceMargin::from_raw(1).to_position(&ax);
        assert!((margin.radians() - 0.005_12).abs() < 1e-4, "{margin:?}");
        assert_eq!(margin.radians(), ax.radians_per_tick());
        assert_eq!(ComplianceMargin::from_raw(3).raw(), 3);
    }

    #[test]
    fn slope_steps_follow_the_emanual_table() {
        let table = [
            (0, 1),
            (3, 1),
            (4, 2),
            (7, 2),
            (8, 3),
            (15, 3),
            (16, 4),
            (31, 4),
        ];
        let table2 = [
            (32, 5),
            (63, 5),
            (64, 6),
            (127, 6),
            (128, 7),
            (254, 7),
            (255, 7),
        ];
        for (raw, step) in table.into_iter().chain(table2) {
            assert_eq!(ComplianceSlope::from_raw(raw).step(), step, "raw {raw}");
        }
        for step in 1..=7 {
            let slope = ComplianceSlope::new(step).unwrap();
            assert_eq!(slope.raw(), 1 << step);
            assert_eq!(slope.step(), step);
        }
        assert_eq!(
            ComplianceSlope::new(0),
            Err(ValidationError::InvalidSlopeStep {
                step: 0,
                request: Request::NONE,
            })
        );
        assert_eq!(
            ComplianceSlope::new(8),
            Err(ValidationError::InvalidSlopeStep {
                step: 8,
                request: Request::NONE,
            })
        );
    }
}
