# Typed limit setters: implementation plan

**Goal:** every EEPROM limit that most actuators share is read and written through typed calls on `ActuatorRef`, in SI units, with checks of their own, so a caller never needs `write_raw` for it. `raw-registers` stays for registers specific to one series (maintainer, 2026-11-09).

**Why now:** kmr widens an AX-12A's voltage limits for a run and restores them at the end (NFR-11, decided 2026-11-09: the caller saves and restores). Today that needs `write_raw` with no torque, range or order check, and a hand-made V → raw conversion (`examples/ax12_sine`).

**Spec:** maintainer's request of 2026-11-09 ("we need proper setters", "also implement the other limit setters"). It overrides FR-9 "Out of scope: writing EEPROM registers … still `write_raw`" and answers part of § 8 "Typed EEPROM writes" (`specs/ideas/eeprom-writes.md`); the SRS text is T096, the architect's. NFR-11 restore stays the caller's: no save/restore helper in the crate.

## Decisions (taken on the maintainer's order, recorded for T096)

- **D1. Scope: the limits most models share.**

  | Field | Registers (table names) | Value | Tables with it |
  |---|---|---|---|
  | `MinVoltageLimit`, `MaxVoltageLimit` | Min / Max Voltage Limit | `Voltage` (V) | 76 of 76 |
  | `TemperatureLimit` | Temperature Limit | `Temperature` (°C) | 64 |
  | `MinPositionLimit`, `MaxPositionLimit` | Min / Max Position Limit (P2), CW / CCW Angle Limit (P1) | `Position` (rad) | 76 |
  | `VelocityLimit` | Velocity Limit | `Velocity` (rad/s) | 66 |
  | `CurrentLimit` | Current Limit | `Current` (A) | 60 |
  | `PwmLimit` | PWM Limit | `Pwm` (new, a ratio 0..=1) | 66 |
  | `MaxTorque` | Max Torque (P1 EEPROM) | `Load` (a ratio of the maximum torque) | 10 |

  Left raw, specific to one or two series: Acceleration Limit (PRO, Y), Motor and Inverter Temperature Limit (Y), Position Limit Threshold, In-Position and Following Error Threshold (PRO). Not limits, left for later: Moving Threshold, Torque Limit (P1 RAM), ID, baud rate, Return Delay Time, Status Return Level.
- **D2. Writes are unicast only, through a trait of their own.** New sealed `pub trait LimitField: Field` (`type Value`), implemented by the D1 fields and by `Raw<F>` (`i32`). The D1 fields implement `ReadField` (so `read`, `read_field`, `sync_read`, `bulk_read` save them, for NFR-11) but not `WriteField`: `write`, `sync_write` and `bulk_write` refuse them at compile time (Principle II). A sync or bulk write gets no reply, so the bus could neither confirm the write nor keep its cache true (D4).
- **D3. API on `ActuatorRef`** (so on `Bus` and `DynBus` handles alike):
  - `write_limit<F: LimitField>(&mut self, field: F, value: F::Value) -> Result<(), Error>`;
  - pair setters `set_voltage_limits(min: Voltage, max: Voltage)` and `set_position_limits(min: Position, max: Position)`;
  - setters `set_temperature_limit(Temperature)`, `set_velocity_limit(Velocity)`, `set_current_limit(Current)`, `set_pwm_limit(Pwm)`, `set_max_torque(Load)`;
  - getters, one unicast read each, never cached: `voltage_limits() -> (Voltage, Voltage)`, `temperature_limit()`, `velocity_limit()`, `current_limit()`, `pwm_limit()`, `max_torque()`. Position limits read through `Actuator::position_limits` (the cache, kept true by D4) or `read(MinPositionLimit)`; no inherent `position_limits` so the trait method is not shadowed.
- **D4. Checks before a byte is sent, in this order:** the model has the register (`Unsupported`); torque is not `On` (new `ValidationError::ActuatorIsOn { id, register }`, hint "disable torque before writing EEPROM"); `Off` and `Faulted` both pass, so a faulted actuator held by Setup (D-01) can be fixed; the value converts (`NoUnit`, `NotFinite`, `DoesNotFit`); it lies in the e-Manual range (`OutOfLimits`): `unit_info` range, or the table's raw position range for position limits; pairs stay ordered (`InvertedLimits`): a pair setter checks its two values, a single position-limit write checks against the other cached limit, a single voltage-limit write cannot (the other limit is not cached and a read would be hidden, FR-9.6) and says so in its docs.
- **D5. The cache follows the write.** A write acknowledged without error updates what Setup cached: position limits (`Limits`), on Protocol 1 also the limit mode (`LimitMode::from_angle_limits`: CW = CCW = 0 is wheel mode), and the goal bound when the recorded mode uses Velocity or Current Limit. A write that fails after bytes went out leaves that cache unknown: every goal that depends on it is refused (`LimitsUnknown`) until the next Setup. Voltage, temperature, PWM and Max Torque are not cached.
- **D6. Units from the tables only (FR-9.2, Principle IV).** A model whose table has no `unit_info` for the register refuses the typed form with `NoUnit`; its raw form (`write_limit(Raw(F), raw)`) still works, with the torque check and the size check. T097 adds the missing `unit_info` rows to the six verified tables from their e-Manual pages. The other 70 tables get typed limits when they are verified, as every FR-9 register.
- **D7. `Pwm`** is a new newtype beside `Load` (`units`), same shape: a ratio, `from_ratio`, `ratio`, raw conversion through the model's scale (`unit: "%"`).

## Tasks

- [ ] T096 [human] Architect: SRS text for D1 to D7: a new FR-9 item for typed limit writes, the FR-9 "Out of scope" line on EEPROM writes, the § 8 "Typed EEPROM writes" question (limits answered; ID, baud and the rest still open), FR-6.4 for `ActuatorIsOn`.
- [x] T097 tables: `unit_info` for PWM Limit in `mx_64.ron` and `xm430_w350.ron`, and for any D1 register a verified table has without one, read online from the page its `unit_source` cites; `verified` date 2026-11-09. `check_tables.py` and its tests pass.
- [x] T098 xtask: roles `MinPositionLimit`, `MaxPositionLimit`, `VelocityLimit`, `CurrentLimit`, `PwmLimit`, `MaxTorque` (P1 CW/CCW Angle Limit as the position-limit roles there), a quantity for `Pwm`; regenerate `src/models.rs` and `src/models/`.
- [x] T099 core: `units::Pwm`; `ValidationError::ActuatorIsOn { id, register }` with its hint and the NFR-12 tests; the new roles in `src/actuator/roles.rs`.
- [x] T100 bus: the D1 fields, `LimitField`, `ActuatorRef::write_limit`, the D3 setters and getters, the D4 checks and the D5 cache updates, each with `MockTransport` tests (AX-12A on P1 and XM430-W350 on P2 at least): byte-exact packets, every refusal sends nothing, cache and limit mode after a write and after a failed write, a faulted actuator fixed through `set_voltage_limits` then `clear_fault`.
- [x] T101 bus: docs: the table in `field`, the `ActuatorRef` overview, the sentence "Nor is any EEPROM register … stay raw" (now: ID, baud rate and series-specific registers), rustdoc of every new item with its errors; step end (ROADMAP row, plan ticks, `docs/learnings.md`).

## Review focus

- No path writes EEPROM while torque is `On`, on any tier.
- The cache can never hold a limit the actuator does not have: after a failed write, goals that depend on it are refused.
- Protocol 1 angle-limit writes switch the limit mode exactly as Setup would have recorded it.
- Every unit and range comes from a table row backed by the e-Manual.
