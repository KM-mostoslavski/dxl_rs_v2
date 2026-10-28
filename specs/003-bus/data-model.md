# Data Model: Bus

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md) | **API**: [contracts/bus-api.md](contracts/bus-api.md)

Types owned by other modules (`Id`, `ModelNumber`, `Position`, `PositionScale`, `TorqueState`,
`Cache`, `Limits`, `LimitMode`, `ModeSource`, `Register`, `ModelData`, `ActuatorKind`, the
errors, `Link`, `Pong`) are defined in `specs/ideas/module-interfaces.md` §2 to §5. This file
only says how the bus uses them.

## Bus<P, T, C = DefaultClock, N = 32>

| Field | Type | Notes |
|---|---|---|
| `transport` | `T: Transport` | Owned. Given back by `into_parts`. |
| `clock` | `C` (`C: Clock` on every impl) | Owned. Zero bytes for a stateless clock. |
| `config` | `Config` | Fixed at construction. |
| `actuators` | `heapless::Vec<ActuatorKind, N>` | Sorted by `Id`, no duplicates. |
| `states` | `[TorqueState; N]` | `states[i]` belongs to `actuators[i]`. Entries at `i >= len` are unused and kept `Off`. |
| `_protocol` | `PhantomData<P>` | |

**Invariants**:
- `actuators` is strictly ascending by `Id`.
- `N <= 253`, checked at compile time in `new`.
- After a failed Setup, `actuators` is empty.
- After a successful Setup, every slot is `Off` and every `Cache` has limits and the limit mode
  read at Setup.
- No field holds a borrow, `Rc` or `RefCell`.

**Lifecycle**:

```
new ──► Empty ──declare / autoscan──► Ready(k actuators, all Off)
          ▲            │ any error                │ declare / autoscan again
          └────────────┘ (list cleared)           └──► replaces the list (or Empty on error)
Ready ──into_parts──► (T, C)
```

`new` and `into_parts` send nothing (FR-005).

## Config

| Field | Type | Default | Meaning |
|---|---|---|---|
| `timeout` | `Duration` | 10 ms | Wait for one unicast or sync-read reply (FR-1.4). |
| `scan_timeout` | `Duration` | 10 ms | P2 broadcast-ping window; per-ID wait of the P1 sweep. |
| `retries` | `u8` | 0 | Extra attempts of a unicast exchange after a timeout or corruption. |

`#[non_exhaustive]`, `Copy`, `const` builder, `Default` = `new()`. Fields are readable.

## NoClock, DefaultClock

- `NoClock`: an empty enum. It implements no `Clock`.
- `DefaultClock`: `StdClock` with `std`, `NoClock` without. A no_std user names a clock.

## Wire<'b, P, T, C> (crate-internal)

| Field | Type |
|---|---|
| `transport` | `&'b mut T` |
| `clock` | `&'b mut C` |
| `config` | `&'b Config` |

| Operation | Protocol call | Link timeout | Retried |
|---|---|---|---|
| `ping(id)` | `P::ping` | `timeout` | yes |
| `read(id, reg, buf)` | `P::read` | `timeout` | yes |
| `write(id, reg, bytes)` | `P::write` | `timeout` | yes |
| `sync_write(addr, size, entries)` | `P::sync_write` | `timeout` (drain only) | no |
| `read_many(addr, size, ids, scratch, cb)` | `P::read_many` | `timeout` | no |
| `scan(cb)` | `P::scan` | `scan_timeout` | no |

**Retry rule**: there are at most `retries` extra attempts. A retry happens only after
`Transport(Timeout)` or `Protocol(_)`. `Status`, `Validation` and `Transport(Io)` return at
once.

## Slots (crate-internal trait)

| Method | Meaning |
|---|---|
| `kinds(&self) -> &[ActuatorKind]` | Sorted list. |
| `split_mut(&mut self) -> (&mut [ActuatorKind], &mut [TorqueState])` | Same length, same order. |
| `clear(&mut self)` | Empties the list. States become `Off`. |
| `push(&mut self, kind) -> Result<(), SetupError>` | `TooManyActuators` past capacity. |
| `stages(&mut self) -> &mut [Stage]` | Staging for multi-actuator ops (`DynBus` only; `Bus` uses a stack array). |

