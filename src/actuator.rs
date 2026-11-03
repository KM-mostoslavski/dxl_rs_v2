//! What every actuator model has in common.

use core::fmt;
use core::ops::RangeInclusive;

use crate::Id;
use crate::error::ValidationError;
use crate::protocol::ProtocolVersion;
use crate::register::Register;
use crate::units::{Limits, Position, PositionScale, RegisterScale};

mod roles;

pub use roles::{
    HasCcwComplianceMargin, HasCcwComplianceSlope, HasCwComplianceMargin, HasCwComplianceSlope,
    HasDGain, HasGoalCurrent, HasGoalVelocity, HasHardwareErrorStatus, HasLed, HasLedBlue,
    HasLedGreen, HasLedRed, HasMoving, HasPGain, HasPresentCurrent, HasPresentLoad,
    HasPresentTemperature, HasPresentVelocity, HasPresentVoltage, HasRealtimeTick, HasSpeedLimit,
    HasTorqueCtrlModeEnable, RoleRegister,
};
#[allow(
    unused_imports,
    reason = "used by generated models and the bus part of FR-9"
)]
pub(crate) use roles::{ModeScale, ModeSet, Role, Roles};

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

/// Common interface of every actuator model: what it is and what Setup read
/// from it. It performs no I/O; reading and writing an actuator goes through
/// the bus.
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
///
/// /// The width of the position limits in radians, `None` before Setup
/// /// read them.
/// fn span<A: Actuator>(a: &A) -> Option<f32> {
///     let limits = a.position_limits()?;
///     Some((*limits.end() - *limits.start()).radians())
/// }
///
/// fn zero_tick<A: Actuator>(a: &A) -> i32 {
///     a.position_scale().zero_raw()
/// }
/// # let _ = (
/// #     describe::<dxl_rs_v2::ActuatorKind>,
/// #     span::<dxl_rs_v2::ActuatorKind>,
/// #     zero_tick::<dxl_rs_v2::ActuatorKind>,
/// # );
/// ```
///
/// The trait is sealed: only the crate's model types implement it.
///
/// ```compile_fail,E0277
/// struct Mine;
///
/// impl dxl_rs_v2::Actuator for Mine {}
/// ```
///
/// What the bus reads from a model (its registers and its cache) stays
/// inside the crate:
///
/// ```compile_fail,E0603
/// use dxl_rs_v2::actuator::Describe;
/// ```
///
/// [`ActuatorKind`]: crate::ActuatorKind
pub trait Actuator: sealed::Describe {
    /// Firmware ID of this actuator.
    fn id(&self) -> Id {
        self.cache().id()
    }

    /// Model number reported by the actuator (register 0).
    fn model_number(&self) -> u16 {
        self.data().model_number
    }

    /// Display name, e.g. `"XM430-W350"`.
    fn name(&self) -> &'static str {
        self.data().name
    }

    /// How this model maps raw position ticks to radians, from its control
    /// table.
    fn position_scale(&self) -> PositionScale {
        self.data().scale
    }

    /// The position limits read from the actuator's EEPROM at Setup, in raw
    /// ticks: Min/Max Position Limit, or CW/CCW Angle Limit. `None` until
    /// Setup read them; never a control table's default.
    ///
    /// They are returned in every operating mode, but goals are checked
    /// against them only in position (joint) mode. In extended position,
    /// current-based position and multi-turn mode the firmware ignores them,
    /// and in velocity, current, PWM and wheel mode goals are refused.
    fn position_limits_raw(&self) -> Option<RangeInclusive<i32>> {
        let limits = self.cache().limits()?;
        Some(limits.min_raw()..=limits.max_raw())
    }

    /// The limit mode Setup recorded from the actuator's operating mode
    /// (Operating Mode, or the angle limits and torque switch on Protocol
    /// 1). It decides which goals the bus accepts: position modes take
    /// goal positions, [`LimitMode::Velocity`] and [`LimitMode::Wheel`]
    /// goal velocities, [`LimitMode::Current`] and
    /// [`LimitMode::UnboundedCurrent`] goal currents. A mode changed after
    /// Setup is not seen until the next Setup.
    fn limit_mode(&self) -> LimitMode {
        self.cache().mode()
    }

    /// The limits of [`position_limits_raw`](Actuator::position_limits_raw)
    /// in radians, through [`position_scale`](Actuator::position_scale). The
    /// same rule applies: returned in every operating mode, applied to goals
    /// only in position mode.
    fn position_limits(&self) -> Option<RangeInclusive<Position>> {
        let limits = self.position_limits_raw()?;
        let scale = self.position_scale();
        Some(scale.to_position(*limits.start())..=scale.to_position(*limits.end()))
    }
}

/// A generated model type, such as `Ax12a` or `Xm430W350` in
/// [`models`](crate::models): the type parameter of the Tier 2 handle
/// [`ActuatorRef::model`](crate::ActuatorRef::model).
///
/// Its `Has*` impls ([`HasPGain`], [`HasCwComplianceSlope`], ...) say which
/// typed registers it has, so a Tier 2 call on a register the model lacks
/// does not compile. Sealed like [`Actuator`]: only the generated model
/// types implement it, and [`ActuatorKind`](crate::ActuatorKind) does not.
pub trait Model: Actuator {
    /// The model's control-table data. Not part of the public API.
    #[doc(hidden)]
    const DATA: &'static ModelData;
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
    #[cfg_attr(
        not(any(
            feature = "ax",
            feature = "mx",
            feature = "rx",
            feature = "x-series",
            feature = "y-series",
            feature = "pro"
        )),
        allow(dead_code, reason = "only generated models construct it")
    )]
    pub trait Describe {
        /// The model's description, from its control table.
        fn data(&self) -> &'static ModelData;

        /// The actuator's cache.
        fn cache(&self) -> &Cache;

        /// The actuator's cache, for Setup to record what it read.
        fn cache_mut(&mut self) -> &mut Cache;
    }
}

