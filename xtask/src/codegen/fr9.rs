//! FR-9: the typed register roles of each model, with the scales its table's
//! `unit_info` gives them, and the checks of the FR-9 table fields
//! (checks 18 to 22).
//!
//! A role is matched by exact e-Manual row name, like the common roles. Its
//! scales come only from a table with `unit_source`, converted to SI here:
//! rpm to rad/s, mA to A, V, °C, % to a ratio, ms to s, deg to rad. A
//! register whose table gives no unit, or a unit that is not the role's
//! quantity, gets no scale: raw only (FR-9.2).
//!
//! A direction-bit load is the one exception to `value × factor`: the
//! e-Manual gives its step as "about 0.1%" and its full magnitude as 100 %
//! (AX-12A Max Torque(14): "Data 1,023 (0x3FF) means that DYNAMIXEL will
//! use 100% of the maximum torque",
//! <https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>, 2026-11-03), so a
//! step is 1/1023 of the maximum and raw 1023 reads as exactly 1.0
//! (specs/008-typed-registers-core). Check 22 refuses a table whose
//! "about" step is more than 5 % off that.

use std::f64::consts::PI;
use std::ops::RangeInclusive;

use super::Diagnostic;
use super::schema::{Access, Area, ControlTable, RawRange, Row, Source, UnitEntry};

/// The `LimitMode` variants, in their `repr(u8)` order.
pub(crate) const LIMIT_MODES: [&str; 7] = [
    "Bounded",
    "Unbounded",
    "Pwm",
    "UnboundedCurrent",
    "Velocity",
    "Current",
    "Wheel",
];

/// A set of limit modes, bit `i` for `LIMIT_MODES[i]`.
pub(crate) type Modes = u8;

/// Every limit mode.
pub(crate) const ALL: Modes = 0x7F;
/// `LimitMode::Wheel` alone.
const WHEEL: Modes = 1 << 6;

/// The limit mode of each Protocol 1 angle-limit mode name of `unit_info`.
const P1_MODES: [(&str, Modes); 4] = [
    ("joint", 1 << 0),
    ("multi_turn", 1 << 1),
    ("torque_control", 1 << 5),
    ("wheel", WHEEL),
];

/// How far a direction-bit load's table step may sit from 1 / full
/// magnitude, as a share of it.
const LOAD_STEP_TOLERANCE: f64 = 0.05;

/// The units `unit_info` may give.
const UNITS: [&str; 8] = ["deg", "rpm", "mA", "V", "°C", "%", "ms", "deg/s^2"];

// DECISION: refuse ranged or unit rows outside 1..=7 bytes instead of
// widening to i128: the e-Manual tables use 1, 2 and 4, and 8 overflows i64.
/// The register sizes, in bytes, whose raw span fits an `i64`: a range or
/// a `sign_type` is checked only on a row of one of these sizes.
const SPAN_SIZES: RangeInclusive<u8> = 1..=7;

/// What a role's typed value is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Quantity {
    /// `Velocity`, rad/s.
    Velocity,
    /// `Current`, A.
    Current,
    /// `Voltage`, V.
    Voltage,
    /// `Temperature`, °C.
    Temperature,
    /// `Load`, a ratio.
    Load,
    /// `Tick`, s per raw step.
    Tick,
    /// `ComplianceMargin`, rad per raw step.
    Angle,
    /// A flag, bit set, gain or step: no scale.
    Raw,
}

impl Quantity {
    /// SI units per `unit`, or `None` when `unit` is not this quantity.
    pub(crate) fn factor(self, unit: &str) -> Option<f64> {
        match (self, unit) {
            (Self::Velocity, "rpm") => Some(2.0 * PI / 60.0),
            (Self::Current, "mA") | (Self::Tick, "ms") => Some(1e-3),
            (Self::Voltage, "V") | (Self::Temperature, "°C") => Some(1.0),
            (Self::Load, "%") => Some(1e-2),
            (Self::Angle, "deg") => Some(PI / 180.0),
            _ => None,
        }
    }

    /// Units the e-Manual gives this quantity's register in some mode or
    /// profile that are not the quantity: AX-12A wheel-mode speeds are a
    /// share of output in %, and a time-based Profile Velocity is in ms.
    /// They leave the register raw in that mode instead of failing.
    fn not_typed(self, unit: &str) -> bool {
        matches!((self, unit), (Self::Velocity, "%" | "ms"))
    }
}

/// Where a role's row comes from besides a row name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Switch {
    /// The `torque_control_mode.register` row.
    Register,
    /// The `torque_control_mode.goal` row.
    Goal,
}

/// One FR-9 role, in the order of the core `Role` enum.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RoleDef {
    /// The core `Role` variant.
    pub(crate) variant: &'static str,
    /// The `Roles` field.
    pub(crate) field: &'static str,
    /// The Tier 2 trait.
    pub(crate) has: &'static str,
    /// The typed value.
    pub(crate) quantity: Quantity,
    /// Row names that serve the role, each with the limit modes its scales
    /// count in.
    pub(crate) rows: &'static [(&'static str, Modes)],
    /// A `torque_control_mode` row that serves it.
    pub(crate) switch: Option<Switch>,
}

const fn role(
    variant: &'static str,
    field: &'static str,
    has: &'static str,
    quantity: Quantity,
    rows: &'static [(&'static str, Modes)],
) -> RoleDef {
    RoleDef {
        variant,
        field,
        has,
        quantity,
        rows,
        switch: None,
    }
}

