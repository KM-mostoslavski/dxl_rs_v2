//! [`Current`] and [`Voltage`].

use crate::error::ValidationError;

use super::scale::RegisterScale;

/// An electric current in amperes, held as an `f32`. Positive is CCW
/// torque.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Current(f32);

impl Current {
    /// No current.
    pub const ZERO: Self = Self(0.0);

    /// The current of `amps` amperes.
    pub const fn from_amps(amps: f32) -> Self {
        Self(amps)
    }

    /// The current in amperes.
    pub const fn amps(self) -> f32 {
        self.0
    }
}

/// A voltage in volts, held as an `f32`. Read-only: the input voltage is
/// measured, never written.
///
/// ```compile_fail,E0599
/// fn write(scale: dxl_rs_v2::RegisterScale, v: dxl_rs_v2::Voltage) {
///     let _ = scale.voltage_to_raw(v);
/// }
/// ```
///
/// [`Default`] is zero, to fill a caller-owned buffer before a
/// multi-actuator read.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Voltage(f32);

impl Voltage {
    /// The voltage in volts.
    pub const fn volts(self) -> f32 {
        self.0
    }
}

impl RegisterScale {
    /// The current of raw value `raw`.
    pub const fn current(&self, raw: u32) -> Current {
        Current(self.si_of(raw))
    }

    /// The raw value nearest to `current`.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::NotFinite`] if the current is NaN or infinite.
    /// - [`ValidationError::DoesNotFit`] if it does not fit the register.
    pub fn current_to_raw(&self, current: Current) -> Result<u32, ValidationError> {
        self.raw_of_si(current.0)
    }

    /// The voltage of raw value `raw`.
    pub const fn voltage(&self, raw: u32) -> Voltage {
        Voltage(self.si_of(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::super::scale::Sign;
    use super::super::scale::tests::MX64_CURRENT;
    use super::*;

    #[test]
    fn mx64_current_is_3_36_ma_per_step() {
        let c = MX64_CURRENT.current(0xFFF6);
        assert!((c.amps() - -0.0336).abs() < 1e-6, "{c:?}");
        assert_eq!(MX64_CURRENT.current_to_raw(c), Ok(0xFFF6));
        assert_eq!(MX64_CURRENT.current_to_raw(Current::ZERO), Ok(0));
        assert_eq!(
            MX64_CURRENT.current_to_raw(Current::from_amps(f32::INFINITY)),
            Err(ValidationError::NotFinite)
        );
        assert!(
            MX64_CURRENT
                .current_to_raw(Current::from_amps(200.0))
                .is_err()
        );
    }

    #[test]
    fn voltage_is_scaled() {
        let scale = RegisterScale::new(0.1, Sign::Unsigned, 2, false);
        assert!((scale.voltage(120).volts() - 12.0).abs() < 1e-5);
    }
}