pub(crate) use sealed::Describe;

/// What the bus needs to know about one actuator model, from its control
/// table. One `&'static` value per generated model type.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    not(any(
        feature = "ax",
        feature = "mx",
        feature = "rx",
        feature = "x-series",
        feature = "y-series",
        feature = "pro"
    )),
    allow(dead_code, reason = "only generated models construct it")
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
    /// The FR-9 registers the table has.
    pub(crate) roles: Roles,
    /// Whether the table's `instructions` list Bulk Read on Protocol 1 for
    /// every firmware (MX on Protocol 1, MX(2.0)); `false` where the table
    /// lists no instructions.
    pub(crate) bulk_read_p1: bool,
}

impl ModelData {
    /// The FR-9 register of `role`, or `None` when the model's table lacks
    /// it (a typed call then fails with `ValidationError::Unsupported`).
    #[allow(dead_code, reason = "used by the bus part of FR-9")]
    pub(crate) const fn role(&self, role: Role) -> Option<&RoleRegister> {
        self.roles.get(role)
    }

    /// The register of `role` and its scale in limit mode `mode`: `None`
    /// when the model lacks the register; a `None` scale when the register
    /// has no typed form in that mode, only raw (FR-9.2).
    #[allow(dead_code, reason = "used by the bus part of FR-9")]
    pub(crate) const fn typed(
        &self,
        role: Role,
        mode: LimitMode,
    ) -> Option<(Register, Option<RegisterScale>)> {
        match self.roles.get(role) {
            Some(found) => Some((found.register, found.scale(mode))),
            None => None,
        }
    }
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
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn min(&self) -> Register {
        match *self {
            Self::MinMax { min, .. } => min,
            Self::CwCcw { cw, .. } => cw,
        }
    }

    /// The maximum limit: Max Position Limit, or CCW Angle Limit.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn max(&self) -> Register {
        match *self {
            Self::MinMax { max, .. } => max,
            Self::CwCcw { ccw, .. } => ccw,
        }
    }
}

/// The protocols a model's control table is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(any(
        feature = "ax",
        feature = "mx",
        feature = "rx",
        feature = "x-series",
        feature = "y-series",
        feature = "pro"
    )),
    allow(dead_code, reason = "only generated models construct it")
)]
pub struct Protocols {
    /// Protocol 1.0.
    pub(crate) p1: bool,
    /// Protocol 2.0.
    pub(crate) p2: bool,
}

impl Protocols {
    /// Whether the table is for `protocol`.
    #[allow(dead_code, reason = "used by bus-1")]
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
    /// AX, RX, and MX on Protocol 1 without a torque control switch in
    /// their table: no mode register. The mode follows from the CW and CCW
    /// Angle Limits Setup reads anyway; see [`LimitMode::from_angle_limits`].
    AngleLimits,
    /// MX-64 on Protocol 1: the angle limits, overridden by a torque
    /// control switch. Setup reads the 1-byte `register` (Torque Ctrl Mode
    /// Enable(70)); `on` gives [`LimitMode::Current`], any other value in
    /// `values` leaves the angle-limit rule. The e-Manual: "1 | Turn on the
    /// torque mode. Cannot control the position or moving speed but only
    /// Torque" (<https://emanual.robotis.com/docs/en/dxl/mx/mx-64/>).
    TorqueControl {
        /// The 1-byte switch register.
        register: Register,
        /// The value that turns torque control on.
        on: u8,
        /// Every value the register takes, `min..=max` from the table.
        values: (u8, u8),
    },
}

impl ModeSource {
    /// The 1-byte register Setup reads, or `None` for
    /// [`AngleLimits`](ModeSource::AngleLimits).
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn register(self) -> Option<Register> {
        match self {
            Self::Register { register, .. } | Self::TorqueControl { register, .. } => {
                Some(register)
            }
            Self::AngleLimits => None,
        }
    }

