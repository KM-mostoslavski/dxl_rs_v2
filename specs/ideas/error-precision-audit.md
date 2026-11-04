# Error precision audit

2026-11-03, code at `integration` 9f3d523 (`src/error.rs`, `src/protocol*.rs`, `src/bus/setup.rs`).
Input for an SRS requirement the architect writes: every failure gets an explicit, precise
error. This file lists the cases where today's error is ambiguous; each needs a decision.

## Trigger

RX-28 temperature test, FTDI latency timer at 16 ms, bus timeout 10 ms. `autoscan()` returned
`Transport(Timeout { id: Some(Id(1)) })`, printed "timed out waiting for actuator 1". It does
not say which step failed (P1 ping sweep, model number read, limit read, mode read), the
timeout used, or whether any byte arrived. It took a source read of `src/transport/serial.rs`
to find the cause. Even now the exact setup step is not known from the error.

Second trigger (E13), 2026-11-03: 12 × AX-12A on a 12 V / 5 A supply, `examples/ax12_sine`.
`autoscan()` returned `Status(StatusError { id: Id(5), report: P1(P1Flags(1)) })`, failing on
a different ID each run (5, 5, 7) and once with a timeout on ID 1. Neither the error nor its
`Display` ("actuator 5 reported input voltage error") said whether the voltage was too low or
too high. Finding out took a hand-written raw Protocol 1 tool (`examples/ax12_voltage_fix.rs`
on `integration`, untracked). It read limits of 10.5–13.0 V and present voltages from 12.0 V
down to 9.7 V along the chain: too low, caused by voltage drop in the wiring. The architect's
first guess, and the agent's, was too high.

## Ambiguous cases

