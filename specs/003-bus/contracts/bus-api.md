# Contract: Bus public API

**Feature**: [../spec.md](../spec.md) | **Model**: [../data-model.md](../data-model.md) | **Shared contract**: `specs/ideas/module-interfaces.md` §6

Signatures are the public surface of the bus module. Bodies, private items and tests are not
part of the contract. Every public item is documented. Every method that can fail returns
`Result<_, Error>` (`crate::Error`). The deviation from §6 (no `Clock` bound on the struct) is
marked **[dev-1]**.

## Clocks and config (`src/bus.rs`, always available)

```rust
/// No clock. Implements no `Clock`, so a no_std bus must name its clock.
pub enum NoClock {}

#[cfg(feature = "std")]      pub type DefaultClock = StdClock;
#[cfg(not(feature = "std"))] pub type DefaultClock = NoClock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {
    /// Wait for one reply. Default 10 ms.
    pub timeout: core::time::Duration,
    /// P2 broadcast-ping window and P1 per-ID sweep wait. Default 10 ms.
    pub scan_timeout: core::time::Duration,
    /// Extra attempts of a unicast exchange after a timeout or corrupted reply. Default 0.
    pub retries: u8,
}
impl Config {
    pub const fn new() -> Self;
    pub const fn timeout(self, t: core::time::Duration) -> Self;
    pub const fn scan_timeout(self, t: core::time::Duration) -> Self;
    pub const fn retries(self, n: u8) -> Self;
}
impl Default for Config;   // = new()
```

## Bus (`src/bus.rs`)

```rust
pub struct Bus<P: Protocol, T: Transport, C = DefaultClock, const N: usize = 32> { /* private */ } // [dev-1]

impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    /// No I/O. Compile error if `N > 253`.
    pub fn new(transport: T, clock: C, config: Config) -> Self;
    pub fn into_parts(self) -> (T, C);
    pub fn config(&self) -> Config;

    /// Setup from a list. Pings only the listed IDs. Replaces the held list; empty on `Err`.
    pub fn declare<I>(&mut self, list: I) -> Result<&[ActuatorKind], Error>
    where I: IntoIterator<Item = (Id, ModelNumber)>;
    /// Setup by scanning (P2 broadcast ping, P1 sweep 0..=252). Available without `std`.
    pub fn autoscan(&mut self) -> Result<&[ActuatorKind], Error>;

    /// Held actuators, ascending by `Id`.
    pub fn actuators(&self) -> &[ActuatorKind];
    /// Held IDs, as an owned set (so `bus.enable_torque(bus.ids())` compiles).
    pub fn ids(&self) -> IdSet;
    /// `None` for an `Id` the bus does not hold.
    pub fn actuator(&mut self, id: Id) -> Option<ActuatorRef<'_, P, T, C>>;
    pub fn torque_state(&self, id: Id) -> Option<TorqueState>;
    /// Same order and length as `actuators()`.
    pub fn torque_states(&self) -> &[TorqueState];
}
```

### Easy tier (`src/bus/multi.rs`, every protocol)

Targets are any `IntoIterator<Item = Id>` in the caller's order. Validation covers the whole
call before any byte: unknown (`NotOnBus`), repeated (`DuplicateId`), count (`BufferLength`),
torque state, limits. A refusal sends nothing to anyone. An empty target list sends nothing and
returns `Ok`.

```rust
impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    /// Acknowledged unicast writes, in target order. Refused (nothing sent) if any target is
    /// faulted. Stops at the first error.
    pub fn enable_torque<I: IntoIterator<Item = Id>>(&mut self, ids: I) -> Result<(), Error>;
    /// Acknowledged unicast writes to every target, even after an error. Returns the first
    /// error, a hardware fault first.
    pub fn disable_torque<I: IntoIterator<Item = Id>>(&mut self, ids: I) -> Result<(), Error>;

    /// One Sync Write per register layout. No reply (faults show on the next read).
    /// `goals.len()` must equal the target count. Not atomic across layouts on the wire.
    pub fn write_goal_positions<I: IntoIterator<Item = Id>>(&mut self, ids: I, goals: &[Position])
        -> Result<(), Error>;
    pub fn write_goal_positions_raw<I: IntoIterator<Item = Id>>(&mut self, ids: I, goals: &[i32])
        -> Result<(), Error>;

    /// Strict read. P2: one Sync Read per layout. P1: one Read per target.
    /// `out.len() >= targets`; `out[i]` is target `i`. On `Err`, `out` is unchanged. Several
    /// failures: the first hardware fault wins, else the first error.
    pub fn read_positions<I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [Position])
        -> Result<(), Error>;
    pub fn read_positions_raw<I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [i32])
        -> Result<(), Error>;
}
```

