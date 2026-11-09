//! [`Pwm`].

use crate::error::ValidationError;

use super::scale::RegisterScale;

/// A PWM output: a ratio of the full output, 1.0 being 100 %, held as an
/// `f32`. Positive is CCW, as for [`Position`](crate::Position).
///
/// It is a ratio, not a voltage. The step of one raw value comes from the
/// model's control table (`unit: "%"`), so a model whose table gives no
/// unit has no typed form, only raw.
///
/// Any `f32` can be stored. NaN and infinity are refused when converted to
/// a raw value, and a value outside the register's e-Manual range is
/// refused before anything is sent.
///
/// ```
/// use dxl_rs_v2::Pwm;
///
/// let half = Pwm::from_ratio(0.5);
/// assert_eq!(half.ratio(), 0.5);
/// ```
///
/// [`Default`] is zero, to fill a caller-owned buffer before a
/// multi-actuator read.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Pwm(f32);

impl Pwm {
    /// The PWM output of `ratio` times the full output: 1.0 is 100 %.
    pub const fn from_ratio(ratio: f32) -> Self {
        Self(ratio)
    }

    /// The PWM output as a ratio of the full output.
    pub const fn ratio(self) -> f32 {
        self.0
    }
}

impl RegisterScale {
    /// The PWM output of raw value `raw`.
    pub const fn pwm(&self, raw: u32) -> Pwm {
        // DECISION: no clamp to ±1.0, unlike Load: a table's "about 0.113 %" step puts raw 885 at 1.00005, and a clamp would hide that value.
        Pwm(self.si_of(raw))
    }

    /// The raw value nearest to `pwm`.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::NotFinite`] if the ratio is NaN or infinite.
    /// - [`ValidationError::DoesNotFit`] if it does not fit the register.
    pub fn pwm_to_raw(&self, pwm: Pwm) -> Result<u32, ValidationError> {
        self.raw_of_si(pwm.0)
    }
}

#[cfg(test)]
mod tests {
    use super::super::scale::Sign;
    use super::*;
    use crate::error::Request;

    /// XM430-W350 PWM Limit: 0.113 %, 2 bytes, 0 to 885.
    const XM_PWM_LIMIT: RegisterScale = RegisterScale::new(0.001_13, Sign::Unsigned, 2, false);

    /// A signed PWM register (Goal PWM): 0.113 %, two's complement.
    const XM_GOAL_PWM: RegisterScale = RegisterScale::new(0.001_13, Sign::TwosComplement, 2, false);

    #[test]
    fn ratio_round_trips() {
        assert_eq!(Pwm::from_ratio(0.25).ratio(), 0.25);
        assert_eq!(Pwm::default().ratio(), 0.0);
        assert!(Pwm::from_ratio(0.5) < Pwm::from_ratio(0.6));
    }

    #[test]
    fn xm_pwm_is_0_113_percent_per_step() {
        assert_eq!(XM_PWM_LIMIT.pwm(0).ratio(), 0.0);
        assert!((XM_PWM_LIMIT.pwm(885).ratio() - 1.000_05).abs() < 1e-5);
        assert!((XM_PWM_LIMIT.pwm(100).ratio() - 0.113).abs() < 1e-6);
        assert_eq!(XM_PWM_LIMIT.pwm_to_raw(Pwm::from_ratio(1.0)), Ok(885));
        assert_eq!(XM_PWM_LIMIT.pwm_to_raw(Pwm::from_ratio(0.5)), Ok(442));
        assert_eq!(XM_PWM_LIMIT.pwm_to_raw(Pwm::default()), Ok(0));
    }

    #[test]
    fn signed_pwm_is_negative_cw() {
        let full_cw = XM_GOAL_PWM.pwm(0x1_0000 - 885).ratio();
        assert!((full_cw - -1.000_05).abs() < 1e-5, "{full_cw}");
        assert_eq!(
            XM_GOAL_PWM.pwm_to_raw(Pwm::from_ratio(-1.0)),
            Ok(0x1_0000 - 885)
        );
    }

    #[test]
    fn bad_ratios_are_refused() {
        for ratio in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                XM_PWM_LIMIT.pwm_to_raw(Pwm::from_ratio(ratio)),
                Err(ValidationError::NOT_FINITE)
            );
        }
        assert_eq!(
            XM_PWM_LIMIT.pwm_to_raw(Pwm::from_ratio(-0.5)),
            Err(ValidationError::DoesNotFit {
                raw: -442,
                size: 2,
                request: Request::NONE,
            })
        );
        assert!(XM_PWM_LIMIT.pwm_to_raw(Pwm::from_ratio(100.0)).is_err());
    }

    #[test]
    fn every_raw_value_round_trips() {
        for raw in 0..=885 {
            assert_eq!(
                XM_PWM_LIMIT.pwm_to_raw(XM_PWM_LIMIT.pwm(raw)),
                Ok(raw),
                "raw {raw}"
            );
        }
    }
}
