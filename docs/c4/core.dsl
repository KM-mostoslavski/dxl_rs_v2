// C3/C4 for core.
// Source: .worktrees/int (src/lib.rs, id.rs, error.rs, actuator.rs, register.rs, units.rs, units/**), at core-3.
!element dxl.lib {

    # ---------------------------------------------------------------- C3
    core_root = component "core: Crate root" "src/lib.rs: #![no_std] crate root with missing_docs; declares the private modules and is the one public surface: re-exports core types, Bus/ActuatorRef/Config/IdSet, Protocol/P1/P2/Clock, Transport, embedded_io; DynBus, SerialBus, StdClock, SerialTransport and serialport only under std; MockTransport, MockTime, Mismatch only under mock." "Rust module (lib.rs)"
    core_ident = component "core: Identifiers" "Id (firmware ID 0..=252, const-checked) and ModelNumber (any u16, support decided at Setup)." "Rust newtypes"
    core_errors = component "core: Error taxonomy" "One Copy, no_std thiserror enum Error split into Validation, Transport, Protocol, Status and Setup, with decoded P1/P2 status error bytes." "thiserror 2 (no_std)"
    core_contract = component "core: Actuator contract" "Sealed, I/O-free Actuator trait over the crate-private Describe trait, and the static per-model description (ModelData) generated models provide: key registers, limit registers, protocols, mode source, position scale." "Rust traits + structs"
    core_state = component "core: Actuator state" "Runtime state the bus keeps per actuator: TorqueState (off/on/faulted), the 7-value LimitMode decided at Setup, and the Cache (Id, LimitMode, EEPROM Limits) that owns the one goal check." "Rust enums + structs"
    core_register = component "core: Register encoding" "Crate-private Register location (addr, size, Access, Area) with size-checked little-endian encode/decode; never truncates." "Rust struct"
    core_si = component "core: SI units" "Typed SI values: Position + PositionScale (ticks <-> radians), and RegisterScale + Sign converting raw registers to Velocity, SpeedLimit, Current, Voltage, Temperature, Load. f32, raw form always kept; scales have no public constructor." "Rust newtypes, f32"
    core_rawvals = component "core: Raw-backed register values" "Register values that stay raw-backed rather than SI: HardwareErrors flags, wrapping Realtime Tick, PID Gain, AX/RX ComplianceMargin and ComplianceSlope." "Rust newtypes"

    core_root -> core_ident "Re-exports Id, ModelNumber"
    core_root -> core_errors "Re-exports Error and its sub-enums"
    core_root -> core_contract "Re-exports the Actuator trait"
    core_root -> core_state "Re-exports TorqueState, LimitMode"
    core_root -> core_si "Re-exports units"
    core_root -> core_rawvals "Re-exports units"
    core_ident -> core_errors "Rejects IDs above 252 with ValidationError::InvalidId"
    core_contract -> core_ident "Exposes id() and model_number()"
    core_contract -> core_state "Reads and records the Cache via Describe::cache / cache_mut"
    core_contract -> core_register "Names Torque Enable, Goal/Present Position, limit, Hardware Error and mode registers as"
    core_contract -> core_si "Converts limits to radians with PositionScale"
    core_state -> core_errors "Refuses goals with ActuatorIsOff, ActuatorFaulted, OutOfLimits, LimitsUnknown, NotPositionMode, NotVelocityMode, NotCurrentMode"
    core_state -> core_si "Holds Limits (raw ticks)"
    core_register -> core_errors "Fails with DoesNotFit, DataLength"
    core_si -> core_errors "Fails with NotFinite, DoesNotFit"
    core_rawvals -> core_si "Converts ComplianceMargin to Position with PositionScale"
    core_rawvals -> core_errors "Fails with DoesNotFit, InvalidSlopeStep"

    # ---------------------------------------------------------------- C4
    group "core: Crate root" {
        core_lib = component "crate (lib.rs)" "#![no_std]; extern crate alloc under alloc, std under std; pub mod models, every other module private; pub use of the whole public API with feature-gated items (std, mock)." "Rust module" {
            tags "Code"
        }
    }

    group "core: Identifiers" {
        core_id = component "id::Id" "Firmware ID, 0..=Id::MAX (252) on both protocols; const fn new() -> Result, get(), TryFrom<u8>, From<Id> for u8." "Rust struct (u8 newtype)" {
            tags "Code"
        }
        core_modelnumber = component "actuator::ModelNumber" "Model Number register value (address 0, 2 bytes) as the actuator reports it; used by declared setup." "Rust struct (u16 newtype)" {
            tags "Code"
        }
    }

    group "core: Error taxonomy" {
        core_error = component "error::Error" "Top-level #[non_exhaustive] error: Validation | Transport | Protocol | Status | Setup, each #[from] and #[error(transparent)]." "Rust enum, thiserror" {
            tags "Code"
        }
        core_validationerror = component "error::ValidationError" "Rejected before a byte is sent: InvalidId, AddressOutOfRange, PacketTooLarge, DataLength, ActuatorIsOff, ActuatorFaulted, OutOfLimits, LimitsUnknown, InvertedLimits, NotFinite, DoesNotFit, BufferLength, DuplicateId, NotPositionMode, NotVelocityMode, NotCurrentMode, Unsupported, InvalidSlopeStep." "Rust enum, thiserror" {
            tags "Code"
        }
        core_setuperror = component "error::SetupError" "Init failures: UnsupportedModel { id, model_number, protocol, feature hint }, TooManyActuators, Missing, WrongModel, NotOnBus, UnknownOperatingMode." "Rust enum, thiserror" {
            tags "Code"
        }
        core_transporterror = component "error::TransportError" "Io(embedded_io::ErrorKind) and Timeout { id: Option<Id> }." "Rust enum, thiserror" {
            tags "Code"
        }
        core_protocolerror = component "error::ProtocolError" "Malformed replies: Checksum (P1), Crc (P2), WrongId, WrongLength, Malformed." "Rust enum, thiserror" {
            tags "Code"
        }
        core_statuserror = component "error::StatusError" "Non-zero status packet error byte from actuator id: StatusReport::P1(P1Flags) or P2 { alert, code: P2Code }; is_hardware_fault() = P2 alert bit, or P1 Overload/Overheating/InputVoltage." "Rust struct + enums (StatusReport, P2Code, P1Flag, P1Flags)" {
            tags "Code"
        }
    }

    group "core: Actuator contract" {
        core_actuator_trait = component "actuator::Actuator" "Public sealed trait, no I/O, all default methods: id, model_number, name, position_scale, position_limits_raw, position_limits (None before Setup, never table defaults). Implemented by every model type and by ActuatorKind." "Rust trait (sealed)" {
            tags "Code"
        }
        core_describe = component "actuator::sealed::Describe" "Crate-private supertrait that seals Actuator: data() -> &'static ModelData, cache(), cache_mut()." "Rust trait, pub(crate)" {
            tags "Code"
        }
        core_modeldata = component "actuator::ModelData" "Static per-model description, one &'static per generated type: name, model_number, Protocols { p1, p2 }, torque_enable / goal_position / present_position Registers, LimitRegisters (MinMax | CwCcw), optional hardware_error Register, ModeSource, PositionScale." "Rust struct (pub(crate) fields)" {
            tags "Code"
        }
        core_modesource = component "actuator::ModeSource" "Where Setup gets the LimitMode: Register { register, values: ModeValues (bounded / unbounded / no_position lists from the table's operating_modes) } or AngleLimits (AX, MX on P1)." "Rust enum, pub(crate)" {
            tags "Code"
        }
    }

    group "core: Actuator state" {
        core_torquestate = component "actuator::TorqueState" "Off (default) | On | Faulted; faulted means off. Crate-only transitions enable / disable / fault / clear_fault and check_goal; only On accepts a goal." "Rust enum (repr u8)" {
            tags "Code"
        }
        core_limitmode = component "actuator::LimitMode" "7-value #[non_exhaustive] goal-check mode: Bounded, Unbounded, Pwm, UnboundedCurrent, Velocity, Current, Wheel; allows_goal_position / _velocity / _current; from_angle_limits() for AX / MX on P1 (0,0 = Wheel, 4095,4095 = Unbounded)." "Rust enum (repr u8)" {
            tags "Code"
        }
        core_cache = component "actuator::Cache" "Per-actuator cache inside each model type: Id, LimitMode, Option<Limits>. record_limits, record_mode, and the one goal check check_goal (typed and raw alike), plus check_goal_velocity / check_goal_current." "Rust struct" {
            tags "Code"
        }
        core_limits = component "units::Limits" "EEPROM position limits min..=max in raw ticks, built only when min <= max (else InvertedLimits); crate-private." "Rust struct, pub(crate)" {
            tags "Code"
        }
    }

    group "core: Register encoding" {
        core_reg = component "register::Register" "Control-table location: addr, size (1, 2 or 4), Access (R | RW), Area (Eeprom | Ram | Hybrid). encode(): little-endian, refuses values that do not fit; decode(): sizes 1 and 2 unsigned, 4 two's complement." "Rust struct + enums, crate-private" {
            tags "Code"
        }
    }

    group "core: SI units" {
        core_position = component "units::Position" "Angle in radians (f32), zero at the model's zero tick, CCW positive; ZERO, from_radians, radians, Add/Sub." "Rust struct (f32 newtype)" {
            tags "Code"
        }
        core_positionscale = component "units::PositionScale" "Per-model tick <-> radian map from the table's type_info: zero_raw, radians_per_tick, to_position, to_raw (rounds half away, refuses non-finite). No public constructor, no range clamp." "Rust struct" {
            tags "Code"
        }
        core_registerscale = component "units::RegisterScale" "Scale of one register of one model: si_per_raw, Sign, size, zero_is_unlimited. pub(crate) new only. Conversions: velocity / velocity_to_raw, speed_limit / speed_limit_to_raw, current / current_to_raw, voltage, temperature, load; never truncates." "Rust struct" {
            tags "Code"
        }
        core_sign = component "units::Sign" "Sign encoding of a register: Unsigned, DirectionBit(n) (P1 speed/load, bit n = CW) or TwosComplement (P2)." "Rust enum" {
            tags "Code"
        }
        core_velocity = component "units::Velocity" "Angular velocity in rad/s (f32), CCW positive; ZERO, from_rad_per_s, rad_per_s." "Rust struct (f32 newtype)" {
            tags "Code"
        }
        core_speedlimit = component "units::SpeedLimit" "P1 Moving Speed in joint mode: Unlimited (raw 0) or Limited(Velocity)." "Rust enum" {
            tags "Code"
        }
        core_current = component "units::Current" "Electric current in amperes (f32), CCW positive; ZERO, from_amps, amps." "Rust struct (f32 newtype)" {
            tags "Code"
        }
        core_voltage = component "units::Voltage" "Input voltage in volts (f32); read-only, no constructor." "Rust struct (f32 newtype)" {
            tags "Code"
        }
        core_temperature = component "units::Temperature" "Internal temperature in degrees Celsius (f32); read-only." "Rust struct (f32 newtype)" {
            tags "Code"
        }
        core_load = component "units::Load" "P1 Present Load as a ratio of max torque, clamped to -1.0..=1.0; read-only." "Rust struct (f32 newtype)" {
            tags "Code"
        }
    }

    group "core: Raw-backed register values" {
        core_hardwareerrors = component "units::HardwareErrors" "Hardware Error Status flags: input_voltage, overheating, encoder, electrical_shock, overload, is_empty; raw() keeps unused bits." "Rust struct (u8 newtype)" {
            tags "Code"
        }
        core_tick = component "units::Tick" "Realtime Tick in ms; elapsed_since(earlier, modulus) handles the wrap." "Rust struct (u16 newtype)" {
            tags "Code"
        }
        core_gain = component "units::Gain" "Position P/D gain in the register's own units; new(value, size) checks the register size." "Rust struct (u16 newtype)" {
            tags "Code"
        }
        core_compliance = component "units::ComplianceMargin / ComplianceSlope" "AX/RX compliance: margin in ticks (to_position via PositionScale); slope as e-Manual step 1..=7 (new(step), step())." "Rust structs (u8 newtypes)" {
            tags "Code"
        }
    }

    # Code-level relationships
    core_lib -> core_id "pub use"
    core_lib -> core_error "pub use"
    core_lib -> core_actuator_trait "pub use"
    core_lib -> core_registerscale "pub use"
    core_id -> core_validationerror "new() fails with InvalidId"
    core_error -> core_validationerror "Wraps" "#[from]"
    core_error -> core_setuperror "Wraps" "#[from]"
    core_error -> core_transporterror "Wraps" "#[from]"
    core_error -> core_protocolerror "Wraps" "#[from]"
    core_error -> core_statuserror "Wraps" "#[from]"
    core_setuperror -> core_modelnumber "Reports UnsupportedModel / WrongModel with"
    core_actuator_trait -> core_describe "Is sealed by (supertrait)"
    core_actuator_trait -> core_positionscale "Returns, and converts limits with"
    core_describe -> core_modeldata "data() returns"
    core_describe -> core_cache "cache() / cache_mut() return"
    core_modeldata -> core_reg "Holds key registers as"
    core_modeldata -> core_modesource "Holds"
    core_modeldata -> core_positionscale "Holds"
    core_modesource -> core_limitmode "Decodes a mode value into (no_position values give Pwm)"
    core_cache -> core_id "Keeps"
    core_cache -> core_limitmode "Checks goals according to"
    core_cache -> core_limits "Keeps, recorded at Setup"
    core_cache -> core_validationerror "Refuses goals with"
    core_limitmode -> core_limits "from_angle_limits() reads"
    core_torquestate -> core_validationerror "Refuses with ActuatorIsOff / ActuatorFaulted"
    core_reg -> core_validationerror "Fails with DoesNotFit / DataLength"
    core_positionscale -> core_position "Converts raw ticks to and from"
    core_registerscale -> core_sign "Decodes raw values with"
    core_registerscale -> core_velocity "Converts to and from"
    core_registerscale -> core_speedlimit "Converts to and from"
    core_registerscale -> core_current "Converts to and from"
    core_registerscale -> core_voltage "Converts raw to"
    core_registerscale -> core_temperature "Converts raw to"
    core_registerscale -> core_load "Converts raw to"
    core_speedlimit -> core_velocity "Wraps"
    core_compliance -> core_positionscale "Converts the margin with"
    core_gain -> core_validationerror "new() fails with DoesNotFit"
}

