# Bus design draft: dx-first (2026-10-27)

One of three independent drafts merged into `bus-proposal.md`.

# Bus module design: developer experience first

I checked the design against the code that exists today. I wrote two small probe crates in the scratchpad and confirmed by compiling them. Nothing in the repo was edited.

## 1. Type signatures

```rust
// ---- src/bus.rs ----
pub struct Bus<P: Protocol, T: Transport, const N: usize = 32, C: Clock = fn() -> u64> {
    transport: T,
    clock: C,                         // the Clock lives here, beside T
    config: Config,
    actuators: heapless::Vec<ActuatorKind, N>,
    slot: [u8; 253],                  // Id -> index, 0xFF = absent; const-assert N < 255
    _p: PhantomData<P>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config { pub timeout: Duration, pub retries: u8 }   // Default: 10 ms, 0 (FR-1.4)
impl Config {
    pub const fn new() -> Self;
    pub const fn timeout(self, t: Duration) -> Self;
    pub const fn retries(self, n: u8) -> Self;
}

impl<P: Protocol, T: Transport, const N: usize, C: Clock> Bus<P, T, N, C> {
    pub fn new(transport: T, clock: C) -> Self;
    pub fn with_config(transport: T, clock: C, config: Config) -> Self;
    pub fn into_parts(self) -> (T, C);

    // Setup (Init). Each call replaces the list; on error the list is left empty.
    pub fn declare<I: IntoIterator<Item = (Id, ModelNumber)>>(&mut self, expected: I)
        -> Result<&[ActuatorKind], Error>;
    #[cfg(feature = "std")]                                   // SRS 2.1, see Q5
    pub fn autoscan(&mut self) -> Result<&[ActuatorKind], Error>;

    pub fn actuators(&self) -> &[ActuatorKind];
    pub fn ids(&self) -> IdSet;                               // Copy bitmap, see below
    pub fn actuator(&mut self, id: Id) -> Option<ActuatorRef<'_, P, T, C>>;   // FR-1.5

    // Easy tier: P picks the wire strategy
    pub fn read<F: ReadField, I>(&mut self, ids: I, out: &mut [F::Value]) -> Result<(), Error>
        where I: IntoIterator<Item = Id>, I::IntoIter: Clone;
    pub fn write<F: WriteField, I>(&mut self, ids: I, values: &[F::Value]) -> Result<(), Error>
        where I: IntoIterator<Item = Id>, I::IntoIter: Clone;
    pub fn read_positions<I>(&mut self, ids: I, out: &mut [Position]) -> Result<(), Error>
        where I: IntoIterator<Item = Id>, I::IntoIter: Clone;   // = read::<PresentPosition>
    pub fn write_goal_positions<I>(&mut self, ids: I, goals: &[Position]) -> Result<(), Error>
        where I: IntoIterator<Item = Id>, I::IntoIter: Clone;
    pub fn enable_torque<I: IntoIterator<Item = Id>>(&mut self, ids: I) -> Result<(), Error>;  // unicast, acked
    pub fn disable_torque<I: IntoIterator<Item = Id>>(&mut self, ids: I) -> Result<(), Error>;

    // Explicit tier
    pub fn sync_write<F: WriteField, I>(&mut self, ids: I, values: &[F::Value]) -> Result<(), Error>
        where I: IntoIterator<Item = Id>, I::IntoIter: Clone;

    #[cfg(feature = "partial-reads")]
    pub fn read_positions_partial<I>(&mut self, ids: I, out: &mut [Result<Position, Error>])
        -> Result<(), Error> where I: IntoIterator<Item = Id>, I::IntoIter: Clone;
}

impl<T: Transport, const N: usize, C: Clock> Bus<P2, T, N, C> {
    pub fn sync_read<F: ReadField, I>(&mut self, ids: I, out: &mut [F::Value]) -> Result<(), Error>
        where I: IntoIterator<Item = Id>, I::IntoIter: Clone;   // absent on P1
}

#[cfg(feature = "std")]
pub type SerialBus<P, const N: usize = 32> = Bus<P, SerialTransport, N, StdClock>;
#[cfg(feature = "std")]
impl<P: Protocol, const N: usize> Bus<P, SerialTransport, N, StdClock> {
    pub fn open(path: &str, baud: u32) -> Result<Self, serialport::Error>;
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct IdSet([u32; 8]);   // IntoIterator<Item = Id>, ascending; From<[Id; K]>

// ---- src/bus/actuator_ref.rs ----
pub struct ActuatorRef<'b, P: Protocol, T: Transport, C: Clock> {
    kind: &'b mut ActuatorKind,
    link: Link<'b, T, C>,      // reuses protocol::Link {transport, clock, timeout}
    retries: u8,
    _p: PhantomData<P>,
}
impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> {
    pub fn id(&self) -> Id;
    pub fn kind(&self) -> &ActuatorKind;                          // Tier 2: match
    pub fn is_torque_enabled(&self) -> bool;                      // cache
    pub fn fault(&self) -> Option<StatusReport>;                  // cache
    pub fn position_limits(&self) -> RangeInclusive<Position>;    // cache
    pub fn enable_torque(&mut self) -> Result<(), Error>;         // faulted -> ActuatorFaulted, nothing sent
    pub fn disable_torque(&mut self) -> Result<(), Error>;        // always allowed
    pub fn set_goal_position(&mut self, goal: Position) -> Result<(), Error>;
    pub fn set_goal_position_raw(&mut self, ticks: i32) -> Result<(), Error>; // still guarded + limit-checked
    pub fn present_position(&mut self) -> Result<Position, Error>;
    pub fn present_position_raw(&mut self) -> Result<i32, Error>;
    pub fn goal_position(&mut self) -> Result<Position, Error>;   // explicit read-back (FR-3.8)
    pub fn clear_fault(&mut self) -> Result<(), Error>;           // FR-6.5
    pub fn as_model<M: Model>(&mut self) -> Option<ModelRef<'_, M, P, T, C>>; // Tier 2 I/O
    #[cfg(feature = "raw-registers")]
    pub fn read_raw(&mut self, addr: u16, out: &mut [u8]) -> Result<(), Error>;
    #[cfg(feature = "raw-registers")]
    pub fn write_raw(&mut self, addr: u16, data: &[u8]) -> Result<(), Error>;
}
```

