//! FR-9 register roles: which register of a model holds which typed
//! quantity, and the scale its control table gives for it in each limit
//! mode.
//!
//! Codegen fills one [`RoleRegister`] per role a model's table has. Tier 1
//! code looks a role up at run time through [`ModelData::roles`]; Tier 2
//! code names it at compile time through the model type's `Has*` trait
//! impls, which exist only for the roles the model has.
//!
//! [`ModelData::roles`]: super::ModelData

use core::ops::RangeInclusive;

use super::{Actuator, LimitMode};
use crate::register::Register;
use crate::units::RegisterScale;

/// A set of [`LimitMode`]s, one bit per mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeSet(u8);

impl ModeSet {
    /// Every limit mode.
    #[allow(dead_code, reason = "built only by generated models")]
    pub(crate) const ALL: Self = Self(0x7F);

    /// The set of `modes`.
    #[allow(dead_code, reason = "built only by generated models")]
    pub(crate) const fn of(mut modes: &[LimitMode]) -> Self {
        let mut bits = 0;
        while let [first, rest @ ..] = modes {
            bits |= 1 << *first as u8;
            modes = rest;
        }
        Self(bits)
    }

    /// Whether `mode` is in the set.
    pub(crate) const fn contains(self, mode: LimitMode) -> bool {
        self.0 & (1 << mode as u8) != 0
    }
}

/// What one `unit_info` entry of the model's table gives a register in some
/// limit modes: its scale, and the raw range the e-Manual gives there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ModeScale {
    /// The modes the entry applies in.
    pub(crate) modes: ModeSet,
    /// Raw to SI; `None` where the entry's unit is not the role's quantity
    /// (AX-12A wheel-mode Moving Speed, in %), so the register is raw only
    /// there but keeps the entry's range.
    pub(crate) scale: Option<RegisterScale>,
    /// The raw range the e-Manual gives in these modes, if it gives one.
    pub(crate) range: Option<(i64, i64)>,
}

/// One FR-9 register of one model: where it is, and how its raw value maps
/// to a typed value in each limit mode.
///
/// Built only by the generated models; read it through a model's `Has*`
/// trait, e.g. `<Xm430W350 as HasPresentTemperature>::ROLE`.
///
/// A role with no scale in a mode has no typed form there, only raw
/// (FR-9.2): its table gives no unit for that mode, or a unit that is not
/// the role's quantity (AX-12A wheel-mode Moving Speed is a share of output
/// in %, not a velocity).
#[cfg_attr(
    all(feature = "ax", feature = "x-series"),
    doc = r#"
```
use dxl_rs_v2::LimitMode;
use dxl_rs_v2::models::{Ax12a, HasPresentTemperature, HasSpeedLimit, Xm430W350};

let role = <Xm430W350 as HasPresentTemperature>::ROLE;
assert_eq!((role.name(), role.address(), role.size()), ("Present Temperature", 146, 1));
let celsius = role.scale(LimitMode::Bounded).unwrap().temperature(40).celsius();
assert_eq!(celsius, 40.0);

let moving_speed = <Ax12a as HasSpeedLimit>::ROLE;
assert!(moving_speed.scale(LimitMode::Bounded).is_some());
assert!(moving_speed.scale(LimitMode::Wheel).is_none());
```

A model without the register has no impl, so naming it fails to
compile (the AX-12A has no Present Current):

```compile_fail,E0277
use dxl_rs_v2::models::{Ax12a, HasPresentCurrent};

let _ = <Ax12a as HasPresentCurrent>::ROLE;
```
"#
)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoleRegister {
    /// The register's name, as the control table writes it.
    pub(crate) name: &'static str,
    /// Where it is.
    pub(crate) register: Register,
    /// Its `unit_info` entries; at most one applies in each mode. Empty, or
    /// no scale in the entry of a mode: raw only there.
    pub(crate) scales: &'static [ModeScale],
    /// The EEPROM register that bounds its magnitude (Velocity Limit for
    /// Goal Velocity, Current Limit for Goal Current), if any.
    pub(crate) bounded_by: Option<Register>,
    /// The first firmware version that has the register; 0 for all.
    pub(crate) from_fw: u8,
    /// The raw range the e-Manual gives for the register in every mode
    /// (gains, compliance slopes, a mode switch), if the table records one;
    /// a scale's own range wins in that scale's modes.
    pub(crate) row_range: Option<(i64, i64)>,
}

