# Bus module: design proposal (draft, 2026-10-27)

Input for `/speckit-specify` of the `bus` module (spec 003). Three independent designs (DX first, embedded first, safety first) were written from the SRS and the existing code, then scored and merged by a judge agent. Nothing here is decided; the questions for the architect are at the end of each section and in "Contradictions with the SRS".

## Bus module: judgement and merged proposal

I re-read SRS §2.1, §3.3, FR-1 to FR-8, NFR-1 to NFR-4, §7 and §8, constitution v1.1.0, `src/protocol*`, `src/transport*`, `src/models*`, `src/error.rs` and `src/lib.rs`. A scratch probe compiles cleanly under `#![deny(warnings)]` and clippy. It checks three things:
- `Bus<P, T, const N = 32, C: Clock = fn() -> u64>` compiles. Both a `fn` item and a capturing closure are inferred as `C`.
- With a `pub` trait and a `pub Link` inside a private `sealed` module, made a supertrait of `Protocol`, `P::method` resolves with no `private_bounds` warning.
- `bus.enable(bus.ids())` compiles when `IdSet: Copy`.

The probe was a throwaway scratch crate; its three checks are listed above.

### 1. Scores

| Criterion | dx-first | embedded-first | safety-first |
|---|---|---|---|
| SRS / constitution | **7**. `Bus<P,T,N>` still compiles as written; `actuator()` keeps the Decided `Option`. But `as_model<M: Model>` uses a `Model` trait that does not exist. | **6**. `Bus<P,T,C,N>` breaks the FR-1.1 spelling and makes `C` mandatory. Autoscan is not std-gated (flagged honestly). | **5**. The `Setup` typestate goes against §7 and ARCHITECTURE (`bus.autoscan()`). Raw access running `observe` goes against FR-8.1. |
| DX | **9**. `IdSet`, `SerialBus::open`, two-phase borrow, short examples. | **6**. 3-arg constructor, `C` always written, `with_model(closure(&kind, &mut Port))`, `clear_fault -> Result<bool>`. | **5**. `SetupError<S>`. `ids()` borrows the bus, so the example needs `Vec` + `unwrap`. |
| Embedded fitness | **6**. A 253-byte slot table per bus. Autoscan is std-only. No scan timeout. | **9**. Grouping compiled once per `(P, T, C)` (no `N`), binary search, `const` config, `scan_timeout`, published timing formula, `size_of` test. | **7**. Transport is recoverable on a failed Setup; `Plan<N>` on the stack. Transactional buffers cost an extra `4N` bytes. |
| Safety | **7**. Guard, `observe`, enable refused while faulted. Retry policy unspecified; no protocol reachability check. | **7**. Protocol-support check, narrow retries, raises the faulted-actuator-in-strict-read question. | **9**. `TorqueState` enum, single `observe` path, failed torque write → `Off`, transactional reads, fault outranks timeout, proptest invariant. |
| Implementability | **8**. Probe-verified. Correctly finds the `Exchange` visibility blocker and the `read_many`/`scan` hooks. Invented `Model`. | **7**. Spots the duplicate `pub(crate) trait Transport` and that P1 ping carries no model number. `reboot()` has no exchange function in the protocol layer (only test vectors); `Port` has no methods. | **7**. `firmware_register(model)` is real and necessary. Invents `ModelNumber::XM430_W350`; typestate adds code. |
| **Total** | **37** | **35** | **33** |

### 2. Merged proposal (base: dx-first)

#### 2.1 Signatures