| # | Error today | Causes it merges or context it lacks |
| --- | --- | --- |
| E1 | `ProtocolError::Malformed` | Merges 4 causes: P1 length byte < 2 (`p1.rs:145`), P2 length < 3 (`p2.rs:258`), P2 length above the longest body for the instruction (`p2.rs:269`), scan reply from an ID outside 0..=252 (`p2.rs:372`). Carries no ID, no length, no instruction. |
| E2 | `TransportError::Timeout { id }` | Merges: no byte before the bus deadline; a partial packet then silence; the OS port's own 50 ms timeout (`io_error`, `protocol.rs:254`). No timeout value, elapsed time, bytes received, instruction or address. `id` is `None` in `drain` and broadcast waits. |
| E3 | `TransportError::Io(ErrorKind)` | `embedded_io::ErrorKind` is coarse (`Other` covers most OS failures); the OS code and message are dropped. No direction (read, write, read-ready, flush). End of stream is reported as `NotConnected` (`protocol.rs:249`), indistinguishable from a real `NotConnected`. |
| E4 | `ProtocolError::Checksum { id: u8 }`, `Crc { id: u8 }` | The ID comes from the corrupted packet itself. No expected and received value, no instruction. |
| E5 | `ProtocolError::WrongLength { expected, got }` | No ID, instruction or address. |
| E6 | `ValidationError::DoesNotFit`, `NotFinite`, `DataLength`, `PacketTooLarge`, `AddressOutOfRange`, `BufferLength`, `InvalidSlopeStep` | No actuator ID, register or call. E.g. "value 70000 does not fit a 2-byte register" without saying which register on which actuator. |
| E7 | `StatusError { id, report }` | No instruction or address that drew the report: a P2 `DataRange` or P1 `Range` flag does not say which register write was refused. |
| E8 | `SetupError::Missing { id }` | `setup.rs:66` maps every `Timeout` of the ping to "did not answer", including a reply that arrived late (today's latency case) or a partial one. The message can be false. |
| E9 | `SetupError::TooManyActuators { capacity }` | Does not say how many answered or which IDs. |
| E10 | No operation context anywhere | No error names the phase or call: autoscan sweep, Setup reading model number, limits or mode, a user read of a named register, sync read, torque write, reboot. Today's case. |
| E11 | Multi-actuator strict calls | Not verified: whether a failed sync read, bulk read or per-ID P1 loop says which target (index and ID) failed and how many slots were already filled. |
| E12 | Hints | Whether an error may carry a likely cause or fix ("FTDI latency timer above the timeout"). The crate never touches OS settings, but it can still say so. |
| E13 | `StatusError` with a P1 hardware flag (`InputVoltage`, `Overheating`, `Overload`) or a P2 alert | Says only which flag is set. It doesn't say what was measured, against which limit, or in which direction (voltage below Min or above Max Voltage Limit; temperature against Temperature Limit). The P1 error byte has no low/high bit, so a precise error needs a read of Present Voltage / Present Temperature and the matching EEPROM limits after the fault (on P2, Hardware Error Status plus the same registers). Open: does Setup and every call do that read automatically (extra traffic only on the fault path), or does a separate `diagnose_fault()` call do it? Also `Debug`, which `?` out of `main` prints, shows `P1Flags(1)`: raw bits, no flag name. |

Precise already (no action proposed): `InvalidId`, `ActuatorIsOff`, `ActuatorFaulted`,
`OutOfLimits`, `LimitsUnknown`, `InvertedLimits`, `DuplicateId`, `NotPositionMode`,
`NotVelocityMode`, `NotCurrentMode`, `Unsupported`, `NoUnit`, `UnsupportedInstruction`,
`UnsupportedModel`, `WrongModel`, `NotOnBus`, `UnknownOperatingMode`, `WrongId`,
`P2Code::Unknown(u8)`, unknown P1 bits.

## Constraints the requirement has to settle

- `Error` is `Copy`, `no_std`, heap-free (NFR-1, NFR-2): context must be fixed-size (IDs,
  `u16` address, `&'static str` register and instruction names, `u32` microseconds, byte
  counts). No `String`, no OS message text unless behind `std`.
- A larger `Error` makes every `Result` larger (performance is priority 2, 6.0).
- `#[non_exhaustive]` already allows new variants and fields without a semver break.
- Who tests it: one test per variant that checks the `Display` text names every field.

## Decisions (2026-11-04, branch `013-error-precision`)

Every case above now has its own variant. The common piece is
`Request { instruction, id, register, addr, len, step }`, carried by every
transport, protocol, status, fault and request-shaped validation error. The
protocol layer fills in the instruction, target, address and length. The
bus fills in the register name (`Register` now carries its table name, from
codegen) and the setup `Step`.

| # | Now |
| --- | --- |
| E1 | `Malformed` split: `LengthTooShort { protocol, length, min }` (P1 < 2, P2 < 3), `LengthTooLong { length, max }`, `InvalidReplyId { got }`; all with `request`. |
| E2 | `Timeout` split: `NoReply { timeout_us }`, `PartialReply { timeout_us, bytes }`, `LateReply { timeout_us, bytes }` (after a final silent attempt, the bus listens one more timeout; bytes then = late reply), `Skipped { answered }` (Sync/Bulk Read turn taken by a later ID), `PortTimeout { direction }` (the OS port's own timeout). All carry `request`; broadcast waits carry `id: None` *and* the instruction. |
| E3 | `Io { request, direction, kind }` (`Write`, `Flush`, `ReadReady`, `Read`); end of stream is its own `EndOfStream`. The OS code is not reachable through `embedded_io::Error` (kind only), not added. |
| E4 | `Checksum`/`Crc { request, packet_id, computed, received }`. |
| E5 | `WrongLength { request, expected, got }`. |
| E6 | `AddressOutOfRange`, `PacketTooLarge`, `DataLength`, `NotFinite`, `DoesNotFit`, `BufferLength`, `InvalidSlopeStep` carry `request` (actuator, register, instruction), e.g. "value 70000 does not fit a 2-byte register (in Write of Goal Position (address 30, 2 bytes) to actuator 3)". |
| E7 | `StatusError { id, report, request }`. P1 flags and P2 codes now say what they mean: "a checksum error: the instruction packet it received was corrupted on the line (check wiring, connectors, supply noise and the baud rate)". |
| E8 | `Missing { id, timeout_us }` only for `NoReply`; a late or partial ping reply stays its own transport error. |
| E9 | `TooManyActuators { capacity, ids, declared }`: every ID that answered or was declared. |
| E10 | `Step` in every request during Setup: `Scan`, `Ping`, `ModelNumber`, `Firmware`, `PositionLimits`, `OperatingMode`, `GoalBound`, `FaultDiagnosis`. |
| E11 | Per-target errors of Sync/Bulk Read carry that target's `request.id`, `addr`, `len`; multi-actuator instructions carry the register name. Slot counts not added: the ID names the target. |
| E12 | Hints in `Display` where the cause is known: power/wiring/baud/ID for silence, latency timer when the timeout is ≤ 16 ms, wiring/noise for a corrupted instruction, supply/wiring for low voltage. |
| E13 | Automatic measurement on the fault path only (no traffic otherwise): `Error::Fault(FaultError::Voltage(VoltageError::BelowMin { id, measured, min, max, faults, request }))`, `AboveMax`, `InsideLimits`; `TemperatureError::AboveLimit { measured, limit }`, `WithinLimit`; `Overload`, `ElectricalShock`, `Encoder`, `Unmeasurable { register }`, `Unidentified`. New roles `MinVoltageLimit`, `MaxVoltageLimit`, `TemperatureLimit` (with `Has*` traits). `Debug` of `Error` prints the sentence; `P1Flags` `Debug` names its flags. |

The SRS requirement text is still the architect's to write (docs/srs/SRS.md
is read-only for agents).
