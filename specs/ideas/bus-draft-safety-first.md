# Bus design draft: safety-first (2026-10-27)

One of three independent drafts merged into `bus-proposal.md`.

# Bus module design: safety-first proposal

## 0. Prerequisites the bus cannot provide itself

These are things the code does not have yet. The bus module may only touch `src/bus.rs` and `src/bus/**`.

- **Cache and register roles live in `src/models/**`, which the bus may not edit.** Model types hold only `id`. There is no torque, fault or limit state, and nothing tells the bus "goal position is at (116, 4)" without naming the model. This needs an xtask PR first, adding to every model:
  - a `cache: Cache` field;
  - `pub(crate) const ROLES: Roles` with `torque_enable`, `goal_position`, `present_position`, `position_limits: Limits2 { MinMax(Register, Register) | CwCcw(Register, Register) }`, `fault_register: Option<Register>`, `firmware: Register`, and a `PositionScale` taken from `type_info`;
  - on `ActuatorKind`: `roles(&self) -> &'static Roles`, `cache(&self)`, `cache_mut(&mut self)`, and `firmware_register(model: u16) -> Option<Register>`.

  The last one exists because Protocol 1 Ping returns no model or firmware, and Firmware Version sits at address 2 on AX/XL-320 but at 6 on X, MX(2.0) and PRO.
- **Core PR** (`src/error.rs`, `src/units.rs`): `Position`, `ModelNumber`, and new `ValidationError` variants: `UnknownId`, `DuplicateId`, `ActuatorIsOff{id}`, `Faulted{id}`, `OutOfLimits{id, raw, min, max}`, `BufferLength{expected, got}`, `TooManyActuators{capacity}`, `Missing{id}`, `ModelMismatch{id, declared, found}`, `Unreachable{id, addr}`.
- **Protocol fix:** public `Bus` impls are bounded on `P: Protocol`. The `pub(crate) Exchange` bound would trip `private_bounds` under `-D warnings`, so `Exchange` should move into the sealed module (the "pub trait in a private module" pattern).
- Add `heapless` to `Cargo.toml`.

## 1. Type signatures

