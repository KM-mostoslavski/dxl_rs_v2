//! What every actuator model has in common.

use core::fmt;

use crate::Id;
use crate::error::ValidationError;
use crate::protocol::ProtocolVersion;
use crate::register::Register;
use crate::units::{Limits, PositionScale};

/// A model number as the actuator reports it (Model Number register, address
/// 0, 2 bytes).
///
/// Any `u16` is a model number; whether this build supports it is decided
/// at Setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelNumber(pub u16);

impl ModelNumber {
    /// The model number as a plain `u16`.
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl From<u16> for ModelNumber {
    fn from(number: u16) -> Self {
        Self(number)
    }
}

impl fmt::Display for ModelNumber {
    /// The plain number, e.g. `1020`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Common interface of every actuator model.
///
/// Implemented by each generated model type and by [`ActuatorKind`], so code
/// that only needs the common features works with either:
///
/// ```
/// use dxl_rs_v2::Actuator;
///
/// fn describe<A: Actuator>(a: &A) -> (u8, &'static str) {
///     (a.id().get(), a.name())
/// }
/// # let _ = describe::<dxl_rs_v2::ActuatorKind>;
/// ```
///
/// What the bus reads from a model (its registers, scale and cache) stays
/// inside the crate:
///
/// ```compile_fail,E0603
/// use dxl_rs_v2::actuator::Describe;
/// ```
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

/// Torque and fault state of one actuator. Faulted always means torque off.
///
/// Every actuator starts [`Off`](TorqueState::Off). The bus keeps one state
/// per actuator and moves it by these rules:
///
/// | From \ event | enable torque | disable torque | hardware fault | fault cleared |
/// |---|---|---|---|---|
/// | `Off` | `On` | `Off` | `Faulted` | `Off` |
/// | `On` | `On` | `Off` | `Faulted` | `On` |
/// | `Faulted` | refused, [`ActuatorFaulted`] | `Faulted` | `Faulted` | `Off` |
///
/// Only `On` accepts a goal; `Off` and `Faulted` refuse it with
/// [`ActuatorIsOff`]. A cleared fault gives `Off`, never `On`.
///
/// The three states are final, so a `match` needs no `_` arm:
///
/// ```
/// use dxl_rs_v2::TorqueState;
///
/// fn label(state: TorqueState) -> &'static str {
///     match state {
///         TorqueState::Off => "torque off",
///         TorqueState::On => "torque on",
///         TorqueState::Faulted => "faulted, torque off",
///     }
/// }
/// assert_eq!(label(TorqueState::default()), "torque off");
/// ```
///
/// Only the crate moves a state, so "faulted means off" holds:
///
/// ```compile_fail,E0624
/// use dxl_rs_v2::{Id, TorqueState};
///
/// let id = Id::new(1).unwrap();
/// let _ = TorqueState::Off.enable(id);
/// ```
///
/// [`ActuatorFaulted`]: crate::ValidationError::ActuatorFaulted
/// [`ActuatorIsOff`]: crate::ValidationError::ActuatorIsOff
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TorqueState {
    /// Torque off. Goals are refused.
    #[default]
    Off = 0,
    /// Torque on. Goals are accepted.
    On = 1,
    /// The actuator reported a hardware fault. Torque is off, goals are
    /// refused, and torque stays off until the fault is cleared.
    Faulted = 2,
}

impl TorqueState {
    /// Whether torque is on. `Faulted` counts as off.
    pub const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }

    /// Whether the actuator reported a hardware fault that is not cleared.
    pub const fn is_faulted(self) -> bool {
        matches!(self, Self::Faulted)
    }

    /// The state after torque is enabled on actuator `id`.
    ///
    /// # Errors
    ///
    /// [`ValidationError::ActuatorFaulted`] when faulted; checked before
    /// anything is sent.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn enable(self, id: Id) -> Result<Self, ValidationError> {
        match self {
            Self::Off | Self::On => Ok(Self::On),
            Self::Faulted => Err(ValidationError::ActuatorFaulted { id }),
        }
    }