/// Every role. Moving Speed serves two: in wheel mode it is the goal
/// velocity, in every other mode a speed limit.
pub(crate) const ROLES: [RoleDef; 25] = [
    role(
        "PresentTemperature",
        "present_temperature",
        "HasPresentTemperature",
        Quantity::Temperature,
        &[("Present Temperature", ALL)],
    ),
    role(
        "PresentVoltage",
        "present_voltage",
        "HasPresentVoltage",
        Quantity::Voltage,
        &[("Present Voltage", ALL), ("Present Input Voltage", ALL)],
    ),
    role(
        "PresentVelocity",
        "present_velocity",
        "HasPresentVelocity",
        Quantity::Velocity,
        &[("Present Velocity", ALL), ("Present Speed", ALL)],
    ),
    role(
        "GoalVelocity",
        "goal_velocity",
        "HasGoalVelocity",
        Quantity::Velocity,
        &[("Goal Velocity", ALL), ("Moving Speed", WHEEL)],
    ),
    role(
        "SpeedLimit",
        "speed_limit",
        "HasSpeedLimit",
        Quantity::Velocity,
        &[("Profile Velocity", ALL), ("Moving Speed", ALL & !WHEEL)],
    ),
    role(
        "PresentLoad",
        "present_load",
        "HasPresentLoad",
        Quantity::Load,
        &[("Present Load", ALL)],
    ),
    role(
        "PresentCurrent",
        "present_current",
        "HasPresentCurrent",
        Quantity::Current,
        &[("Present Current", ALL)],
    ),
    RoleDef {
        switch: Some(Switch::Goal),
        ..role(
            "GoalCurrent",
            "goal_current",
            "HasGoalCurrent",
            Quantity::Current,
            &[("Goal Current", ALL)],
        )
    },
    role(
        "Moving",
        "moving",
        "HasMoving",
        Quantity::Raw,
        &[("Moving", ALL)],
    ),
    role("Led", "led", "HasLed", Quantity::Raw, &[("LED", ALL)]),
    role(
        "LedRed",
        "led_red",
        "HasLedRed",
        Quantity::Raw,
        &[("LED Red", ALL)],
    ),
    role(
        "LedGreen",
        "led_green",
        "HasLedGreen",
        Quantity::Raw,
        &[("LED Green", ALL)],
    ),
    role(
        "LedBlue",
        "led_blue",
        "HasLedBlue",
        Quantity::Raw,
        &[("LED Blue", ALL)],
    ),
    role(
        "HardwareErrorStatus",
        "hardware_error_status",
        "HasHardwareErrorStatus",
        Quantity::Raw,
        &[("Hardware Error Status", ALL)],
    ),
    role(
        "RealtimeTick",
        "realtime_tick",
        "HasRealtimeTick",
        Quantity::Tick,
        &[("Realtime Tick", ALL)],
    ),
    role(
        "PGain",
        "p_gain",
        "HasPGain",
        Quantity::Raw,
        &[("Position P Gain", ALL), ("P Gain", ALL)],
    ),
    role(
        "DGain",
        "d_gain",
        "HasDGain",
        Quantity::Raw,
        &[("Position D Gain", ALL), ("D Gain", ALL)],
    ),
    role(
        "CwComplianceMargin",
        "cw_compliance_margin",
        "HasCwComplianceMargin",
        Quantity::Angle,
        &[("CW Compliance Margin", ALL)],
    ),
    role(
        "CcwComplianceMargin",
        "ccw_compliance_margin",
        "HasCcwComplianceMargin",
        Quantity::Angle,
        &[("CCW Compliance Margin", ALL)],
    ),
    role(
        "CwComplianceSlope",
        "cw_compliance_slope",
        "HasCwComplianceSlope",
        Quantity::Raw,
        &[("CW Compliance Slope", ALL)],
    ),
    role(
        "CcwComplianceSlope",
        "ccw_compliance_slope",
        "HasCcwComplianceSlope",
        Quantity::Raw,
        &[("CCW Compliance Slope", ALL)],
    ),
    RoleDef {
        switch: Some(Switch::Register),
        ..role(
            "TorqueCtrlModeEnable",
            "torque_ctrl_mode_enable",
            "HasTorqueCtrlModeEnable",
            Quantity::Raw,
            &[],
        )
    },
    role(
        "MinVoltageLimit",
        "min_voltage_limit",
        "HasMinVoltageLimit",
        Quantity::Voltage,
        &[("Min Voltage Limit", ALL)],
    ),
    role(
        "MaxVoltageLimit",
        "max_voltage_limit",
        "HasMaxVoltageLimit",
        Quantity::Voltage,
        &[("Max Voltage Limit", ALL)],
    ),
    role(
        "TemperatureLimit",
        "temperature_limit",
        "HasTemperatureLimit",
        Quantity::Temperature,
        &[("Temperature Limit", ALL)],
    ),
];

/// How a register carries its sign, from `sign_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignKind {
    /// `unsigned`.
    Unsigned,
    /// `twos_complement`.
    TwosComplement,
    /// `direction_bit_N`.
    DirectionBit(u8),
}

impl SignKind {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "unsigned" => Some(Self::Unsigned),
            "twos_complement" => Some(Self::TwosComplement),
            _ => text
                .strip_prefix("direction_bit_")
                .and_then(|n| n.parse().ok())
                .map(Self::DirectionBit),
        }
    }

    /// The raw values a `size`-byte register holds under this sign, or
    /// `None` for a size outside [`SPAN_SIZES`] or a direction bit outside
    /// the register.
    fn raw_span(self, size: u8) -> Option<(i64, i64)> {
        if !SPAN_SIZES.contains(&size) {
            return None;
        }
        let bits = u32::from(size) * 8;
        let top = (1i64 << bits) - 1;
        match self {
            Self::Unsigned => Some((0, top)),
            Self::TwosComplement => Some((-(1i64 << (bits - 1)), (1i64 << (bits - 1)) - 1)),
            Self::DirectionBit(n) if n >= 1 && u32::from(n) < bits => Some((0, top)),
            Self::DirectionBit(_) => None,
        }
    }
}