```rust
// Clock: 4th parameter, appended after N. Deviates from FR-1.1's `Bus<P, T, N>`.
// The default is the same in every build, so no feature changes the type (FR-4.9).
// I checked this parameter order with rustc.
pub struct Bus<P: Protocol, T: Transport, const N: usize = 32, C: Clock = fn() -> u64> {
    transport: T, clock: C, config: Config,
    actuators: heapless::Vec<ActuatorKind, N>, _p: PhantomData<P>,
}
#[cfg(feature = "std")] pub type StdBus<P, const N: usize = 32> = Bus<P, SerialTransport, N, StdClock>;
#[cfg(feature = "std")] pub type DynBus<P, T, C> = /* alloc::Vec storage, FR-1.3 */;

#[derive(Debug, Clone, Copy)] #[non_exhaustive]
pub struct Config { pub timeout: Duration /* 10 ms */, pub retries: u8 /* 0 */ }
impl Config { pub const fn new() -> Self; pub const fn timeout(self, t: Duration) -> Self; pub const fn retries(self, n: u8) -> Self; }

// Setup typestate: a Bus exists only after a successful Setup.
// "Op before Init" and "re-scan mid-Cycle" cannot be written.
pub struct Setup<P, T, const N: usize = 32, C = fn() -> u64> { /* transport, clock, config */ }
pub struct SetupError<S> { pub error: Error, pub setup: S }   // gives the UART back on embedded
impl<S> From<SetupError<S>> for Error { /* so `?` works */ }
impl<P: Protocol, T: Transport, const N: usize, C: Clock> Setup<P, T, N, C> {
    pub fn new(transport: T, clock: C, config: Config) -> Self;           // no I/O
    pub fn declare<I: IntoIterator<Item = (Id, ModelNumber)>>(self, list: I) -> Result<Bus<P, T, N, C>, SetupError<Self>>;
    #[cfg(feature = "std")]                                               // SRS 2.1: "autoscan only when std"
    pub fn autoscan(self) -> Result<Bus<P, T, N, C>, SetupError<Self>>;
    pub fn into_inner(self) -> (T, C);
}

impl<P: Protocol, T: Transport, const N: usize, C: Clock> Bus<P, T, N, C> {
    pub fn actuator(&mut self, id: Id) -> Option<ActuatorRef<'_, P, T, C>>;
    pub fn ids(&self) -> impl Iterator<Item = Id> + '_;
    pub fn kinds(&self) -> &[ActuatorKind];
    // Easy tier: P picks the wire strategy
    pub fn write_goal_positions<I: IntoIterator<Item = Id>>(&mut self, ids: I, goals: &[Position]) -> Result<(), Error>;
    pub fn write_goal_positions_raw<I: IntoIterator<Item = Id>>(&mut self, ids: I, goals: &[i32]) -> Result<(), Error>;
    pub fn read_positions<I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [Position]) -> Result<(), Error>;
    pub fn read_positions_raw<I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [i32]) -> Result<(), Error>;
    #[cfg(feature = "partial-reads")]
    pub fn read_positions_partial<I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [Result<Position, Error>]) -> Result<(), Error>;
    // Explicit tier: sync write on P1 + P2
    pub fn sync_write<F: WriteField, I: IntoIterator<Item = Id>>(&mut self, ids: I, values: &[F::Value]) -> Result<(), Error>;
    pub fn into_inner(self) -> (T, C);
}
impl<T: Transport, const N: usize, C: Clock> Bus<P2, T, N, C> {   // absent on P1
    pub fn sync_read<F: ReadField, I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [F::Value]) -> Result<(), Error>;
}

// Sealed markers: GoalPosition, PresentPosition, later Goal/PresentVelocity, etc.
pub trait WriteField: sealed::Field { type Value: Copy; }
pub trait ReadField:  sealed::Field { type Value: Copy; }

// Split borrow: no N in the type. One live ref at a time (FR-3.1).
pub struct ActuatorRef<'b, P, T, C> {
    wire: Wire<'b, P, T, C>,
    slot: &'b mut ActuatorKind,
}
impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> {
    pub fn id(&self) -> Id;
    pub fn kind(&self) -> &ActuatorKind;                  // Tier 2: match on it
    pub fn torque(&self) -> TorqueState;                  // Off | On | Faulted(StatusReport)
    pub fn fault_state(&self) -> Option<StatusReport>;
    pub fn position_limits(&self) -> (i32, i32);
    pub fn enable_torque(&mut self) -> Result<(), Error>;  // Err(Faulted) while faulted
    pub fn disable_torque(&mut self) -> Result<(), Error>;
    pub fn set_goal_position(&mut self, goal: Position) -> Result<(), Error>;
    pub fn set_goal_position_raw(&mut self, ticks: i32) -> Result<(), Error>; // same guard and limits
    pub fn present_position(&mut self) -> Result<Position, Error>;
    pub fn present_position_raw(&mut self) -> Result<i32, Error>;
    pub fn clear_fault(&mut self) -> Result<(), Error>;
    #[cfg(feature = "raw-registers")] pub fn read_raw(&mut self, addr: u16, out: &mut [u8]) -> Result<(), Error>;
    #[cfg(feature = "raw-registers")] pub fn write_raw(&mut self, addr: u16, data: &[u8]) -> Result<(), Error>;
}
```

**Two [decision]s I overrule:**

1. **Strict-read buffer is `&mut [Position]`, not `&mut [Option<Position>]` (FR-4.5).** In a strict read, `Option` carries no information. Instead, strict reads become transactional: replies go into a stack `heapless::Vec<[u8; 4], N>` and are copied to `out` only on `Ok`. On `Err` the buffer is left untouched, so no half-stale data reaches the user.
2. **A faulted actuator refuses `enable_torque`.** This is an addition: `enable_torque` returns `Faulted{id}` until `clear_fault()` succeeds. It makes FR-6.5's "explicit clear" something you cannot skip.

## 2. Internal structure

```
src/bus.rs          Bus, Config, aliases, actuator(), into_inner
src/bus/setup.rs    Setup, SetupError, declare, autoscan, init_one()
src/bus/wire.rs     Wire: the ONLY path to Exchange; every reply goes through observe()
src/bus/cache.rs    TorqueState transitions, observe(), limit check
src/bus/actuator.rs ActuatorRef
src/bus/plan.rs     Plan<N>: resolve ids -> slots, validate, encode, group by (addr, size)
src/bus/multi.rs    Easy tier + sync_write; ReadMany strategy (P1 sequential, P2 sync_read)
src/bus/field.rs    sealed Field markers -> Roles lookup + encode/decode
src/bus/raw.rs      raw-registers
src/bus/tests.rs, src/bus/tests/*.rs
```

**Making the wrong states impossible:**

