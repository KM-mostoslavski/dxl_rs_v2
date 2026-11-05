//! Setup: find the actuators, read what the bus must know about each, and
//! hold them with torque off (research R-6, R-9).

use crate::actuator::{Describe, LimitMode, ModelNumber, TorqueState};
use crate::error::{
    Error, Instruction, Request, SetupError, StatusError, StatusReport, Step, TransportError,
    ValidationError,
};
use crate::models::{self, ActuatorKind};
use crate::protocol::{Clock, Pong, Protocol};
use crate::register::Register;
use crate::{Id, IdSet, Transport};

use super::diagnose::diagnose_unknown;
use super::field;
use super::slots::Slots;
use super::wire::Wire;

/// What a successful Setup holds: returned by `declare` and `autoscan` on
/// [`Bus`](crate::Bus) and `DynBus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SetupReport<'a> {
    /// The held actuators, ascending by `Id`, as
    /// [`Bus::actuators`](crate::Bus::actuators) returns them.
    pub actuators: &'a [ActuatorKind],
    /// The held actuators whose Setup replies reported only a hardware
    /// fault: held [`Faulted`](crate::TorqueState::Faulted), so torque and
    /// goals are refused until
    /// [`clear_fault`](crate::ActuatorRef::clear_fault), which also measures
    /// and returns the fault. Empty when no actuator reported one.
    pub faulted: IdSet,
}

/// Declared Setup: pings only the listed actuators.
///
/// Replaces what `slots` held. The list is checked before any byte: more
/// than `N` entries, a repeated `Id` or a model this build cannot drive on
/// `P` fail at once. On any error `slots` is left empty. Returns the
/// actuators held `Faulted`.
pub(crate) fn declare<P, T, C, S, I, const N: usize>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    list: I,
) -> Result<IdSet, Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
    I: IntoIterator<Item = (Id, ModelNumber)>,
{
    slots.clear();
    let result = declare_all::<P, T, C, S, I, N>(wire, slots, list);
    wire.set_context(Request::NONE);
    if result.is_err() {
        slots.clear();
    }
    result
}

fn declare_all<P, T, C, S, I, const N: usize>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    list: I,
) -> Result<IdSet, Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
    I: IntoIterator<Item = (Id, ModelNumber)>,
{
    let mut entries = heapless::Vec::<(Id, ModelNumber), N>::new();
    let mut ids = IdSet::new();
    let mut overflow = false;
    for (id, model) in list {
        if !ids.insert(id) {
            return Err(ValidationError::DuplicateId { id }.into());
        }
        overflow |= entries.push((id, model)).is_err();
    }
    if overflow {
        return Err(SetupError::TooManyActuators {
            capacity: N,
            ids,
            declared: true,
        }
        .into());
    }
    entries.sort_unstable_by_key(|&(id, _)| id);
    for &(id, model) in &entries {
        ActuatorKind::new(P::VERSION, model, 0, id).map(drop)?;
    }
    let mut held_faulted = IdSet::new();
    for &(id, declared) in &entries {
        wire.set_context(Request::NONE.step(Step::Ping));
        let mut faulted = false;
        // DECISION: a ping reporting only a hardware fault is an answer; on
        // Protocol 2 its model number and firmware are lost with the error,
        // so Model Number is read as on Protocol 1, and firmware is read
        // only where the model's table depends on it.
        let (model_number, firmware) = match wire.ping(id) {
            Err(Error::Transport(TransportError::NoReply { timeout_us, .. })) => {
                return Err(SetupError::Missing { id, timeout_us }.into());
            }
            Err(Error::Status(status)) if hardware_only(&status) => {
                faulted = true;
                (None, None)
            }
            Err(err) => return Err(diagnose_unknown(wire, id, Some(declared.get()), err)),
            Ok(pong) => (pong.model_number, pong.firmware),
        };
        let found = match model_number {
            Some(number) => number,
            None => read_model_number(wire, id, &mut faulted)
                .map_err(|err| diagnose_unknown(wire, id, Some(declared.get()), err))?,
        };
        if found != declared.get() {
            return Err(SetupError::WrongModel {
                id,
                declared: declared.get(),
                found,
            }
            .into());
        }
        if init_one(wire, slots, id, declared, firmware, faulted)? {
            held_faulted.insert(id);
        }
    }
    Ok(held_faulted)
}

/// Setup by scanning: a P2 broadcast ping, or a P1 sweep of IDs 0..=252.
///
/// Replaces what `slots` held. Every reply is collected first; a corrupted
/// reply, a reply reporting an error other than only a hardware fault, two
/// replies with one `Id`, or more than `N` replies fail Setup before
/// anything else is sent. The replies are
/// then set up in `Id` order like declared actuators. On any error `slots`
/// is left empty. Returns the actuators held `Faulted`.
pub(crate) fn autoscan<P, T, C, S, const N: usize>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
) -> Result<IdSet, Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
{
    slots.clear();
    let result = autoscan_all::<P, T, C, S, N>(wire, slots);
    wire.set_context(Request::NONE);
    if result.is_err() {
        slots.clear();
    }
    result
}