    /// The limit mode of mode value `value`, read from the mode register,
    /// with `limits` the angle limits Setup read.
    ///
    /// `None`: the value is in none of the model's lists, or outside the
    /// switch register's values; Setup fails with
    /// `SetupError::UnknownOperatingMode`, no mode is guessed. Also `None`
    /// for [`AngleLimits`](ModeSource::AngleLimits), which has no register.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn limit_mode(self, value: u8, limits: Limits) -> Option<LimitMode> {
        match self {
            Self::Register { values, .. } => values.limit_mode(value),
            Self::TorqueControl { on, .. } if value == on => Some(LimitMode::Current),
            Self::TorqueControl {
                values: (min, max), ..
            } if value >= min && value <= max => Some(LimitMode::from_angle_limits(limits)),
            Self::TorqueControl { .. } | Self::AngleLimits => None,
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
/// `[3]`, `unbounded` `[4, 5]`, `no_position` `[0, 1, 16]`, and per mode
/// `current` `[0]`, `velocity` `[1]`, `pwm` `[16]`, `current_position`
/// `[5]`.
///
/// The per-mode lists are all empty for a table that does not give them
/// yet; its `no_position` values then give [`LimitMode::Pwm`], which
/// refuses every goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code, reason = "built only from a table's `operating_modes`")]
pub(crate) struct ModeValues {
    /// Values that give [`LimitMode::Bounded`] (position, joint mode).
    pub(crate) bounded: &'static [u8],
    /// Values that give [`LimitMode::Unbounded`] (extended or current-based
    /// position).
    pub(crate) unbounded: &'static [u8],
    /// Values whose mode ignores Goal Position (current, velocity, PWM).
    /// Those not in a per-mode list give [`LimitMode::Pwm`].
    pub(crate) no_position: &'static [u8],
    /// Current control values, a split of `no_position`: give
    /// [`LimitMode::Current`].
    pub(crate) current: &'static [u8],
    /// Velocity control values, a split of `no_position`: give
    /// [`LimitMode::Velocity`].
    pub(crate) velocity: &'static [u8],
    /// PWM control values, a split of `no_position`: give
    /// [`LimitMode::Pwm`].
    pub(crate) pwm: &'static [u8],
    /// Current-based position values, a subset of `unbounded`: give
    /// [`LimitMode::UnboundedCurrent`].
    pub(crate) current_position: &'static [u8],
}

impl ModeValues {
    /// The mode of the list that holds `value`; a per-mode list wins over
    /// the coarse list holding the same value.
    ///
    /// `None`: the value is in none of the model's lists; Setup fails with
    /// `SetupError::UnknownOperatingMode`, no mode is guessed.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) const fn limit_mode(self, value: u8) -> Option<LimitMode> {
        if holds(self.bounded, value) {
            Some(LimitMode::Bounded)
        } else if holds(self.current_position, value) {
            Some(LimitMode::UnboundedCurrent)
        } else if holds(self.unbounded, value) {
            Some(LimitMode::Unbounded)
        } else if holds(self.current, value) {
            Some(LimitMode::Current)
        } else if holds(self.velocity, value) {
            Some(LimitMode::Velocity)
        } else if holds(self.pwm, value) || holds(self.no_position, value) {
            Some(LimitMode::Pwm)
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
#[non_exhaustive]
pub enum LimitMode {
    /// Position (joint) mode: the goal must lie in the EEPROM limits read at
    /// Setup.
    #[default]
    Bounded = 0,
    /// Extended position, MX multi-turn, or a current-based position mode
    /// the model's lists do not single out: the firmware does not use the
    /// EEPROM limits; only the register size check applies.
    Unbounded = 1,
    /// PWM mode, or any mode the model's lists only say ignores Goal
    /// Position: every goal position, velocity and current is refused.
    Pwm = 2,
    /// Current-based position mode: an unbounded goal position, and a goal
    /// current as its torque limit.
    UnboundedCurrent = 3,
    /// Velocity mode: goal velocities only.
    Velocity = 4,
    /// Current (torque) mode: goal currents only.
    Current = 5,
    /// AX / MX on Protocol 1 wheel mode (CW = CCW Angle Limit = 0): moving
    /// speed as a goal velocity only.
    Wheel = 6,
}

impl LimitMode {
    /// The limit mode of an AX or an MX on Protocol 1, from the CW and CCW
    /// Angle Limits Setup read:
    ///
    /// - CW = CCW = 0: wheel mode, [`Wheel`](LimitMode::Wheel).
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
            (0, 0) => Self::Wheel,
            (4095, 4095) => Self::Unbounded,
            _ => Self::Bounded,
        }
    }

    /// Whether the firmware uses Goal Position in this mode.
    pub const fn allows_goal_position(self) -> bool {
        matches!(
            self,
            Self::Bounded | Self::Unbounded | Self::UnboundedCurrent
        )
    }

    /// Whether the firmware uses a goal velocity in this mode.
    pub const fn allows_goal_velocity(self) -> bool {
        matches!(self, Self::Velocity | Self::Wheel)
    }

    /// Whether the firmware uses Goal Current in this mode.
    pub const fn allows_goal_current(self) -> bool {
        matches!(self, Self::Current | Self::UnboundedCurrent)
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
    #[allow(dead_code, reason = "used by bus-1")]
    mode: LimitMode,
    /// The EEPROM limits, or `None` until Setup records them.
    limits: Option<Limits>,
}

impl Cache {
    /// The cache of actuator `id`: limit mode [`LimitMode::Bounded`], limits
    /// unknown.
    #[cfg_attr(
        not(any(
            feature = "ax",
            feature = "mx",
            feature = "rx",
            feature = "x-series",
            feature = "y-series",
            feature = "pro"
        )),
        allow(dead_code, reason = "only generated models construct it")
    )]
    pub(crate) const fn new(id: Id) -> Self {
        Self {
            id,
            mode: LimitMode::Bounded,
            limits: None,
        }
    }

    /// The actuator's ID.
    pub(crate) const fn id(&self) -> Id {
        self.id
    }

    /// The limits read at Setup, or `None` before Setup read them.
    pub(crate) const fn limits(&self) -> Option<Limits> {
        self.limits
    }

    /// Records the limits `min..=max` read from the actuator's EEPROM.
    ///
    /// # Errors
    ///
    /// [`ValidationError::InvertedLimits`] if `min > max`; the recorded
    /// limits do not change.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) fn record_limits(&mut self, min: i32, max: i32) -> Result<Limits, ValidationError> {
        let limits = Limits::new(self.id, min, max)?;
        self.limits = Some(limits);
        Ok(limits)
    }