```rust
// src/bus.rs
pub struct Bus<P: Protocol, T: Transport, const N: usize = 32, C: Clock = fn() -> u64> {
    transport: T, clock: C, config: Config,
    actuators: heapless::Vec<ActuatorKind, N>,   // sorted by Id          [embedded]
    _p: PhantomData<P>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)] #[non_exhaustive]
pub struct Config { pub timeout: Duration, pub scan_timeout: Duration, pub retries: u8 } // 10ms/10ms/0
impl Config {                                                            // const builder [embedded]
    pub const fn new() -> Self;
    pub const fn timeout(self, t: Duration) -> Self;
    pub const fn scan_timeout(self, t: Duration) -> Self;
    pub const fn retries(self, n: u8) -> Self;
}
impl Default for Config { /* = new() */ }

impl<P: Protocol, T: Transport, const N: usize, C: Clock> Bus<P, T, N, C> {
    pub fn new(transport: T, clock: C, config: Config) -> Self;          // §7 + clock, no I/O
    pub fn into_parts(self) -> (T, C);
    pub fn declare<I: IntoIterator<Item = (Id, ModelNumber)>>(&mut self, list: I)
        -> Result<&[ActuatorKind], Error>;                                // replaces list; empty on Err
    #[cfg(feature = "std")]                                               // SRS 2.1; see Q2
    pub fn autoscan(&mut self) -> Result<&[ActuatorKind], Error>;
    pub fn actuators(&self) -> &[ActuatorKind];
    pub fn ids(&self) -> IdSet;                                           // [dx]
    pub fn actuator(&mut self, id: Id) -> Option<ActuatorRef<'_, P, T, C>>; // FR-1.5 as Decided

    // Easy tier: P picks the strategy (FR-4.2)
    pub fn enable_torque<I: IntoIterator<Item = Id>>(&mut self, ids: I) -> Result<(), Error>;  // unicast, acked [dx]
    pub fn disable_torque<I: IntoIterator<Item = Id>>(&mut self, ids: I) -> Result<(), Error>;
    pub fn write_goal_positions<I: IntoIterator<Item = Id>>(&mut self, ids: I, goals: &[Position]) -> Result<(), Error>;
    pub fn write_goal_positions_raw<I: IntoIterator<Item = Id>>(&mut self, ids: I, goals: &[i32]) -> Result<(), Error>; // [embedded/safety]
    pub fn read_positions<I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [Position]) -> Result<(), Error>;
    pub fn read_positions_raw<I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [i32]) -> Result<(), Error>;
    #[cfg(feature = "partial-reads")]
    pub fn read_positions_partial<I: IntoIterator<Item = Id>>(&mut self, ids: I,
        out: &mut [Result<Position, Error>]) -> Result<(), Error>;      // Error is Copy today

    // Explicit tier
    pub fn sync_write<F: WriteField, I: IntoIterator<Item = Id>>(&mut self, ids: I, values: &[F::Value]) -> Result<(), Error>;
}
impl<T: Transport, const N: usize, C: Clock> Bus<P2, T, N, C> {
    pub fn sync_read<F: ReadField, I: IntoIterator<Item = Id>>(&mut self, ids: I, out: &mut [F::Value]) -> Result<(), Error>;
}

#[cfg(feature = "std")]
pub type SerialBus<P, const N: usize = 32> = Bus<P, SerialTransport, N, StdClock>;
#[cfg(feature = "std")]
impl<P: Protocol, const N: usize> SerialBus<P, N> {
    pub fn open(path: &str, baud: u32) -> Result<Self, serialport::Error>;  // wraps SerialTransport::open [dx]
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct IdSet([u32; 8]);   // IntoIterator<Item = Id> ascending; From<[Id; K]>        [dx]

pub trait ReadField: sealed::Field { type Value: Copy; }   // PresentPosition
pub trait WriteField: sealed::Field { type Value: Copy; }  // GoalPosition. TorqueEnable deliberately NOT a field [judge]

// src/bus/actuator_ref.rs — no N in the type [embedded/safety]
pub struct ActuatorRef<'b, P: Protocol, T: Transport, C: Clock> { kind: &'b mut ActuatorKind, wire: Wire<'b, P, T, C> }
impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> {
    pub fn id(&self) -> Id;
    pub fn kind(&self) -> &ActuatorKind;                  // Tier 2 = match (FR-3.5)
    pub fn torque(&self) -> TorqueState;                  // Off | On | Faulted(StatusReport) [safety]
    pub fn is_torque_enabled(&self) -> bool;              // [dx] convenience
    pub fn fault_state(&self) -> Option<StatusReport>;    // §7 name
    pub fn position_limits(&self) -> RangeInclusive<Position>;
    pub fn position_limits_raw(&self) -> RangeInclusive<i32>;
    pub fn enable_torque(&mut self) -> Result<(), Error>;   // Faulted{id} while faulted, nothing sent
    pub fn disable_torque(&mut self) -> Result<(), Error>;  // always allowed
    pub fn set_goal_position(&mut self, goal: Position) -> Result<(), Error>;
    pub fn set_goal_position_raw(&mut self, ticks: i32) -> Result<(), Error>; // same guard + limits
    pub fn present_position(&mut self) -> Result<Position, Error>;
    pub fn present_position_raw(&mut self) -> Result<i32, Error>;
    pub fn goal_position(&mut self) -> Result<Position, Error>;  // explicit read-back FR-3.8 [dx]
    pub fn clear_fault(&mut self) -> Result<(), Error>;          // Ok = cleared; still faulted = Err(Status)
    #[cfg(feature = "raw-registers")] pub fn read_raw(&mut self, addr: u16, out: &mut [u8]) -> Result<(), Error>;
    #[cfg(feature = "raw-registers")] pub fn write_raw(&mut self, addr: u16, data: &[u8]) -> Result<(), Error>;
}
```