impl RoleRegister {
    /// The register's name, as the control table writes it, e.g.
    /// `"Present Input Voltage"`.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The register's control-table address.
    pub const fn address(&self) -> u16 {
        self.register.addr
    }

    /// The register's width in bytes.
    pub const fn size(&self) -> u8 {
        self.register.size
    }

    /// The first firmware version that has the register, from the model's
    /// table; 0 when every firmware has it. MX-64 on Protocol 1 has
    /// Realtime Tick from firmware 40 on.
    pub const fn from_firmware(&self) -> u8 {
        self.from_fw
    }

    /// The scale of the register in limit mode `mode`, or `None` when the
    /// table gives it no typed form in that mode.
    pub const fn scale(&self, mode: LimitMode) -> Option<RegisterScale> {
        match self.entry(mode) {
            Some(entry) => entry.scale,
            None => None,
        }
    }

    /// The raw range the e-Manual gives for the register in `mode`, or
    /// `None` when the table records none: the range of the `unit_info`
    /// entry that applies in `mode`, typed or not (AX-12A wheel-mode Moving
    /// Speed 0 to 2047), else the register's own range (MX-64 Protocol 1
    /// P Gain 0 to 254, AX-12A Compliance Slope 0 to 254).
    pub const fn raw_range(&self, mode: LimitMode) -> Option<RangeInclusive<i64>> {
        match self.range_in(mode) {
            Some((min, max)) => Some(min..=max),
            None => None,
        }
    }

    /// The modulus of a wrapping counter such as Realtime Tick, for
    /// [`Tick::elapsed_since`](crate::Tick::elapsed_since): one above the
    /// top of its raw range, 0 meaning 65,536. `None` without a recorded
    /// range.
    pub const fn tick_modulus(&self, mode: LimitMode) -> Option<u16> {
        match self.range_in(mode) {
            Some((_, max)) => Some((max + 1) as u16),
            None => None,
        }
    }

    /// The raw range in `mode`, `(min, max)`.
    const fn range_in(&self, mode: LimitMode) -> Option<(i64, i64)> {
        match self.entry(mode) {
            Some(ModeScale {
                range: Some(range), ..
            }) => Some(*range),
            _ => self.row_range,
        }
    }

    /// The register.
    #[allow(dead_code, reason = "used by the bus part of FR-9")]
    pub(crate) const fn register(&self) -> Register {
        self.register
    }

    /// The EEPROM register bounding the value's magnitude, if any.
    #[allow(dead_code, reason = "used by the bus part of FR-9")]
    pub(crate) const fn bounded_by(&self) -> Option<Register> {
        self.bounded_by
    }

    /// The entry that applies in `mode`.
    const fn entry(&self, mode: LimitMode) -> Option<&'static ModeScale> {
        let mut scales = self.scales;
        while let [first, rest @ ..] = scales {
            if first.modes.contains(mode) {
                return Some(first);
            }
            scales = rest;
        }
        None
    }
}