### Explicit tier (`src/bus/multi.rs`)

```rust
impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    /// Sync Write of field `F`, same validation as the Easy tier. Both protocols.
    pub fn sync_write<F: WriteField, I: IntoIterator<Item = Id>>(
        &mut self, field: F, ids: I, values: &[F::Value],
    ) -> Result<(), Error>;
}

impl<T: Transport, C: Clock, const N: usize> Bus<P2, T, C, N> {
    /// Sync Read of field `F`. Protocol 2 only: absent on `Bus<P1, …>` (compile error E0599).
    pub fn sync_read<F: ReadField, I: IntoIterator<Item = Id>>(
        &mut self, field: F, ids: I, out: &mut [F::Value],
    ) -> Result<(), Error>;
}
```

### `partial-reads` feature (`src/bus/partial.rs`)

```rust
#[cfg(feature = "partial-reads")]
impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    /// One result per slot. Validation errors still fail the call (nothing sent).
    /// P1: each slot is independent. P2: a group whose instruction fails gives that error to
    /// all its slots; per-ID errors stay per slot (narrows SRS FR-4.7, plan SRS deviation 1).
    /// Faults mark state as usual.
    pub fn read_positions_partial<I: IntoIterator<Item = Id>>(
        &mut self, ids: I, out: &mut [Result<Position, Error>],
    ) -> Result<(), Error>;
    pub fn read_positions_raw_partial<I: IntoIterator<Item = Id>>(
        &mut self, ids: I, out: &mut [Result<i32, Error>],
    ) -> Result<(), Error>;
}
```

## ActuatorRef (`src/bus/actuator_ref.rs`)

```rust
pub struct ActuatorRef<'b, P: Protocol, T: Transport, C: Clock> { /* private */ }

impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> {
    /// Tier 2: match on the model.
    pub fn kind(&self) -> &ActuatorKind;
    pub fn torque_state(&self) -> TorqueState;

    /// Refused while faulted (`ActuatorFaulted`, nothing sent). A failed write leaves `Off`
    /// (`Faulted` on a hardware fault).
    pub fn enable_torque(&mut self) -> Result<(), Error>;
    /// Always sent. A failed write applies `disable()`: `On`/`Off` become `Off`, a `Faulted`
    /// slot stays `Faulted` (the fault is never lost). A hardware fault gives `Faulted`.
    pub fn disable_torque(&mut self) -> Result<(), Error>;

    /// Guard (`ActuatorIsOff`), then limits (`OutOfLimits`), then one write. No read-back.
    /// Limits are applied in every operating mode (see docs).
    pub fn set_goal_position(&mut self, goal: Position) -> Result<(), Error>;
    pub fn set_goal_position_raw(&mut self, raw: i32) -> Result<(), Error>;

    /// Explicit read-back (FR-3.8).
    pub fn goal_position(&mut self) -> Result<Position, Error>;
    pub fn goal_position_raw(&mut self) -> Result<i32, Error>;
    pub fn present_position(&mut self) -> Result<Position, Error>;
    pub fn present_position_raw(&mut self) -> Result<i32, Error>;

    /// Asks the actuator (Hardware Error Status, or a ping where the model has none).
    /// Clears to `Off` only if it reports no fault; otherwise returns its `Status` error.
    pub fn clear_fault(&mut self) -> Result<(), Error>;
}

/// `id`, `model_number`, `name`, `position_scale`, `position_limits[_raw]`.
impl<P: Protocol, T: Transport, C: Clock> Actuator for ActuatorRef<'_, P, T, C> {}
```

### `raw-registers` feature (`src/bus/raw.rs`)

```rust
#[cfg(feature = "raw-registers")]
impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> {
    /// Reads `out.len()` bytes at `addr`. No guard, no state change; a reported fault is
    /// returned but not recorded (FR-8.1; exception to constitution III awaiting architect
    /// approval, plan Complexity Tracking). May desync the bus's view (documented escape hatch).
    pub fn read_raw(&mut self, addr: u16, out: &mut [u8]) -> Result<(), Error>;
    /// Writes `data` at `addr`. No torque guard, no limit check, no state change, even on a
    /// reported hardware fault (same exception as `read_raw`).
    pub fn write_raw(&mut self, addr: u16, data: &[u8]) -> Result<(), Error>;
}
```

## Field markers (`src/bus/field.rs`, public module `field`)