**Dropped:**
- dx `as_model` and embedded `with_model`. Tier 2 I/O waits on OQ-1 and OQ-2; `kind()` already meets FR-3.5.
- Safety's `Setup` typestate, because §7 is Decided.
- Embedded's `C`-before-`N` ordering.
- dx's 253-byte slot table, replaced by binary search on the sorted `Vec` (NFR-4 allows the index lookup).

**`clear_fault` returns `Result<(), Error>`, not `Result<bool>`.** A still-faulted actuator returns its `StatusError`, which carries the report. With `Ok(false)` that information would be easy to ignore.

**Generic `Bus::read<F>` / `write<F>` from dx: left out.** With only two fields (OQ-2), `read_positions`/`write_goal_positions` plus `sync_write<F>`/`sync_read<F>` already cover the surface.

#### 2.2 Internal structure

```
src/bus.rs              Bus, Config, IdSet, new/into_parts, actuator(), binary-search lookup
src/bus/wire.rs         Wire: builds protocol::Link, retry loop, ONLY caller of Exchange   [safety]
src/bus/cache.rs        TorqueState transitions, observe(), limit check                    [safety]
src/bus/actuator_ref.rs ActuatorRef
src/bus/setup.rs        declare, autoscan, init_one()
src/bus/field.rs        sealed Field markers -> Layout register + encode/decode
src/bus/group.rs        N-free core on slices; thin N-generic shell owns the stack arrays    [embedded]
src/bus/multi.rs        Easy + Explicit tiers
src/bus/serial.rs       #[cfg(std)] SerialBus::open
src/bus/partial.rs, src/bus/raw.rs   feature-gated
```

**Protocol prerequisite** (dx R1; the protocol module is still untracked on `002-transport`):
- Make `Exchange` a `pub` trait inside private `sealed`, as a supertrait of `Protocol`. `Link`, `Pong` and `Status` become `pub` there.
- Add two hooks with sequential defaults:
  - `read_many`: P2 overrides with `P2::sync_read`;
  - `scan`: P2 overrides with `broadcast_ping`. P1 sweeps 0..=252 using `scan_timeout`.
- Remove the duplicate `pub(crate) trait Transport` in `protocol.rs` in favour of `crate::Transport` [embedded].