/// One scale of one role: what one `unit_info` entry gives in some modes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GenScale {
    /// The limit modes it applies in.
    pub(crate) modes: Modes,
    /// SI units per raw step.
    pub(crate) si_per_raw: f64,
    /// The table's raw step, in `unit`.
    pub(crate) value: f64,
    /// The table's unit.
    pub(crate) unit: String,
    /// The sign encoding.
    pub(crate) sign: SignKind,
    /// The register size.
    pub(crate) size: u8,
    /// Raw 0 means no limit.
    pub(crate) zero_is_unlimited: bool,
    /// The raw range in these modes.
    pub(crate) range: Option<(i64, i64)>,
    /// The Drive Mode profile the entry is for, if only one.
    pub(crate) profile: Option<String>,
}

/// A `unit_info` entry of a role's row whose unit is not the role's
/// quantity, so the register stays raw in its modes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Untyped {
    /// The limit modes it applies in.
    pub(crate) modes: Modes,
    /// The table's raw step, in `unit`.
    pub(crate) value: f64,
    /// The table's unit.
    pub(crate) unit: String,
    /// The raw range the e-Manual gives in these modes, if it gives one.
    pub(crate) range: Option<(i64, i64)>,
    /// The Drive Mode profile the entry is for, if only one.
    pub(crate) profile: Option<String>,
}

/// One role a model has.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GenRole {
    /// Which role.
    pub(crate) def: &'static RoleDef,
    /// The row that serves it.
    pub(crate) row: Row,
    /// Its scales, disjoint in modes; empty: raw only.
    pub(crate) scales: Vec<GenScale>,
    /// Units of its row the role cannot type.
    pub(crate) untyped: Vec<Untyped>,
    /// The row bounding its magnitude.
    pub(crate) bounded_by: Option<Row>,
    /// The first firmware with the register; 0 for all.
    pub(crate) from_fw: u8,
    /// The row's own raw range (gains, compliance slopes, the torque
    /// switch), valid in every mode; a scale's range wins in its modes.
    pub(crate) row_range: Option<(i64, i64)>,
}

/// The torque control switch of a table, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TorqueSwitch {
    /// The switch row.
    pub(crate) row: Row,
    /// The value that turns torque control on.
    pub(crate) on: u8,
    /// The values the switch takes.
    pub(crate) values: (u8, u8),
}

fn find<'a>(table: &'a ControlTable, name: &str) -> Option<&'a Row> {
    table.control_table.iter().find(|r| r.data_name == name)
}

/// Check 18: `unit_source` is `Verified`, and every FR-9 field sits under
/// it.
pub(crate) fn check_unit_source(file: &str, table: &ControlTable, errors: &mut Vec<Diagnostic>) {
    let mut fail = |message: String| errors.push(Diagnostic::new(file, message));
    match &table.unit_source {
        Some(Source::Unverified) => {
            fail("unit_source must be Verified(url, verified), found Unverified".to_string());
        }
        Some(Source::Verified { .. }) => {}
        None => {
            let per_mode = table.operating_modes.as_ref().is_some_and(|m| {
                m.current.is_some()
                    || m.velocity.is_some()
                    || m.pwm.is_some()
                    || m.current_position.is_some()
            });
            if per_mode {
                fail("per-mode operating_modes lists without unit_source".to_string());
            }
            if table.torque_control_mode.is_some() {
                fail("torque_control_mode without unit_source".to_string());
            }
            if let Some(row) = table.control_table.iter().find(|r| r.range.is_some()) {
                fail(format!(
                    "row {:?} has a range without unit_source",
                    row.data_name
                ));
            }
        }
    }
}

/// Whether `range` fits a `size`-byte register under `sign`.
fn fits(range: RawRange, sign: SignKind, size: u8) -> bool {
    sign.raw_span(size)
        .is_some_and(|(lo, hi)| range.min <= range.max && range.min >= lo && range.max <= hi)
}

/// Check 19 for every `unit_info` entry of a table with `unit_source`, and
/// for every row range.
pub(crate) fn check_unit_info(file: &str, table: &ControlTable, errors: &mut Vec<Diagnostic>) {
    for row in &table.control_table {
        let Some(range) = row.range else {
            continue;
        };
        if !SPAN_SIZES.contains(&row.size) {
            errors.push(Diagnostic::new(
                file,
                format!(
                    "row {:?}: size {} is outside 1..=7 bytes, so range {}..={} cannot be checked",
                    row.data_name, row.size, range.min, range.max
                ),
            ));
        } else if !fits(range, SignKind::Unsigned, row.size)
            && !fits(range, SignKind::TwosComplement, row.size)
        {
            errors.push(Diagnostic::new(
                file,
                format!(
                    "row {:?}: range {}..={} does not fit {} byte(s)",
                    row.data_name, range.min, range.max, row.size
                ),
            ));
        }
    }
    if table.unit_source.is_none() {
        return;
    }
    let angle_limit_table = table.operating_modes.is_none();
    for entry in &table.unit_info {
        let mut fail = |message: String| {
            errors.push(Diagnostic::new(
                file,
                format!("unit_info {:?}: {message}", entry.data_name),
            ));
        };
        let row = find(table, &entry.data_name);
        if row.is_none() {
            fail("names no row".to_string());
        }
        if !(entry.value.is_finite() && entry.value > 0.0) {
            fail(format!("value {} must be finite and above 0", entry.value));
        }
        if !UNITS.contains(&entry.unit.as_str()) {
            fail(format!(
                "unknown unit {:?} (expected {})",
                entry.unit,
                UNITS.join(", ")
            ));
        }
        let sign = SignKind::parse(&entry.sign_type);
        match (sign, row) {
            (None, _) => fail(format!(
                "unknown sign_type {:?} (expected unsigned, twos_complement or direction_bit_N)",
                entry.sign_type
            )),
            (Some(_), Some(row)) if !SPAN_SIZES.contains(&row.size) => fail(format!(
                "row size {} is outside 1..=7 bytes, so sign_type {} cannot be checked",
                row.size, entry.sign_type
            )),
            (Some(sign), Some(row)) if sign.raw_span(row.size).is_none() => fail(format!(
                "sign_type {} does not fit a {}-byte register",
                entry.sign_type, row.size
            )),
            (Some(sign), Some(row)) => {
                if let Some(range) = entry.range
                    && !fits(range, sign, row.size)
                {
                    fail(format!(
                        "range {}..={} does not fit a {}-byte {} register",
                        range.min, range.max, row.size, entry.sign_type
                    ));
                }
            }
            (Some(_), None) => {}
        }
        if let Some(modes) = &entry.modes {
            if !angle_limit_table {
                fail("modes is only allowed in an angle-limit table".to_string());
            }
            for mode in modes {
                if !P1_MODES.iter().any(|(name, _)| name == mode) {
                    fail(format!(
                        "unknown mode {mode:?} (expected joint, wheel, multi_turn or torque_control)"
                    ));
                }
            }
        }
        if let Some(profile) = &entry.profile
            && profile != "velocity_based"
            && profile != "time_based"
        {
            fail(format!(
                "unknown profile {profile:?} (expected velocity_based or time_based)"
            ));
        }
        if let Some(bound) = &entry.bounded_by
            && find(table, bound).is_none()
        {
            fail(format!("bounded_by {bound:?} names no row"));
        }
    }
}