`Field` is a sealed public marker trait. `PresentPosition` and `GoalPosition` are the only fields until OQ-2 is answered. `F::register(&ActuatorKind)`, `encode` and `decode` live on a supertrait that is `pub` inside a private module.

**Why `C = fn() -> u64`:**
- The default type is the same in every build, so a feature never changes it.
- An embedded user's free function, or a closure that captures nothing, coerces to it.
- `MockTime::ticker()` is a capturing closure; `C` is inferred for it.
- The `<P, T, N>` spelling from FR-1.1 still compiles.

**The protocol layer needs one change before the bus can build.** I compiled this check:

- `impl<P: Protocol + Exchange> Bus<P>` with today's `pub(crate) trait Exchange` fails under `-D warnings` with "trait `Exchange` is more private than the item `Bus<P>`".
- Without that bound, `P::read` does not resolve.
- What compiled cleanly under `-D warnings`: make `Exchange` a `pub trait` inside the private `sealed` module and a supertrait of `Protocol`. `Link`, `Pong` and `Status` must then be `pub`-in-private too.
- `Exchange` also gets two methods with a sequential default, which `P2` overrides:
  - `read_many`: P1 reads sequentially, P2 overrides with `sync_read`.
  - `scan`: P1 pings each ID 0..=252, P2 overrides with `broadcast_ping`.
- This is how "strategy picked by `P`" works without `dyn`.

## 2. Internal structure

```
src/bus.rs              Bus, Config, constructors, actuator(), ids(), Id->slot table
src/bus/actuator_ref.rs ActuatorRef, ModelRef
src/bus/setup.rs        declare, autoscan (via Exchange::scan), firmware pick, limits read
src/bus/fault.rs        observe(): the only path from a received status to the caller
src/bus/field.rs        sealed Field/ReadField/WriteField, PresentPosition, GoalPosition
src/bus/group.rs        heap-free (addr, size) grouping
src/bus/multi.rs        Easy + Explicit + partial
src/bus/id_set.rs       IdSet
src/bus/serial.rs       #[cfg(std)] SerialBus + open
src/bus/raw.rs          #[cfg(raw-registers)]
```

**Caches.** These need an xtask PR first, because the bus module is not allowed to edit `src/models*`.
- Each model type gets one generated field, `cache: Cache { torque: bool, fault: Option<StatusReport>, limits: (i32, i32) }`.
- Each model type also gets `pub const MODEL: ModelNumber` and `const LAYOUT: Layout { torque_enable, goal_position, present_position, min_position, max_position, hardware_error: Option<Register>, firmware: Register, scale: PositionScale }`.
- `ActuatorKind::{layout(), cache(), cache_mut()}` are generated `match` arms.
- `Cache` and `Layout` are hand-written in core.