fn autoscan_all<P, T, C, S, const N: usize>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
) -> Result<IdSet, Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
{
    let mut pongs = heapless::Vec::<Pong, N>::new();
    let mut answered = IdSet::new();
    let mut failure = None;
    wire.set_context(Request::NONE.step(Step::Scan));
    wire.scan(|reply| {
        if let Ok(pong) = &reply {
            answered.insert(pong.id);
        }
        if failure.is_none() {
            failure = collect::<N>(&mut pongs, reply).err();
        }
    })?;
    match failure {
        Some(Error::Setup(SetupError::TooManyActuators { capacity, .. })) => {
            return Err(SetupError::TooManyActuators {
                capacity,
                ids: answered,
                declared: false,
            }
            .into());
        }
        Some(err @ Error::Status(StatusError { id, .. })) => {
            let model = pongs
                .iter()
                .find(|pong| pong.id == id)
                .and_then(|pong| pong.model_number);
            return Err(diagnose_unknown(wire, id, model, err));
        }
        Some(err) => return Err(err),
        None => {}
    }
    pongs.sort_unstable_by_key(|pong| pong.id);
    let mut held_faulted = IdSet::new();
    for pong in &pongs {
        let mut faulted = pong.fault.is_some();
        let model = match pong.model_number {
            Some(number) => number,
            None => read_model_number(wire, pong.id, &mut faulted)
                .map_err(|err| diagnose_unknown(wire, pong.id, None, err))?,
        };
        let model = ModelNumber(model);
        if init_one(wire, slots, pong.id, model, pong.firmware, faulted)? {
            held_faulted.insert(pong.id);
        }
    }
    Ok(held_faulted)
}

/// Adds one scan reply to `pongs`, or says why Setup fails. A reply that
/// reports only a hardware fault is added like a clean one.
fn collect<const N: usize>(
    pongs: &mut heapless::Vec<Pong, N>,
    reply: Result<Pong, Error>,
) -> Result<(), Error> {
    let pong = reply?;
    let id = pong.id;
    if let Some(report) = pong.fault {
        let request = Request::NONE
            .instruction(Instruction::Ping)
            .id(id)
            .step(Step::Scan);
        let status = StatusError {
            id,
            report,
            request,
        };
        if !hardware_only(&status) {
            let _ = pongs.push(pong);
            return Err(status.into());
        }
    }
    if pongs.iter().any(|held| held.id == id) {
        return Err(ValidationError::DuplicateId { id }.into());
    }
    pongs.push(pong).map_err(|_| {
        SetupError::TooManyActuators {
            capacity: N,
            ids: IdSet::new(),
            declared: false,
        }
        .into()
    })
}

/// Reads Model Number, which every table has at the same place. Sets
/// `faulted` if the reply reports only a hardware fault.
fn read_model_number<P: Protocol, T: Transport, C: Clock>(
    wire: &mut Wire<'_, P, T, C>,
    id: Id,
    faulted: &mut bool,
) -> Result<u16, Error> {
    wire.set_context(Request::NONE.step(Step::ModelNumber));
    let raw = read_value(wire, id, models::MODEL_NUMBER, faulted)?;
    Ok(u16::try_from(raw).unwrap_or(u16::MAX))
}

/// Whether `status` reports a hardware fault and nothing else: on
/// Protocol 1 only Input Voltage, Overheating or Overload, on Protocol 2
/// the alert bit with no error code. The instruction then ran, and the
/// reply's data is the register's value.
fn hardware_only(status: &StatusError) -> bool {
    match status.report {
        StatusReport::P1(flags) => flags.0 != 0 && flags.0 == flags.hardware().raw(),
        StatusReport::P2 { alert, code } => alert && code.is_none(),
    }
}

/// `result` of a Setup exchange, with a reply that reports only a hardware
/// fault taken as an answer: `faulted` is set and the data it carried is
/// kept. Any other result as it is.
fn tolerate(result: Result<(), Error>, faulted: &mut bool) -> Result<(), Error> {
    match result {
        // DECISION: the data is trusted as diagnose.rs trusts it (decided
        // 2026-11-04). `protocol::check` zero-fills a faulted reply of the
        // wrong length, which cannot be told apart here; flagged in the PR.
        Err(Error::Status(status)) if hardware_only(&status) => {
            *faulted = true;
            Ok(())
        }
        other => other,
    }
}

/// As [`Wire::read_value`], through [`tolerate`].
fn read_value<P: Protocol, T: Transport, C: Clock>(
    wire: &mut Wire<'_, P, T, C>,
    id: Id,
    reg: Register,
    faulted: &mut bool,
) -> Result<i32, Error> {
    let mut buf = [0; 4];
    let request = wire
        .context()
        .filled(reg.request().instruction(Instruction::Read).id(id));
    let bytes = reg.buf(&mut buf).map_err(|e| e.within(request))?;
    tolerate(wire.read(id, reg, bytes), faulted)?;
    reg.decode(bytes).map_err(|e| e.within(request).into())
}

