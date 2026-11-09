//! [`Load`].

use super::scale::RegisterScale;

/// A Protocol 1 load: the share of the maximum torque in use, from −1.0
/// (full CW) to 1.0 (full CCW). Present Load reads as one; Max Torque, the
/// EEPROM bound of the output, is one from 0.0 to 1.0, written with
/// [`ActuatorRef::set_max_torque`](crate::ActuatorRef::set_max_torque).
///
/// It is a ratio, not a torque: AX-12A Present Load(40) "is an inferred
/// value based on the internal output value; not a measured value using
/// torque sensor", values 0~1,023 CCW and 1,024~2,047 CW, "the 10th bit becomes
/// the direction bit" (<https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>).
///
/// A [`RegisterScale`] converts it from raw only: the bus converts a Max
/// Torque to its raw value, after its own checks.
///
/// ```compile_fail,E0599
/// fn write(scale: dxl_rs_v2::RegisterScale, l: dxl_rs_v2::Load) {
///     let _ = scale.load_to_raw(l);
/// }
/// ```
///
/// [`Default`] is zero, to fill a caller-owned buffer before a
/// multi-actuator read.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Load(f32);

impl Load {
    /// The load of `ratio` times the maximum torque: 1.0 is the full
    /// torque, CCW.
    pub const fn from_ratio(ratio: f32) -> Self {
        Self(ratio)
    }

    /// The load as a ratio of the maximum torque, in `-1.0..=1.0` when read.
    pub const fn ratio(self) -> f32 {
        self.0
    }
}

impl RegisterScale {
    /// The load of raw value `raw`, clamped to `-1.0..=1.0`.
    pub const fn load(&self, raw: u32) -> Load {
        Load(self.si_of(raw).clamp(-1.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::super::scale::Sign;
    use super::*;

    const AX_LOAD: RegisterScale =
        RegisterScale::new(1.0 / 1023.0, Sign::DirectionBit(10), 2, false);

    #[test]
    fn full_scale_is_one() {
        assert_eq!(AX_LOAD.load(1023).ratio(), 1.0);
        assert_eq!(AX_LOAD.load(2047).ratio(), -1.0);
        assert_eq!(AX_LOAD.load(0).ratio(), 0.0);
        assert_eq!(AX_LOAD.load(1024).ratio(), 0.0);
        assert!((AX_LOAD.load(512).ratio() - 0.5).abs() < 1e-3);
        assert_eq!(Load::from_ratio(0.25).ratio(), 0.25);
    }
}