# C2 components of workspace.dsl refined by this module
dxl.lib.units -> dxl.lib.core_si "Is refined at C3 by"
dxl.lib.units -> dxl.lib.core_ident "Is refined at C3 by"
dxl.lib.errors -> dxl.lib.core_errors "Is refined at C3 by"
dxl.lib.kind -> dxl.lib.core_actuator_trait "Implements by match" "static dispatch"
dxl.lib.models -> dxl.lib.core_actuator_trait "Implements"
dxl.lib.models -> dxl.lib.core_modeldata "Provides one &'static per model" "generated const"
dxl.lib.models -> dxl.lib.core_cache "Embeds one per actuator"
dxl.lib.models -> dxl.lib.core_reg "Declares registers as associated consts of"
dxl.lib.bus -> dxl.lib.core_torquestate "Keeps one per actuator slot"
dxl.lib.scan -> dxl.lib.core_cache "Records EEPROM limits and LimitMode into"
dxl.lib.scan -> dxl.lib.core_limitmode "Derives AX / MX P1 mode with from_angle_limits"
dxl.lib.handle -> dxl.lib.core_cache "Checks every goal with check_goal"
dxl.lib.handle -> dxl.lib.core_reg "Encodes and decodes register bytes with"
dxl.lib.fault -> dxl.lib.core_statuserror "Raises hardware faults as"
dxl.lib.transport -> dxl.lib.core_transporterror "Reports I/O and timeouts as"
app -> dxl.lib.core_root "Imports the public API from" "use dxl_rs_v2::..."
