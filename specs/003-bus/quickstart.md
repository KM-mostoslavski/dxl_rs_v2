# Quickstart: validating the bus module

**Feature**: [spec.md](spec.md) | **API**: [contracts/bus-api.md](contracts/bus-api.md) | **Model**: [data-model.md](data-model.md)

## Prerequisites

- Rust stable (latest; development uses 1.97.1) and the target:
  `rustup target add thumbv7em-none-eabihf`.
- `cargo-llvm-cov` for the coverage gate.
- Protocol PR #7 with module-interfaces §5 (`ProtocolVersion`, `Protocol::VERSION`,
  `Exchange::scan`, `Exchange::read_many`) is merged into `main` (2026-10-28).
- Merged before bus-1: core-1 (with `LimitMode`, `NotPositionMode`, `UnknownOperatingMode`),
  xtask X1 (regenerated `src/models*` with `ModelData::mode`), core-2.
- Optional, manual: a U2D2, one XM430-W350 at ID 1 (Protocol 2, 1 Mbaud) and one AX-12A
  (Protocol 1, 1 Mbaud). Lower the FTDI latency timer as in `specs/002-transport/quickstart.md`
  §4.

## 1. Gates every bus PR runs

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features mock,partial-reads,raw-registers -- -D warnings
cargo test --workspace
cargo test --features mock                          # runs tests/bus_no_alloc.rs
cargo test --features mock,partial-reads,raw-registers
cargo test --no-default-features --features std,mock,ax,mx,x-series,y-series   # family off: US2-4 (`pro`), from bus-2
cargo test --doc --all-features                     # doc examples, every feature on
cargo test --doc                                    # compile_fail examples (feature-off cases)
cargo test --doc --no-default-features --features alloc,ax,mx,x-series,y-series,pro,mock   # doctests with std off (ci's $NO_STD; + partial-reads,raw-registers from bus-5)
cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf -- -D warnings
cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro,partial-reads,raw-registers --target thumbv7em-none-eabihf -- -D warnings
# ci's feature-set legs (bus code must build on each, -D warnings)
RUSTFLAGS="-D warnings" cargo build --lib --no-default-features                     # no models: ActuatorKind uninhabited
RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features std
RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features partial-reads
RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features raw-registers
RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --target thumbv7em-none-eabihf
RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features partial-reads --target thumbv7em-none-eabihf
RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features raw-registers --target thumbv7em-none-eabihf
cargo xtask codegen && git diff --exit-code src/models.rs src/models/
cargo llvm-cov --workspace --features mock --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70
```

Expected: all pass.
- The two no_std lines prove `Bus`, `autoscan` and every Cycle path build with no `std` and no
  heap (FR-042, SC-005).
- `cargo xtask codegen` leaves no diff, because the bus edits no generated file.
- The feature lines exist from bus-5 on.
- The family-off line exists from bus-2 on. It turns `pro` off so the
  `#[cfg(not(feature = "pro"))]` test can script a PRO model number and expect
  `UnsupportedModel { feature: Some("pro") }` (US2-4).
- The three `cargo test --doc` lines mirror ci's compile-fail job. The std-off pass builds every
  doc example without `std`, so an example that uses a std-only item (`SerialBus`, `DynBus`,
  `SerialTransport`, `StdClock`) is gated with `#[cfg_attr(feature = "std", doc = …)]`. A
  `compile_fail` example that must fail only with a feature or `std` off is gated with
  `#[cfg_attr(not(feature = "…"), doc = …)]`.
- The `cargo build --lib` lines mirror ci's feature-set legs. With no model family on,
  `ActuatorKind` has no variants, so every bus path works on an uninhabited type and must still
  build without warnings (no unreachable-code or unused-item lint left over).
- PR #7's `expect(dead_code)` on `mod protocol;` and `P2::broadcast_ping`, which the bus never
  calls, are gone before bus-1: core-1 removes both (architect 2026-10-28; plan.md, deviation
  6). A bus PR edits only its own lines in `src/lib.rs`; anything wrong in `src/protocol*` is
  raised to the protocol owner.

## 2. Scenario checks (all in `cargo test`, no hardware)

Every row runs against `MockTransport` with `MockTime::ticker`. "Zero bytes" means
`line.sent()` is empty and `line.flushes() == 0` after the call.

