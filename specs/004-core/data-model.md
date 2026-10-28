# Data Model: Core

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md) | **API**:
[contracts/core-api.md](contracts/core-api.md)

Visibility: **public** = re-exported at the crate root. **crate** = `pub(crate)`, or `pub` inside a
private module (never re-exported). PR column: where the item first lands.

## Id (exists, unchanged)

| Field | Type | Rule |
|---|---|---|
| `0` | `u8` | `0..=252`. 253 and above give `ValidationError::InvalidId`. |

## ModelNumber (public, core-1, `src/actuator.rs`)

| Field | Type | Rule |
|---|---|---|
| `0` | `pub u16` | Any value. Whether this build supports it is decided at Setup. |

Derives `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash`. `From<u16>`, `Display`
(the plain number). `Actuator::model_number()` and errors keep `u16`.

## Position (public, core-1, `src/units/position.rs`)

| Field | Type | Rule |
|---|---|---|
| `0` | `f32` (private) | Radians. Zero is the model's zero-radian tick. Any `f32` is storable; NaN and infinity are refused when converted to ticks. |

Operations: `ZERO`, `from_radians`, `radians`, `+`, `-`, `+=`, `-=`, `PartialEq`, `PartialOrd`,
`Default` (= `ZERO`).

## PositionScale (public type, crate constructor, core-1, `src/units/position.rs`)

| Field | Type | Source |
|---|---|---|
| `zero_raw` | `i32` | `type_info.value_of_zero_radian_position` |
| `min_raw`, `max_raw` | `i32` | `value_of_min/max_radian_position` |
| `min_rad`, `max_rad` | `f32` | `min_radian`, `max_radian` |
| `radians_per_tick` | `f32` (private, derived in `const fn new`) | `(max_rad - min_rad) / (max_raw - min_raw)` |
| `ticks_per_radian` | `f32` (private, derived) | its inverse |

| Conversion | Rule | Errors |
|---|---|---|
| `to_position(raw)` | `(raw − zero_raw)` in `i64`, as `f32`, × `radians_per_tick` | none |
| `to_raw(p)` | `p × ticks_per_radian`, round half away from zero, + `zero_raw` in `i64` | `NotFinite` (NaN, ±∞); `DoesNotFit { raw, size: 0 }` outside `i32` |

No range clamp: ticks outside `min_raw..=max_raw` convert linearly (multi-turn). Codegen checks
`max_raw > min_raw`, `max_rad > min_rad`, `zero_raw` in range (module-interfaces §4.3).

Test-only helper (core-2, T044): `#[cfg(test)] pub(crate) const fn raw_range(&self) ->
core::ops::RangeInclusive<i32>`, giving `min_raw..=max_raw`. The per-model round-trip test
(SC-001) loops over it. It is not part of the contract and does not exist outside `cargo test`.

## Limits (crate, core-1, `src/units/position.rs`)

| Field | Type | Rule |
|---|---|---|
| `min` | `i32` | Read from the minimum limit register (Min Position Limit, or CW Angle Limit). |
| `max` | `i32` | Read from the maximum limit register (Max Position Limit, or CCW Angle Limit). |

Validation: `min > max` gives `InvertedLimits { id, min, max }`. `min == max` is accepted.
No constructor from table defaults (FR-007).

## Cache (crate, core-1, `src/actuator.rs`)

| Field | Type | Rule |
|---|---|---|
| `id` | `Id` | Set at construction. Never changes. |
| `mode` | `LimitMode` | `Bounded` at construction. Setup records the actuator's mode (2026-10-28). |
| `limits` | `Option<Limits>` | `None` until Setup records the EEPROM values. |

No torque state, no fault state, no fault report (SC-008). 16 bytes: `mode` sits in the padding
after `id` (tested with `size_of`).

| Operation | Result |
|---|---|
| `new(id)` | mode `Bounded`, limits `None` |
| `record_limits(min, max)` | `Ok(Limits)` and stored, or `InvertedLimits` and unchanged |
| `record_mode(mode)` | stored; limits unchanged |
| `check_goal(raw)` | by mode, below |