    /// The state after torque is disabled. A fault stays.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn disable(self) -> Self {
        match self {
            Self::Off | Self::On => Self::Off,
            Self::Faulted => Self::Faulted,
        }
    }

    /// The state after a hardware fault: always `Faulted`.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn fault(self) -> Self {
        Self::Faulted
    }

    /// The state after the actuator reports no fault: `Faulted` becomes
    /// `Off`; other states do not change.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn clear(self) -> Self {
        match self {
            Self::Faulted => Self::Off,
            Self::Off | Self::On => self,
        }
    }

    /// Whether actuator `id` accepts a goal: only `On` does.
    ///
    /// # Errors
    ///
    /// [`ValidationError::ActuatorIsOff`] when `Off` or `Faulted`.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn check_goal(self, id: Id) -> Result<(), ValidationError> {
        match self {
            Self::On => Ok(()),
            Self::Off | Self::Faulted => Err(ValidationError::ActuatorIsOff { id }),
        }
    }
}

pub(crate) mod sealed {
    use super::{Cache, ModelData};

    /// What the bus needs from a model. No I/O.
    ///
    /// It cannot be named outside the crate, so only the crate's own types
    /// implement it.
    #[allow(dead_code, reason = "implemented by X1's generated code")]
    pub trait Describe {
        /// The model's description, from its control table.
        fn data(&self) -> &'static ModelData;

        /// The actuator's cache.
        fn cache(&self) -> &Cache;

        /// The actuator's cache, for Setup to record what it read.
        fn cache_mut(&mut self) -> &mut Cache;
    }
}

#[allow(unused_imports, reason = "implemented by X1's generated code")]
pub(crate) use sealed::Describe;

/// What the bus needs to know about one actuator model, from its control
/// table. One `&'static` value per generated model type.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(
    dead_code,
    reason = "built by X1's generated DATA consts; read by bus-1"
)]
pub struct ModelData {
    /// Display name, e.g. `"XM430-W350"`.
    pub(crate) name: &'static str,
    /// Model number, e.g. 1020.
    pub(crate) model_number: u16,
    /// The protocols the table is for, from its `protocols` field.
    pub(crate) protocols: Protocols,
    /// Torque Enable.
    pub(crate) torque_enable: Register,
    /// Goal Position.
    pub(crate) goal_position: Register,
    /// Present Position.
    pub(crate) present_position: Register,
    /// The two position limit registers.
    pub(crate) limits: LimitRegisters,
    /// Hardware Error Status, or `None` (AX, Y-series tables have none).
    pub(crate) hardware_error: Option<Register>,
    /// Where Setup gets the limit mode.
    pub(crate) mode: ModeSource,
    /// Tick to radian mapping, from the table's `type_info`.
    pub(crate) scale: PositionScale,
}

/// The two registers that hold a model's position limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "each variant is built only by the generated models of its families"
)]
pub enum LimitRegisters {
    /// Min and Max Position Limit (X, MX(2.0), PRO, PRO+, Y, RH-P12-RN).
    MinMax {
        /// Min Position Limit.
        min: Register,
        /// Max Position Limit.
        max: Register,
    },
    /// CW and CCW Angle Limit (AX, MX on Protocol 1, XL-320).
    CwCcw {
        /// CW Angle Limit, the minimum.
        cw: Register,
        /// CCW Angle Limit, the maximum.
        ccw: Register,
    },
}

impl LimitRegisters {
    /// The minimum limit: Min Position Limit, or CW Angle Limit.
    #[allow(
        dead_code,
        reason = "built by X1's generated DATA consts; read by bus-1"
    )]
    pub(crate) const fn min(&self) -> Register {
        match *self {
            Self::MinMax { min, .. } => min,
            Self::CwCcw { cw, .. } => cw,
        }
    }

    /// The maximum limit: Max Position Limit, or CCW Angle Limit.
    #[allow(
        dead_code,
        reason = "built by X1's generated DATA consts; read by bus-1"
    )]
    pub(crate) const fn max(&self) -> Register {
        match *self {
            Self::MinMax { max, .. } => max,
            Self::CwCcw { ccw, .. } => ccw,
        }
    }
}

/// The protocols a model's control table is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "built by X1's generated DATA consts; read by bus-1"
)]
pub struct Protocols {
    /// Protocol 1.0.
    pub(crate) p1: bool,
    /// Protocol 2.0.
    pub(crate) p2: bool,
}

impl Protocols {
    /// Whether the table is for `protocol`.
    #[allow(
        dead_code,
        reason = "built by X1's generated DATA consts; read by bus-1"
    )]
    pub(crate) const fn supports(self, protocol: ProtocolVersion) -> bool {
        match protocol {
            ProtocolVersion::P1 => self.p1,
            ProtocolVersion::P2 => self.p2,
        }
    }
}