| Spec item | Test lives in | Expected |
|---|---|---|
| US1-1, FR-014 | `src/bus/setup.rs` | Declared XM430 (P2) and AX-12A (P1, own bus): pings, P1 model read, limit reads, XM430 Operating Mode read (11, 1 byte; none on AX-12A), all byte-exact; `torque_states()` all `Off`; no Torque Enable write in `sent()` |
| FR-013, research R-9 | `setup.rs` | XM430 Operating Mode 3 → `Bounded`, 4 and 5 → `Unbounded`, 1 → `NoPosition`; AX-12A CW = CCW = 0 → `NoPosition` with no extra read; mode read timeout → `Err(Timeout)`, list empty |
| US1-13 | `setup.rs` | XM430 Operating Mode 2 (in none of its table's lists) → `SetupError::UnknownOperatingMode { id, value: 2 }`, list empty |
| US1-2 | `setup.rs` | Silent ID → `SetupError::Missing`; other model → `WrongModel`; `actuators()` empty afterwards |
| US1-2 (early) | `setup.rs` | Compiled-out or unknown model in a declared list → `UnsupportedModel`, zero bytes |
| US1-3 | `setup.rs` | Limit-read timeout → `Err(Timeout)`; list empty; no default used |
| US1-4 | `setup.rs` | 3 declared on `Bus<_, _, _, 2>` → `TooManyActuators { capacity: 2 }`, zero bytes |
| US1-5, US3-2 | `src/bus/actuator_ref.rs` | Goal while `Off` or `Faulted` → `ActuatorIsOff`, zero bytes |
| US1-6, US4-3 | `actuator_ref.rs` | Position mode, goal outside limits (raw, and radians that round outside) → `OutOfLimits`, zero bytes |
| US1-11 | `actuator_ref.rs` | XM430 in extended position mode (4), torque on: goal outside the EEPROM limits → one Write packet, byte-exact |
| US1-12 | `actuator_ref.rs` | XM430 in velocity mode (1), and AX-12A in wheel mode (CW = CCW = 0), torque on: goal → `NotPositionMode { id }`, zero bytes |
| US1-7, FR-022 | `actuator_ref.rs` | Goal inside limits → exactly one Write packet, byte-exact; nothing read after it |
| US1-8 | `src/bus.rs` | `actuator(unknown)` is `None` |
| US1-9, SC-006 | `compile_fail,E0499` example on `Bus::actuator` | Two live refs do not compile |
| US1-10, FR-005 | `bus.rs` | `new` then `into_parts` → `sent()` empty, transport and clock returned |
| US2-1 | `setup.rs` | P2 autoscan: one broadcast ping, three scripted pongs (out of order) plus limit and mode reads → three held, ascending |
| US2-2, SC-007 | `setup.rs` | P1 autoscan of an empty bus: 253 pings, `time.now() <= 253 × scan_timeout + 10 ms`; P2 empty: `<= scan_timeout + 1 ms` |
| US2-3 | `setup.rs` | XL330-M077 at firmware 52 → `Xl330M077Fw52`; at 53 → `Xl330M077` |
| US2-4, US2-6 | `setup.rs` | Model with family feature off (`#[cfg(not(feature = "pro"))]`, run by the §1 family-off line) / no table on this protocol → `UnsupportedModel` with `feature` / `protocol` set |
| US2 edge | `setup.rs` | Corrupted scan reply → `Err(Protocol)`; two pongs with one ID → `DuplicateId`; pong with alert → `Err(Status)`; list empty |
| US2 no_std | no_std clippy line of §1 | `autoscan` compiles with no `std` |
| US2-7 | `src/bus/serial.rs` (pty, `cfg(unix)`) | `SerialBus::<P2>::open(pty_path, 1_000_000)` returns a bus, nothing written |
| US3-1, SC-003 | `actuator_ref.rs`, `multi.rs` | Alert (P2) / overload flag (P1) on present read, multi-read slot, torque write, clear check → `Err(Status)` and slot `Faulted` on that call |
| US3-3 | `actuator_ref.rs` | `enable_torque` while `Faulted` → `ActuatorFaulted`, zero bytes |
| US3-4, US3-5 | `src/bus/observe.rs` | Hardware Error Status 0 → `Off`; alert reply → error returned, still `Faulted`; P1 ping clean / with overheating flag |
| US3-6, FR-027 | `observe.rs` | Torque enable timeout or bad CRC → state `Off` |
| FR-028, plan deviation 5 | `actuator_ref.rs` | `Faulted` slot, `disable_torque` times out → state stays `Faulted`; then `enable_torque` → `ActuatorFaulted`, zero bytes |
| SC-008 | `src/bus/wire.rs` | `retries = 1`: timeout then reply → `Ok`, 2 flushes; status error → 1 flush; silent line → `time.now() <= 2 × timeout + 1 ms` |
| US4-1, US4-2, US4-4 | `actuator_ref.rs` | XM430 and AX-12A: radians ↔ register value on the wire; limits typed and raw agree |
| US5-1, US5-2, SC-004 | `src/bus/multi.rs` | XM430 (116/4) + XL320 (30/2) → exactly two Sync Write packets, byte-exact, no read |
| US5-3, US5-4, SC-002 | `multi.rs` | One target `Off`, unknown ID, repeated ID, short value slice → matching error, zero bytes |
| US5-5 | `multi.rs` | `sync_write(GoalPosition, …)` on `Bus<P1, …>` and `Bus<P2, …>` |
| US5-6 | `multi.rs` | One target in velocity mode → `NotPositionMode` naming it, zero bytes; an extended-mode target outside its EEPROM limits passes validation |
| proptest | `multi.rs` | For random states and goals, no goal byte is sent for a slot not `On` |
| US6-1, US6-2 | `multi.rs` | P2: one Sync Read, buffer in target order; P1: three Read packets |
| US6-3, US6-4 | `multi.rs` | Slot 1 times out, slot 3 alerts → returns slot 3's fault, slot 3 `Faulted`, buffer unchanged |
| US6-5 | `multi.rs` | `out` shorter than targets → `BufferLength`, zero bytes |
| US6-6, SC-006 | `compile_fail,E0599` example on `sync_read` | `sync_read` on `Bus<P1, …>` does not compile |
| SC-005 | `tests/bus_no_alloc.rs` | Zero allocations across every Cycle call on `Bus` and `DynBus` |
| US7-1, US8-1 | `compile_fail` examples, built only with the feature off | `read_positions_partial`, `read_raw`, `write_raw` absent |
| US7-2, US7-3 | `src/bus/partial.rs` | P1 silent slot → that slot `Err(Timeout)`, others `Ok`; P2 transport failure → every slot of the group `Err` |
| US7-4, FR-4.7 (decided 2026-10-28) | `partial.rs` | P2 sync read, one ID silent → that slot `Err(Timeout)`, others `Ok` |
| US8-2, US8-3 | `src/bus/raw.rs` | Raw write while `Off` → sent byte-exact, state still `Off`; raw write to Torque Enable → state unchanged; raw alert reply → `Err(Status)` and slot `Faulted` |
| US9-1 | `src/bus/dyn_bus.rs` | `DynBus` autoscan finds 40 actuators (more than 32), all held |
| US9-2 | `compile_fail` example, built without `std` | `DynBus` absent |

## 3. The five-line desktop path (SC-001)

The rustdoc example on `Bus`, compiled as a doctest (`no_run`):

```text
const ID = Id 1
1  open SerialBus::<P2> on "/dev/ttyUSB0" at 1 Mbaud
2  autoscan
3  bus.enable_torque([ID])
4  bus.actuator(ID) → set_goal_position(0.5 rad)
5  bus.actuator(ID) → present_position()
```

Expected: five statements after the `ID` constant, each ending in `?`. The doctest builds. It
uses `SerialBus`, so it is gated with `#[cfg_attr(feature = "std", doc = …)]`. It needs
`SerialBus` and `autoscan` (bus-2) and `enable_torque(ids)` (bus-3), so it lands in bus-4,
which waits for both (plan, PR split).

## 4. Manual hardware check (optional)

1. XM430-W350 on P2:
   - `SerialBus::<P2>::open("/dev/ttyUSB0", 1_000_000)?`, then `autoscan()`. It lists
     `XM430-W350` at ID 1 with limits equal to registers 48/52.
   - `set_goal_position` before `enable_torque` returns `ActuatorIsOff` and the horn does not
     move.
   - After `enable_torque`, a goal of 0.5 rad moves it. `present_position()` reads about 0.5 rad.
   - With Operating Mode set to 4 (extended position) and a new `autoscan`, a goal past
     register 48 moves the horn. With mode 1 (velocity), a goal returns `NotPositionMode`.
2. Block the horn until the overload alert:
   - the next `present_position()` returns `Err(Status)` and `torque_state()` is `Faulted`;
   - `clear_fault()` keeps failing until the actuator is power-cycled. That is e-Manual
     behaviour: reboot is the only recovery.
3. AX-12A on P1: the same steps, with `autoscan` taking about 2.5 s at the default scan wait.
   Then repeat with `Config::new().scan_timeout(2 ms)`.