**Cache:**
- xtask adds `cache: Cache` to every model and `pub(crate) const LAYOUT: Layout` [dx/embedded/safety].
- `Layout` holds:
  - `torque_enable`, `goal_position`, `present_position`;
  - `limits: MinMax(Register, Register) | CwCcw(Register, Register)`;
  - `hardware_error: Option<Register>`;
  - `scale: PositionScale`;
  - `protocols: ProtocolSet` [embedded].
- xtask also generates, on `ActuatorKind`: `layout()`, `cache()`, `cache_mut()` and `firmware_register(model) -> Option<Register>` [safety].
- `Cache { torque: TorqueState, min_raw: i32, max_raw: i32 }`: raw-tick limits [embedded], so a limit check is an integer compare.
- `ActuatorKind` stays the type the bus stores. A test pins its size with `size_of`.

**`observe`.** Every unicast result and every per-slot `sync_read` callback goes through `cache::observe(&mut Cache, &Result)`. When `Status(e)` has `e.is_hardware_fault()`, the state becomes `Faulted(e.report)`. If a torque write fails for any reason, the state becomes `Off` [safety]. Sync writes never reach `observe` (FR-6.3, documented).

**Retries.** Retries cover unicast only, and only `Timeout` or `Protocol` corruption: never `Status`, never sync writes [embedded/safety]. The worst-case formula is in the rustdoc [embedded].

**`init_one`:**
1. Ping. On P1, read Model Number, then `firmware_register(model)` [safety].
2. `ActuatorKind::new`.
3. Check that `P` is in `LAYOUT.protocols` and that every address fits `P` (P1 addresses are 8-bit); otherwise `Unreachable` [embedded + safety]. Principle II: fail at Init, not Cycle.
4. Read the limits (`CwCcw` maps to min/max); any failure fails Init.
5. Set `TorqueState::Off`.

On overflow, `CapacityExceeded`. The list is left empty on any error.

**Multi-actuator ops.** `group.rs`:
1. Resolve each target by binary search. A 32-byte bitset rejects duplicates [embedded].
2. Validate: buffer length, `On` state (FR-4.8 all-or-nothing), limits.
3. Encode into `[[u8; 4]; N]`.
4. Collect the distinct `(addr, size)` keys in first-appearance order. Emit one instruction per key, using a `Clone` filter iterator [all three].

**Strict reads are transactional.** Data is staged in `[[u8; 4]; N]` and copied to `out` only on `Ok`. When several slots fail, a hardware fault outranks a timeout in the returned error [safety].

#### 2.3 Example code

```rust
// std
use dxl_rs_v2::{P2, Position, SerialBus};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bus = SerialBus::<P2>::open("/dev/ttyUSB0", 1_000_000)?;
    for a in bus.autoscan()? { println!("{} at {}", a.name(), a.id().get()); }
    bus.enable_torque(bus.ids())?;
    let n = bus.actuators().len();
    let mut pos = [Position::ZERO; 32];
    loop {
        bus.read_positions(bus.ids(), &mut pos[..n])?;
        let goals = pos.map(|p| p + Position::from_radians(0.01));
        bus.write_goal_positions(bus.ids(), &goals[..n])?;
    }
}
```

```rust
// no_std, heap-free, nameable for RTIC/statics
const CFG: Config = Config::new().timeout(Duration::from_millis(2));
type Arm = Bus<P2, Uart1, 2>;                        // C defaults to fn() -> u64
fn now_us() -> u64 { MONO.now().ticks() }
fn init(uart: Uart1) -> Result<Arm, Error> {
    let ids = [Id::new(1)?, Id::new(2)?];
    let mut bus: Arm = Bus::new(uart, now_us, CFG);
    bus.declare(ids.map(|id| (id, ModelNumber(1020))))?;
    bus.enable_torque(ids)?;
    Ok(bus)
}
fn tick(bus: &mut Arm, ids: [Id; 2], out: &mut [i32; 2]) -> Result<(), Error> {
    bus.read_positions_raw(ids, out)
}
```

#### 2.4 Test strategy