Implementors:
- `FixedSlots<'a, N>`: borrows `Bus`'s `actuators` and `states`.
- `DynSlots<'a>` (`std`): borrows `DynBus`'s three Vecs.

`Bus` methods split `&mut self` into `(Wire, FixedSlots)` with disjoint field borrows.

## ActuatorRef<'b, P, T, C>

| Field | Type | Notes |
|---|---|---|
| `kind` | `&'b mut ActuatorKind` | The model and its `Cache`. |
| `state` | `&'b mut TorqueState` | This actuator's slot. |
| `wire` | `Wire<'b, P, T, C>` | No `N`, so `Bus` and `DynBus` share the type. |

It borrows `&mut Bus`, so two cannot be live at once (`E0499`). It cannot outlive the bus.

## TorqueState as the bus applies it

The transitions are core's (module-interfaces §2.5). The bus fires them on these events:

| Event (bus) | Core transition | Sent first? |
|---|---|---|
| Setup adds the slot | slot = `Off` | nothing sent |
| `enable_torque` | `enable(id)?` before sending; `Ok` → `On` | refused while `Faulted` (`ActuatorFaulted`), zero bytes |
| `disable_torque` | `Ok` → `disable()` | always sent |
| Torque write `Err`, hardware fault | `fault()` | |
| Torque write `Err`, anything else | `disable()`: `On`/`Off` → `Off`, `Faulted` stays `Faulted` (never `On`; plan deviation 5, now in §2.5) | |
| Any reply with `is_hardware_fault()` (ref calls, multi-read slots, Easy-tier torque, clear check) | `fault()` | |
| `sync_write` | none (no reply) | |
| `raw-registers` reply with `is_hardware_fault()` | `fault()`, like every other reply (architect, 2026-10-28; research R-21) | the raw exchange |
| `raw-registers` write to Torque Enable | none: the state is not updated (documented on `write_raw`, FR-8.2) | sent, no guard |
| `clear_fault` check passes | `clear()` (`Faulted` → `Off`) | |
| `clear_fault` check fails | stays `Faulted`, error returned | |

**Goal guard**: `state.check_goal(id)`. Only `On` passes. `Off` and `Faulted` give
`ActuatorIsOff { id }`.

## LimitMode as the bus applies it

`LimitMode` lives in each actuator's `Cache` (core, module-interfaces §2.4b). The bus sets it once
at Setup and never stores a copy (research R-9).

| `data().mode` (`ModeSource`) | Setup read | Example → `LimitMode` |
|---|---|---|
| `OperatingMode` (X, MX(2.0), PRO, PRO+, Y) | 1 byte at `register()` | X: 3 → `Bounded`; 4, 5 → `Unbounded`; 0, 1, 16 → `NoPosition` |
| `ControlMode` (XL-320) | 1 byte at `register()` | 2 (joint) → `Bounded`; 1 (wheel) → `NoPosition` |
| `AngleLimits` (AX, MX on P1) | none (`register()` is `None`); CW/CCW from the limit reads | CW = CCW = 0 → `NoPosition`; MX P1 CW = CCW = 4095 → `Unbounded`; else `Bounded` |

The mapping is core's: `ModeSource::limit_mode(value)` for a register value,
`LimitMode::from_angle_limits(limits)` for `AngleLimits`. When `limit_mode` returns `None`, the
bus fails Setup with `SetupError::UnknownOperatingMode { id, value }`.

| `LimitMode` | `cache.check_goal(raw)` | Then |
|---|---|---|
| `Bounded` | `LimitsUnknown` or `OutOfLimits` outside the EEPROM limits | `Register::encode`, send |
| `Unbounded` | `Ok(raw)` | `Register::encode` (`DoesNotFit` beyond the register), send |
| `NoPosition` | `NotPositionMode { id }` | nothing sent |

## Setup records

| Name | Type | Lives | Notes |
|---|---|---|---|
| Declared list | `heapless::Vec<(Id, ModelNumber), N>` | stack, during `declare` | Rejects overflow and duplicates before any byte. |
| Scan replies | `heapless::Vec<Pong, N>` | stack, during `autoscan` | Sorted by `Id` before `init_one`. |