- **One torque/fault state.** The torque and fault cache is one enum, `TorqueState { Off, On, Faulted(StatusReport) }`, so "faulted but on" cannot exist. Transitions are private methods in `cache.rs`.
- **Fault check on every reply.** `Wire::{read, write}` take `&mut ActuatorKind` and call `observe(cache, &result)` before returning. When `Err(Status(e))` has `e.is_hardware_fault()`, the state becomes `Faulted(e.report)`. No code path can receive a status packet without passing through `observe`. For `sync_read`, the per-ID closure calls `observe` on that slot's cache.
- **Failed torque write counts as Off.** If an `enable_torque` or `disable_torque` write fails for any reason, the cache becomes `Off`, the conservative side for the goal guard.
- **Retries are narrow.** Only timeouts and protocol corruption are retried, never `Status`, and never sync writes.

**Grouping.** `Plan<N>` is a `heapless::Vec<(slot: u8, reg: Register, bytes: [u8; 4]), N>` on the stack. Building it is pass 1, and it does all of this before anything is sent:
1. resolve each `Id` (`UnknownId`, `DuplicateId`);
2. check the buffer length;
3. check torque `On` (`ActuatorIsOff{id}`);
4. check the EEPROM limits;
5. look up `roles().goal_position` and encode.

Only then does pass 2 run: collect the distinct `(addr, size)` keys (≤ 14 layouts) and make one `Exchange::sync_write` per key, using a cloneable filter iterator over the plan. This gives all-or-nothing validation (FR-4.8) with no allocation. The same structure can carry bulk read and write later.

**Init, per actuator:**
1. Ping. On P1, also read Model Number at (0, 2) and the firmware register (`firmware_register(model)`).
2. `ActuatorKind::new`.
3. **Reachability check:** every `Roles` address must fit `P` (P1 has 8-bit addresses). An address that does not fit fails Init with `Unreachable`, instead of failing at Cycle time.
4. Read the limits from EEPROM (AX maps CW/CCW to min/max). Any failure fails Init.
5. Set state to `Off`.

Running out of capacity gives `TooManyActuators`, never a panic.

## 3. Example user code

```rust
// std desktop
use dxl_rs_v2::{Setup, StdBus, StdClock, Config, P2, Position, SerialTransport, Id};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = SerialTransport::open("/dev/ttyUSB0", 1_000_000)?;
    let mut bus: StdBus<P2, 8> = Setup::new(port, StdClock::default(), Config::new()).autoscan()?;
    let ids: Vec<Id> = bus.ids().collect();
    for &id in &ids { bus.actuator(id).unwrap().enable_torque()?; }
    let mut now = [Position::ZERO; 8];
    loop {
        bus.read_positions(ids.iter().copied(), &mut now[..ids.len()])?;
        let goals: Vec<Position> = now[..ids.len()].iter().map(|p| *p + Position::from_rad(0.01)).collect();
        bus.write_goal_positions(ids.iter().copied(), &goals)?;   // Err(ActuatorIsOff) if any faulted
    }
}
```

```rust
// no_std embedded: declared setup, fn-pointer clock, no heap
const ARM: [(Id, ModelNumber); 3] = [(id!(1), ModelNumber::XM430_W350), (id!(2), ModelNumber::XM430_W350), (id!(3), ModelNumber::XL430_W250)];
fn now_us() -> u64 { TIMER.micros() }
let mut bus: Bus<P2, Uart1, 3> = match Setup::new(uart, now_us as fn() -> u64, Config::new()).declare(ARM) {
    Ok(bus) => bus,
    Err(e) => { log_and_halt(e.error, e.setup.into_inner()) }
};
for (id, _) in ARM { bus.actuator(id).ok_or(Error::from(ValidationError::UnknownId(id)))?.enable_torque()?; }
let mut pos = [0i32; 3];
loop {
    bus.sync_read::<PresentPosition, _>(ARM.iter().map(|a| a.0), &mut pos)?;
    bus.write_goal_positions_raw(ARM.iter().map(|a| a.0), &control(&pos))?;
}
```

## 4. Test strategy

Bus tests live in the crate (`src/bus/tests/`) and reuse the protocol layer's `packet()` builders and e-Manual vectors. They use `MockTransport` with `reply_to(nth_flush, …)` and a `MockTime::ticker`.