**Fault handling.** Every exchange result goes through `fault::observe(kind, result)`. A `StatusError` with `is_hardware_fault()` sets `fault = Some(report)` and `torque = false`, then returns the error unchanged.

**Grouping** (heap-free):
1. **Pass 1, validate:** for every target, resolve `Id` to its slot. Unknown ID, a duplicate ID, torque off (`ActuatorIsOff{id}`, FR-4.8), a fault, or a value outside the limits fails the call before anything is sent. Each value is encoded into a stack `[[u8; 4]; N]` in target order.
2. **Collect keys:** the distinct `F::register(kind)` pairs go into a `heapless::Vec<Register, 8>`. Xtask asserts that no field has more than 8 distinct `(addr, size)` pairs.
3. **Send, one instruction per key:** a cloned target iterator is filtered by key and passed to `P::sync_write` or `P::read_many`. The read callback walks a lockstep iterator to find each output slot, because `sync_read` replies in request order.
4. **Strict mode** stops before the next group after the first error. Fault caches are still updated for every reply that arrived.

## 3. Example user code

```rust
// std desktop
use dxl_rs_v2::{prelude::*, P2, Position, SerialBus};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bus = SerialBus::<P2>::open("/dev/ttyUSB0", 1_000_000)?;
    for a in bus.autoscan()? {
        println!("found {} at id {}", a.name(), a.id().get());
    }
    bus.enable_torque(bus.ids())?;                    // two-phase borrow: compiles
    let mut pos = vec![Position::ZERO; bus.actuators().len()];
    loop {
        bus.read_positions(bus.ids(), &mut pos)?;     // ascending-ID order
        let goals: Vec<Position> = pos.iter().map(|p| *p + Position::from_radians(0.01)).collect();
        bus.write_goal_positions(bus.ids(), &goals)?;
    }
}
```

```rust
// no_std embedded (default-features = false, features = ["x-series"])
use dxl_rs_v2::{models::Xm430W350, Bus, Id, Position, P2};

fn now_us() -> u64 { MONO.now().ticks() }

fn run(uart: Uart) -> Result<(), dxl_rs_v2::Error> {
    let ids = [Id::new(1)?, Id::new(2)?];
    let mut bus: Bus<P2, _, 2> = Bus::new(uart, now_us);
    bus.declare(ids.map(|id| (id, Xm430W350::MODEL)))?;
    bus.enable_torque(ids)?;
    let mut pos = [Position::ZERO; 2];
    loop {
        bus.read_positions(ids, &mut pos)?;
        let goals = controller_step(&pos);
        bus.write_goal_positions(ids, &goals)?;
    }
}
```

## 4. Test strategy

Every test builds its bus as `Bus::new(MockTransport::new(&time), time.ticker(1))`, scripts replies with `reply_to(nth_flush, ..)`, checks the bytes sent with `assert_sent`, and ends with `assert_done()`.

- **Setup:**
  - P2 autoscan: three pongs, then their limit reads. Assert the variants, the IDs, and that the limits came from the reply bytes, not from table defaults.
  - XL330 at firmware 52 picks the `Fw52` type.
  - An unknown model returns `UnsupportedModel`.
  - `Bus<_, _, 1>` with two replies returns `CapacityExceeded`.
  - A limit read that times out fails Init and leaves `actuators()` empty.
  - P1 sweep: 253 pings; check how long a silent line takes on the ticker.
  - Declared setup: a missing ID returns `Timeout{id}`, a wrong model returns `ModelMismatch`, and `sent()` contains only the declared pings.
- **Guards:** a goal write with torque off returns `ActuatorIsOff` and `writes()` is unchanged. An out-of-limits goal returns a validation error and sends nothing.
- **Faults:** a reply with the alert bit set returns `Err(Status)`, after which `fault()` is `Some` and `is_torque_enabled()` is false. `enable_torque` then returns `ActuatorFaulted` and sends nothing. `clear_fault` clears when Hardware Error Status is 0 and keeps the fault otherwise. On P1, `clear_fault` pings and checks the status byte.
- **Multi:**
  - An XM430 (116/4) plus an XL320 (30/2) on P2 produces exactly two sync-write packets, byte-exact.
  - One target with torque off means zero writes in total.
  - P2 reads send one sync read per group. P1 reads send N unicast reads.
  - Strict mode with a silent actuator returns `Timeout`. Partial mode gives per-slot errors and the other slots filled.
