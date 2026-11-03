//! [`Temperature`].

use super::scale::RegisterScale;

/// A temperature in degrees Celsius, held as an `f32`. Read-only: the
/// internal temperature is measured, never written.
///
/// ```compile_fail,E0599
/// fn write(scale: dxl_rs_v2::RegisterScale, t: dxl_rs_v2::Temperature) {
///     let _ = scale.temperature_to_raw(t);
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Temperature(f32);

impl Temperature {
    /// The temperature in degrees Celsius.
    pub const fn celsius(self) -> f32 {
        self.0
    }
}

impl RegisterScale {
    /// The temperature of raw value `raw`.
    pub const fn temperature(&self, raw: u32) -> Temperature {
        Temperature(self.si_of(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::super::scale::Sign;
    use super::*;

    #[test]
    fn one_degree_per_step() {
        let scale = RegisterScale::new(1.0, Sign::Unsigned, 1, false);
        assert_eq!(scale.temperature(45).celsius(), 45.0);
    }
}
