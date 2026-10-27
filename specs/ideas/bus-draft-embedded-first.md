# Bus design draft: embedded-first (2026-10-27)

One of three independent drafts merged into `bus-proposal.md`.

# Bus module design: embedded-first proposal

Scope is SRS FR-1, FR-2, FR-3.1–3.5, FR-3.8, FR-4, FR-6.2/6.3/6.5, FR-8, NFR-1/2 and delivery steps 4–6. I read `src/protocol*`, `src/transport*`, `src/actuator.rs`, `src/register.rs`, `src/error.rs`, `src/models*` and the constitution before writing this. I changed no files.

**Five facts in the code shape this design:**
- **Models hold only an `Id`.** Each generated type (e.g. `Xm430W350 { id }`) has no cache, and its registers are `pub(crate)` consts with model-specific names. AX uses `CW_ANGLE_LIMIT` where X uses `MIN_POSITION_LIMIT`, and AX has no `HARDWARE_ERROR_STATUS`. The bus may not touch `src/models*`, so whatever the bus needs from the models has to come from xtask.
- **Exchanges need a clock.** `Exchange` takes a `Link { &mut T, &mut C, timeout }`, but `Bus<P, T, N>` in the SRS has no place for the clock.
- **P1 ping returns no model number.** P1 Setup must read the model number and firmware version afterwards.
- **No `heapless` dependency yet.** `Cargo.toml` belongs to the `ci` module.
- **Two `Transport` traits.** `protocol.rs` defines its own `pub(crate) trait Transport` next to the public `crate::Transport`. They should be merged into one.

## 1. Type signatures

```rust
// Deviation from FR-1.1 / §7: the clock is a 4th type parameter, owned by the bus.
pub struct Bus<P: Protocol, T: Transport, C: Clock, const N: usize = 32> {
    transport: T, clock: C, config: BusConfig,
    actuators: heapless::Vec<ActuatorKind, N>,   // sorted by Id after Setup
    _p: PhantomData<P>,
}
#[cfg(feature = "std")]
pub type StdBus<P, const N: usize = 32> = Bus<P, SerialTransport, StdClock, N>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)] #[non_exhaustive]
pub struct BusConfig { pub timeout: Duration, pub scan_timeout: Duration, pub retries: u8 }
impl BusConfig {                       // const so firmware can write `const CFG: BusConfig = ...`
    pub const fn new() -> Self;        // 10 ms / 10 ms / 0  (FR-1.4 default)
    pub const fn timeout(self, t: Duration) -> Self;
    pub const fn scan_timeout(self, t: Duration) -> Self;
    pub const fn retries(self, n: u8) -> Self;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct ModelNumber(pub u16);

impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    pub fn new(transport: T, clock: C, config: BusConfig) -> Self;   // empty, no I/O
    pub fn into_parts(self) -> (T, C);
    pub fn autoscan(&mut self) -> Result<(), Error>;                  // clears first; empty on Err
    pub fn declare<I: IntoIterator<Item = (Id, ModelNumber)>>(&mut self, list: I) -> Result<(), Error>;
    pub fn actuators(&self) -> &[ActuatorKind];                       // read-only Tier 2 / cache view
    pub fn actuator(&mut self, id: Id) -> Option<ActuatorRef<'_, P, T, C>>;
    // Easy tier: strategy picked by P
    pub fn write_goal_positions<I: IntoIterator<Item = Id>>(&mut self, ids: I, goals: &[Position]) -> Result<(), Error>;
    pub fn write_goal_positions_raw<I: IntoIterator<Item = Id>>(&mut self, ids: I, goals: &[i32]) -> Result<(), Error>;
    pub fn read_positions<I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [Position]) -> Result<(), Error>;
    pub fn read_positions_raw<I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [i32]) -> Result<(), Error>;
    #[cfg(feature = "partial-reads")]
    pub fn read_positions_partial<I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [Result<Position, Error>]) -> Result<(), Error>;
    // Explicit tier
    pub fn sync_write<F: Writable, I: IntoIterator<Item = Id>>(&mut self, ids: I, values: &[F::Value]) -> Result<(), Error>;
}
impl<T: Transport, C: Clock, const N: usize> Bus<P2, T, C, N> {
    pub fn sync_read<F: Readable, I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [F::Value]) -> Result<(), Error>;
    #[cfg(feature = "partial-reads")]
    pub fn sync_read_partial<F: Readable, I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [Result<F::Value, Error>]) -> Result<(), Error>;
}
pub trait Field: sealed::Sealed { type Value: Copy; }   // GoalPosition, PresentPosition, TorqueEnable
pub trait Readable: Field {}  pub trait Writable: Field {}

// No N: the handle erases capacity, so its methods are compiled once per (P, T, C).
pub struct ActuatorRef<'b, P, T, C> { kind: &'b mut ActuatorKind, port: Port<'b, P, T, C> }
pub struct Port<'b, P, T, C> { transport: &'b mut T, clock: &'b mut C, config: &'b BusConfig, _p: PhantomData<P> }

impl<'b, P: Protocol, T: Transport, C: Clock> ActuatorRef<'b, P, T, C> {
    pub fn id(&self) -> Id;
    pub fn kind(&self) -> &ActuatorKind;
    pub fn is_torque_enabled(&self) -> bool;
    pub fn fault(&self) -> Option<StatusReport>;
    pub fn position_limits(&self) -> (Position, Position);
    pub fn enable_torque(&mut self) -> Result<(), Error>;   // Err(Faulted) while a fault is set
    pub fn disable_torque(&mut self) -> Result<(), Error>;
    pub fn set_goal_position(&mut self, goal: Position) -> Result<(), Error>;
    pub fn set_goal_position_raw(&mut self, ticks: i32) -> Result<(), Error>;
    pub fn present_position(&mut self) -> Result<Position, Error>;
    pub fn present_position_raw(&mut self) -> Result<i32, Error>;
    pub fn clear_fault(&mut self) -> Result<bool, Error>;   // FR-6.5
    /// Tier 2: the only route to model-specific ops; applies fault bookkeeping to the result.
    pub fn with_model<R>(&mut self, f: impl FnOnce(&ActuatorKind, &mut Port<'b, P, T, C>) -> Result<R, Error>) -> Result<R, Error>;
    #[cfg(feature = "raw-registers")] pub fn read_raw(&mut self, addr: u16, out: &mut [u8]) -> Result<(), Error>;
    #[cfg(feature = "raw-registers")] pub fn write_raw(&mut self, addr: u16, data: &[u8]) -> Result<(), Error>;
}
impl<T: Transport, C: Clock> ActuatorRef<'_, P2, T, C> { pub fn reboot(&mut self) -> Result<(), Error>; }
```