/// Where Setup gets an actuator's [`LimitMode`]. One per model, in
/// [`ModelData::mode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "each variant is built only by the generated models of its families"
)]
pub(crate) enum ModeSource {
    /// Operating Mode (X, MX(2.0), PRO, PRO+, Y, RH-P12-RN) or XL-320
    /// Control Mode: 1 byte read at Setup, decoded with the model's own
    /// lists.
    Register {
        /// The 1-byte mode register.
        register: Register,
        /// What each of its values means for this model.
        values: ModeValues,
    },
    /// AX, MX on Protocol 1: no mode register. The mode follows from the
    /// CW and CCW Angle Limits Setup reads anyway; see
    /// [`LimitMode::from_angle_limits`].
    AngleLimits,
}

impl ModeSource {
    /// The 1-byte register Setup reads, or `None` for
    /// [`AngleLimits`](ModeSource::AngleLimits).
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn register(self) -> Option<Register> {
        match self {
            Self::Register { register, .. } => Some(register),
            Self::AngleLimits => None,
        }
    }

    /// The limit mode of mode value `value`, read from the mode register.
    ///
    /// `None`: the value is in none of the model's lists; Setup fails with
    /// `SetupError::UnknownOperatingMode`, no mode is guessed. Also `None`
    /// for [`AngleLimits`](ModeSource::AngleLimits), which has no register.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn limit_mode(self, value: u8) -> Option<LimitMode> {
        match self {
            Self::Register { values, .. } => values.limit_mode(value),
            Self::AngleLimits => None,
        }
    }
}

/// The mode values of one model: which values of its mode register give
/// which [`LimitMode`].
///
/// X1 generates it per model from the table's `operating_modes` field,
/// which is copied from that model's e-Manual page (architect, 2026-10-28).
/// Core holds no list of its own, so a new model or family is a table
/// change only. The lists are disjoint: codegen and the tables check refuse
/// overlaps. Example, XM430-W350 Operating Mode(11)
/// (<https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>): `bounded`
/// `[3]`, `unbounded` `[4, 5]`, `no_position` `[0, 1, 16]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code, reason = "built by X1's generated DATA consts")]
pub(crate) struct ModeValues {
    /// Values that give [`LimitMode::Bounded`] (position, joint mode).
    pub(crate) bounded: &'static [u8],
    /// Values that give [`LimitMode::Unbounded`] (extended or current-based
    /// position).
    pub(crate) unbounded: &'static [u8],
    /// Values that give [`LimitMode::NoPosition`] (current, velocity, PWM,
    /// wheel).
    pub(crate) no_position: &'static [u8],
}

impl ModeValues {
    /// The mode of the list that holds `value`.
    ///
    /// `None`: the value is in none of the model's lists; Setup fails with
    /// `SetupError::UnknownOperatingMode`, no mode is guessed.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn limit_mode(self, value: u8) -> Option<LimitMode> {
        if holds(self.bounded, value) {
            Some(LimitMode::Bounded)
        } else if holds(self.unbounded, value) {
            Some(LimitMode::Unbounded)
        } else if holds(self.no_position, value) {
            Some(LimitMode::NoPosition)
        } else {
            None
        }
    }
}