All tests run with `MockTransport` and `MockTime::ticker`, using `reply_to(nth_flush, …)`, `assert_sent` and `assert_done`. Packets come from the `p1`/`p2` `packet()` helpers and `vectors.rs`.

- **Setup:**
  - P2: 3 pongs plus limit reads; a pong that arrives after `scan_timeout` is ignored.
  - P1 sweep: total time ≤ 253 × `scan_timeout`, followed by the model, firmware and limit reads.
  - Firmware variant pick (XL330 at firmware 52); `UnsupportedModel` with a feature hint.
  - `CapacityExceeded` at N = 1; declared missing / mismatch / duplicate.
  - A limit-read timeout fails Init and leaves the list empty.
  - A P2-only model on a P1 bus gives `Unreachable`.
  - `sent()` holds no Torque Enable write (FR-2.7).
- **Guard and limits.** A goal write while `Off`, out of limits, to an unknown ID, or with a short buffer returns the matching error with `flushes() == 0`.
- **Faults:**
  - An alert reply leads to `Faulted`; then `enable_torque` returns `Faulted` with no flush, and a goal write returns `ActuatorIsOff`.
  - `clear_fault` with Hardware Error Status 0, and with it non-zero. The P1 variant pings and checks the flags.
  - A torque-write timeout leaves the cache `Off`.
  - `retries = 1`: timeout then reply succeeds with 2 flushes; a status error gets 1 flush.
- **Multi:**
  - XM430 (116/4) + XL320 (30/2) produce two byte-exact sync writes.
  - One target `Off` means zero flushes.
  - In `sync_read`, slot 1 times out and slot 3 alerts: the call returns slot 3's fault, `out` is untouched, and slot 3's cache is `Faulted`.
  - On P1, strict reads go out as N unicast reads.
- **Proptest** [safety], already a dev-dependency: no goal frame is sent for a slot that was not `On`.
- **No allocation.** Wrap Cycle calls in the counting allocator from `protocol/no_alloc.rs` (NFR-2).
- **trybuild** with `.stderr` snapshots [all three]:
  - `sync_read` on P1 → E0599;
  - two live refs → E0499;
  - a ref that outlives the bus;
  - partial or raw methods without their feature.

#### 2.5 Risks and open questions

**Risks:**
- **R1.** The protocol prerequisite (2.2) has to merge first. The protocol module is still uncommitted.
- **R2.** No position scale in the tables. `xm430_w350.ron` `unit_info` has no position row, and AX uses `"Position"` in degrees. The tables PR needs an e-Manual lookup (Principle IV, NFR-9), as does the P1 Model Number/Firmware address per model.
- **R3.** Position limits may only apply in single-turn mode (unverified [safety]). Extended or wheel mode goals could be wrongly rejected. Check against the e-Manual before the limit check ships.
- **R4.** A faulted P2 actuator keeps the alert bit set until it is rebooted (to verify), so strict reads of its group keep failing. `reboot()` needs a protocol exchange; today only test vectors exist.
- **R5.** Multi-group writes are validated as a whole but are not atomic on the wire: a transport failure between groups leaves earlier groups sent [safety]. This needs documenting.
- **R6.** The P1 autoscan takes about 2.5 s at 10 ms per ID; `scan_timeout` mitigates it.

**Questions for the architect:**
- **Q1.** Accept a 4th parameter `C: Clock = fn() -> u64` after `N`, with `ActuatorRef<'_, P, T, C>`?
- **Q2.** Lift the "autoscan only with std" rule (§2.1)? It allocates nothing.
- **Q3.** Use `&mut [Position]` for strict reads, transactional, instead of `&mut [Option<_>]`?
- **Q4.** Use a `TorqueState` enum instead of `bool` plus a separate fault flag?
- **Q5.** Should raw access still run `observe`?
- **Q6.** An actuator that alerts at Init: fail Init (proposed) or instantiate it as `Faulted`?
- **Q7.** Add `reboot()` on `ActuatorRef<P2>` only?