| `mode` | `check_goal(raw)` |
|---|---|
| `Bounded` | `Ok(raw)` inside `min..=max`; `LimitsUnknown { id }`; `OutOfLimits { id, raw, min, max }` |
| `Unbounded` | `Ok(raw)` for any `raw`, limits known or not. `Register::encode` still refuses a value that does not fit. |
| `NoPosition` | `NotPositionMode { id }` for any `raw`, limits known or not |

## LimitMode (crate, core-1, `src/actuator.rs`)

`#[repr(u8)]`, `Debug, Clone, Copy, PartialEq, Eq, Default`. Exactly 1 byte. Decided by the
architect on 2026-10-28: limits follow the operating mode (module-interfaces §2.4b).

| Variant | Value | Meaning |
|---|---|---|
| `Bounded` (default) | 0 | Position (joint) mode: the goal must lie in the EEPROM limits read at Setup. |
| `Unbounded` | 1 | Extended or current-based position, MX multi-turn: the firmware does not use the EEPROM limits; only the register size check applies. |
| `NoPosition` | 2 | Velocity, current, PWM, AX / XL-320 wheel mode: the firmware ignores Goal Position; a goal is refused. |

`LimitMode::from_angle_limits(limits: Limits) -> LimitMode` (`const fn`, for `AngleLimits`):
CW = CCW = 0 gives `NoPosition`; CW = CCW = 4095 gives `Unbounded`; any other pair gives
`Bounded`.

## ModeSource, ModeValues (crate, core-1, `src/actuator.rs`)

Where Setup gets an actuator's `LimitMode`. One value per generated model, in `ModelData::mode`.

| `ModeSource` variant | Models | Setup reads |
|---|---|---|
| `OperatingMode { register: Register, values: ModeValues }` | X-series, MX(2.0), PRO, PRO+, Y, RH-P12-RN | the 1-byte Operating Mode register |
| `ControlMode(Register)` | XL-320 | the 1-byte Control Mode register |
| `AngleLimits` | AX; MX on Protocol 1 | nothing more: the CW/CCW limits it reads anyway |

| Operation | Result |
|---|---|
| `ModeSource::register(self) -> Option<Register>` | the register for `OperatingMode` and `ControlMode`; `None` for `AngleLimits` |
| `ModeSource::limit_mode(self, value: u8) -> Option<LimitMode>` | `OperatingMode`: `values.limit_mode(value)`; `ControlMode`: 1 → `NoPosition`, 2 → `Bounded`, else `None`; `AngleLimits`: `None` |
| `ModeValues::limit_mode(self, value: u8) -> Option<LimitMode>` | the table below; any value not listed gives `None` |

`None` means the value is not in the family's e-Manual list. The bus returns
`SetupError::UnknownOperatingMode { id, value }` for it; core never picks a fallback mode.

| `ModeValues` | 0 | 1 | 3 | 4 | 5 | 16 |
|---|---|---|---|---|---|---|
| `X` (X-series, MX(2.0)) | `NoPosition` | `NoPosition` | `Bounded` | `Unbounded` | `Unbounded` | `NoPosition` |
| `Pro` (PRO (A), PRO+) | `NoPosition` | `NoPosition` | `Bounded` | `Unbounded` | `None` | `NoPosition` |
| `Y` | `NoPosition` | `NoPosition` | `Bounded` | `None` | `None` | `None` |
| `RhP12Rn` | `NoPosition` | `None` | `None` | `None` | `Bounded` | `None` |

Every value not in a column is `None` for every variant. e-Manual citations: research R-17.
`ModeValues` is not in module-interfaces §2.4b: plan deviation D3 (value 5 differs between X
and RH-P12-RN).

## TorqueState (public, core-1, `src/actuator.rs`)

`#[repr(u8)]`: `Off = 0` (default), `On = 1`, `Faulted = 2`. Exactly 1 byte. Faulted counts as off.
Stored by the bus, one per slot, beside the actuator list.

| From \ event | `enable(id)` | `disable()` | `fault()` | `clear()` |
|---|---|---|---|---|
| `Off` | `On` | `Off` | `Faulted` | `Off` |
| `On` | `On` | `Off` | `Faulted` | `On` |
| `Faulted` | Err `ActuatorFaulted { id }` | `Faulted` | `Faulted` | `Off` |

| Query | `Off` | `On` | `Faulted` |
|---|---|---|---|
| `is_on()` | false | true | false |
| `is_faulted()` | false | false | true |
| `check_goal(id)` | Err `ActuatorIsOff { id }` | `Ok(())` | Err `ActuatorIsOff { id }` |