    /// Records the limit mode Setup found. The limits do not change.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) fn record_mode(&mut self, mode: LimitMode) {
        self.mode = mode;
    }

    /// Checks goal tick `raw` against the limit mode and returns it.
    ///
    /// - [`LimitMode::Bounded`]: `raw` must lie in the recorded limits, both
    ///   bounds included.
    /// - [`LimitMode::Unbounded`]: any `raw` passes.
    /// - Any mode that ignores Goal Position: every goal is refused.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::LimitsUnknown`]: bounded, and no limits were
    ///   recorded.
    /// - [`ValidationError::OutOfLimits`]: bounded, and `raw` is outside
    ///   them.
    /// - [`ValidationError::NotPositionMode`]: the mode ignores goal
    ///   positions, whether limits are known or not.
    #[allow(dead_code, reason = "used by bus-1")]
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
            (mode, _) if mode.allows_goal_position() => Ok(raw),
            _ => Err(ValidationError::NotPositionMode { id: self.id }),
        }
    }

    /// The limit mode Setup recorded.
    #[allow(dead_code, reason = "used by the bus part of FR-9")]
    pub(crate) const fn mode(&self) -> LimitMode {
        self.mode
    }

    /// Checks that the recorded mode uses a goal velocity.
    ///
    /// # Errors
    ///
    /// [`ValidationError::NotVelocityMode`] in any other mode.
    #[allow(dead_code, reason = "used by the bus part of FR-9")]
    pub(crate) const fn check_goal_velocity(&self) -> Result<(), ValidationError> {
        if self.mode.allows_goal_velocity() {
            Ok(())
        } else {
            Err(ValidationError::NotVelocityMode {
                id: self.id,
                mode: self.mode,
            })
        }
    }

    /// Checks that the recorded mode uses a goal current.
    ///
    /// # Errors
    ///
    /// [`ValidationError::NotCurrentMode`] in any other mode.
    #[allow(dead_code, reason = "used by the bus part of FR-9")]
    pub(crate) const fn check_goal_current(&self) -> Result<(), ValidationError> {
        if self.mode.allows_goal_current() {
            Ok(())
        } else {
            Err(ValidationError::NotCurrentMode {
                id: self.id,
                mode: self.mode,
            })
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
    use crate::register::{Access, Area, Register};
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

    /// The register at `addr`, `size` bytes wide, with its e-Manual access
    /// and area.
    const fn reg(addr: u16, size: u8, access: Access, area: Area) -> Register {
        Register {
            addr,
            size,
            access,
            area,
        }
    }

    /// Operating Mode(11) of the X series, MX(2.0), PRO and RH-P12-RN, and
    /// Control Mode(11) of the XL-320.
    const MODE_11: Register = reg(11, 1, Access::RW, Area::Eeprom);

    /// XM430-W350 Operating Mode values (e-Manual, Operating Mode(11)).
    const X_MODES: ModeValues = ModeValues {
        bounded: &[3],
        unbounded: &[4, 5],
        no_position: &[0, 1, 16],
        current: &[0],
        velocity: &[1],
        pwm: &[16],
        current_position: &[5],
    };

    /// Mode values of a table that gives only the three coarse lists.
    const fn coarse(
        bounded: &'static [u8],
        unbounded: &'static [u8],
        no_position: &'static [u8],
    ) -> ModeValues {
        ModeValues {
            bounded,
            unbounded,
            no_position,
            current: &[],
            velocity: &[],
            pwm: &[],
            current_position: &[],
        }
    }

    const XM430: ModelData = ModelData {
        name: "XM430-W350",
        model_number: 1020,
        protocols: Protocols { p1: true, p2: true },
        torque_enable: reg(64, 1, Access::RW, Area::Ram),
        goal_position: reg(116, 4, Access::RW, Area::Ram),
        present_position: reg(132, 4, Access::R, Area::Ram),
        limits: LimitRegisters::MinMax {
            min: reg(52, 4, Access::RW, Area::Eeprom),
            max: reg(48, 4, Access::RW, Area::Eeprom),
        },
        hardware_error: Some(reg(70, 1, Access::R, Area::Ram)),
        mode: ModeSource::Register {
            register: MODE_11,
            values: X_MODES,
        },
        scale: PositionScale::new(2048, 0, 4095, -PI, PI),
        roles: Roles::NONE,
        bulk_read_p1: false,
    };

    const AX12A: ModelData = ModelData {
        name: "AX-12A",
        model_number: 12,
        protocols: Protocols {
            p1: true,
            p2: false,
        },
        torque_enable: reg(24, 1, Access::RW, Area::Ram),
        goal_position: reg(30, 2, Access::RW, Area::Ram),
        present_position: reg(36, 2, Access::R, Area::Ram),
        limits: LimitRegisters::CwCcw {
            cw: reg(6, 2, Access::RW, Area::Eeprom),
            ccw: reg(8, 2, Access::RW, Area::Eeprom),
        },
        hardware_error: None,
        mode: ModeSource::AngleLimits,
        scale: PositionScale::new(512, 0, 1023, 0.0, 5.2359877),
        roles: Roles::NONE,
        bulk_read_p1: false,
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
        assert_eq!(XM430.hardware_error, Some(reg(70, 1, Access::R, Area::Ram)));
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
        const RH_P12_RN_MODES: ModeValues = coarse(&[5], &[], &[0]);

        /// XL-320 Control Mode values (e-Manual, Control Mode(11)).
        const XL320_MODES: ModeValues = coarse(&[2], &[], &[1]);

        const NO_MODES: ModeValues = coarse(&[], &[], &[]);

        /// The XM430-W350 lists without their per-mode split.
        const X_COARSE: ModeValues = coarse(&[3], &[4, 5], &[0, 1, 16]);

        fn limits(cw: i32, ccw: i32) -> Limits {
            Limits::new(id(1), cw, ccw).unwrap()
        }

        /// The mode of the list that holds `value`, read from the lists.
        fn listed(values: ModeValues, value: u8) -> Option<LimitMode> {
            [
                (values.bounded, LimitMode::Bounded),
                (values.current_position, LimitMode::UnboundedCurrent),
                (values.unbounded, LimitMode::Unbounded),
                (values.current, LimitMode::Current),
                (values.velocity, LimitMode::Velocity),
                (values.pwm, LimitMode::Pwm),
                (values.no_position, LimitMode::Pwm),
            ]
            .into_iter()
            .find(|(list, _)| list.contains(&value))
            .map(|(_, mode)| mode)
        }

        #[test]
        fn values_map_to_their_list() {
            for values in [X_MODES, X_COARSE, RH_P12_RN_MODES, XL320_MODES] {
                for value in 0..=255 {
                    assert_eq!(values.limit_mode(value), listed(values, value));
                }
            }
            for value in [0, 1, 16] {
                assert_eq!(X_COARSE.limit_mode(value), Some(LimitMode::Pwm));
            }
            assert_eq!(X_MODES.limit_mode(3), Some(LimitMode::Bounded));
            assert_eq!(X_MODES.limit_mode(4), Some(LimitMode::Unbounded));
            for values in [X_MODES, X_COARSE] {
                let listed_count = (0..=255).filter(|&v| values.limit_mode(v).is_some());
                assert_eq!(listed_count.count(), 6);
            }
        }

        #[test]
        fn per_mode_lists_split_the_coarse_ones() {
            assert_eq!(X_MODES.limit_mode(0), Some(LimitMode::Current));
            assert_eq!(X_MODES.limit_mode(1), Some(LimitMode::Velocity));
            assert_eq!(X_MODES.limit_mode(16), Some(LimitMode::Pwm));
            assert_eq!(X_MODES.limit_mode(5), Some(LimitMode::UnboundedCurrent));
            assert_eq!(X_COARSE.limit_mode(5), Some(LimitMode::Unbounded));
            let pwm_only = ModeValues {
                pwm: &[7],
                ..NO_MODES
            };
            assert_eq!(pwm_only.limit_mode(7), Some(LimitMode::Pwm));
        }

        #[test]
        fn value_5_differs_by_list() {
            let gripper = ModeSource::Register {
                register: MODE_11,
                values: RH_P12_RN_MODES,
            };
            let x = ModeSource::Register {
                register: MODE_11,
                values: X_MODES,
            };
            assert_eq!(
                gripper.limit_mode(5, limits(0, 0)),
                Some(LimitMode::Bounded)
            );
            assert_eq!(
                x.limit_mode(5, limits(0, 0)),
                Some(LimitMode::UnboundedCurrent)
            );
        }

        #[test]
        fn control_mode_is_a_register_source() {
            let control_mode = ModeSource::Register {
                register: MODE_11,
                values: XL320_MODES,
            };
            for value in 0..=255 {
                let expected = match value {
                    1 => Some(LimitMode::Pwm),
                    2 => Some(LimitMode::Bounded),
                    _ => None,
                };
                assert_eq!(control_mode.limit_mode(value, limits(0, 0)), expected);
            }
        }

        #[test]
        fn empty_lists_give_no_mode() {
            let source = ModeSource::Register {
                register: MODE_11,
                values: NO_MODES,
            };
            for value in 0..=255 {
                assert_eq!(NO_MODES.limit_mode(value), None);
                assert_eq!(source.limit_mode(value, limits(0, 1023)), None);
            }
        }

        #[test]
        fn angle_limits_decide_the_mode() {
            let mode = |cw, ccw| LimitMode::from_angle_limits(Limits::new(id(1), cw, ccw).unwrap());
            assert_eq!(mode(0, 0), LimitMode::Wheel);
            assert_eq!(mode(4095, 4095), LimitMode::Unbounded);
            for (cw, ccw) in [(0, 1023), (1, 1023), (0, 4095), (1023, 1023), (0, 1)] {
                assert_eq!(mode(cw, ccw), LimitMode::Bounded);
            }
        }

        #[test]
        fn register_to_read() {
            let register = MODE_11;
            let source = ModeSource::Register {
                register,
                values: X_MODES,
            };
            assert_eq!(source.register(), Some(register));
            assert_eq!(ModeSource::AngleLimits.register(), None);
            for value in 0..=255 {
                assert_eq!(
                    ModeSource::AngleLimits.limit_mode(value, limits(0, 1023)),
                    None
                );
            }
            assert_eq!(XM430.mode, source);
            assert_eq!(AX12A.mode, ModeSource::AngleLimits);
        }

        /// MX-64 (Protocol 1.0) Torque Ctrl Mode Enable(70), values 0 and
        /// 1 (<https://emanual.robotis.com/docs/en/dxl/mx/mx-64/>).
        const MX64_P1_TORQUE: ModeSource = ModeSource::TorqueControl {
            register: Register {
                addr: 70,
                size: 1,
                access: Access::RW,
                area: Area::Ram,
            },
            on: 1,
            values: (0, 1),
        };

        #[test]
        fn torque_switch_overrides_the_angle_limits() {
            assert_eq!(MX64_P1_TORQUE.register().map(|r| r.addr), Some(70));
            for (cw, ccw) in [(0, 0), (4095, 4095), (0, 4095)] {
                assert_eq!(
                    MX64_P1_TORQUE.limit_mode(1, limits(cw, ccw)),
                    Some(LimitMode::Current)
                );
                assert_eq!(
                    MX64_P1_TORQUE.limit_mode(0, limits(cw, ccw)),
                    Some(LimitMode::from_angle_limits(limits(cw, ccw)))
                );
            }
            assert_eq!(
                MX64_P1_TORQUE.limit_mode(0, limits(0, 0)),
                Some(LimitMode::Wheel)
            );
            for value in 2..=255 {
                assert_eq!(MX64_P1_TORQUE.limit_mode(value, limits(0, 4095)), None);
            }
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
            assert_eq!(LimitMode::Pwm as u8, 2);
            assert_eq!(LimitMode::Wheel as u8, 6);
        }

        const ALL_MODES: [LimitMode; 7] = [
            LimitMode::Bounded,
            LimitMode::Unbounded,
            LimitMode::Pwm,
            LimitMode::UnboundedCurrent,
            LimitMode::Velocity,
            LimitMode::Current,
            LimitMode::Wheel,
        ];

        #[test]
        fn each_mode_allows_the_goals_its_firmware_uses() {
            use LimitMode as M;
            for mode in ALL_MODES {
                assert_eq!(
                    mode.allows_goal_position(),
                    matches!(mode, M::Bounded | M::Unbounded | M::UnboundedCurrent),
                    "{mode:?}"
                );
                assert_eq!(
                    mode.allows_goal_velocity(),
                    matches!(mode, M::Velocity | M::Wheel),
                    "{mode:?}"
                );
                assert_eq!(
                    mode.allows_goal_current(),
                    matches!(mode, M::Current | M::UnboundedCurrent),
                    "{mode:?}"
                );
            }
        }

        #[test]
        fn goal_velocity_and_current_follow_the_mode() {
            for mode in ALL_MODES {
                let mut cache = limited(id(4));
                cache.record_mode(mode);
                assert_eq!(cache.mode(), mode);
                let velocity = if mode.allows_goal_velocity() {
                    Ok(())
                } else {
                    Err(ValidationError::NotVelocityMode { id: id(4), mode })
                };
                let current = if mode.allows_goal_current() {
                    Ok(())
                } else {
                    Err(ValidationError::NotCurrentMode { id: id(4), mode })
                };
                assert_eq!(cache.check_goal_velocity(), velocity, "{mode:?}");
                assert_eq!(cache.check_goal_current(), current, "{mode:?}");
                if !mode.allows_goal_position() {
                    assert_eq!(
                        cache.check_goal(2048),
                        Err(ValidationError::NotPositionMode { id: id(4) })
                    );
                }
            }
        }

        #[test]
        fn new_errors_name_the_actuator() {
            let msg = |e: ValidationError| e.to_string();
            assert_eq!(
                msg(ValidationError::Unsupported {
                    id: id(7),
                    register: "Present Current"
                }),
                "actuator 7 has no Present Current register"
            );
            assert!(
                msg(ValidationError::NotVelocityMode {
                    id: id(7),
                    mode: LimitMode::Bounded
                })
                .starts_with("actuator 7 is in Bounded mode")
            );
            assert!(
                msg(ValidationError::NotCurrentMode {
                    id: id(7),
                    mode: LimitMode::Velocity
                })
                .starts_with("actuator 7 is in Velocity mode")
            );
            assert_eq!(
                msg(ValidationError::InvalidSlopeStep(9)),
                "compliance slope step 9 is outside 1..=7"
            );
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
            known.record_mode(LimitMode::Pwm);
            let mut unknown = Cache::new(id(4));
            unknown.record_mode(LimitMode::Pwm);
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
            for mode in [LimitMode::Unbounded, LimitMode::Pwm, LimitMode::Bounded] {
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

    /// Tests over the generated models: every variant the lookup returns,
    /// and one named model per family.
    #[allow(
        dead_code,
        unused_macros,
        reason = "helpers for the named models are unused when their families are off"
    )]
    mod generated {
        use core::mem::discriminant;
        use core::ops::RangeInclusive;

        use std::vec::Vec;

        use super::*;
        use crate::models::ActuatorKind;
        use crate::units::Position;

        /// Every variant [`ActuatorKind::new`] returns for actuator `id`, once
        /// each: every model number on both protocols, with every firmware
        /// when the model's table depends on it.
        fn every_variant(id: Id) -> Vec<ActuatorKind> {
            let mut found: Vec<ActuatorKind> = Vec::new();
            for protocol in [ProtocolVersion::P1, ProtocolVersion::P2] {
                for number in 0..=u16::MAX {
                    let model = ModelNumber(number);
                    let firmwares = match ActuatorKind::firmware_register(protocol, model) {
                        Some(_) => 0..=u8::MAX,
                        None => 0..=0,
                    };
                    for firmware in firmwares {
                        let Ok(kind) = ActuatorKind::new(protocol, model, firmware, id) else {
                            continue;
                        };
                        if !found.iter().any(|k| discriminant(k) == discriminant(&kind)) {
                            found.push(kind);
                        }
                    }
                }
            }
            found
        }

        /// What every model answers before Setup, and what its table must
        /// hold for the mode source.
        fn check<A: Actuator>(a: &A, id: Id) {
            let data = a.data();
            let name = data.name;
            assert_eq!(a.id(), id, "{name}");
            assert_eq!(a.model_number(), data.model_number, "{name}");
            assert_eq!(a.name(), name);
            assert_eq!(a.position_scale(), data.scale, "{name}");
            assert_eq!(a.position_limits_raw(), None, "{name}");
            assert_eq!(a.position_limits(), None, "{name}");
            match data.mode {
                ModeSource::Register { register, values } => {
                    assert_eq!(
                        (register.size, register.access, register.area),
                        (1, Access::RW, Area::Eeprom),
                        "{name}"
                    );
                    let lists = [values.bounded, values.unbounded, values.no_position];
                    for value in 0..=u8::MAX {
                        let holding = lists.iter().filter(|list| list.contains(&value));
                        assert!(holding.count() <= 1, "{name}: {value} is in two lists");
                    }
                }
                ModeSource::AngleLimits => assert!(
                    matches!(data.limits, LimitRegisters::CwCcw { .. }),
                    "{name}: angle-limit mode without CW/CCW limits"
                ),
                ModeSource::TorqueControl {
                    register,
                    on,
                    values: (min, max),
                } => {
                    assert!(
                        matches!(data.limits, LimitRegisters::CwCcw { .. }),
                        "{name}: torque switch without CW/CCW limits"
                    );
                    assert_eq!(
                        (register.size, register.access, register.area),
                        (1, Access::RW, Area::Ram),
                        "{name}"
                    );
                    assert!(min <= on && on <= max, "{name}");
                }
            }
        }

        #[test]
        fn generic_check_passes_for_every_model() {
            let variants = every_variant(id(7));
            #[cfg(any(
                feature = "ax",
                feature = "mx",
                feature = "rx",
                feature = "x-series",
                feature = "y-series",
                feature = "pro"
            ))]
            assert!(!variants.is_empty());
            for kind in &variants {
                check(kind, id(7));
            }
        }

        /// Scales whose tick is about one `f32` step of an angle near ±π
        /// (Y series, from T1's `type_info`), as `(min_raw, max_raw)`: a tick
        /// read as a [`Position`] can come back up to 3 ticks off. Every other
        /// scale round-trips every tick exactly.
        const COARSER_THAN_F32: [(i32, i32); 2] =
            [(-13_369_344, 13_369_344), (-25_952_256, 25_952_256)];

        #[test]
        fn every_tick_of_every_model_round_trips() {
            let mut scales: Vec<PositionScale> = Vec::new();
            for kind in every_variant(id(1)) {
                let scale = kind.position_scale();
                if !scales.contains(&scale) {
                    scales.push(scale);
                }
            }
            for scale in scales {
                let range = scale.raw_range();
                let bound = (*range.start(), *range.end());
                let allowed = if COARSER_THAN_F32.contains(&bound) {
                    3
                } else {
                    0
                };
                for tick in range {
                    let back = scale.to_raw(scale.to_position(tick));
                    let off = back.map(|raw| (i64::from(raw) - i64::from(tick)).abs());
                    assert!(
                        off.is_ok_and(|off| off <= allowed),
                        "{scale:?}: tick {tick} came back as {back:?}"
                    );
                }
            }
        }

        #[test]
        fn goal_register_per_model() {
            #[cfg(feature = "x-series")]
            {
                let xm = ActuatorKind::new(ProtocolVersion::P2, ModelNumber(1020), 0, id(3));
                let goal = xm.map(|xm| xm.data().goal_position);
                assert_eq!(goal.map(|r| (r.addr, r.size)), Ok((116, 4)));
            }
            #[cfg(feature = "ax")]
            {
                let ax = ActuatorKind::new(ProtocolVersion::P1, ModelNumber(12), 0, id(9));
                let goal = ax.map(|ax| ax.data().goal_position);
                assert_eq!(goal.map(|r| (r.addr, r.size)), Ok((30, 2)));
            }
        }

        #[test]
        fn limit_registers_share_names() {
            fn min_max<D: Describe>(d: &D) -> (u16, u16) {
                let limits = d.data().limits;
                (limits.min().addr, limits.max().addr)
            }
            #[cfg(feature = "ax")]
            assert_eq!(min_max(&crate::models::Ax12a::new(id(9))), (6, 8));
            #[cfg(feature = "x-series")]
            assert_eq!(min_max(&crate::models::Xm430W350::new(id(3))), (52, 48));
        }

        /// Records limits 1000..=3000 on `a`, then checks both getters in
        /// every limit mode.
        fn limits_follow_the_cache<A: Actuator>(a: &mut A) {
            let scale = a.position_scale();
            let raw: Option<RangeInclusive<i32>> = Some(1000..=3000);
            let typed: Option<RangeInclusive<Position>> =
                Some(scale.to_position(1000)..=scale.to_position(3000));
            assert_eq!(a.cache_mut().record_limits(1000, 3000).map(|_| ()), Ok(()));
            for mode in [LimitMode::Bounded, LimitMode::Unbounded, LimitMode::Pwm] {
                a.cache_mut().record_mode(mode);
                assert_eq!(a.position_limits_raw(), raw, "{}", a.name());
                assert_eq!(a.position_limits(), typed, "{}", a.name());
            }
        }

        #[test]
        fn limits_show_after_recording() {
            for mut kind in every_variant(id(5)) {
                limits_follow_the_cache(&mut kind);
            }
            #[cfg(feature = "x-series")]
            limits_follow_the_cache(&mut crate::models::Xm430W350::new(id(5)));
            #[cfg(feature = "ax")]
            limits_follow_the_cache(&mut crate::models::Ax12a::new(id(5)));
        }

        /// A mode register at `addr` (1 byte, RW, EEPROM) with these lists.
        fn mode_register(
            addr: u16,
            bounded: &'static [u8],
            unbounded: &'static [u8],
            no_position: &'static [u8],
        ) -> ModeSource {
            ModeSource::Register {
                register: reg(addr, 1, Access::RW, Area::Eeprom),
                values: coarse(bounded, unbounded, no_position),
            }
        }

        /// The XM430-W350 and MX-64 (Protocol 2.0) Operating Mode(11).
        fn x_mode_register() -> ModeSource {
            ModeSource::Register {
                register: reg(11, 1, Access::RW, Area::Eeprom),
                values: X_MODES,
            }
        }

        fn mode_of<D: Describe>(d: &D) -> ModeSource {
            d.data().mode
        }

        #[test]
        fn mode_source_per_model() {
            #[cfg(feature = "x-series")]
            {
                use crate::models::{Xl320, Xm430W350};
                assert_eq!(mode_of(&Xm430W350::new(id(1))), x_mode_register());
                assert_eq!(
                    mode_of(&Xl320::new(id(1))),
                    mode_register(11, &[2], &[], &[1])
                );
            }
            #[cfg(feature = "mx")]
            {
                assert_eq!(mode_of(&crate::models::Mx64::new(id(1))), x_mode_register());
                assert!(matches!(
                    mode_of(&crate::models::Mx64P1::new(id(1))),
                    ModeSource::TorqueControl {
                        on: 1,
                        values: (0, 1),
                        ..
                    }
                ));
            }
            #[cfg(feature = "pro")]
            {
                use crate::models::{H54200S500Ra, RhP12Rn};
                let h54 = mode_register(11, &[3], &[4], &[0, 1, 16]);
                assert_eq!(mode_of(&H54200S500Ra::new(id(1))), h54);
                assert_eq!(
                    mode_of(&RhP12Rn::new(id(1))),
                    mode_register(11, &[5], &[], &[0])
                );
            }
            #[cfg(feature = "y-series")]
            {
                let ym = mode_register(33, &[3], &[], &[0, 1]);
                assert_eq!(mode_of(&crate::models::Ym070210M001::new(id(1))), ym);
            }
            #[cfg(feature = "ax")]
            assert_eq!(
                mode_of(&crate::models::Ax12a::new(id(1))),
                ModeSource::AngleLimits
            );
        }

        /// Every [`Actuator`] answer of `kind` equals that of `inner`.
        fn same_answers<A: Actuator>(kind: &ActuatorKind, inner: &A) {
            assert_eq!(kind.id(), inner.id());
            assert_eq!(kind.model_number(), inner.model_number());
            assert_eq!(kind.name(), inner.name());
            assert_eq!(kind.position_scale(), inner.position_scale());
            assert_eq!(kind.position_limits_raw(), inner.position_limits_raw());
            assert_eq!(kind.position_limits(), inner.position_limits());
        }

        /// Looks up `$number` on `$protocol`, expects variant `$variant`, and
        /// compares the enum with the model inside it before and after
        /// limits are recorded through the enum.
        macro_rules! answers_like_inner {
            ($variant:ident, $protocol:ident, $number:literal) => {{
                let protocol = ProtocolVersion::$protocol;
                let found = ActuatorKind::new(protocol, ModelNumber($number), 0, id(6));
                let Ok(mut kind) = found else {
                    panic!("{} not found", stringify!($variant));
                };
                let ActuatorKind::$variant(inner) = &kind else {
                    panic!("{} not looked up", stringify!($variant));
                };
                same_answers(&kind, inner);
                assert_eq!(
                    kind.cache_mut().record_limits(-100, 100).map(|_| ()),
                    Ok(())
                );
                let ActuatorKind::$variant(inner) = &kind else {
                    panic!("{} changed variant", stringify!($variant));
                };
                same_answers(&kind, inner);
                assert_eq!(inner.position_limits_raw(), Some(-100..=100));
            }};
        }

        #[test]
        fn enum_answers_like_the_inner_model() {
            #[cfg(feature = "x-series")]
            answers_like_inner!(Xm430W350, P2, 1020);
            #[cfg(feature = "ax")]
            answers_like_inner!(Ax12a, P1, 12);
            #[cfg(feature = "mx")]
            answers_like_inner!(Mx64, P2, 311);
            #[cfg(feature = "pro")]
            answers_like_inner!(H54200S500Ra, P2, 54025);
            #[cfg(feature = "y-series")]
            answers_like_inner!(Ym070210M001, P2, 4000);
        }
    }
}