/// Check 20: the per-mode lists come all four or none, `current`,
/// `velocity` and `pwm` split `no_position` exactly, and `current_position`
/// sits inside `unbounded`.
pub(crate) fn check_per_mode(file: &str, table: &ControlTable, errors: &mut Vec<Diagnostic>) {
    let Some(values) = &table.operating_modes else {
        return;
    };
    let mut fail = |message: String| {
        errors.push(Diagnostic::new(file, format!("operating_modes: {message}")));
    };
    let lists = [
        &values.current,
        &values.velocity,
        &values.pwm,
        &values.current_position,
    ];
    let (Some(current), Some(velocity), Some(pwm), Some(current_position)) =
        (lists[0], lists[1], lists[2], lists[3])
    else {
        if lists.iter().any(|l| l.is_some()) {
            fail(
                "current, velocity, pwm and current_position come all together or not at all"
                    .to_string(),
            );
        }
        return;
    };
    let mut split: Vec<u8> = current.iter().chain(velocity).chain(pwm).copied().collect();
    split.sort_unstable();
    let mut no_position = values.no_position.clone();
    no_position.sort_unstable();
    if split != no_position {
        fail(format!(
            "current, velocity and pwm {split:?} must split no_position {no_position:?} exactly"
        ));
    }
    for value in current_position {
        if !values.unbounded.contains(value) {
            fail(format!(
                "current_position value {value} is not in unbounded"
            ));
        }
    }
}

/// Check 21: the torque control switch of an angle-limit table. Returns it
/// when it passes.
pub(crate) fn torque_switch(
    file: &str,
    table: &ControlTable,
    angle_limits: bool,
    errors: &mut Vec<Diagnostic>,
) -> Option<TorqueSwitch> {
    let mode = table.torque_control_mode.as_ref()?;
    let before = errors.len();
    let mut fail = |message: String| {
        errors.push(Diagnostic::new(
            file,
            format!("torque_control_mode: {message}"),
        ));
    };
    if !angle_limits {
        fail("only an angle-limit table (no mode register) may have one".to_string());
    }
    let register = find(table, &mode.register);
    match register {
        None => fail(format!("register {:?} names no row", mode.register)),
        Some(row) if row.size != 1 || row.access != Access::RW || row.area != Area::Ram => {
            fail(format!(
                "register {:?} must be 1 byte RW/RAM",
                mode.register
            ));
        }
        Some(row) => match row.range {
            Some(range) if (range.min..=range.max).contains(&i64::from(mode.on)) => {}
            Some(range) => fail(format!(
                "on {} is outside {:?}'s range {}..={}",
                mode.on, mode.register, range.min, range.max
            )),
            None => fail(format!(
                "register {:?} needs a range, so values other than on are known",
                mode.register
            )),
        },
    }
    match find(table, &mode.goal) {
        None => fail(format!("goal {:?} names no row", mode.goal)),
        Some(row) if row.access != Access::RW || row.area != Area::Ram => {
            fail(format!("goal {:?} must be RW/RAM", mode.goal));
        }
        Some(_) if mode.goal == mode.register => {
            fail("goal and register are the same row".to_string());
        }
        Some(_) => {}
    }
    if errors.len() > before {
        return None;
    }
    let row = register?.clone();
    let range = row.range?;
    Some(TorqueSwitch {
        on: mode.on,
        values: (u8::try_from(range.min).ok()?, u8::try_from(range.max).ok()?),
        row,
    })
}

/// The modes of `entry`.
fn entry_modes(entry: &UnitEntry) -> Modes {
    entry.modes.as_ref().map_or(ALL, |modes| {
        modes
            .iter()
            .filter_map(|m| P1_MODES.iter().find(|(name, _)| name == m))
            .fold(0, |acc, (_, bits)| acc | bits)
    })
}

/// `LimitMode` names of `modes`.
pub(crate) fn mode_names(modes: Modes) -> Vec<&'static str> {
    LIMIT_MODES
        .iter()
        .enumerate()
        .filter(|(i, _)| modes & (1 << i) != 0)
        .map(|(_, name)| *name)
        .collect()
}