**Why the clock is owned by the bus as `C`:** I considered three alternatives and rejected each.
- **A `fn() -> u64` pointer.** It cannot capture a HAL timer peripheral or `&MockTime`.
- **A per-call `&mut clock` argument.** It adds noise to every call.
- **A `Timed<T, C>` wrapper.** It produces unreadable type names.

The program has only one concrete `C`, so making it a type parameter adds no extra monomorphisation. To keep the type nameable in RTIC or `static` storage, users pass a named struct or `fn() -> u64`.

**New `ValidationError` variants** (this respects the FR-6.4 split): `CapacityExceeded { capacity }`, `ModelMismatch { id, expected, found }`, `DuplicateId(Id)`, `NotOnBus(Id)`, `ActuatorIsOff(Id)`, `Faulted(Id)`, `OutOfLimits { id }`. A buffer that is too short reuses the existing `DataLength`.

**The `Exchange` trait can't appear in public bounds.** It is `pub(crate)`, so `P: Exchange` on public methods trips the `private_bounds` lint. Fix: move the `Exchange` methods into `sealed::Sealed`, the supertrait of `Protocol`.

## 2. Internal structure

```
src/bus.rs              Bus, new/into_parts, id→index binary search, mod decls
src/bus/config.rs       BusConfig
src/bus/port.rs         Port: builds Link, retry loop, fault choke point
src/bus/actuator_ref.rs Tier 1 + with_model
src/bus/setup.rs        declare, autoscan (P2 broadcast_ping / P1 sweep), limit reads
src/bus/group.rs        target resolution + (addr,size) grouping, N-erased
src/bus/easy.rs         Easy tier + sealed ReadMany { P1: unicast loop, P2: sync_read }
src/bus/explicit.rs     Field/Readable/Writable, sync_write, sync_read
src/bus/partial.rs      #[cfg(partial-reads)]      src/bus/raw.rs  #[cfg(raw-registers)]
```

**Caches are reached through generated data, not generated code.** The xtask module would emit, for each model, `pub(crate) const LAYOUT: Layout`. `Layout` holds the torque, goal and present registers, a limits pair (`MinMax` or `CwCcw`), `hardware_error: Option<Register>`, a position `Scale`, and the protocol bitset. The model struct becomes `{ core: Core }`, with `Core { id, torque_on: bool, fault: Option<StatusReport>, min_raw: i32, max_raw: i32 }`. `Layout` and `Core` are hand-written in `src/actuator.rs`.