**`init_one` steps** (research R-6):
1. Firmware (`Pong` on P2, or a register read).
2. `ActuatorKind::new`.
3. Limit min/max reads → `record_limits`.
4. Mode: `register()` read (1 byte) → `limit_mode(value)`, or `from_angle_limits` on the limits
   of step 3 → `record_mode`.
5. `push` and slot `Off`.

## IdSet

| Field | Type | Notes |
|---|---|---|
| bits | `[u32; 8]` | Bit `n` set ⇔ `Id(n)` present. Only bits 0..=252 are used. |

`Copy`, `Default` (empty), `PartialEq`, `Eq`, `Debug`. Iterates ascending.

## Stage (crate-internal)

| Field | Type | Notes |
|---|---|---|
| `slot` | `u8` | Index into the slot list. |
| `addr` | `u16` | Register address of this target's field. |
| `size` | `u8` | 1, 2 or 4. |
| `bytes` | `[u8; 4]` | Encoded value (writes) or received bytes (reads). |

8 bytes. `Bus` uses `[Stage; N]` on the stack per call (256 B at N = 32). `DynBus` uses a Vec sized
at Setup.

Per-call read bookkeeping is the first error and whether it is a hardware fault, so a hardware
fault wins.

## Group

Not stored. A group is the distinct `(addr, size)` keys of the staged targets, in
first-appearance order. One instruction is sent per key, over a `Clone` iterator of the
matching stages.

## Field markers (`field` module)

| Marker | Trait | `Value` | `ModelData` register |
|---|---|---|---|
| `GoalPosition` | `WriteField` | `Position` | `goal_position` |
| `PresentPosition` | `ReadField` | `Position` | `present_position` |
| `Raw<F>` | same as `F` | `i32` | same as `F` |

The traits are sealed. `TorqueEnable` is deliberately absent (research R-7).

## DynBus<P, T, C = StdClock> (`std`)

| Field | Type |
|---|---|
| `transport`, `clock`, `config` | as `Bus` |
| `actuators` | `alloc::Vec<ActuatorKind>` |
| `states` | `alloc::Vec<TorqueState>` |
| `stages` | `alloc::Vec<Stage>`, length = actuator count after Setup |

The Vecs grow only during Setup. No Cycle call allocates.

## Error mapping (which error, when)

| Condition | Error | Bytes sent |
|---|---|---|
| Declared list longer than `N`, or scan replies past `N` | `SetupError::TooManyActuators { capacity }` | none (declared); scan only (autoscan) |
| Repeated `Id` in a declared list, scan replies or targets | `ValidationError::DuplicateId { id }` | none (declared, targets) |
| Model compiled out / no table on this protocol / unknown | `SetupError::UnsupportedModel { id, model_number, protocol, feature }` | none for declared |
| Declared actuator silent | `SetupError::Missing { id }` | ping(s) |
| Declared actuator with another model | `SetupError::WrongModel { id, declared, found }` | ping (+ P1 model read) |
| Target `Id` not held | `SetupError::NotOnBus { id }` | none |
| Mode value not listed for the family | `SetupError::UnknownOperatingMode { id, value }` | Setup reads up to the mode read |
| Goal while `Off` or `Faulted` | `ValidationError::ActuatorIsOff { id }` | none |
| Enable while `Faulted` | `ValidationError::ActuatorFaulted { id }` | none |
| Goal outside limits (`Bounded`) | `ValidationError::OutOfLimits { id, raw, min, max }` | none |
| Goal while `NoPosition` | `ValidationError::NotPositionMode { id }` | none |
| Goal beyond the register (`Unbounded`, or a typed angle past `i32`) | `ValidationError::DoesNotFit { raw, size }` | none |
| NaN or infinite angle | `ValidationError::NotFinite` | none |
| Value count / buffer length | `ValidationError::BufferLength { targets, got }` | none |
| Hardware fault in any reply after Setup, raw included | `Error::Status(StatusError)` and the slot becomes `Faulted` | the exchange |
| Hardware fault in a reply during Setup | `Error::Status(StatusError)`, list empty | Setup reads so far |
| Silent actuator after retries | `TransportError::Timeout { id }` | the exchange(s) |
| Corrupted reply after retries | `Error::Protocol(_)` | the exchange(s) |