```rust
pub mod field {
    pub trait ReadField: sealed::Field { type Value: Copy; }
    pub trait WriteField: sealed::Field { type Value: Copy; }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub struct GoalPosition;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub struct PresentPosition;
    /// The raw register value of `F`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub struct Raw<F>(pub F);

    impl WriteField for GoalPosition    { type Value = Position; }
    impl ReadField  for PresentPosition { type Value = Position; }
    impl<F: WriteField> WriteField for Raw<F> { type Value = i32; }
    impl<F: ReadField>  ReadField  for Raw<F> { type Value = i32; }
}
```

## IdSet (`src/bus/id_set.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IdSet { /* [u32; 8] */ }
impl IdSet {
    pub const fn new() -> Self;
    /// `first..=last`; empty if `first > last`.
    pub const fn range(first: Id, last: Id) -> Self;
    /// Returns `false` if already present.
    pub fn insert(&mut self, id: Id) -> bool;
    pub fn remove(&mut self, id: Id) -> bool;
    pub const fn contains(&self, id: Id) -> bool;
    pub const fn len(&self) -> usize;
    pub const fn is_empty(&self) -> bool;
}
impl IntoIterator for IdSet { type Item = Id; type IntoIter = IdSetIter; } // ascending
impl FromIterator<Id> for IdSet;
impl<const K: usize> From<[Id; K]> for IdSet;
pub struct IdSetIter { /* private */ }   // Iterator<Item = Id> + Clone
```

## `std` feature

```rust
// src/bus/serial.rs
pub type SerialBus<P, const N: usize = 32> = Bus<P, SerialTransport, StdClock, N>;
impl<P: Protocol, const N: usize> Bus<P, SerialTransport, StdClock, N> {
    /// `SerialTransport::open(path, baud)`, `StdClock::default()`, `Config::new()`. No I/O.
    pub fn open(path: &str, baud: u32) -> Result<Self, serialport::Error>;
}

// src/bus/dyn_bus.rs
/// A bus with no capacity limit. Storage grows at Setup only.
pub struct DynBus<P: Protocol, T: Transport, C = StdClock> { /* private */ }
impl<P: Protocol, T: Transport, C: Clock> DynBus<P, T, C> {
    // Same methods as Bus, same signatures and behaviour, without N:
    // new, into_parts, config, declare, autoscan, actuators, ids, actuator, torque_state,
    // torque_states, enable_torque, disable_torque, write_goal_positions(_raw),
    // read_positions(_raw), sync_write, and (partial-reads) read_positions(_raw)_partial.
}
impl<T: Transport, C: Clock> DynBus<P2, T, C> { pub fn sync_read<F: ReadField, I: IntoIterator<Item = Id>>(&mut self, field: F, ids: I, out: &mut [F::Value]) -> Result<(), Error>; }
```

## Re-exports added to `src/lib.rs` (bus lines only)

```rust
mod bus;
pub use bus::{ActuatorRef, Bus, Config, DefaultClock, IdSet, NoClock, field};
#[cfg(feature = "std")]
pub use bus::{DynBus, SerialBus};
```

Each PR adds only the lines for the items it ships.

## Cargo features (bus additions)

| Feature | Default | Adds |
|---|---|---|
| `partial-reads` | off | `read_positions_partial`, `read_positions_raw_partial` (on `Bus` and `DynBus`) |
| `raw-registers` | off | `ActuatorRef::read_raw`, `ActuatorRef::write_raw` |
| `std` (existing) | on | `SerialBus` + `open`, `DynBus` |

No method changes behaviour when a feature is enabled (constitution VII).

## Documented guarantees (rustdoc, checked in review)

- Module docs:
  - broadcast writes get no reply, so a write-only loop sees faults on its next read (FR-029);
  - a multi-write across layouts is not atomic on the wire;
  - the timing formulas of research R-4 and R-11;
  - a X-series fault needs a reboot outside this crate.
- `set_goal_position`: limits are those read at Setup. They are applied in every operating mode.
  X-series extended and current-based position modes and AX wheel mode refuse more than the
  hardware would.
- `compile_fail` examples:
  - two live refs (E0499);
  - a ref outliving its bus;
  - `sync_read` on P1 (E0599);
  - each feature-gated method without its feature;
  - `DynBus` without `std`;
  - a no_std `Bus::new` without a clock (E0277).
- Doc examples and features: ci builds every doc example once more with `std` off
  (specs/007-ci contracts). An example that uses a std-only item (`SerialBus`, `DynBus`,
  `SerialTransport`, `StdClock`) is gated with `#[cfg_attr(feature = "std", doc = …)]`. A
  `compile_fail` example that must fail only with a feature or `std` off (`DynBus` without `std`,
  a feature-gated method without its feature, `Bus::new` without a clock) is gated with
  `#[cfg_attr(not(feature = "…"), doc = …)]`.