- **NFR-2:** reuse `protocol/no_alloc.rs` to assert that the Cycle-path calls allocate nothing.
- **Compile-fail** with `trybuild` (std dev-dependency), `tests/ui/*.rs` plus `.stderr` snapshots:
  - `sync_read` on `Bus<P1, …>` (E0599, "method not found in `Bus<P1, …>`", confirmed in the probe);
  - two live `ActuatorRef`s (E0499);
  - an `ActuatorRef` outliving its bus;
  - `read_positions_partial` without the feature.
  - Snapshots pin the error code. Plain `compile_fail` doctests would pass on any error.

## 5. Risks and open questions

**Risks:**
- **R1. Protocol visibility** (FR-4.2, FR-5.5, FR-2.1): the change described in section 1 has to land in the protocol PR that is still uncommitted, before bus work starts.
- **R2. Position units are missing from the tables** (FR-3.6, FR-2.6, FR-7.2):
  - `xm430_w350.ron`'s `unit_info` has no position entry, while AX uses a different shape (`"Position"` in degrees).
  - AX uses `CW/CCW_ANGLE_LIMIT` where X-series has `MIN/MAX_POSITION_LIMIT`, and AX has no Hardware Error Status register.
  - Tables and xtask must add an e-Manual-verified scale and a name mapping.
- **R3. P1 autoscan takes about 2.5 s** (253 × 10 ms) (FR-2.1, FR-1.4). That is a boot-time cost, but a scan timeout separate from the reply timeout may be worth adding.
- **R4. `DynBus` cannot be a plain type alias** (FR-1.3): changing the storage type needs a storage type parameter on `Bus`. Defer to delivery step 10.
- **R5. Prerequisites outside this module** (no-go files, one-module rule):
  - `heapless` is not a dependency yet, and `Cargo.toml` belongs to the ci module.
  - The new `ValidationError` variants (`ActuatorIsOff`, `ActuatorFaulted`, `OutOfLimits`, `NotOnBus`, `DuplicateId`, `ModelMismatch`, `CapacityExceeded`) belong to core.

**Open questions for the architect:**
- **Q1 (FR-1.1).** Accept `C: Clock = fn() -> u64` as an additive 4th parameter on `Bus`?
- **Q2 (3.3, FR-2.6).** Put the cache in generated model types as one `cache: Cache` field plus a `Layout` const, with the AX↔X register-name mapping done in xtask?
- **Q3 (FR-4.5), overrule proposed.** Strict reads should take `&mut [Position]`, not `&mut [Option<Position>]`: on success every slot is `Some`, so `Option` forces an `unwrap` on every slot. Partial reads would take `&mut [Result<Position, Error>]`.
- **Q4 (FR-1.5), overrule proposed.** `actuator(id)` should return `Result<_, Error>` with a `NotOnBus` error, so callers can write `bus.actuator(id)?`. An `Option` needs `.ok_or(..)` at every call site.
- **Q5 (2.1).** Autoscan allocates nothing, so why limit it to std? I propose making it available in no_std as well.

## 6. Delivery split

Each step must pass the 70 % coverage gate.

| PR | Module | Scope | SRS step |
|---|---|---|---|
| 0a | protocol | `Exchange` made a sealed supertrait of `Protocol`; `read_many` and `scan` hooks | prerequisite |
| 0b | ci + core | `heapless` dependency; `Position`, `ModelNumber`, `Cache`, `Layout`, new `ValidationError` variants | 5 (types) |
| 0c | tables + xtask | position scale per table, `cache` field, `MODEL`, `LAYOUT`, `ActuatorKind` accessors | 3/7 |
| 1 | bus | `Bus`, `Config`, `actuator()`, `ActuatorRef` torque/goal/present, declared setup with limits, `IdSet` | 4 |
| 2 | bus | autoscan P1/P2, `SerialBus::open` | 4 |
| 3 | bus | `fault::observe`, `clear_fault`, `ActuatorFaulted`, NFR-2 no-alloc test, trybuild | 5 |
| 4 | bus | `Field`, `group.rs`, Easy tier plus `sync_write`/`sync_read` | 6 |
| 5 | bus | `partial-reads`, `raw-registers` | 9 |
| 6 | bus | `DynBus` after R4 is decided | 10 |

Probe crates used for the compile checks: /tmp/claude-1000/-home-ovsiankina-Projects-km-robota-projects-Architecture-dxl-rs-v2/9ebdc03f-eae0-4da7-9895-a9d6a776b477/scratchpad/probe