| Script | Assertion |
|---|---|
| Broadcast-ping 3 pongs + 6 limit replies | 3 kinds, all `Off`; `assert_sent` exact bytes; `assert_done` |
| P1 autoscan, silent except IDs 1 and 7 | 253 pings, then model, firmware and limit reads |
| Declared: missing, wrong model, `N+1` actuators, duplicate | `Missing` / `ModelMismatch` / `TooManyActuators` / `DuplicateId`; transport recoverable |
| Limit read times out | Init fails, no table-default fallback |
| `--no-default-features --features x-series,mock` + AX pong | `UnsupportedModel{feature: Some("ax")}` |
| Goal while `Off` / out of limits / unknown ID / short buffer | Matching error **and `line.flushes() == 0`** |
| Read reply with error byte `0x80`, then a goal write, then `enable_torque` | `Status` error; `Faulted`; `ActuatorIsOff`; `Faulted`; zero further flushes |
| `clear_fault`: Hardware Error Status 0, then non-zero | Cleared / still faulted |
| Multi-write, slot 3 of 4 `Off` | `ActuatorIsOff{3}`, nothing sent |
| XM430 + PH54 on P2 | Two sync-write frames, (116, 4) and (564, 4), byte-exact |
| `sync_read`: slot 1 times out, slot 3 alerts | Returns slot 3's hardware fault (fault outranks timeout); slot 3 cache `Faulted`; `out` untouched |
| `retries = 1`: timeout then reply; status error | `Ok` with 2 flushes; status error with 1 flush |
| Torque write times out | Cache `Off` |

On top of the table:
- **Proptest:** random op sequences and random error bytes, checking the invariant "no goal-position frame on the wire for a slot whose state was not `On` at call time".
- **Allocation check:** reuse `protocol/no_alloc.rs`'s counting allocator to assert zero allocations per Cycle op (NFR-2).

**Compile-fail tests: `trybuild`** with committed `.stderr` snapshots, so each test proves it fails for the right reason. Plain `compile_fail` doctests pass on any error at all. Cases:
- `sync_read` on `Bus<P1, …>` (E0599);
- two live `ActuatorRef`s (E0499);
- an `ActuatorRef` that outlives the bus (E0597);
- no public `Bus` constructor;
- `read_positions_partial` and `write_raw` without their features;
- a `const` `Id` of 253.

## 5. Risks and open questions

**Risks:**
1. **Position limits depend on Operating Mode** (FR-3.7, FR-2.6). I recall that X-series Min/Max Position Limit apply only in single-turn position mode; I have not checked this against the e-Manual. If true, extended-position goals would be wrongly rejected, and AX wheel mode (CW = CCW = 0) has the same problem. It must be verified per Principle IV.
2. **Multi-group writes are not atomic on the wire** (FR-4.4, FR-4.8). Validation is all-or-nothing, but if the transport fails between groups, group A has already gone out. Document it, and consider a `PartialSend` error.
3. **The protocol layer's `check()` drops the data when the alert bit is set.** A Protocol 2 reply with the alert bit still carries valid data, but `clear_fault` and Init lose it (FR-6.2, FR-6.5).
4. **Raw access vs. fault checks** (FR-8.1 vs. FR-6.1). I propose raw access skips the torque guard and limits but **still** runs `observe`.
5. **Sequencing:** the cache and roles block on the xtask PR, and the error variants and `Position` block on the core PR (delivery step 4).

**Questions for the architect:**
1. Is the 4th parameter `C` (default `fn() -> u64`) on `Bus` accepted?
2. Is the `Setup` typestate, instead of `bus.autoscan()` (§7), accepted?
3. Is autoscan really std-only (2.1 vs FR-2.1)? A heapless autoscan works on `no_std`.
4. An actuator that alerts at Init: fail Init (my default), or instantiate it as `Faulted`? Note that FR-2.7 says "clear whatever the hardware reports".
5. Are both overrules accepted: strict reads with `&mut [Position]` and transactional buffers, and enable refused while faulted?

## 6. Delivery split

Each PR stays at or above 70 % coverage, and each is tested with the mock only.

| PR | Contents | Prerequisite |
|---|---|---|
| 0a (xtask) | `Cache` field, `Roles`, `firmware_register`, scale | — |
| 0b (core) | `Position`, `ModelNumber`, new error variants | — |
| 1 | `bus.rs` + `wire.rs` + `cache.rs` + `setup.rs`, declared setup only; `Config`; Init limits and reachability check | 0a, 0b |
| 2 | `ActuatorRef`: torque guard, goal/present position (typed and raw), fault `observe`, `clear_fault`; trybuild for borrow cases | PR 1 |
| 3 | Autoscan, P1 and P2, firmware variants, `UnsupportedModel`; no_std CI on `declare` | PR 1 |
| 4 | `plan.rs` + Easy-tier writes + `sync_write<F>`; proptest guard invariant | PR 2 |
| 5 | Easy-tier reads (P1 sequential, P2 sync), `Bus<P2>::sync_read`, transactional buffers, no-alloc test, trybuild `sync_read`-on-P1 | PR 4 |
| 6 | `partial-reads` | PR 5 |
| 7 | `raw-registers` | PR 2 |
| 8 | `DynBus` | PR 1 |

PRs 1 to 5 cover delivery steps 4 to 6. PRs 6 to 8 are steps 9 and 10.