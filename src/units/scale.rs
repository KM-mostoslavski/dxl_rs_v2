//! [`RegisterScale`]: how one register of one model maps raw values to an
//! SI quantity.

use crate::error::{Request, ValidationError};

use super::position::round_half_away;

/// How a register encodes the sign of its value.
///
/// The e-Manual uses two encodings for signed registers. Protocol 1 speed
/// and load registers keep a magnitude below a direction bit: AX-12A Present
/// Speed(38), "0~1,023 … CCW direction … 1,024~2,047 … CW direction"
/// (<https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>). Protocol 2
/// registers such as Present Velocity are two's complement of the register
/// size. CCW is positive, as for [`Position`](crate::Position).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    /// No sign: the raw value is the magnitude.
    Unsigned,
    /// A magnitude in the bits below bit `n`; bit `n` set means CW
    /// (negative).
    DirectionBit(u8),
    /// Two's complement over the whole register.
    TwosComplement,
}

/// The scale of one register of one model: SI units per raw step, the sign
/// encoding and the register size.
///
/// There is no public constructor, so every scale comes from a table:
///
/// ```compile_fail,E0624
/// use dxl_rs_v2::{RegisterScale, Sign};
/// let scale = RegisterScale::new(0.1, Sign::Unsigned, 1, false);
/// ```
///
/// Read-only quantities ([`Voltage`](crate::Voltage),
/// [`Temperature`](crate::Temperature), [`Load`](crate::Load)) convert from
/// raw only. Writable ones ([`Velocity`](crate::Velocity),
/// [`SpeedLimit`](crate::SpeedLimit), [`Current`](crate::Current),
/// [`Pwm`](crate::Pwm)) also convert back, rounding to the nearest raw
/// step, and never truncate.
///
/// Values are `f32`: a raw value up to ±2^20 converts to SI and back to
/// itself; beyond it, a round trip may land one step off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegisterScale {
    /// SI units of one raw step.
    si_per_raw: f32,
    /// The inverse, so conversions to raw need no division.
    raw_per_si: f32,
    /// How the raw value carries its sign.
    sign: Sign,
    /// The register size in bytes: 1, 2 or 4.
    size: u8,
    /// Raw 0 means "no limit" (Protocol 1 Moving Speed in joint mode), not
    /// zero.
    zero_is_unlimited: bool,
}