/// Builds one actuator of `model` at `id` from what it reports, and pushes
/// it with torque `Off`, or `Faulted` if `faulted` (its ping or Model Number
/// reply reported only a hardware fault) or a reply here does. Returns
/// whether it is held `Faulted`. `firmware` is the version a P2 ping
/// returned.
///
/// 1. Firmware, read only where the model's table depends on it or has a
///    register only from some firmware on (MX-64 Protocol 1 Realtime
///    Tick), and recorded.
/// 2. The model variant for this protocol and firmware.
/// 3. The EEPROM limits (CW is the minimum, CCW the maximum).
/// 4. The limit mode: the mode register's value through the model's own
///    lists, or the angle limits where the model has no mode register (on
///    MX-64 Protocol 1, unless its torque control switch is on).
/// 5. The EEPROM bound of the goal that mode uses, where the model's table
///    records one: Velocity Limit in velocity mode, Current Limit in
///    current and current-based position mode (FR-9.3).
/// 6. The slot, `Off` or `Faulted`. No torque write.
fn init_one<P, T, C, S>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    id: Id,
    model: ModelNumber,
    firmware: Option<u8>,
    mut faulted: bool,
) -> Result<bool, Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
{
    let step = |wire: &mut Wire<'_, P, T, C>, step| wire.set_context(Request::NONE.step(step));
    let firmware = match (firmware, ActuatorKind::firmware_register(P::VERSION, model)) {
        (Some(firmware), _) => firmware,
        (None, Some(reg)) => {
            step(wire, Step::Firmware);
            let read = read_value(wire, id, reg, &mut faulted);
            let read = read.map_err(|err| diagnose_unknown(wire, id, Some(model.get()), err));
            u8::try_from(read?).unwrap_or(u8::MAX)
        }
        (None, None) => 0,
    };
    let mut kind = ActuatorKind::new(P::VERSION, model, firmware, id)?;
    kind.cache_mut().record_firmware(firmware);
    let data = kind.data();
    step(wire, Step::PositionLimits);
    let min = read_value(wire, id, data.limits.min(), &mut faulted);
    let min = super::diagnose::diagnosed(wire, &kind, min)?;
    let max = read_value(wire, id, data.limits.max(), &mut faulted);
    let max = super::diagnose::diagnosed(wire, &kind, max)?;
    let limits = kind.cache_mut().record_limits(min, max)?;
    let mode = match data.mode.register() {
        Some(reg) => {
            step(wire, Step::OperatingMode);
            let mut value = [0];
            let read = tolerate(wire.read(id, reg, &mut value), &mut faulted);
            super::diagnose::diagnosed(wire, &kind, read)?;
            let [value] = value;
            data.mode
                .limit_mode(value, limits)
                .ok_or(SetupError::UnknownOperatingMode {
                    id,
                    value,
                    register: reg.name,
                })?
        }
        None => LimitMode::from_angle_limits(limits),
    };
    kind.cache_mut().record_mode(mode);
    let bound = match field::bounded_goal(mode)
        .and_then(|role| data.role(role))
        .and_then(|goal| goal.bounded_by())
    {
        Some(reg) => {
            step(wire, Step::GoalBound);
            let read = read_value(wire, id, reg, &mut faulted);
            Some(super::diagnose::diagnosed(wire, &kind, read)?.cast_unsigned())
        }
        None => None,
    };
    let state = if faulted {
        TorqueState::Faulted
    } else {
        TorqueState::Off
    };
    slots.push(kind, bound, state)?;
    Ok(faulted)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::format;
    use std::string::ToString;

    use super::*;
    use crate::bus::testing::*;
    use crate::error::Instruction;
    use crate::protocol::{P1, P2, ProtocolVersion};
    use crate::transport::MockTime;

    fn model(n: u16) -> ModelNumber {
        ModelNumber(n)
    }

    /// A Read of `register` at `addr` (`len` bytes) from actuator `n` during
    /// `step`.
    fn read(n: u8, register: &'static str, addr: u16, len: u16, step: Step) -> Request {
        Request::NONE
            .instruction(Instruction::Read)
            .id(id(n))
            .register(register)
            .addr(addr)
            .len(len)
            .step(step)
    }

    fn no_reply(request: Request) -> Error {
        TransportError::NoReply {
            request,
            timeout_us: 10_000,
        }
        .into()
    }

    fn ids(list: &[u8]) -> IdSet {
        list.iter().map(|&n| id(n)).collect()
    }

    #[test]
    fn unsupported_declared_model_sends_nothing() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        let mark = mark(&bus);
        assert_eq!(
            bus.declare([(id(1), model(9999))]),
            Err(SetupError::UnsupportedModel {
                id: id(1),
                model_number: 9999,
                protocol: ProtocolVersion::P2,
                feature: None,
            }
            .into())
        );
        assert_quiet(&bus, mark);

        let mut bus = test_bus::<P1, 32>(&time);
        assert_eq!(
            bus.declare([(id(3), model(XL320))]),
            Err(SetupError::UnsupportedModel {
                id: id(3),
                model_number: XL320,
                protocol: ProtocolVersion::P1,
                feature: None,
            }
            .into())
        );
        assert_quiet(&bus, mark);
        assert!(bus.actuators().is_empty());
    }

    #[test]
    fn too_many_declared() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 2>(&time);
        let mark = mark(&bus);
        let list = [1, 2, 3].map(|n| (id(n), model(XM430)));
        assert_eq!(
            bus.declare(list),
            Err(SetupError::TooManyActuators {
                capacity: 2,
                ids: ids(&[1, 2, 3]),
                declared: true,
            }
            .into())
        );
        assert_quiet(&bus, mark);
    }

    #[test]
    fn duplicate_declared_id() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        let mark = mark(&bus);
        assert_eq!(
            bus.declare([(id(4), model(XM430)), (id(4), model(XM430))]),
            Err(ValidationError::DuplicateId { id: id(4) }.into())
        );
        assert_quiet(&bus, mark);
    }

    #[test]
    fn empty_list_sets_up_nothing() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        let mark = mark(&bus);
        assert_eq!(
            bus.declare([]),
            Ok(SetupReport {
                actuators: &[],
                faulted: IdSet::new(),
            })
        );
        assert_quiet(&bus, mark);
    }

    #[cfg(feature = "x-series")]
    mod x_series {
        use super::*;
        use crate::actuator::{Actuator, TorqueState};
        use crate::error::P2Code;

        fn xm430_bus(time: &MockTime, mode: u8) -> TestBus<'_, P2> {
            let mut bus = test_bus(time);
            xm430_p2(line(&mut bus), 1, 1, 1000, 3000, mode);
            bus.declare([(id(1), model(XM430))]).unwrap();
            bus
        }

        #[test]
        fn declare_reads_limits_and_starts_off() {
            let time = MockTime::new();
            let bus = xm430_bus(&time, 3);
            assert_eq!(sent(&bus), xm430_p2_sent(1));
            assert_done(&bus);
            assert_eq!(bus.torque_states(), [TorqueState::Off]);
            let held = &bus.actuators()[0];
            assert!(matches!(held, ActuatorKind::Xm430W350(_)));
            assert_eq!(held.id(), id(1));
            assert_eq!(held.position_limits_raw(), Some(1000..=3000));
        }

        #[test]
        fn declare_xl320_reads_control_mode() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            xl320_p2(line(&mut bus), 1, 2, 0, 1023, 2);
            bus.declare([(id(2), model(XL320))]).unwrap();
            assert_eq!(sent(&bus), xl320_p2_sent(2));
            assert_eq!(bus.actuators()[0].cache().check_goal(512), Ok(512));

            let mut bus = test_bus::<P2, 32>(&time);
            xl320_p2(line(&mut bus), 1, 2, 0, 1023, 1);
            bus.declare([(id(2), model(XL320))]).unwrap();
            assert_eq!(
                bus.actuators()[0].cache().check_goal(512),
                Err(ValidationError::NotPositionMode { id: id(2) })
            );
        }

        #[test]
        fn silent_declared_id_is_missing() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            assert_eq!(
                bus.declare([(id(5), model(XM430))]),
                Err(SetupError::Missing {
                    id: id(5),
                    timeout_us: 10_000,
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn late_ping_reply_is_late_not_missing() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            let pong = p2_pong(5, XM430, XM430_FW);
            line(&mut bus).reply_at(15_000, &pong);
            let err = bus.declare([(id(5), model(XM430))]).unwrap_err();
            assert_eq!(
                err,
                TransportError::LateReply {
                    request: Request::NONE
                        .instruction(Instruction::Ping)
                        .id(id(5))
                        .step(Step::Ping),
                    timeout_us: 10_000,
                    bytes: pong.len(),
                }
                .into()
            );
            assert_eq!(
                err.to_string(),
                format!(
                    "id 5: late reply to Ping in setup: {} bytes after 10 ms",
                    pong.len()
                )
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn other_model_is_wrong_model() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus).reply_to(1, &p2_pong(5, 1060, 45));
            assert_eq!(
                bus.declare([(id(5), model(XM430))]),
                Err(SetupError::WrongModel {
                    id: id(5),
                    declared: XM430,
                    found: 1060
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn limit_read_timeout_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p2_pong(1, XM430, XM430_FW))
                .reply_to(2, &p2_status(1, 0, &1000i32.to_le_bytes()));
            assert_eq!(
                bus.declare([(id(1), model(XM430))]),
                Err(no_reply(read(
                    1,
                    "Max Position Limit",
                    48,
                    4,
                    Step::PositionLimits
                )))
            );
            assert!(bus.actuators().is_empty());
            assert!(bus.torque_states().is_empty());
        }

        #[test]
        fn inverted_limits_fail_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            xm430_p2(line(&mut bus), 1, 1, 3000, 1000, 3);
            assert_eq!(
                bus.declare([(id(1), model(XM430))]),
                Err(ValidationError::InvertedLimits {
                    id: id(1),
                    min: 3000,
                    max: 1000
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn fault_at_setup_holds_faulted() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p2_status(1, 0x80, &[0xFC, 0x03, XM430_FW]))
                .reply_to(2, &p2_status(1, 0x80, &XM430.to_le_bytes()))
                .reply_to(3, &p2_status(1, 0x80, &1000i32.to_le_bytes()))
                .reply_to(4, &p2_status(1, 0x80, &3000i32.to_le_bytes()))
                .reply_to(5, &p2_status(1, 0x80, &[3]));
            let report = bus.declare([(id(1), model(XM430))]).unwrap();
            assert_eq!(report.faulted, ids(&[1]));
            assert_eq!(report.actuators[0].position_limits_raw(), Some(1000..=3000));
            assert_eq!(bus.torque_states(), [TorqueState::Faulted]);
            assert_eq!(
                sent(&bus),
                concat(&[
                    p2_instruction(1, PING, &[]),
                    p2_read(1, 0, 2),
                    xm430_p2_reads_sent(1),
                ])
            );
            assert_done(&bus);
        }

        #[test]
        fn fault_on_mode_read_only_holds_faulted() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p2_pong(1, XM430, XM430_FW))
                .reply_to(2, &p2_status(1, 0, &1000i32.to_le_bytes()))
                .reply_to(3, &p2_status(1, 0, &3000i32.to_le_bytes()))
                .reply_to(4, &p2_status(1, 0x80, &[3]));
            let report = bus.declare([(id(1), model(XM430))]).unwrap();
            assert_eq!(report.faulted, ids(&[1]));
            assert_eq!(bus.torque_states(), [TorqueState::Faulted]);
            assert_eq!(sent(&bus), xm430_p2_sent(1));
        }

        #[test]
        fn p2_alert_with_error_code_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus).reply_to(1, &p2_status(1, 0x84, &[0xFC, 0x03, XM430_FW]));
            assert_eq!(
                bus.declare([(id(1), model(XM430))]),
                Err(StatusError {
                    id: id(1),
                    report: StatusReport::P2 {
                        alert: true,
                        code: Some(P2Code::DataRange),
                    },
                    request: Request::NONE
                        .instruction(Instruction::Ping)
                        .id(id(1))
                        .step(Step::Ping),
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn faulted_with_unknown_mode_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p2_pong(1, XM430, XM430_FW))
                .reply_to(2, &p2_status(1, 0, &0i32.to_le_bytes()))
                .reply_to(3, &p2_status(1, 0, &4095i32.to_le_bytes()))
                .reply_to(4, &p2_status(1, 0x80, &[2]));
            assert_eq!(
                bus.declare([(id(1), model(XM430))]),
                Err(SetupError::UnknownOperatingMode {
                    id: id(1),
                    value: 2,
                    register: "Operating Mode",
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn faulted_at_setup_refuses_enable_torque() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            let at = xm430_p2(line(&mut bus), 1, 1, 0, 4095, 3);
            line(&mut bus)
                .reply_to(at, &p2_pong(2, XM430, XM430_FW))
                .reply_to(at + 1, &p2_status(2, 0, &0i32.to_le_bytes()))
                .reply_to(at + 2, &p2_status(2, 0, &4095i32.to_le_bytes()))
                .reply_to(at + 3, &p2_status(2, 0x80, &[3]));
            let list = [1, 2].map(|n| (id(n), model(XM430)));
            assert_eq!(bus.declare(list).unwrap().faulted, ids(&[2]));
            assert_eq!(
                bus.torque_states(),
                [TorqueState::Off, TorqueState::Faulted]
            );
            let mark = mark(&bus);
            assert_eq!(
                bus.enable_torque(bus.ids()),
                Err(ValidationError::ActuatorFaulted { id: id(2) }.into())
            );
            assert_quiet(&bus, mark);
            assert_eq!(
                bus.torque_states(),
                [TorqueState::Off, TorqueState::Faulted]
            );
        }

        #[test]
        fn second_declare_clears_a_held_fault() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p2_pong(1, XM430, XM430_FW))
                .reply_to(2, &p2_status(1, 0, &1000i32.to_le_bytes()))
                .reply_to(3, &p2_status(1, 0, &3000i32.to_le_bytes()))
                .reply_to(4, &p2_status(1, 0x80, &[3]));
            bus.declare([(id(1), model(XM430))]).unwrap();
            xm430_p2(line(&mut bus), 5, 1, 1000, 3000, 3);
            let report = bus.declare([(id(1), model(XM430))]).unwrap();
            assert_eq!(report.faulted, IdSet::new());
            assert_eq!(bus.torque_states(), [TorqueState::Off]);
        }

        #[test]
        fn second_declare_replaces_and_failed_one_empties() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            let next = xm430_p2(line(&mut bus), 1, 1, 0, 4095, 3);
            bus.declare([(id(1), model(XM430))]).unwrap();
            xm430_p2(line(&mut bus), next, 2, 0, 4095, 3);
            let held = bus.declare([(id(2), model(XM430))]).unwrap().actuators;
            assert_eq!(held.len(), 1);
            assert_eq!(held[0].id(), id(2));
            assert_eq!(
                bus.declare([(id(3), model(XM430))]),
                Err(SetupError::Missing {
                    id: id(3),
                    timeout_us: 10_000,
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn declared_list_is_held_in_id_order() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            let mut at = 1;
            for n in [1, 2, 3] {
                at = xm430_p2(line(&mut bus), at, n, 0, 4095, 3);
            }
            let list = [3, 1, 2].map(|n| (id(n), model(XM430)));
            let held = bus.declare(list).unwrap().actuators;
            let ids: [u8; 3] = core::array::from_fn(|i| held[i].id().get());
            assert_eq!(ids, [1, 2, 3]);
            assert_eq!(
                sent(&bus),
                concat(&[xm430_p2_sent(1), xm430_p2_sent(2), xm430_p2_sent(3)])
            );
            assert_eq!(bus.torque_states(), [TorqueState::Off; 3]);
        }

        #[test]
        fn mode_is_recorded_at_setup() {
            for (mode, expected) in [
                (
                    3,
                    Err(ValidationError::OutOfLimits {
                        id: id(1),
                        raw: 3500,
                        min: 1000,
                        max: 3000,
                    }),
                ),
                (4, Ok(3500)),
                (5, Ok(3500)),
                (1, Err(ValidationError::NotPositionMode { id: id(1) })),
                (0, Err(ValidationError::NotPositionMode { id: id(1) })),
                (16, Err(ValidationError::NotPositionMode { id: id(1) })),
            ] {
                let time = MockTime::new();
                let bus = xm430_bus(&time, mode);
                assert_eq!(
                    bus.actuators()[0].cache().check_goal(3500),
                    expected,
                    "mode {mode}"
                );
            }
        }

        #[test]
        fn unknown_mode_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            xm430_p2(line(&mut bus), 1, 1, 0, 4095, 2);
            assert_eq!(
                bus.declare([(id(1), model(XM430))]),
                Err(SetupError::UnknownOperatingMode {
                    id: id(1),
                    value: 2,
                    register: "Operating Mode",
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn mode_read_timeout_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p2_pong(1, XM430, XM430_FW))
                .reply_to(2, &p2_status(1, 0, &0i32.to_le_bytes()))
                .reply_to(3, &p2_status(1, 0, &4095i32.to_le_bytes()));
            assert_eq!(
                bus.declare([(id(1), model(XM430))]),
                Err(no_reply(read(
                    1,
                    "Operating Mode",
                    11,
                    1,
                    Step::OperatingMode
                )))
            );
            assert_eq!(sent(&bus), xm430_p2_sent(1));
            assert!(bus.actuators().is_empty());
        }
    }

    #[cfg(feature = "ax")]
    mod ax {
        use super::*;
        use crate::actuator::{Actuator, TorqueState};
        use crate::error::{Decivolts, FaultError, P1Flags, VoltageError};
        use crate::units::HardwareErrors;

        #[test]
        fn p1_scan_voltage_fault_is_held_and_measured_by_clear_fault() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            line(&mut bus)
                .reply_to(2, &p1_status(1, 0x01, &[]))
                .reply_to(254, &p1_status(1, 0x01, &AX12.to_le_bytes()))
                .reply_to(255, &p1_status(1, 0x01, &0u16.to_le_bytes()))
                .reply_to(256, &p1_status(1, 0x01, &1023u16.to_le_bytes()))
                .reply_to(257, &p1_status(1, 0x01, &[]))
                .reply_to(258, &p1_status(1, 0x01, &[97]))
                .reply_to(259, &p1_status(1, 0x01, &[105]))
                .reply_to(260, &p1_status(1, 0x01, &[130]));
            let report = bus.autoscan().unwrap();
            assert_eq!(report.faulted, ids(&[1]));
            assert_eq!(report.actuators[0].position_limits_raw(), Some(0..=1023));
            assert_eq!(bus.torque_states(), [TorqueState::Faulted]);
            let err = bus.actuator(id(1)).unwrap().clear_fault().unwrap_err();
            assert_eq!(
                err,
                Error::Fault(FaultError::Voltage(VoltageError::BelowMin {
                    id: id(1),
                    measured: Decivolts(97),
                    min: Decivolts(105),
                    max: Decivolts(130),
                    faults: HardwareErrors::from_raw(0x01),
                    request: Request::NONE.instruction(Instruction::Ping).id(id(1)),
                }))
            );
            assert_eq!(err.to_string(), "id 1: voltage 9.7 V below 10.5 V");
            assert_eq!(format!("{err:?}"), err.to_string());
            assert!(sent(&bus).ends_with(&concat(&[
                p1_read(1, 0, 2),
                p1_read(1, 6, 2),
                p1_read(1, 8, 2),
                p1_instruction(1, PING, &[]),
                p1_read(1, 42, 1),
                p1_read(1, 12, 1),
                p1_read(1, 13, 1),
            ])));
            assert_eq!(bus.torque_states(), [TorqueState::Faulted]);
        }

        #[test]
        fn p1_scan_instruction_and_voltage_flags_fail() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            line(&mut bus).reply_to(2, &p1_status(1, 0x41, &[]));
            assert_eq!(
                bus.autoscan(),
                Err(StatusError {
                    id: id(1),
                    report: StatusReport::P1(P1Flags(0x41)),
                    request: Request::NONE
                        .instruction(Instruction::Ping)
                        .id(id(1))
                        .step(Step::Scan),
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn p1_model_read_fault_holds_faulted() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p1_status(1, 0, &[]))
                .reply_to(2, &p1_status(1, 0x20, &AX12.to_le_bytes()))
                .reply_to(3, &p1_status(1, 0, &0u16.to_le_bytes()))
                .reply_to(4, &p1_status(1, 0, &1023u16.to_le_bytes()));
            let report = bus.declare([(id(1), model(AX12))]).unwrap();
            assert_eq!(report.faulted, ids(&[1]));
            assert_eq!(report.actuators[0].position_limits_raw(), Some(0..=1023));
            assert_eq!(bus.torque_states(), [TorqueState::Faulted]);
            assert_eq!(sent(&bus), ax12_p1_sent(1));
        }

        #[test]
        fn p1_instruction_and_voltage_flags_fail_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p1_status(1, 0, &[]))
                .reply_to(2, &p1_status(1, 0x41, &AX12.to_le_bytes()));
            assert_eq!(
                bus.declare([(id(1), model(AX12))]),
                Err(StatusError {
                    id: id(1),
                    report: StatusReport::P1(P1Flags(0x41)),
                    request: read(1, "Model Number", 0, 2, Step::ModelNumber),
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn declare_ax12_reads_model_and_angle_limits_without_mode() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            ax12_p1(line(&mut bus), 1, 1, 0, 1023);
            bus.declare([(id(1), model(AX12))]).unwrap();
            assert_eq!(sent(&bus), ax12_p1_sent(1));
            assert_done(&bus);
            assert_eq!(bus.torque_states(), [TorqueState::Off]);
            assert_eq!(bus.actuators()[0].position_limits_raw(), Some(0..=1023));
            assert_eq!(bus.actuators()[0].cache().check_goal(512), Ok(512));
        }

        #[test]
        fn ax12_wheel_mode_is_recorded() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            ax12_p1(line(&mut bus), 1, 1, 0, 0);
            bus.declare([(id(1), model(AX12))]).unwrap();
            assert_eq!(sent(&bus), ax12_p1_sent(1));
            assert_eq!(
                bus.actuators()[0].cache().check_goal(0),
                Err(ValidationError::NotPositionMode { id: id(1) })
            );
        }

        #[test]
        fn p1_model_mismatch_is_wrong_model() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            angle_p1(line(&mut bus), 1, 1, 18, 0, 1023);
            assert_eq!(
                bus.declare([(id(1), model(AX12))]),
                Err(SetupError::WrongModel {
                    id: id(1),
                    declared: AX12,
                    found: 18
                }
                .into())
            );
        }
    }

    #[cfg(feature = "rx")]
    mod rx {
        use super::*;

        #[test]
        fn declare_eight_rx28_on_p1() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            let mut at = 1;
            for n in 1..=8 {
                at = angle_p1(line(&mut bus), at, n, RX28, 0, 1023);
            }
            let held = bus
                .declare((1..=8).map(|n| (id(n), model(RX28))))
                .unwrap()
                .actuators;
            assert_eq!(held.len(), 8);
            assert!(
                held.iter()
                    .all(|kind| matches!(kind, ActuatorKind::Rx28(_)))
            );
            assert_eq!(bus.actuators()[7].cache().check_goal(1023), Ok(1023));
            assert_done(&bus);
        }
    }

    #[cfg(feature = "mx")]
    mod mx_p1 {
        use super::*;

        /// MX-64 on Protocol 1.0, model number 310.
        const MX64: u16 = 310;

        /// Scripts Setup of an MX-64 (Protocol 1.0) at ID 1 with these
        /// angle limits and Torque Ctrl Mode Enable value.
        fn mx64_p1(time: &MockTime, cw: u16, ccw: u16, switch: u8) -> TestBus<'_, P1> {
            let mut bus = test_bus::<P1, 32>(time);
            crate::bus::testing::mx64_p1(line(&mut bus), 1, 1, cw, ccw, switch);
            bus
        }

        #[test]
        fn firmware_then_torque_switch_after_the_angle_limits() {
            let time = MockTime::new();
            let mut bus = mx64_p1(&time, 0, 4095, 1);
            bus.declare([(id(1), model(MX64))]).unwrap();
            assert_eq!(sent(&bus), mx64_p1_sent(1));
            assert_done(&bus);
            let cache = bus.actuators()[0].cache();
            assert_eq!(cache.firmware(), MX64_P1_FW);
            assert_eq!(cache.mode(), LimitMode::Current);
            assert_eq!(
                cache.check_goal(2048),
                Err(ValidationError::NotPositionMode { id: id(1) })
            );
            assert_eq!(cache.check_goal_current(), Ok(()));
        }

        #[test]
        fn switch_off_keeps_the_angle_limit_rule() {
            for ((cw, ccw), mode) in [
                ((0, 4095), LimitMode::Bounded),
                ((0, 0), LimitMode::Wheel),
                ((4095, 4095), LimitMode::Unbounded),
            ] {
                let time = MockTime::new();
                let mut bus = mx64_p1(&time, cw, ccw, 0);
                bus.declare([(id(1), model(MX64))]).unwrap();
                assert_eq!(bus.actuators()[0].cache().mode(), mode, "{cw}/{ccw}");
            }
        }

        #[test]
        fn switch_value_outside_its_range_fails_setup() {
            let time = MockTime::new();
            let mut bus = mx64_p1(&time, 0, 4095, 2);
            assert_eq!(
                bus.declare([(id(1), model(MX64))]),
                Err(SetupError::UnknownOperatingMode {
                    id: id(1),
                    value: 2,
                    register: "Torque Ctrl Mode Enable",
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn a_failed_firmware_read_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p1_status(1, 0, &[]))
                .reply_to(2, &p1_status(1, 0, &MX64.to_le_bytes()));
            assert_eq!(
                bus.declare([(id(1), model(MX64))]),
                Err(no_reply(read(1, "Firmware Version", 2, 1, Step::Firmware)))
            );
            assert!(bus.actuators().is_empty());
        }

        #[cfg(feature = "ax")]
        #[test]
        fn models_without_a_firmware_dependent_register_record_no_firmware() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            ax12_p1(line(&mut bus), 1, 1, 0, 1023);
            bus.declare([(id(1), model(AX12))]).unwrap();
            assert_eq!(sent(&bus), ax12_p1_sent(1));
            assert_eq!(bus.actuators()[0].cache().firmware(), 0);
        }
    }

    mod scan {
        extern crate std;

        use super::*;
        use crate::error::ProtocolError;

        fn p1_sweep() -> std::vec::Vec<u8> {
            concat(
                &(0..=Id::MAX)
                    .map(|n| p1_instruction(n, PING, &[]))
                    .collect::<std::vec::Vec<_>>(),
            )
        }

        fn broadcast_ping() -> std::vec::Vec<u8> {
            p2_instruction(0xFE, PING, &[])
        }

        #[test]
        fn p2_autoscan_empty_is_bounded() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            assert_eq!(
                bus.autoscan(),
                Ok(SetupReport {
                    actuators: &[],
                    faulted: IdSet::new(),
                })
            );
            assert_eq!(
                sent(&bus),
                [0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x03, 0x00, 0x01, 0x31, 0x42]
            );
            assert!(time.now() <= 10_000 + 1_000, "took {} us", time.now());
        }

        #[test]
        fn p1_autoscan_empty_is_bounded() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            assert_eq!(
                bus.autoscan(),
                Ok(SetupReport {
                    actuators: &[],
                    faulted: IdSet::new(),
                })
            );
            assert_eq!(sent(&bus), p1_sweep());
            assert!(
                time.now() <= 253 * 10_000 + 10_000,
                "took {} us",
                time.now()
            );
        }

        #[test]
        fn corrupt_scan_reply_fails() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            let mut pong = p2_pong(3, XM430, XM430_FW);
            let last = pong.len() - 1;
            pong[last] ^= 0xFF;
            line(&mut bus).reply_to(1, &pong);
            assert!(matches!(
                bus.autoscan(),
                Err(Error::Protocol(ProtocolError::Crc { .. }))
            ));
            assert_eq!(sent(&bus), broadcast_ping());
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn two_pongs_one_id() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p2_pong(3, XM430, XM430_FW))
                .reply_to(1, &p2_pong(3, XL320, 37));
            assert_eq!(
                bus.autoscan(),
                Err(ValidationError::DuplicateId { id: id(3) }.into())
            );
            assert_eq!(sent(&bus), broadcast_ping());
            assert!(bus.actuators().is_empty());
        }

        #[cfg(feature = "x-series")]
        #[test]
        fn pong_with_alert_is_held() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            let [lo, hi] = XM430.to_le_bytes();
            line(&mut bus)
                .reply_to(1, &p2_status(3, 0x80, &[lo, hi, XM430_FW]))
                .reply_to(2, &p2_status(3, 0x80, &1000i32.to_le_bytes()))
                .reply_to(3, &p2_status(3, 0x80, &3000i32.to_le_bytes()))
                .reply_to(4, &p2_status(3, 0x80, &[3]));
            let report = bus.autoscan().unwrap();
            assert_eq!(report.faulted, ids(&[3]));
            assert_eq!(bus.torque_states(), [FAULTED]);
            assert_eq!(
                sent(&bus),
                concat(&[broadcast_ping(), xm430_p2_reads_sent(3)])
            );
            assert_done(&bus);
        }

        #[test]
        fn scan_past_capacity() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 2>(&time);
            for n in [1, 2, 3] {
                line(&mut bus).reply_to(1, &p2_pong(n, XM430, XM430_FW));
            }
            assert_eq!(
                bus.autoscan(),
                Err(SetupError::TooManyActuators {
                    capacity: 2,
                    ids: ids(&[1, 2, 3]),
                    declared: false,
                }
                .into())
            );
            assert_eq!(sent(&bus), broadcast_ping());
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn p2_only_model_on_p1_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            line(&mut bus)
                .reply_to(5, &p1_status(4, 0, &[]))
                .reply_to(254, &p1_status(4, 0, &XL320.to_le_bytes()));
            assert_eq!(
                bus.autoscan(),
                Err(SetupError::UnsupportedModel {
                    id: id(4),
                    model_number: XL320,
                    protocol: ProtocolVersion::P1,
                    feature: None,
                }
                .into())
            );
            assert_eq!(sent(&bus), concat(&[p1_sweep(), p1_read(4, 0, 2)]));
            assert!(bus.actuators().is_empty());
        }

        #[cfg(not(feature = "pro"))]
        #[test]
        fn family_off_names_feature() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus).reply_to(1, &p2_pong(7, 2000, 1));
            assert_eq!(
                bus.autoscan(),
                Err(SetupError::UnsupportedModel {
                    id: id(7),
                    model_number: 2000,
                    protocol: ProtocolVersion::P2,
                    feature: Some("pro"),
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn autoscan_replaces_a_declared_list() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            assert_eq!(
                bus.declare([]),
                Ok(SetupReport {
                    actuators: &[],
                    faulted: IdSet::new(),
                })
            );
            line(&mut bus).reply_to(1, &p2_pong(1, 9999, 1));
            assert!(bus.autoscan().is_err());
            assert!(bus.actuators().is_empty());
        }

        #[cfg(feature = "x-series")]
        mod x_series {
            use super::*;
            use crate::actuator::{Actuator, TorqueState};

            #[test]
            fn p2_autoscan_sorts_three() {
                let time = MockTime::new();
                let mut bus = test_bus::<P2, 32>(&time);
                for n in [3, 1, 2] {
                    line(&mut bus).reply_to(1, &p2_pong(n, XM430, XM430_FW));
                }
                let mut at = 2;
                for n in [1, 2, 3] {
                    at = xm430_p2_reads(line(&mut bus), at, n, 0, 4095, 3);
                }
                let held = bus.autoscan().unwrap().actuators;
                let ids: [u8; 3] = core::array::from_fn(|i| held[i].id().get());
                assert_eq!(ids, [1, 2, 3]);
                assert_eq!(
                    sent(&bus),
                    concat(&[
                        broadcast_ping(),
                        xm430_p2_reads_sent(1),
                        xm430_p2_reads_sent(2),
                        xm430_p2_reads_sent(3),
                    ])
                );
                assert_eq!(bus.torque_states(), [TorqueState::Off; 3]);
                assert_done(&bus);
            }

            #[test]
            fn firmware_picks_the_table() {
                for (fw, fw52) in [(52, true), (53, false)] {
                    let time = MockTime::new();
                    let mut bus = test_bus::<P2, 32>(&time);
                    line(&mut bus).reply_to(1, &p2_pong(4, 1190, fw));
                    xm430_p2_reads(line(&mut bus), 2, 4, 0, 4095, 3);
                    let held = bus.autoscan().unwrap().actuators;
                    assert_eq!(
                        matches!(held[0], ActuatorKind::Xl330M077Fw52(_)),
                        fw52,
                        "firmware {fw}"
                    );
                    assert_eq!(
                        matches!(held[0], ActuatorKind::Xl330M077(_)),
                        !fw52,
                        "firmware {fw}"
                    );
                }
            }

            #[test]
            fn autoscan_reads_the_mode_like_declare() {
                let time = MockTime::new();
                let mut bus = test_bus::<P2, 32>(&time);
                line(&mut bus).reply_to(1, &p2_pong(1, XM430, XM430_FW));
                xm430_p2_reads(line(&mut bus), 2, 1, 0, 4095, 2);
                assert_eq!(
                    bus.autoscan(),
                    Err(SetupError::UnknownOperatingMode {
                        id: id(1),
                        value: 2,
                        register: "Operating Mode",
                    }
                    .into())
                );
                assert!(bus.actuators().is_empty());
            }
        }

        #[cfg(feature = "rx")]
        mod rx {
            use super::*;
            use crate::actuator::TorqueState;

            #[test]
            fn p1_autoscan_some_answer() {
                let time = MockTime::new();
                let mut bus = test_bus::<P1, 32>(&time);
                line(&mut bus)
                    .reply_to(2, &p1_status(1, 0, &[]))
                    .reply_to(6, &p1_status(5, 0, &[]));
                let at = angle_p1_reads(line(&mut bus), 254, 1, RX28, 0, 1023);
                angle_p1_reads(line(&mut bus), at, 5, RX28, 0, 1023);
                let held = bus.autoscan().unwrap().actuators;
                assert_eq!(held.len(), 2);
                assert!(matches!(held[0], ActuatorKind::Rx28(_)));
                assert!(matches!(held[1], ActuatorKind::Rx28(_)));
                assert_eq!(
                    sent(&bus),
                    concat(&[
                        p1_sweep(),
                        p1_read(1, 0, 2),
                        p1_read(1, 6, 2),
                        p1_read(1, 8, 2),
                        p1_read(5, 0, 2),
                        p1_read(5, 6, 2),
                        p1_read(5, 8, 2),
                    ])
                );
                assert_eq!(bus.torque_states(), [TorqueState::Off; 2]);
                assert_done(&bus);
            }
        }
    }
}