On `ActuatorKind`, three crate-internal accessors are generated, each a single `match`: `layout() -> &'static Layout`, `core()` and `core_mut()`. All bus logic is then model-agnostic: one copy of the code, with no 72-arm matches spread through the ops.

Limits are cached in **raw ticks**, so a limit check is an integer compare after one conversion. My estimate is about 20 B per `ActuatorKind`, or about 640 B at N = 32. That figure should be measured with `size_of` in a test.

**Grouping in `group.rs`** works on `&mut [ActuatorKind]`, so it is compiled once per (P, T, C) rather than once per N:
1. **Resolve.** Iterate the targets once. Find each one by binary search, and reject unknown or duplicate IDs using a 256-bit bitset (32 B). Store the indices in `heapless::Vec<u8, N>`.
2. **Validate everything before sending anything.** Check buffer length, the torque guard (all-or-nothing, FR-4.8), the fault state and the limits. Encode the values into `[[u8; 4]; N]` on the stack.
3. **Group.** Compute `key[i] = layout.goal_position`. For each slot not yet done, emit one sync instruction whose entries iterator is `(i..n).filter(key == k).map(|j| (id[j], &bytes[j][..size]))`. That iterator is `Clone` and allocates nothing. The work is O(n·G), with G ≤ 4 RAM tiers. Output slots keep the target order.

Stack cost is about 9N + 64 B, roughly 350 B at N = 32, independent of the call site.

**Fault choke point.** `Port::exchange` and the `sync_read` callback both pass every `StatusError` through `is_hardware_fault()`. A hardware fault sets `core.fault` and `torque_on = false` (FR-6.2). A sync write never reaches this point (FR-6.3, documented).

## 3. Example user code

```rust
// std desktop
use dxl_rs_v2::{Bus, BusConfig, Id, P2, Position, SerialTransport, StdClock};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = SerialTransport::open("/dev/ttyUSB0", 1_000_000)?;
    let mut bus: Bus<P2, _, _, 8> = Bus::new(port, StdClock::default(), BusConfig::new());
    bus.autoscan()?;
    let ids = [Id::new(1)?, Id::new(2)?];
    for id in ids { bus.actuator(id).ok_or("missing")?.enable_torque()?; }
    let mut pos = [Position::ZERO; 2];
    loop {
        bus.read_positions(ids, &mut pos)?;
        let goals = pos.map(|p| p + Position::from_rad(0.01));
        bus.write_goal_positions(ids, &goals)?;
    }
}
```

```rust
// no_std, heap-free (default-features = false, features = ["x-series"])
const CFG: BusConfig = BusConfig::new().timeout(Duration::from_millis(2));
type Arm = Bus<P2, Uart1, fn() -> u64, 4>;          // nameable for RTIC / statics
fn now_us() -> u64 { MONO.now().ticks() }

fn init(uart: Uart1) -> Result<Arm, Error> {
    let mut bus: Arm = Bus::new(uart, now_us as fn() -> u64, CFG);
    bus.declare([(id!(1), ModelNumber(1020)), (id!(2), ModelNumber(1020))])?;
    Ok(bus)
}
fn tick(bus: &mut Arm, out: &mut [i32; 2]) -> Result<(), Error> {
    bus.read_positions_raw([id!(1), id!(2)], out)  // ≤ 2 × (timeout + tx) worst case
}
```

## 4. Test strategy

- **`MockTransport` + `MockTime`.** Every test scripts replies with `reply_to(nth_flush, ..)` and asserts the bytes with `assert_sent` and `assert_done`. Packets come from the existing `protocol/vectors.rs` builders.
- **Setup.**
  - P2 broadcast ping: two replies, plus one late reply after `scan_timeout` that must be ignored.
  - P1 sweep: about 253 silent pings with an advancing `ticker`, asserting the elapsed time is ≤ 253 × timeout.
  - The firmware-variant pick, `UnsupportedModel`, `CapacityExceeded` at N = 1, and a declared model mismatch.
  - A failed limit read fails Init and leaves the bus empty.
  - `sent()` must not contain any Torque Enable write (FR-2.7).