impl RegisterScale {
    /// The scale of a `size`-byte register whose raw step is `si_per_raw`
    /// SI units, with sign encoding `sign`. With `zero_is_unlimited`, raw 0
    /// reads as [`SpeedLimit::Unlimited`](crate::SpeedLimit::Unlimited).
    #[allow(
        dead_code,
        reason = "built only by generated models, from the xtask part of FR-9"
    )]
    pub(crate) const fn new(
        si_per_raw: f32,
        sign: Sign,
        size: u8,
        zero_is_unlimited: bool,
    ) -> Self {
        Self {
            si_per_raw,
            raw_per_si: 1.0 / si_per_raw,
            sign,
            size,
            zero_is_unlimited,
        }
    }

    /// The register size in bytes.
    pub const fn size(&self) -> u8 {
        self.size
    }

    /// The sign encoding.
    pub const fn sign(&self) -> Sign {
        self.sign
    }

    /// SI units of one raw step.
    pub const fn si_per_raw(&self) -> f32 {
        self.si_per_raw
    }

    /// Whether raw 0 means "no limit".
    pub const fn zero_is_unlimited(&self) -> bool {
        self.zero_is_unlimited
    }

    /// The bits of the register, `raw` with every bit above the register
    /// size cleared.
    const fn bits(&self) -> u32 {
        match self.size {
            1 => 0xFF,
            2 => 0xFFFF,
            _ => u32::MAX,
        }
    }

    /// The signed value of raw register value `raw`.
    pub(crate) const fn signed_of(&self, raw: u32) -> i64 {
        let raw = raw & self.bits();
        match self.sign {
            Sign::Unsigned => raw as i64,
            Sign::DirectionBit(bit) => {
                let magnitude = (raw & ((1u32 << bit) - 1)) as i64;
                if raw & (1u32 << bit) != 0 {
                    -magnitude
                } else {
                    magnitude
                }
            }
            Sign::TwosComplement => match self.size {
                1 => raw as u8 as i8 as i64,
                2 => raw as u16 as i16 as i64,
                _ => raw as i32 as i64,
            },
        }
    }

    /// The raw register value of `signed`.
    ///
    /// # Errors
    ///
    /// [`ValidationError::DoesNotFit`] if `signed` is outside what the
    /// register encodes.
    pub(crate) const fn raw_of(&self, signed: i64) -> Result<u32, ValidationError> {
        let does_not_fit = ValidationError::DoesNotFit {
            raw: signed,
            size: self.size,
            request: Request::NONE,
        };
        let bits = self.bits() as i64;
        match self.sign {
            Sign::Unsigned => {
                if signed < 0 || signed > bits {
                    Err(does_not_fit)
                } else {
                    Ok(signed as u32)
                }
            }
            Sign::DirectionBit(bit) => {
                let max = (1i64 << bit) - 1;
                let direction = 1i64 << bit;
                if signed.abs() > max || direction > bits {
                    Err(does_not_fit)
                } else if signed < 0 {
                    Ok((-signed | direction) as u32)
                } else {
                    Ok(signed as u32)
                }
            }
            Sign::TwosComplement => {
                let half = (bits + 1) / 2;
                if signed < -half || signed >= half {
                    Err(does_not_fit)
                } else {
                    Ok((signed & bits) as u32)
                }
            }
        }
    }

    /// The value of raw `raw`, in SI units.
    pub(crate) const fn si_of(&self, raw: u32) -> f32 {
        self.signed_of(raw) as f32 * self.si_per_raw
    }

    /// The raw value nearest to `si` SI units, half a step rounding away
    /// from zero.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::NotFinite`] if `si` is NaN or infinite.
    /// - [`ValidationError::DoesNotFit`] if the raw value does not fit the
    ///   register.
    pub(crate) fn raw_of_si(&self, si: f32) -> Result<u32, ValidationError> {
        if !si.is_finite() {
            return Err(ValidationError::NOT_FINITE);
        }
        self.raw_of(round_half_away(si * self.raw_per_si))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// `rpm` in rad/s.
    pub(crate) const fn rpm(rpm: f32) -> f32 {
        rpm * core::f32::consts::TAU / 60.0
    }

    /// MX-64 (Protocol 2) Present/Goal Velocity: 0.229 rpm, 4 bytes.
    pub(crate) const MX64_VELOCITY: RegisterScale =
        RegisterScale::new(rpm(0.229), Sign::TwosComplement, 4, false);
    /// AX-12A Present Speed: 0.111 rpm, direction bit 10, 2 bytes.
    pub(crate) const AX_SPEED: RegisterScale =
        RegisterScale::new(rpm(0.111), Sign::DirectionBit(10), 2, false);
    /// AX-12A Moving Speed in joint mode: raw 0 is "no limit".
    pub(crate) const AX_MOVING_SPEED: RegisterScale =
        RegisterScale::new(rpm(0.111), Sign::Unsigned, 2, true);
    /// MX-64 (Protocol 2) Present/Goal Current: 3.36 mA, 2 bytes.
    pub(crate) const MX64_CURRENT: RegisterScale =
        RegisterScale::new(0.003_36, Sign::TwosComplement, 2, false);

    #[test]
    fn unsigned_is_the_raw_value() {
        let scale = RegisterScale::new(1.0, Sign::Unsigned, 2, false);
        assert_eq!(scale.signed_of(0), 0);
        assert_eq!(scale.signed_of(1124), 1124);
        assert_eq!(scale.signed_of(0xFFFF), 0xFFFF);
        assert_eq!(scale.raw_of(0xFFFF), Ok(0xFFFF));
        assert_eq!(
            scale.raw_of(0x1_0000),
            Err(ValidationError::DoesNotFit {
                raw: 0x1_0000,
                size: 2,
                request: Request::NONE,
            })
        );
        assert_eq!(
            scale.raw_of(-1),
            Err(ValidationError::DoesNotFit {
                raw: -1,
                size: 2,
                request: Request::NONE,
            })
        );
    }

    #[test]
    fn direction_bit_set_is_negative() {
        assert_eq!(AX_SPEED.signed_of(100), 100);
        assert_eq!(AX_SPEED.signed_of(1124), -100);
        assert_eq!(AX_SPEED.signed_of(1024), 0);
        assert_eq!(AX_SPEED.signed_of(2047), -1023);
        assert_eq!(AX_SPEED.raw_of(-100), Ok(1124));
        assert_eq!(AX_SPEED.raw_of(1023), Ok(1023));
        assert_eq!(
            AX_SPEED.raw_of(1024),
            Err(ValidationError::DoesNotFit {
                raw: 1024,
                size: 2,
                request: Request::NONE,
            })
        );
        assert_eq!(
            AX_SPEED.raw_of(-1024),
            Err(ValidationError::DoesNotFit {
                raw: -1024,
                size: 2,
                request: Request::NONE,
            })
        );
    }

    #[test]
    fn direction_bit_above_the_register_never_fits() {
        let scale = RegisterScale::new(1.0, Sign::DirectionBit(10), 1, false);
        assert!(scale.raw_of(0).is_err());
    }

    #[test]
    fn twos_complement_sign_extends_from_the_register_size() {
        let one = RegisterScale::new(1.0, Sign::TwosComplement, 1, false);
        assert_eq!(one.signed_of(0xFF), -1);
        assert_eq!(one.raw_of(-128), Ok(0x80));
        assert!(one.raw_of(128).is_err());
        assert_eq!(MX64_CURRENT.signed_of(0xFFFF), -1);
        assert_eq!(MX64_CURRENT.signed_of(0x7FFF), 32_767);
        assert_eq!(MX64_CURRENT.raw_of(-1), Ok(0xFFFF));
        assert_eq!(
            MX64_CURRENT.raw_of(32_768),
            Err(ValidationError::DoesNotFit {
                raw: 32_768,
                size: 2,
                request: Request::NONE,
            })
        );
        assert_eq!(MX64_VELOCITY.signed_of(0xFFFF_FFFF), -1);
        assert_eq!(MX64_VELOCITY.raw_of(i64::from(i32::MIN)), Ok(0x8000_0000));
        assert!(MX64_VELOCITY.raw_of(i64::from(i32::MAX) + 1).is_err());
    }

    #[test]
    fn bits_above_the_register_are_ignored() {
        assert_eq!(MX64_CURRENT.signed_of(0x1_0005), 5);
    }

    #[test]
    fn non_finite_si_is_refused() {
        for si in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                MX64_VELOCITY.raw_of_si(si),
                Err(ValidationError::NOT_FINITE)
            );
        }
    }

    #[test]
    fn accessors_return_the_table_values() {
        assert_eq!(AX_SPEED.size(), 2);
        assert_eq!(AX_SPEED.sign(), Sign::DirectionBit(10));
        assert_eq!(AX_SPEED.si_per_raw(), rpm(0.111));
        assert!(!AX_SPEED.zero_is_unlimited());
        assert!(AX_MOVING_SPEED.zero_is_unlimited());
    }

    /// Every raw value converts to SI and back to itself (SC-002): sizes 1
    /// and 2 exhaustively, size 4 sampled up to ±2^20, where an `f32` still
    /// holds every step exactly enough; real velocity and current ranges
    /// stay far below it.
    #[test]
    fn every_raw_value_round_trips() {
        let small = [
            AX_SPEED,
            MX64_CURRENT,
            RegisterScale::new(0.1, Sign::Unsigned, 1, false),
            RegisterScale::new(rpm(0.114), Sign::TwosComplement, 1, false),
        ];
        for scale in small {
            let raws: &[core::ops::RangeInclusive<u32>] = match scale.sign {
                Sign::DirectionBit(_) => &[0..=1023, 1025..=2047],
                _ => &[0..=scale.bits()],
            };
            for raw in raws.iter().cloned().flatten() {
                assert_eq!(
                    scale.raw_of_si(scale.si_of(raw)),
                    Ok(raw),
                    "{scale:?} raw {raw}"
                );
            }
        }
        for raw in (0..=u32::MAX).step_by(65_537) {
            let signed = MX64_VELOCITY.signed_of(raw);
            if signed.unsigned_abs() <= 1 << 20 {
                assert_eq!(
                    MX64_VELOCITY.raw_of_si(MX64_VELOCITY.si_of(raw)),
                    Ok(raw),
                    "raw {raw}"
                );
            }
        }
    }
}