/// Declares [`Role`], [`Roles`] and one `Has*` trait per role.
macro_rules! roles {
    ($(
        $(#[$doc:meta])*
        $variant:ident, $field:ident, $has:ident, $name:literal;
    )*) => {
        /// An FR-9 register role: the quantity a register holds, whatever
        /// its name and address in a given model.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[allow(dead_code, reason = "used by the bus part of FR-9")]
        pub(crate) enum Role {
            $( $(#[$doc])* $variant, )*
        }

        impl Role {
            /// Every role, in declaration order.
            #[allow(dead_code, reason = "used by the bus part of FR-9")]
            pub(crate) const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// The role's register name for an error about a model that
            /// lacks it, e.g. `"Present Temperature"`.
            #[allow(dead_code, reason = "used by the bus part of FR-9")]
            pub(crate) const fn name(self) -> &'static str {
                match self {
                    $( Self::$variant => $name, )*
                }
            }
        }

        /// The FR-9 registers of one model, one field per [`Role`]; `None`
        /// where the model's table lacks the register.
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub(crate) struct Roles {
            $(
                #[doc = concat!("The ", $name, " role.")]
                pub(crate) $field: Option<RoleRegister>,
            )*
        }

        impl Roles {
            /// No role at all.
            #[allow(dead_code, reason = "used by generated models")]
            pub(crate) const NONE: Self = Self { $( $field: None, )* };

            /// The register of `role`, or `None` when the model lacks it.
            pub(crate) const fn get(&self, role: Role) -> Option<&RoleRegister> {
                match role {
                    $( Role::$variant => self.$field.as_ref(), )*
                }
            }
        }

        $(
            #[doc = concat!(
                "A model whose control table has the ", $name,
                " register (FR-9 role `", stringify!($field), "`).\n\n",
                "Implemented only by the generated model types that have it, so a ",
                "bound on this trait fails to compile for a model without the register. ",
                "Sealed: only the crate's model types implement [`Actuator`]."
            )]
            pub trait $has: Actuator {
                /// Where the register is and how it scales.
                const ROLE: RoleRegister;
            }
        )*
    };
}