How the bus applies these rules is in module-interfaces §2.5. Core defines them only. Decided
2026-10-28: a hardware fault on a `raw-registers` reply also gives `fault()`; no path is an
exception (research R-18).

## Register, Access, Area (crate, `src/register.rs`)

| Field | Type | Rule |
|---|---|---|
| `addr` | `u16` | Control-table address. |
| `size` | `u8` | 1, 2 or 4 for every register the core code encodes. |
| `access` | `Access` = `R \| RW` | From the table row (core-1 shim: placeholder `R`). |
| `area` | `Area` = `Eeprom \| Ram` | From the table row (core-1 shim: placeholder `Eeprom`). |

| Operation | Rule | Errors |
|---|---|---|
| `encode(raw, out)` | size 1: `0..=255`; size 2: `0..=65535`; size 4: any `i32`; little-endian into `out[..size]` | `DoesNotFit { raw, size }` (also for any other size) |
| `decode(bytes)` | `bytes.len() == size`; sizes 1, 2 unsigned; size 4 two's-complement | `DataLength { expected, got }`; `DoesNotFit` for another size |

Invariant: `decode(encode(x)) == x` for every `x` that `encode` accepts.

Construction: core-1 keeps the 2-arg `new(addr, size)` shim (placeholders, `#[doc(hidden)]`,
unconditional `allow(dead_code)`). core-2 removes it and adds no constructor: registers are built
only by X1's generated `reg()` helper (D1).

## ModelData, LimitRegisters, Protocols (crate, core-1, `src/actuator.rs`)

`ModelData` is one `&'static` value per generated model type:

| Field | Type | Meaning |
|---|---|---|
| `name` | `&'static str` | e.g. `"XM430-W350"` |
| `model_number` | `u16` | e.g. 1020 |
| `protocols` | `Protocols { p1: bool, p2: bool }` | From the table's `protocols` field |
| `torque_enable` | `Register` | Torque Enable |
| `goal_position` | `Register` | Goal Position |
| `present_position` | `Register` | Present Position |
| `limits` | `LimitRegisters` | `MinMax { min, max }` or `CwCcw { cw, ccw }`; `min()` = Min or CW, `max()` = Max or CCW |
| `hardware_error` | `Option<Register>` | Hardware Error Status, or `None` (AX, Y-series tables have none) |
| `mode` | `ModeSource` | Where Setup gets the limit mode (2026-10-28, research R-17) |
| `scale` | `PositionScale` | from `type_info` |

Examples (e-Manual, research R-9, R-17): XM430-W350 goal 116/4, present 132/4, limits MinMax
52/48, mode `OperatingMode { 11/1, X }`; AX-12A goal 30/2, present 36/2, limits CwCcw 6/8, mode
`AngleLimits`.

## Describe and Actuator (traits, `src/actuator.rs`)

| Trait | Visibility | Methods | Implemented by |
|---|---|---|---|
| `sealed::Describe` | crate | `data() -> &'static ModelData`, `cache() -> &Cache`, `cache_mut() -> &mut Cache` | every generated model type, `ActuatorKind` (X1), `ActuatorRef` (bus) |
| `Actuator` (core-1) | public | `id`, `model_number`, `name` (required, unchanged) | generated types, `ActuatorKind` |
| `Actuator: Describe` (core-2) | public, sealed | `id`, `model_number`, `name`, `position_scale`, `position_limits_raw`, `position_limits` (all provided) | same, plus `ActuatorRef` |

No I/O in either trait. `ActuatorKind` dispatches by `match` and returns `&'static ModelData`: no
runtime address lookup.

## Errors (`src/error.rs`)

```text
Error (non_exhaustive, Copy)
├── Transport(TransportError)        PR #7
├── Protocol(ProtocolError)          PR #7
├── Status(StatusError)              PR #7
├── Validation(ValidationError)      existing + 10 new variants (core-1)
├── Setup(SetupError)                new, 6 variants (core-1)
└── UnsupportedModel {..}            #[doc(hidden)] in core-1, removed in core-2
```

Variant list and messages: [contracts/core-api.md § Errors](contracts/core-api.md#errors).