/// Check 22 and the roles of one table: one candidate row per role, every
/// unit of a typed role its quantity (or a known untyped unit), and at most
/// one scale per mode. Call only after checks 18, 19 and 21 passed.
pub(crate) fn roles(
    file: &str,
    table: &ControlTable,
    switch: Option<&TorqueSwitch>,
    errors: &mut Vec<Diagnostic>,
) -> Vec<GenRole> {
    let goal = table
        .torque_control_mode
        .as_ref()
        .and_then(|m| find(table, &m.goal));
    let mut out = Vec::new();
    for def in &ROLES {
        let mut found: Vec<(&Row, Modes)> = def
            .rows
            .iter()
            .filter_map(|&(name, modes)| find(table, name).map(|row| (row, modes)))
            .collect();
        match def.switch {
            Some(Switch::Register) => found.extend(switch.map(|s| (&s.row, ALL))),
            Some(Switch::Goal) if switch.is_some() => found.extend(goal.map(|row| (row, ALL))),
            _ => {}
        }
        let (row, allowed) = match found.as_slice() {
            [] => continue,
            [one] => *one,
            [..] => {
                let names: Vec<String> = found
                    .iter()
                    .map(|(r, _)| format!("{:?}", r.data_name))
                    .collect();
                errors.push(Diagnostic::new(
                    file,
                    format!("role {} has two rows: {}", def.field, names.join(" and ")),
                ));
                continue;
            }
        };
        out.push(role_of(file, table, def, row, allowed, errors));
    }
    out
}

/// One role served by `row`, with its scales in the modes `allowed`.
fn role_of(
    file: &str,
    table: &ControlTable,
    def: &'static RoleDef,
    row: &Row,
    allowed: Modes,
    errors: &mut Vec<Diagnostic>,
) -> GenRole {
    let mut role = GenRole {
        def,
        row: row.clone(),
        scales: Vec::new(),
        untyped: Vec::new(),
        bounded_by: None,
        from_fw: 0,
        row_range: None,
    };
    if table.unit_source.is_none() {
        return role;
    }
    role.row_range = row.range.map(|r| (r.min, r.max));
    if def.quantity == Quantity::Raw {
        return role;
    }
    let mut first_fw: Option<u8> = None;
    for entry in table
        .unit_info
        .iter()
        .filter(|e| e.data_name == row.data_name)
    {
        let modes = entry_modes(entry) & allowed;
        if modes == 0 {
            continue;
        }
        let Some(factor) = def.quantity.factor(&entry.unit) else {
            if def.quantity.not_typed(&entry.unit) {
                role.untyped.push(Untyped {
                    modes,
                    value: entry.value,
                    unit: entry.unit.clone(),
                    range: entry.range.map(|r| (r.min, r.max)),
                    profile: entry.profile.clone(),
                });
            } else {
                errors.push(Diagnostic::new(
                    file,
                    format!(
                        "unit_info {:?}: unit {:?} is not the {} role's quantity ({:?})",
                        entry.data_name, entry.unit, def.field, def.quantity
                    ),
                ));
            }
            continue;
        };
        if let Some(clash) = role.scales.iter().find(|s| s.modes & modes != 0) {
            errors.push(Diagnostic::new(
                file,
                format!(
                    "unit_info {:?}: two {} scales in mode {}",
                    entry.data_name,
                    def.field,
                    mode_names(clash.modes & modes).join(", ")
                ),
            ));
            continue;
        }
        let Some(sign) = SignKind::parse(&entry.sign_type) else {
            continue;
        };
        let si_per_raw = match (def.quantity, sign) {
            (Quantity::Load, SignKind::DirectionBit(n)) => {
                let exact = load_step(n);
                let table_step = entry.value * factor;
                if (table_step / exact - 1.0).abs() > LOAD_STEP_TOLERANCE {
                    errors.push(Diagnostic::new(
                        file,
                        format!(
                            "unit_info {:?}: step {} {} is not about 100 % / {} (the magnitude below direction bit {n})",
                            entry.data_name,
                            entry.value,
                            entry.unit,
                            full_magnitude(n)
                        ),
                    ));
                    continue;
                }
                exact
            }
            _ => entry.value * factor,
        };
        role.scales.push(GenScale {
            modes,
            si_per_raw,
            value: entry.value,
            unit: entry.unit.clone(),
            sign,
            size: row.size,
            zero_is_unlimited: entry.zero_is_unlimited,
            range: entry.range.map(|r| (r.min, r.max)),
            profile: entry.profile.clone(),
        });
        if role.bounded_by.is_none() {
            role.bounded_by = entry
                .bounded_by
                .as_deref()
                .and_then(|name| find(table, name))
                .cloned();
        }
        let fw = entry.from_fw.unwrap_or(0);
        first_fw = Some(first_fw.map_or(fw, |f| f.min(fw)));
    }
    role.from_fw = first_fw.unwrap_or(0);
    role
}

/// The full magnitude below direction bit `n`, 2^n − 1. Check 19 keeps `n`
/// below the 56 bits of the widest [`SPAN_SIZES`] register.
pub(crate) fn full_magnitude(n: u8) -> u64 {
    (1u64 << n) - 1
}

/// The ratio step of a load whose magnitude sits below direction bit `n`:
/// the full magnitude, 2^n − 1, is 100 %.
#[allow(
    clippy::cast_precision_loss,
    reason = "exact up to 2^53; the e-Manual's direction bits are 10"
)]
pub(crate) fn load_step(n: u8) -> f64 {
    1.0 / full_magnitude(n) as f64
}