- **Guards.** A goal write while torque is off returns `ActuatorIsOff` with `sent()` empty; a multi-write with one actuator off also sends nothing. An out-of-limit value is rejected the same way.
- **Faults.** A P2 reply with the alert bit set gives `fault().is_some()`, torque off, and a typed error. `clear_fault` with Hardware Error Status at 0 or non-zero, and the P1 path that pings and checks flags.
- **Grouping.** Mix an XL320 tier with an XM430 tier and assert two sync packets in first-appearance order with slots in target order. A silent middle actuator in `sync_read` fails a strict read but keeps the other slots in a partial read.
- **No allocation.** Reuse the counting allocator from `protocol/no_alloc.rs` around a Cycle loop.
- **Compile-fail with `trybuild`.** Add a `tests/ui/` dev-dependency with `.stderr` snapshots, so a test passes only on the exact error (E0599/E0499), unlike `compile_fail` doctests, which pass on any error. Cases:
  - `Bus<P1, ..>::sync_read` → E0599
  - `ActuatorRef<P1>::reboot` → E0599
  - two live `actuator()` refs → E0499
  - `read_positions_partial` without the feature → E0599
- The existing CI `thumbv7em` clippy job covers NFR-1.

## 5. Risks and open questions

**Risks:**
1. **Worst-case timing** (FR-1.4, FR-2.1, NFR-5). A P1 autoscan costs 253 × (timeout + tx), about 2.6 s at 10 ms. A `sync_read` gives every reply a full timeout, so a dead bus costs n × timeout. `drain` can take up to one timeout. Per attempt the bound is 2 × timeout + tx, multiplied by (retries + 1). I'll publish the formula in the docs. Retries apply to unicast only and never to `StatusError`.
2. **The models are no-go for the bus** (FR-2.6, FR-3.x, Principle V). Without the xtask `Layout`/`Core` PR, the bus can't be written without hand-written per-model matches.
3. **Models don't record which protocol they support** (FR-2.3, Principle II). A P1 bus that finds a 2.0 table fails with `AddressOutOfRange` at Cycle instead of Init. Fix: generate a `protocols` const and check it in Setup.
4. **A faulted P2 actuator breaks strict reads** (FR-4.6, FR-6.2). Every reply carries the alert bit until reboot, so every strict read of a group containing it fails.
5. **Cross-module edits** (NFR-1). `heapless` and `trybuild` belong to Cargo.toml (`ci` module); `Position` and `Core` belong to `core`; `Layout` belongs to `xtask`. The P1 autoscan read at addresses 0..3 for model and firmware also needs an e-Manual check (Principle IV).

**Open questions for the architect:**
1. Do you accept `Bus<P, T, C, N>` with the `StdBus` alias? This deviates from FR-1.1 and §7.
2. Overrule SRS 2.1 ("autoscan only when std is present")? Autoscan needs no heap, and embedded bring-up benefits from it.
3. Overrule the FR-4.5 default: strict reads take `&mut [Position]` and partial reads take `&mut [Result<_, Error>]`? `Option` on a strict read is always `Some` on `Ok` and doubles slot size.
4. How should a faulted actuator behave in strict multi-reads (risk 4)? Options: skip it with an error, fail the call, or return data despite the alert bit.
5. FR-6.5: X-series Hardware Error Status persists until Reboot (to be verified against the e-Manual), so `clear_fault` alone never clears it. Add `reboot()` on P2 only? Accept that P1 and AX, which have no such register, clear by pinging and checking the flags?

## 6. Delivery split

The constitution requires one SRS module per PR, so this sequence needs your approval to spread the bus module over several PRs.

| PR | Content | Coverage plan |
|---|---|---|
| 0 (`xtask`/`core`) | `Layout`, `Core`, `protocols` const, generated accessors; `heapless` dependency | xtask golden tests |
| 1 (step 4a) | `Bus`, `BusConfig`, `Port` + retries, `declare`, limits at Init, `ActuatorRef` torque/goal/present (raw), guard | mock scripts on P1 and P2, about 85 % |
| 2 (step 4b) | `autoscan`: P2 broadcast, P1 sweep + model read, firmware pick, overflow | timing and sweep tests |
| 3 (step 5) | Fault choke point, `clear_fault`, `reboot` (P2), `Position` + limit validation, `with_model` | fault matrix tests |
| 4 (step 6) | `group.rs`, Easy + Explicit tiers, strict reads, `trybuild` UI tests, no-alloc test | grouping and compile-fail |
| 5 (step 9) | `partial-reads`, `raw-registers` | feature-gated tests run in CI with `--all-features` |