roles! {
    /// Present Temperature, °C.
    PresentTemperature, present_temperature, HasPresentTemperature, "Present Temperature";
    /// Present Voltage (Protocol 1, XL-320) or Present Input Voltage, V.
    PresentVoltage, present_voltage, HasPresentVoltage, "Present Voltage";
    /// Present Speed (Protocol 1) or Present Velocity, rad/s.
    PresentVelocity, present_velocity, HasPresentVelocity, "Present Velocity";
    /// Goal Velocity, or Moving Speed in wheel mode on the angle-limit
    /// models, rad/s.
    GoalVelocity, goal_velocity, HasGoalVelocity, "Goal Velocity";
    /// Profile Velocity, or Moving Speed in joint and multi-turn mode on the
    /// angle-limit models: a speed limit, rad/s.
    SpeedLimit, speed_limit, HasSpeedLimit, "Profile Velocity";
    /// Present Load, a ratio of the maximum torque.
    PresentLoad, present_load, HasPresentLoad, "Present Load";
    /// Present Current, A.
    PresentCurrent, present_current, HasPresentCurrent, "Present Current";
    /// Goal Current, or MX Protocol 1 Goal Torque, A.
    GoalCurrent, goal_current, HasGoalCurrent, "Goal Current";
    /// Moving, a flag.
    Moving, moving, HasMoving, "Moving";
    /// LED, on or off.
    Led, led, HasLed, "LED";
    /// LED Red, a brightness (PRO, RH-P12-RN).
    LedRed, led_red, HasLedRed, "LED Red";
    /// LED Green, a brightness (PRO, RH-P12-RN).
    LedGreen, led_green, HasLedGreen, "LED Green";
    /// LED Blue, a brightness (PRO, RH-P12-RN).
    LedBlue, led_blue, HasLedBlue, "LED Blue";
    /// Hardware Error Status, a bit set.
    HardwareErrorStatus, hardware_error_status, HasHardwareErrorStatus, "Hardware Error Status";
    /// Realtime Tick, a wrapping millisecond clock.
    RealtimeTick, realtime_tick, HasRealtimeTick, "Realtime Tick";
    /// Position P Gain, or P Gain (MX Protocol 1), in register units.
    PGain, p_gain, HasPGain, "Position P Gain";
    /// Position D Gain, or D Gain (MX Protocol 1), in register units.
    DGain, d_gain, HasDGain, "Position D Gain";
    /// CW Compliance Margin (AX, RX), position ticks.
    CwComplianceMargin, cw_compliance_margin, HasCwComplianceMargin, "CW Compliance Margin";
    /// CCW Compliance Margin (AX, RX), position ticks.
    CcwComplianceMargin, ccw_compliance_margin, HasCcwComplianceMargin, "CCW Compliance Margin";
    /// CW Compliance Slope (AX, RX), a step.
    CwComplianceSlope, cw_compliance_slope, HasCwComplianceSlope, "CW Compliance Slope";
    /// CCW Compliance Slope (AX, RX), a step.
    CcwComplianceSlope, ccw_compliance_slope, HasCcwComplianceSlope, "CCW Compliance Slope";
    /// MX Protocol 1 Torque Ctrl Mode Enable, a flag.
    TorqueCtrlModeEnable, torque_ctrl_mode_enable, HasTorqueCtrlModeEnable, "Torque Ctrl Mode Enable";
    /// Min Voltage Limit, V: the EEPROM bound below which the actuator
    /// reports an input voltage error.
    MinVoltageLimit, min_voltage_limit, HasMinVoltageLimit, "Min Voltage Limit";
    /// Max Voltage Limit, V: the EEPROM bound above which the actuator
    /// reports an input voltage error.
    MaxVoltageLimit, max_voltage_limit, HasMaxVoltageLimit, "Max Voltage Limit";
    /// Temperature Limit, °C: the EEPROM bound above which the actuator
    /// reports overheating.
    TemperatureLimit, temperature_limit, HasTemperatureLimit, "Temperature Limit";
    /// Min Position Limit, or CW Angle Limit on the angle-limit models: the
    /// EEPROM lower position bound, a position through the model's
    /// position scale.
    MinPositionLimit, min_position_limit, HasMinPositionLimit, "Min Position Limit";
    /// Max Position Limit, or CCW Angle Limit on the angle-limit models: the
    /// EEPROM upper position bound, a position through the model's
    /// position scale.
    MaxPositionLimit, max_position_limit, HasMaxPositionLimit, "Max Position Limit";
    /// Velocity Limit, rad/s: the EEPROM bound of Goal Velocity's
    /// magnitude.
    VelocityLimit, velocity_limit, HasVelocityLimit, "Velocity Limit";
    /// Current Limit, A: the EEPROM bound of Goal Current's magnitude.
    CurrentLimit, current_limit, HasCurrentLimit, "Current Limit";
    /// PWM Limit, a ratio of the full PWM output: the EEPROM bound of the
    /// PWM output.
    PwmLimit, pwm_limit, HasPwmLimit, "PWM Limit";
    /// Max Torque (AX, RX, MX on Protocol 1, XL-320), a ratio of the
    /// maximum torque: the value Torque Limit (RAM) is reset to at power-on
    /// (e-Manual), so it bounds the output from the next power-on.
    MaxTorque, max_torque, HasMaxTorque, "Max Torque";
    /// Acceleration Limit (MX(2.0), PRO, PRO+, Y, RH-P12-RN), rad/s²: the
    /// EEPROM bound of Profile Acceleration.
    AccelerationLimit, acceleration_limit, HasAccelerationLimit, "Acceleration Limit";
    /// Homing Offset (MX(2.0), X, Y, PRO), an angle offset: the EEPROM
    /// value the actuator adds to Present Position, one position tick per
    /// raw step, with no zero point.
    HomingOffset, homing_offset, HasHomingOffset, "Homing Offset";
    /// Moving Threshold (MX(2.0), X, Y, PRO), rad/s: the EEPROM speed above
    /// which the actuator reports Moving.
    MovingThreshold, moving_threshold, HasMovingThreshold, "Moving Threshold";
    /// Return Delay Time, a duration: how long the actuator waits after an
    /// instruction packet before its status packet.
    ReturnDelayTime, return_delay_time, HasReturnDelayTime, "Return Delay Time";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::{Access, Area};
    use crate::units::Sign;

    const fn reg(addr: u16, size: u8) -> Register {
        Register {
            name: "Test",
            addr,
            size,
            access: Access::RW,
            area: Area::Ram,
        }
    }

    const JOINT: ModeScale = ModeScale {
        modes: ModeSet::of(&[LimitMode::Bounded, LimitMode::Unbounded]),
        scale: Some(RegisterScale::new(0.011_938_052, Sign::Unsigned, 2, true)),
        range: Some((0, 1023)),
    };

    const WHEEL: ModeScale = ModeScale {
        modes: ModeSet::of(&[LimitMode::Wheel]),
        scale: Some(RegisterScale::new(
            0.011_938_052,
            Sign::DirectionBit(10),
            2,
            false,
        )),
        range: None,
    };

    const SHARE: ModeScale = ModeScale {
        modes: ModeSet::of(&[LimitMode::Wheel]),
        scale: None,
        range: Some((0, 2047)),
    };

    const AX_GOAL_VELOCITY: RoleRegister = RoleRegister {
        name: "Moving Speed",
        register: reg(32, 2),
        scales: &[SHARE],
        bounded_by: None,
        from_fw: 0,
        row_range: None,
    };

    const MOVING_SPEED: RoleRegister = RoleRegister {
        name: "Moving Speed",
        register: reg(32, 2),
        scales: &[JOINT, WHEEL],
        bounded_by: None,
        from_fw: 0,
        row_range: None,
    };

    const SLOPE: RoleRegister = RoleRegister {
        name: "CW Compliance Slope",
        register: reg(28, 1),
        scales: &[],
        bounded_by: None,
        from_fw: 0,
        row_range: Some((0, 254)),
    };

    const RANGED_ROW: RoleRegister = RoleRegister {
        row_range: Some((0, 2047)),
        ..MOVING_SPEED
    };

    const TICK: RoleRegister = RoleRegister {
        name: "Realtime Tick",
        register: reg(50, 2),
        scales: &[ModeScale {
            modes: ModeSet::ALL,
            scale: Some(RegisterScale::new(0.001, Sign::Unsigned, 2, false)),
            range: Some((0, 32_767)),
        }],
        bounded_by: Some(reg(44, 4)),
        from_fw: 40,
        row_range: None,
    };

    #[test]
    fn mode_sets() {
        let joint = ModeSet::of(&[LimitMode::Bounded, LimitMode::Unbounded]);
        assert!(joint.contains(LimitMode::Bounded));
        assert!(joint.contains(LimitMode::Unbounded));
        assert!(!joint.contains(LimitMode::Wheel));
        assert_eq!(ModeSet::of(&[]), ModeSet(0));
        for mode in [
            LimitMode::Bounded,
            LimitMode::Unbounded,
            LimitMode::Pwm,
            LimitMode::UnboundedCurrent,
            LimitMode::Velocity,
            LimitMode::Current,
            LimitMode::Wheel,
        ] {
            assert!(ModeSet::ALL.contains(mode));
            assert!(ModeSet::of(&[mode]).contains(mode));
        }
    }

    #[test]
    fn scale_follows_the_mode() {
        assert_eq!(MOVING_SPEED.scale(LimitMode::Bounded), JOINT.scale);
        assert_eq!(MOVING_SPEED.scale(LimitMode::Wheel), WHEEL.scale);
        assert_eq!(MOVING_SPEED.scale(LimitMode::Current), None);
        assert_eq!(MOVING_SPEED.raw_range(LimitMode::Unbounded), Some(0..=1023));
        assert_eq!(MOVING_SPEED.raw_range(LimitMode::Wheel), None);
        assert_eq!(MOVING_SPEED.raw_range(LimitMode::Pwm), None);
        assert_eq!(MOVING_SPEED.tick_modulus(LimitMode::Wheel), None);
    }

    #[test]
    fn an_untyped_entry_keeps_its_range() {
        assert_eq!(AX_GOAL_VELOCITY.scale(LimitMode::Wheel), None);
        assert_eq!(AX_GOAL_VELOCITY.raw_range(LimitMode::Wheel), Some(0..=2047));
        assert_eq!(AX_GOAL_VELOCITY.raw_range(LimitMode::Bounded), None);
    }

    #[test]
    fn row_range_fills_in_where_no_scale_gives_one() {
        for mode in [LimitMode::Bounded, LimitMode::Wheel, LimitMode::Current] {
            assert_eq!(SLOPE.raw_range(mode), Some(0..=254));
            assert_eq!(SLOPE.scale(mode), None);
        }
        assert_eq!(RANGED_ROW.raw_range(LimitMode::Bounded), Some(0..=1023));
        assert_eq!(RANGED_ROW.raw_range(LimitMode::Wheel), Some(0..=2047));
        assert_eq!(RANGED_ROW.raw_range(LimitMode::Pwm), Some(0..=2047));
        assert_eq!(SLOPE.tick_modulus(LimitMode::Pwm), Some(255));
    }

    #[test]
    fn accessors() {
        assert_eq!(MOVING_SPEED.name(), "Moving Speed");
        assert_eq!((MOVING_SPEED.address(), MOVING_SPEED.size()), (32, 2));
        assert_eq!(MOVING_SPEED.register(), reg(32, 2));
        assert_eq!(MOVING_SPEED.bounded_by(), None);
        assert_eq!(MOVING_SPEED.from_firmware(), 0);
        assert_eq!(TICK.from_firmware(), 40);
        assert_eq!(TICK.bounded_by(), Some(reg(44, 4)));
        assert_eq!(TICK.tick_modulus(LimitMode::Pwm), Some(32_768));
    }

    #[test]
    fn roles_table() {
        let mut roles = Roles::NONE;
        for &role in Role::ALL {
            assert_eq!(roles.get(role), None);
            assert!(!role.name().is_empty());
        }
        roles.speed_limit = Some(MOVING_SPEED);
        roles.realtime_tick = Some(TICK);
        assert_eq!(roles.get(Role::SpeedLimit), Some(&MOVING_SPEED));
        assert_eq!(roles.get(Role::RealtimeTick), Some(&TICK));
        assert_eq!(roles.get(Role::GoalVelocity), None);
        assert_eq!(Role::ALL.len(), 35);
        assert_eq!(Role::TemperatureLimit.name(), "Temperature Limit");
        assert_eq!(Role::MinPositionLimit.name(), "Min Position Limit");
        assert_eq!(Role::MaxPositionLimit.name(), "Max Position Limit");
        assert_eq!(Role::VelocityLimit.name(), "Velocity Limit");
        assert_eq!(Role::CurrentLimit.name(), "Current Limit");
        assert_eq!(Role::PwmLimit.name(), "PWM Limit");
        assert_eq!(Role::MaxTorque.name(), "Max Torque");
        assert_eq!(Role::AccelerationLimit.name(), "Acceleration Limit");
        assert_eq!(Role::HomingOffset.name(), "Homing Offset");
        assert_eq!(Role::MovingThreshold.name(), "Moving Threshold");
        assert_eq!(Role::ReturnDelayTime.name(), "Return Delay Time");
        roles.max_torque = Some(SLOPE);
        assert_eq!(roles.get(Role::MaxTorque), Some(&SLOPE));
        assert_eq!(roles.get(Role::PwmLimit), None);
        assert_eq!(Role::SpeedLimit.name(), "Profile Velocity");
        assert_eq!(Role::TorqueCtrlModeEnable.name(), "Torque Ctrl Mode Enable");
    }
}