/// The roles of `roles` by field.
#[cfg(test)]
pub(crate) fn by_field(roles: &[GenRole]) -> std::collections::BTreeMap<&'static str, &GenRole> {
    roles.iter().map(|r| (r.def.field, r)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fixtures::{Map, Table, outcome};
    use crate::codegen::resolve::{GenModel, ModeRole, Outcome};

    fn single(table: Table) -> Outcome {
        let map = Map::single(1020, "x-series", "T", "t", &table);
        outcome(&map, &[("t", table)])
    }

    fn model(table: Table) -> GenModel {
        let mut out = single(table);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        out.resolved.models.remove(0)
    }

    #[track_caller]
    fn assert_fails(table: Table, keywords: &[&str]) {
        let out = single(table);
        assert!(
            out.errors
                .iter()
                .any(|d| d.file == "t" && keywords.iter().all(|k| d.message.contains(k))),
            "no error with {keywords:?} in {:?}",
            out.errors
        );
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-12 * b.abs().max(1.0)
    }

    #[test]
    fn signs() {
        assert_eq!(SignKind::parse("unsigned"), Some(SignKind::Unsigned));
        assert_eq!(
            SignKind::parse("twos_complement"),
            Some(SignKind::TwosComplement)
        );
        assert_eq!(
            SignKind::parse("direction_bit_10"),
            Some(SignKind::DirectionBit(10))
        );
        assert_eq!(SignKind::parse("signed"), None);
        assert_eq!(SignKind::parse("direction_bit_x"), None);
        assert_eq!(SignKind::Unsigned.raw_span(1), Some((0, 255)));
        assert_eq!(
            SignKind::TwosComplement.raw_span(2),
            Some((-32_768, 32_767))
        );
        assert_eq!(SignKind::DirectionBit(10).raw_span(2), Some((0, 65_535)));
        assert_eq!(SignKind::DirectionBit(10).raw_span(1), None);
        assert_eq!(SignKind::DirectionBit(0).raw_span(2), None);
    }

    #[test]
    fn unit_factors() {
        assert!(close(Quantity::Velocity.factor("rpm").unwrap(), PI / 30.0));
        assert!(close(Quantity::Current.factor("mA").unwrap(), 0.001));
        assert!(close(Quantity::Tick.factor("ms").unwrap(), 0.001));
        assert!(close(Quantity::Load.factor("%").unwrap(), 0.01));
        assert!(close(Quantity::Angle.factor("deg").unwrap(), PI / 180.0));
        assert_eq!(Quantity::Voltage.factor("V"), Some(1.0));
        assert_eq!(Quantity::Temperature.factor("°C"), Some(1.0));
        assert_eq!(Quantity::Velocity.factor("%"), None);
        assert_eq!(Quantity::Raw.factor("ms"), None);
        assert!(Quantity::Velocity.not_typed("%"));
        assert!(Quantity::Velocity.not_typed("ms"));
        assert!(!Quantity::Current.not_typed("%"));
    }

    #[test]
    fn role_list_matches_core() {
        assert_eq!(ROLES.len(), 25);
        let fields: Vec<&str> = ROLES.iter().map(|r| r.field).collect();
        let mut sorted = fields.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), fields.len());
        for role in &ROLES {
            assert!(role.has.starts_with("Has"));
            assert_eq!(&role.has[3..], role.variant);
        }
        assert_eq!(mode_names(ALL), LIMIT_MODES);
        assert_eq!(mode_names(WHEEL), ["Wheel"]);
        assert_eq!(mode_names(0), Vec::<&str>::new());
    }

    #[test]
    fn ax_roles_and_scales() {
        let m = model(Table::ax_fr9());
        let roles = by_field(&m.typed);
        let fields: Vec<&str> = m.typed.iter().map(|r| r.def.field).collect();
        assert_eq!(
            fields,
            [
                "present_temperature",
                "present_voltage",
                "present_velocity",
                "goal_velocity",
                "speed_limit",
                "present_load",
                "moving",
                "led",
                "cw_compliance_margin",
                "ccw_compliance_margin",
                "cw_compliance_slope",
                "ccw_compliance_slope",
            ]
        );
        let speed = roles["speed_limit"];
        assert_eq!(speed.row.data_name, "Moving Speed");
        let [joint] = speed.scales.as_slice() else {
            panic!("{:?}", speed.scales);
        };
        assert_eq!(joint.modes, 1);
        assert!(close(joint.si_per_raw, 0.111 * PI / 30.0));
        assert!(joint.zero_is_unlimited);
        assert_eq!(joint.range, Some((0, 1023)));
        assert_eq!(joint.sign, SignKind::Unsigned);
        assert!(speed.untyped.is_empty());

        let goal = roles["goal_velocity"];
        assert_eq!(goal.row.data_name, "Moving Speed");
        assert!(goal.scales.is_empty());
        assert_eq!(goal.untyped.len(), 1);
        assert_eq!(
            (goal.untyped[0].modes, goal.untyped[0].unit.as_str()),
            (WHEEL, "%")
        );
        assert_eq!(goal.untyped[0].range, Some((0, 2047)));

        let present = roles["present_velocity"];
        assert_eq!(present.scales.len(), 1);
        assert_eq!(present.scales[0].sign, SignKind::DirectionBit(10));
        assert_eq!(present.untyped.len(), 1);
        assert_eq!(present.untyped[0].range, None);

        let load = roles["present_load"];
        assert!(close(load.scales[0].si_per_raw, 1.0 / 1023.0));
        assert_eq!(load.scales[0].modes, ALL);
        assert_eq!(load.row_range, None);
        assert_eq!(roles["cw_compliance_slope"].row_range, Some((0, 254)));
        let margin = roles["cw_compliance_margin"];
        assert!(close(margin.scales[0].si_per_raw, 0.29 * PI / 180.0));
        assert!(roles["ccw_compliance_margin"].scales.is_empty());
        assert!(roles["moving"].scales.is_empty());
        assert!(close(roles["present_voltage"].scales[0].si_per_raw, 0.1));
        assert_eq!(m.roles.mode, ModeRole::AngleLimits);
    }

    #[test]
    fn mx_p1_switch_and_goal_torque() {
        let m = model(Table::mx_p1());
        let roles = by_field(&m.typed);
        assert_eq!(roles["torque_ctrl_mode_enable"].row.address, 70);
        let goal = roles["goal_current"];
        assert_eq!(goal.row.data_name, "Goal Torque");
        assert!(close(goal.scales[0].si_per_raw, 0.0045));
        assert_eq!(goal.scales[0].sign, SignKind::DirectionBit(10));
        let tick = roles["realtime_tick"];
        assert_eq!(tick.from_fw, 40);
        assert_eq!(tick.scales[0].range, Some((0, 32_767)));
        assert!(close(tick.scales[0].si_per_raw, 0.001));
        assert_eq!(roles["speed_limit"].scales[0].modes, 0b11);
        assert_eq!(roles["goal_velocity"].scales[0].modes, WHEEL);
        assert_eq!(roles["p_gain"].row.data_name, "P Gain");
        assert!(matches!(
            &m.roles.mode,
            ModeRole::TorqueControl { row, on: 1, values: (0, 1) } if row.address == 70
        ));
    }

    #[test]
    fn xm_profile_bounds_and_lists() {
        let m = model(Table::xm_fr9());
        let roles = by_field(&m.typed);
        let limit = roles["speed_limit"];
        assert_eq!(limit.row.data_name, "Profile Velocity");
        assert_eq!(limit.scales.len(), 1);
        assert_eq!(limit.scales[0].profile.as_deref(), Some("velocity_based"));
        assert_eq!(limit.untyped[0].profile.as_deref(), Some("time_based"));
        assert_eq!(limit.from_fw, 0);
        assert_eq!(
            roles["goal_velocity"]
                .bounded_by
                .as_ref()
                .map(|r| r.address),
            Some(44)
        );
        assert_eq!(
            roles["goal_current"].bounded_by.as_ref().map(|r| r.address),
            Some(38)
        );
        assert_eq!(
            roles["goal_current"].scales[0].sign,
            SignKind::TwosComplement
        );
        assert_eq!(
            roles["present_voltage"].row.data_name,
            "Present Input Voltage"
        );
        assert_eq!(roles["hardware_error_status"].row.address, 70);
        let ModeRole::Register { values, .. } = &m.roles.mode else {
            panic!("expected a mode register");
        };
        assert_eq!(values.current_position, Some(vec![5]));
    }

    #[test]
    fn units_count_only_under_unit_source() {
        let mut table = Table::ax_fr9().without_range("CW Compliance Slope");
        table = table.omit_field("unit_source");
        let m = model(table);
        assert!(
            m.typed
                .iter()
                .all(|r| r.scales.is_empty() && r.untyped.is_empty() && r.row_range.is_none())
        );
        assert_eq!(m.typed.len(), 12);
    }

    #[test]
    fn check18_fields_without_unit_source() {
        assert_fails(
            Table::xm().operating_modes(
                "(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: [0], velocity: [1], pwm: [16], current_position: [5])",
            ),
            &["per-mode operating_modes lists without unit_source"],
        );
        assert_fails(
            Table::mx_p1().omit_field("unit_source"),
            &["torque_control_mode without unit_source"],
        );
        assert_fails(
            Table::ax().row_range("Goal Position", 0, 1023),
            &["row \"Goal Position\" has a range without unit_source"],
        );
        assert_fails(
            Table::xm_fr9().unit_source("Unverified"),
            &["unit_source must be Verified"],
        );
    }

    fn with_unit(fields: &str) -> Table {
        Table::xm_fr9().unit(fields)
    }

    #[test]
    fn check19_bad_entries() {
        for (fields, keywords) in [
            (
                r#"data_name: "LED", value: 0.0, unit: "V", sign_type: "unsigned""#,
                &["\"LED\"", "value 0 must be finite and above 0"][..],
            ),
            (
                r#"data_name: "LED", value: 1.0, unit: "lux", sign_type: "unsigned""#,
                &["unknown unit \"lux\""],
            ),
            (
                r#"data_name: "LED", value: 1.0, unit: "V", sign_type: "signed""#,
                &["unknown sign_type \"signed\""],
            ),
            (
                r#"data_name: "LED", value: 1.0, unit: "V", sign_type: "direction_bit_10""#,
                &["sign_type direction_bit_10 does not fit a 1-byte register"],
            ),
            (
                r#"data_name: "LED", value: 1.0, unit: "V", sign_type: "unsigned", range: (min: 0, max: 256)"#,
                &["range 0..=256 does not fit a 1-byte unsigned register"],
            ),
            (
                r#"data_name: "LED", value: 1.0, unit: "V", sign_type: "unsigned", modes: ["joint"]"#,
                &["modes is only allowed in an angle-limit table"],
            ),
            (
                r#"data_name: "LED", value: 1.0, unit: "V", sign_type: "unsigned", profile: "fast""#,
                &["unknown profile \"fast\""],
            ),
            (
                r#"data_name: "LED", value: 1.0, unit: "V", sign_type: "unsigned", bounded_by: "LED Limit""#,
                &["bounded_by \"LED Limit\" names no row"],
            ),
        ] {
            assert_fails(with_unit(fields), keywords);
        }
        assert_fails(
            Table::ax_fr9().unit(
                r#"data_name: "Moving Speed", value: 1.0, unit: "rpm", sign_type: "unsigned", modes: ["spin"]"#,
            ),
            &["unknown mode \"spin\""],
        );
        assert_fails(
            Table::xm_fr9().row_range("LED", 0, 300),
            &["row \"LED\": range 0..=300 does not fit 1 byte(s)"],
        );
        let negative = model(Table::xm_fr9().row_range("Goal Current", -1193, 1193));
        assert_eq!(negative.file, "t");
    }

    #[test]
    fn check19_sizes_without_a_raw_span() {
        for size in [0, 8] {
            assert_fails(
                Table::xm_fr9()
                    .without("LED")
                    .with_row(65, size, "LED", "RW", "RAM")
                    .row_range("LED", 0, 1),
                &[&format!(
                    "row \"LED\": size {size} is outside 1..=7 bytes, so range 0..=1 cannot be checked"
                )],
            );
            assert_fails(
                Table::xm_fr9().without("Present Temperature").with_row(
                    146,
                    size,
                    "Present Temperature",
                    "R",
                    "RAM",
                ),
                &[&format!(
                    "unit_info \"Present Temperature\": row size {size} is outside 1..=7 bytes, so sign_type unsigned cannot be checked"
                )],
            );
        }
        assert_fails(
            Table::ax_fr9()
                .without("Present Load")
                .with_row(40, 5, "Present Load", "R", "RAM")
                .without_unit("Present Load")
                .unit(
                    r#"data_name: "Present Load", value: 0.1, unit: "%", sign_type: "direction_bit_35""#,
                ),
            &["step 0.1 % is not about 100 % / 34359738367"],
        );
    }

    #[test]
    fn check20_per_mode_lists() {
        let lists = |text: &str| Table::xm_fr9().operating_modes(text);
        assert_fails(
            lists(
                "(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: [0], velocity: [1], pwm: [], current_position: [5])",
            ),
            &["must split no_position [0, 1, 16] exactly"],
        );
        assert_fails(
            lists(
                "(bounded: [3], unbounded: [4], no_position: [0, 1, 16], current: [0], velocity: [1], pwm: [16], current_position: [5])",
            ),
            &["current_position value 5 is not in unbounded"],
        );
        assert_fails(
            lists("(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], pwm: [16])"),
            &["all together"],
        );
    }

    fn switch(text: &str) -> Table {
        Table::mx_p1().torque_control_mode(text)
    }

    #[test]
    fn check21_torque_switch() {
        assert_fails(
            switch(r#"(register: "Torque Ctrl Mode Enable", on: 2, goal: "Goal Torque")"#),
            &["on 2 is outside \"Torque Ctrl Mode Enable\"'s range 0..=1"],
        );
        assert_fails(
            switch(r#"(register: "Torque Ctrl Mode Enable", on: 1, goal: "Goal Thrust")"#),
            &["goal \"Goal Thrust\" names no row"],
        );
        assert_fails(
            switch(r#"(register: "Torque Ctrl Mode Enable", on: 1, goal: "Present Position")"#),
            &["goal \"Present Position\" must be RW/RAM"],
        );
        assert_fails(
            switch(
                r#"(register: "Torque Ctrl Mode Enable", on: 1, goal: "Torque Ctrl Mode Enable")"#,
            ),
            &["goal and register are the same row"],
        );
        assert_fails(
            switch(r#"(register: "Moving Speed", on: 1, goal: "Goal Torque")"#),
            &["register \"Moving Speed\" must be 1 byte RW/RAM"],
        );
        assert_fails(
            Table::mx_p1().without("Torque Ctrl Mode Enable").with_row(
                70,
                1,
                "Torque Ctrl Mode Enable",
                "RW",
                "RAM",
            ),
            &["needs a range"],
        );
        assert_fails(
            Table::xm_fr9()
                .with_row(80, 1, "Torque Ctrl Mode Enable", "RW", "RAM")
                .with_row(81, 2, "Goal Torque", "RW", "RAM")
                .row_range("Torque Ctrl Mode Enable", 0, 1)
                .torque_control_mode(
                    r#"(register: "Torque Ctrl Mode Enable", on: 1, goal: "Goal Torque")"#,
                ),
            &["only an angle-limit table"],
        );
    }

    #[test]
    fn check22_role_conflicts() {
        assert_fails(
            Table::ax_fr9().with_row(39, 2, "Present Velocity", "R", "RAM"),
            &["role present_velocity has two rows: \"Present Velocity\" and \"Present Speed\""],
        );
        assert_fails(
            Table::mx_p1().with_row(72, 2, "Goal Current", "RW", "RAM"),
            &["role goal_current has two rows"],
        );
        assert_fails(
            Table::ax_fr9().unit(
                r#"data_name: "Moving", value: 1.0, unit: "V", sign_type: "unsigned""#,
            ).unit(
                r#"data_name: "Present Temperature", value: 1.0, unit: "V", sign_type: "unsigned", modes: ["wheel"]"#,
            ),
            &["unit \"V\" is not the present_temperature role's quantity"],
        );
        assert_fails(
            Table::ax_fr9().unit(
                r#"data_name: "Present Load", value: 0.2, unit: "%", sign_type: "direction_bit_10", modes: ["wheel"]"#,
            ),
            &["two present_load scales in mode Wheel"],
        );
        assert_fails(
            Table::ax_fr9()
                .without_unit("Present Load")
                .unit(
                    r#"data_name: "Present Load", value: 0.2, unit: "%", sign_type: "direction_bit_10""#,
                ),
            &["step 0.2 % is not about 100 % / 1023"],
        );
    }

    #[test]
    fn load_step_is_full_magnitude() {
        assert!(close(load_step(10), 1.0 / 1023.0));
        assert!(close(load_step(1), 1.0));
        let m =
            model(Table::ax_fr9().without_unit("Present Load").unit(
                r#"data_name: "Present Load", value: 0.1, unit: "%", sign_type: "unsigned""#,
            ));
        assert!(close(
            by_field(&m.typed)["present_load"].scales[0].si_per_raw,
            0.001
        ));
    }

    #[test]
    fn row_ranges_only_under_unit_source() {
        let m = model(Table::mx_p1());
        let roles = by_field(&m.typed);
        assert_eq!(roles["p_gain"].row_range, Some((0, 254)));
        assert_eq!(roles["torque_ctrl_mode_enable"].row_range, Some((0, 1)));
        assert_eq!(roles["realtime_tick"].row_range, None);
    }

    #[test]
    fn raw_roles_ignore_units() {
        let m = model(
            Table::ax_fr9()
                .unit(r#"data_name: "Moving", value: 1.0, unit: "V", sign_type: "unsigned""#),
        );
        assert!(by_field(&m.typed)["moving"].scales.is_empty());
    }

    #[test]
    fn check14_fr9_roles_on_p1() {
        assert_fails(
            Table::xm_fr9().without("Present Temperature").with_row(
                300,
                1,
                "Present Temperature",
                "R",
                "RAM",
            ),
            &["listed under p1 but Present Temperature is at address 300 (> 255)"],
        );
        let out = single(
            Table::xm_fr9()
                .protocols("[P2]")
                .without("Present Temperature")
                .with_row(300, 1, "Present Temperature", "R", "RAM"),
        );
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }
}