/// Whether `list` holds `value`.
const fn holds(mut list: &[u8], value: u8) -> bool {
    while let [first, rest @ ..] = list {
        if *first == value {
            return true;
        }
        list = rest;
    }
    false
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

impl LimitMode {
    /// The limit mode of an AX or an MX on Protocol 1, from the CW and CCW
    /// Angle Limits Setup read:
    ///
    /// - CW = CCW = 0: wheel mode, [`NoPosition`](LimitMode::NoPosition).
    /// - CW = CCW = 4095: MX multi-turn mode,
    ///   [`Unbounded`](LimitMode::Unbounded).
    /// - Any other pair: joint mode, [`Bounded`](LimitMode::Bounded).
    ///
    /// Sources: AX-12A, CW/CCW Angle Limit(6, 8), "Wheel Mode | both are 0"
    /// (<https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>); MX-64
    /// (Protocol 1.0), CW/CCW Angle Limit(6, 8), "Multi-turn Mode |
    /// Condition that Both CW and CCW are 4095", joint mode being the
    /// "Conditions excluding Wheel Mode and Multi-turn Mode"
    /// (<https://emanual.robotis.com/docs/en/dxl/mx/mx-64/>).
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn from_angle_limits(limits: Limits) -> Self {
        match (limits.min_raw(), limits.max_raw()) {
            (0, 0) => Self::NoPosition,
            (4095, 4095) => Self::Unbounded,
            _ => Self::Bounded,
        }
    }
}

/// What core keeps per actuator: its ID, its limit mode and the position
/// limits read from its EEPROM at Setup.
///
/// It holds no torque state, no fault state and no fault report: the bus
/// keeps a [`TorqueState`] per actuator beside it.
///
/// [`Cache::check_goal`] is the one limit check, for typed and raw goals
/// alike. Limits follow the operating mode: in [`LimitMode::Unbounded`]
/// only `Register::encode` stops a value that does not fit the register. No
/// constructor fills limits from a control table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cache {
    /// The actuator's ID. Never changes.
    id: Id,
    /// How goals are checked; `Bounded` until Setup records the mode.
    #[allow(dead_code, reason = "read by check_goal, called by bus-1")]
    mode: LimitMode,
    /// The EEPROM limits, or `None` until Setup records them.
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
    extern crate std;

    use core::f32::consts::PI;

    use std::string::ToString;

    use super::*;
    use crate::protocol::ProtocolVersion;
    use crate::register::Register;
    use crate::units::{Limits, PositionScale};

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    #[test]
    fn concrete_types_implement_actuator() {
        #[cfg(feature = "ax")]
        {
            let ax = crate::models::Ax12a::new(id(9));
            assert_eq!(
                (ax.id(), ax.model_number(), ax.name()),
                (id(9), 12, "AX-12A")
            );
        }
        #[cfg(feature = "x-series")]
        {
            let kind = crate::ActuatorKind::Xm430W350(crate::models::Xm430W350::new(id(3)));
            assert_eq!(
                (kind.id(), kind.model_number(), kind.name()),
                (id(3), 1020, "XM430-W350")
            );
        }
    }

    #[test]
    fn new_state_is_off() {
        assert_eq!(TorqueState::default(), TorqueState::Off);
    }

    #[test]
    fn state_is_one_byte() {
        assert_eq!(size_of::<TorqueState>(), 1);
        assert_eq!(size_of::<Option<TorqueState>>(), 1);
        assert_eq!(TorqueState::Off as u8, 0);
        assert_eq!(TorqueState::On as u8, 1);
        assert_eq!(TorqueState::Faulted as u8, 2);
    }

    #[test]
    fn transition_table() {
        use TorqueState::{Faulted, Off, On};
        let faulted = Err(ValidationError::ActuatorFaulted { id: id(4) });
        assert_eq!(Off.enable(id(4)), Ok(On));
        assert_eq!(On.enable(id(4)), Ok(On));
        assert_eq!(Faulted.enable(id(4)), faulted);
        assert_eq!(
            [Off.disable(), On.disable(), Faulted.disable()],
            [Off, Off, Faulted]
        );
        assert_eq!(
            [Off.fault(), On.fault(), Faulted.fault()],
            [Faulted, Faulted, Faulted]
        );
        assert_eq!([Off.clear(), On.clear(), Faulted.clear()], [Off, On, Off]);
    }

    #[test]
    fn fault_is_one_step_and_counts_as_off() {
        let state = TorqueState::On.fault();
        assert_eq!(state, TorqueState::Faulted);
        assert!(!state.is_on());
    }

    #[test]
    fn enable_while_faulted_is_refused() {
        let state = TorqueState::Faulted;
        assert_eq!(
            state.enable(id(4)),
            Err(ValidationError::ActuatorFaulted { id: id(4) })
        );
        assert_eq!(state, TorqueState::Faulted);
    }

    #[test]
    fn queries() {
        use TorqueState::{Faulted, Off, On};
        assert_eq!(
            [Off.is_on(), On.is_on(), Faulted.is_on()],
            [false, true, false]
        );
        assert_eq!(
            [Off.is_faulted(), On.is_faulted(), Faulted.is_faulted()],
            [false, false, true]
        );
        let off = Err(ValidationError::ActuatorIsOff { id: id(4) });
        assert_eq!(Off.check_goal(id(4)), off);
        assert_eq!(On.check_goal(id(4)), Ok(()));
        assert_eq!(Faulted.check_goal(id(4)), off);
    }

    #[test]
    fn model_number_converts_and_displays() {
        let number = ModelNumber::from(1020);
        assert_eq!(number.get(), 1020);
        assert_eq!(number, ModelNumber(1020));
        assert_eq!(number.to_string(), "1020");
        assert!(ModelNumber(12) < number);
        assert_eq!(ModelNumber(12).max(number), number);
    }

    /// XM430-W350 Operating Mode values (e-Manual, Operating Mode(11)).
    const X_MODES: ModeValues = ModeValues {
        bounded: &[3],
        unbounded: &[4, 5],
        no_position: &[0, 1, 16],
    };

    const XM430: ModelData = ModelData {
        name: "XM430-W350",
        model_number: 1020,
        protocols: Protocols { p1: true, p2: true },
        torque_enable: Register::new(64, 1),
        goal_position: Register::new(116, 4),
        present_position: Register::new(132, 4),
        limits: LimitRegisters::MinMax {
            min: Register::new(52, 4),
            max: Register::new(48, 4),
        },
        hardware_error: Some(Register::new(70, 1)),
        mode: ModeSource::Register {
            register: Register::new(11, 1),
            values: X_MODES,
        },
        scale: PositionScale::new(2048, 0, 4095, -PI, PI),
    };

    const AX12A: ModelData = ModelData {
        name: "AX-12A",
        model_number: 12,
        protocols: Protocols {
            p1: true,
            p2: false,
        },
        torque_enable: Register::new(24, 1),
        goal_position: Register::new(30, 2),
        present_position: Register::new(36, 2),
        limits: LimitRegisters::CwCcw {
            cw: Register::new(6, 2),
            ccw: Register::new(8, 2),
        },
        hardware_error: None,
        mode: ModeSource::AngleLimits,
        scale: PositionScale::new(512, 0, 1023, 0.0, 5.2359877),
    };

    struct Fake {
        data: &'static ModelData,
        cache: Cache,
    }

    impl Describe for Fake {
        fn data(&self) -> &'static ModelData {
            self.data
        }

        fn cache(&self) -> &Cache {
            &self.cache
        }

        fn cache_mut(&mut self) -> &mut Cache {
            &mut self.cache
        }
    }

    #[test]
    fn cache_holds_only_id_mode_and_limits() {
        let Cache {
            id: _,
            mode: _,
            limits: _,
        } = Cache::new(id(3));
        let cache = Cache::new(id(3));
        assert_eq!(cache.limits(), None);
        assert_eq!(cache.mode, LimitMode::Bounded);
        assert_eq!(size_of::<Cache>(), 16);
    }

    #[test]
    fn goal_register_per_model() {
        fn goal<D: Describe>(d: &D) -> (u16, u8) {
            let register = d.data().goal_position;
            (register.addr, register.size)
        }
        let mut xm = Fake {
            data: &XM430,
            cache: Cache::new(id(3)),
        };
        let ax = Fake {
            data: &AX12A,
            cache: Cache::new(id(9)),
        };
        assert_eq!(goal(&xm), (116, 4));
        assert_eq!(goal(&ax), (30, 2));
        assert_eq!((xm.cache().id(), ax.cache().id()), (id(3), id(9)));
        xm.cache_mut().record_limits(1000, 3000).unwrap();
        assert_eq!(xm.cache().limits(), Limits::new(id(3), 1000, 3000).ok());
    }

    #[test]
    fn limit_registers_share_names() {
        assert_eq!((AX12A.limits.min().addr, AX12A.limits.max().addr), (6, 8));
        assert_eq!((XM430.limits.min().addr, XM430.limits.max().addr), (52, 48));
        assert_eq!(XM430.hardware_error, Some(Register::new(70, 1)));
        assert_eq!(AX12A.hardware_error, None);
        assert_eq!(XM430, XM430);
        assert_ne!(XM430, AX12A);
    }

    #[test]
    fn protocols_support() {
        let p1_only = Protocols {
            p1: true,
            p2: false,
        };
        assert!(p1_only.supports(ProtocolVersion::P1));
        assert!(!p1_only.supports(ProtocolVersion::P2));
        assert!(XM430.protocols.supports(ProtocolVersion::P1));
        assert!(XM430.protocols.supports(ProtocolVersion::P2));
        assert_eq!(AX12A.protocols, p1_only);
    }

    #[test]
    fn raw_and_typed_goals_hit_the_same_check() {
        let mut cache = Cache::new(id(3));
        cache.record_limits(1000, 3000).unwrap();
        let scale = XM430.scale;
        let typed = scale.to_raw(scale.to_position(3001)).unwrap();
        assert_eq!(
            cache.check_goal(typed),
            Err(ValidationError::OutOfLimits {
                id: id(3),
                raw: 3001,
                min: 1000,
                max: 3000
            })
        );
        assert_eq!(cache.check_goal(typed), cache.check_goal(3001));
        assert_eq!(cache.check_goal(2500), Ok(2500));
        let typed_inside = scale.to_raw(scale.to_position(2500)).unwrap();
        assert_eq!(cache.check_goal(typed_inside), Ok(2500));
    }

    mod mode {
        use super::*;

        /// RH-P12-RN Operating Mode values (e-Manual, Operating Mode(11)).
        const RH_P12_RN_MODES: ModeValues = ModeValues {
            bounded: &[5],
            unbounded: &[],
            no_position: &[0],
        };

        /// XL-320 Control Mode values (e-Manual, Control Mode(11)).
        const XL320_MODES: ModeValues = ModeValues {
            bounded: &[2],
            unbounded: &[],
            no_position: &[1],
        };

        const NO_MODES: ModeValues = ModeValues {
            bounded: &[],
            unbounded: &[],
            no_position: &[],
        };

        /// The mode of the list that holds `value`, read from the lists.
        fn listed(values: ModeValues, value: u8) -> Option<LimitMode> {
            [
                (values.bounded, LimitMode::Bounded),
                (values.unbounded, LimitMode::Unbounded),
                (values.no_position, LimitMode::NoPosition),
            ]
            .into_iter()
            .find(|(list, _)| list.contains(&value))
            .map(|(_, mode)| mode)
        }

        #[test]
        fn values_map_to_their_list() {
            for values in [X_MODES, RH_P12_RN_MODES, XL320_MODES] {
                for value in 0..=255 {
                    assert_eq!(values.limit_mode(value), listed(values, value));
                }
            }
            for value in [0, 1, 16] {
                assert_eq!(X_MODES.limit_mode(value), Some(LimitMode::NoPosition));
            }
            assert_eq!(X_MODES.limit_mode(3), Some(LimitMode::Bounded));
            for value in [4, 5] {
                assert_eq!(X_MODES.limit_mode(value), Some(LimitMode::Unbounded));
            }
            let listed_count = (0..=255).filter(|&v| X_MODES.limit_mode(v).is_some());
            assert_eq!(listed_count.count(), 6);
        }

        #[test]
        fn value_5_differs_by_list() {
            let gripper = ModeSource::Register {
                register: Register::new(11, 1),
                values: RH_P12_RN_MODES,
            };
            let x = ModeSource::Register {
                register: Register::new(11, 1),
                values: X_MODES,
            };
            assert_eq!(gripper.limit_mode(5), Some(LimitMode::Bounded));
            assert_eq!(x.limit_mode(5), Some(LimitMode::Unbounded));
        }

        #[test]
        fn control_mode_is_a_register_source() {
            let control_mode = ModeSource::Register {
                register: Register::new(11, 1),
                values: XL320_MODES,
            };
            for value in 0..=255 {
                let expected = match value {
                    1 => Some(LimitMode::NoPosition),
                    2 => Some(LimitMode::Bounded),
                    _ => None,
                };
                assert_eq!(control_mode.limit_mode(value), expected);
            }
        }

        #[test]
        fn empty_lists_give_no_mode() {
            let source = ModeSource::Register {
                register: Register::new(11, 1),
                values: NO_MODES,
            };
            for value in 0..=255 {
                assert_eq!(NO_MODES.limit_mode(value), None);
                assert_eq!(source.limit_mode(value), None);
            }
        }

        #[test]
        fn angle_limits_decide_the_mode() {
            let mode = |cw, ccw| LimitMode::from_angle_limits(Limits::new(id(1), cw, ccw).unwrap());
            assert_eq!(mode(0, 0), LimitMode::NoPosition);
            assert_eq!(mode(4095, 4095), LimitMode::Unbounded);
            for (cw, ccw) in [(0, 1023), (1, 1023), (0, 4095), (1023, 1023), (0, 1)] {
                assert_eq!(mode(cw, ccw), LimitMode::Bounded);
            }
        }

        #[test]
        fn register_to_read() {
            let register = Register::new(11, 1);
            let source = ModeSource::Register {
                register,
                values: X_MODES,
            };
            assert_eq!(source.register(), Some(register));
            assert_eq!(ModeSource::AngleLimits.register(), None);
            for value in 0..=255 {
                assert_eq!(ModeSource::AngleLimits.limit_mode(value), None);
            }
            assert_eq!(XM430.mode, source);
            assert_eq!(AX12A.mode, ModeSource::AngleLimits);
        }

        #[test]
        fn unlisted_value_has_no_mode() {
            assert_eq!(X_MODES.limit_mode(2), None);
            assert_eq!(X_MODES.limit_mode(255), None);
            assert_eq!(XL320_MODES.limit_mode(0), None);
        }
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