#### 2.6 Delivery split (≥70 % coverage each)

| PR | Module | Scope |
|---|---|---|
| 0a | protocol | Sealed `Exchange` supertrait, `read_many`/`scan` hooks, `Transport` dedupe |
| 0b | ci | `heapless` dependency; `trybuild` dev-dependency |
| 0c | core | `Position`, `ModelNumber`, `TorqueState`, `Cache`, `Layout`, `ProtocolSet`, error variants |
| 0d | tables → xtask | Position scale rows (e-Manual), `cache` field, `LAYOUT`, accessors, `firmware_register` |
| 1 | bus (step 4) | `Bus`, `Config`, `wire`, `cache`, `declare` with limits and reachability, `ActuatorRef` torque/goal/present, `IdSet` |
| 2 | bus (step 4) | `autoscan` P1/P2, `SerialBus::open` |
| 3 | bus (step 5) | `observe` everywhere, `clear_fault`, `Faulted`, retries, trybuild borrow cases |
| 4 | bus (step 6) | `group.rs`, Easy + Explicit writes, proptest |
| 5 | bus (step 6) | Easy reads, `sync_read`, transactional buffers, no-alloc test, trybuild `sync_read`-on-P1 |
| 6 | bus (step 9) | `partial-reads`, `raw-registers` |
| 7 | bus (step 10) | `DynBus`: needs a storage parameter, since a plain alias cannot swap storage (dx R4) |

Spreading the bus module over PRs 1 to 7 needs the architect's approval under the one-module-per-PR rule.

### 3. Contradictions with the SRS

1. **FR-1.1, FR-1.5, §7, 4.1.** `Bus` gets a 4th parameter `C`, and `ActuatorRef` gains `C`. `new()` takes a clock as well as `transport, config`.
2. **§2.1 vs FR-2.1.** The SRS gates autoscan on std, which this proposal keeps for now (Q2). The two items are inconsistent: Default Setup is not available on no_std.
3. **FR-4.5 [decision].** Strict reads take `&mut [T]` and partial reads take `&mut [Result<T, Error>]`, not `&mut [Option<T>]`. Strict reads are also transactional, which is an addition.
4. **§3.3 "Torque state: runtime bool".** The proposal uses the `TorqueState` enum, which merges torque state and fault state.
5. **FR-6.5 [decision].** The decision is kept. Additions:
   - `enable_torque` is refused while faulted;
   - on P1/AX, which have no Hardware Error Status register, clearing works by ping plus flag check: a capability that is not specified;
   - on P2 the fault may need a reboot, which is not specified either (Q7).
6. **FR-8.1.** "Bypassing … cache" conflicts with FR-6.1 and FR-6.2 if raw replies still go through `observe` (Q5).
7. **FR-2.3, Principle II.** The protocol-reachability check and the P1 model and firmware reads at Init are not in the SRS. The P1 register addresses need e-Manual verification (FR-7.2).
8. **FR-2.7.** If an actuator alerts at Init, the proposal fails Init. FR-2.7 says to start with a clear fault "whatever the hardware reports" (Q6).
9. **FR-3.7.** Limits may depend on Operating Mode (R3). The SRS treats them as unconditional.
10. **FR-4.4, FR-4.8.** Multi-group writes are all-or-nothing in validation only, not on the wire (R5).
11. **FR-1.3, §8 step 10.** `DynBus` cannot be a plain alias of `Bus`. It needs a storage parameter or a separate type.
12. **OQ-1 / OQ-2.** Tier 2 I/O is deferred. Only `PresentPosition` and `GoalPosition` are fields, and `TorqueEnable` is left out of `WriteField` on purpose so that sync writes cannot bypass the cache.
13. **Module boundaries (§9, constitution workflow).** Bus work depends on four PRs in other modules first (protocol, ci, core, tables/xtask). Bus is also split into seven PRs, against "one module per PR, one PR per module".