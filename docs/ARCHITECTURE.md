# Architecture

HAL for Dynamixel actuators (Protocol 1 & 2). No robot model, no async.

## Layers

```
API         Bus<P, T, N>  ──owns──▶  heapless::Vec<ActuatorKind, N>
             │  bus.actuator(id)?.set_goal_position(..)   (short-lived handle)
             │  bus.write_goal_positions(..)              (easy: strategy picked by P)
             │  bus.sync_write_goal_positions(..)         (explicit: P1 + P2)
             │  bus.sync_read(..)                         (explicit: P2 only)
Models      Xm430W350, Mx64, …   ◀── generated from control_tables/*.ron (const data)
Protocol    P1 | P2              packet encode/decode, checksum/CRC
Transport   T: embedded_io::{Read, Write}   raw bytes, timeouts (serialport by default)
```

## Decisions

- **AoS**: one struct per actuator, each tied to its control table.
- **Control tables are data, not a layer**: generated into `const` registers per model (see Codegen).
- **Protocol = generic on `Bus`**: fixed at compile time, shared by every actuator on a wire.
- **Transport = generic on `Bus`** (default type): mockable for tests, embedded-ready.
- **Torque state = runtime field** on the actuator, not typestate. Branch cost is negligible vs. bus I/O and keeps `ActuatorKind` free of `<S>`.
- **Heterogeneous storage = `ActuatorKind` enum** (closed set, generated): heap-free, `no_std`, static dispatch; `match` recovers the concrete model.
- **Two API tiers**: generic path through `ActuatorKind` / `Actuator` trait; concrete model types for model-specific methods.
- **Bus owns actuators**: no `Rc`/`RefCell`/stored lifetimes; access through a borrowed handle.
- **Bulk ops on `Bus`**: sync/bulk read/write grouped internally by `(addr, size)`.
- **Typed values**: newtypes (e.g. `Position`) carry units and ranges.
- **`no_std` + heap-free** is a requirement.
- **User picks the abstraction level** for multi-actuator ops:
  - *Easy*: `bus.read_positions(ids)` / `bus.write_goal_positions(..)` on every `Bus<P, T>`. The strategy (sync/bulk vs. sequential, `(addr, size)` grouping) is picked by `P` at compile time. Writes: sync write on P1 and P2. Reads: sync read on P2, sequential on P1.
  - *Explicit*: `bus.sync_write(..)` (P1 + P2), `bus.sync_read(..)` (`impl Bus<P2, T>` only). Unsupported on a protocol = method absent = compile error.
- **IDs**: user-facing API uses the firmware ID as a newtype `Id` (0..=252; 253/254 reserved). Array index stays internal.
- **Multi-actuator targets**: `impl IntoIterator<Item = Id>`. Covers ranges and sparse lists alike.
- **Control table location**: the `ActuatorKind` variant *is* the model. Registers are associated `const`s on the model type, so no pointer and no copy.
- **Actuator cache**: `Id`, torque state, and the real limits read from EEPROM.
- **Construction**: `bus.autoscan()` is the default path and instantiates what it finds. Declaring (`Id`, model) by hand is for when you want to check the robot matches what you expect (missing or wrong model = error at startup, and no full ID sweep).
- **Limits**: read from EEPROM at init. Never silently fall back to table defaults.
- **Torque off by default**. Enabling it is always an explicit call. Goal writes while off return `ActuatorIsOff` (no model-specific side effects such as AX auto-enabling torque).
- **Capacity**: `Bus<P, T, const N: usize = 32>` stores `heapless::Vec<ActuatorKind, N>`. `N` is set per bus (e.g. `Bus<P2, _, { robot::ARM_MOTORS }>`) and is an upper bound, not an exact count; going over it at scan time is an error. Default `std` feature enables `alloc` (`Vec`, no cap) and `serialport`; no_std users set `default-features = false`.
- **Units**: SI newtypes where they make sense, raw access always available.
- **Errors**: `thiserror` (no_std), split into transport / protocol / status packet / validation.
- **Transport**: `embedded-io` blocking `Read + Write`; `serialport` through the std adapter by default.
- **Timeouts / retries**: configurable on the bus, with defaults.
- **Bulk read output**: caller-owned `&mut` buffer, reused across cycles. No allocation, no per-cycle storage.
- **Bulk read failure**:
  - `read_*(ids, &mut buf) -> Result<(), Error>`: strict, like the SDK. Any failure fails the call.
  - `read_*_partial(ids, &mut buf)`: per-slot result, reports which actuators failed. Only exists behind the opt-in `partial-reads` feature ("use at your own risk"). On P2 a failed sync group fails all its slots.
  - Features should add API rather than change an existing method's behaviour. Not binding during alpha: breaking changes are allowed until 1.0.
- **Codegen**: a generator (`cargo xtask`) turns `control_tables/*.ron` into plain `.rs` files that are committed. No `build.rs`, no proc macro: zero codegen at build time, readable code, and table changes show up in diffs.
- **Model families are Cargo features** (`ax`, `mx`, `x-series`, …; all on by default) to cut compile time and size. Autoscan finding a compiled-out model returns `UnsupportedModel { model_number, feature }`, so a missing feature fails loudly.
- **Status replies (same as SDK / v1)**: RX is cleared before every TX. Unicast instructions wait for their status packet (factory default Status Return Level 2). Broadcast instructions (sync write) never wait. No Status Return Level tracking.
- **Write confirmation**: never implicit. Sync writes are fire-and-forget; read-back verification is a separate call the user makes when, and if, they want it.

- **Raw register access**: allowed, but bypasses caches and checks. The caller owns the consequences.
- **Autoscan stays the default** (boot-time cost only). P2 uses broadcast ping (one round-trip); P1 pings IDs one by one. Scans the bus's configured baud only.

- **Fault detection**: every received status packet's error byte is checked (P2 alert bit 7; P1 per-bit errors). A set fault marks the actuator faulted, torque cache = off, and returns a typed hardware error. Free (data is already parsed). Limit: sync writes get no reply, so write-only loops only see faults on their next read.

## Not implemented yet

- **MX on Protocol 1 firmware** (e.g. MX-64 = model 310): required, but the 1.0 tables are missing from `control_tables/`. Autoscan returns `UnsupportedModel` until they're added.
