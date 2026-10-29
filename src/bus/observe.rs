//! The one place where replies move a [`TorqueState`] (research R-7, R-10).

use crate::Transport;
use crate::actuator::{Actuator, Describe, TorqueState};
use crate::error::{Error, StatusError, StatusReport};
use crate::models::ActuatorKind;
use crate::protocol::{Clock, Protocol};

use super::wire::Wire;

/// Whether `err` is a reply that reports a hardware fault.
pub(crate) fn is_hardware_fault(err: &Error) -> bool {
    matches!(err, Error::Status(status) if status.is_hardware_fault())
}

/// Whether `result` carries a reply that reports a hardware fault.
fn is_fault<V>(result: &Result<V, Error>) -> bool {
    result.as_ref().is_err_and(is_hardware_fault)
}

/// Marks the actuator `Faulted` when `result` is a reply reporting a
/// hardware fault. Any other result leaves the state as it is.
pub(crate) fn observe<V>(state: &mut TorqueState, result: &Result<V, Error>) {
    if is_fault(result) {
        *state = state.fault();
    }
}

/// Applies the result of a Torque Enable write: `wanted` on success,
/// `Faulted` on a hardware fault, and `disable()` on any other error, so a
/// failed write never leaves `On` and never drops a fault.
pub(crate) fn after_torque_write(
    state: &mut TorqueState,
    wanted: TorqueState,
    result: &Result<(), Error>,
) {
    *state = match result {
        Ok(()) => wanted,
        Err(_) if is_fault(result) => state.fault(),
        Err(_) => state.disable(),
    };
}

/// Asks the actuator whether its fault is gone.
///
/// Reads Hardware Error Status where the model has it, else pings. Clears
/// `state` to `Off` only on a clean answer. A non-zero Hardware Error Status
/// gives a `Status` error with the alert bit set; a reply reporting a
/// hardware fault is returned as is. Either way the state stays (or
/// becomes) `Faulted`.
pub(crate) fn check_clear<P: Protocol, T: Transport, C: Clock>(
    wire: &mut Wire<'_, P, T, C>,
    kind: &ActuatorKind,
    state: &mut TorqueState,
) -> Result<(), Error> {
    let id = kind.id();
    let result = match kind.data().hardware_error {
        Some(reg) => wire.read_value(id, reg).and_then(|bits| {
            if bits == 0 {
                Ok(())
            } else {
                Err(StatusError {
                    id,
                    report: StatusReport::P2 {
                        alert: true,
                        code: None,
                    },
                }
                .into())
            }
        }),
        None => wire.ping(id).map(drop),
    };
    observe(state, &result);
    if result.is_ok() {
        *state = state.clear();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::testing::*;
    use crate::error::{P1Flag, P1Flags, P2Code, ProtocolError, TransportError};

    fn status(report: StatusReport) -> Result<(), Error> {
        Err(StatusError { id: id(1), report }.into())
    }

    fn p2(alert: bool, code: Option<P2Code>) -> Result<(), Error> {
        status(StatusReport::P2 { alert, code })
    }

    fn p1(flag: P1Flag) -> Result<(), Error> {
        status(StatusReport::P1(P1Flags(1 << flag_bit(flag))))
    }

    fn flag_bit(flag: P1Flag) -> u8 {
        (0..8)
            .find(|&bit| P1Flags(1 << bit).contains(flag))
            .unwrap()
    }

    const ALL: [TorqueState; 3] = [TorqueState::Off, TorqueState::On, TorqueState::Faulted];

    #[test]
    fn hardware_fault_faults_every_state() {
        for start in ALL {
            for result in [
                p2(true, None),
                p1(P1Flag::Overload),
                p1(P1Flag::Overheating),
            ] {
                let mut state = start;
                observe(&mut state, &result);
                assert_eq!(state, TorqueState::Faulted, "from {start:?}");
            }
        }
    }

    #[test]
    fn other_results_leave_the_state() {
        let others: [Result<(), Error>; 5] = [
            Ok(()),
            p2(false, Some(P2Code::DataRange)),
            p1(P1Flag::AngleLimit),
            Err(TransportError::Timeout { id: Some(id(1)) }.into()),
            Err(ProtocolError::Malformed.into()),
        ];
        for start in ALL {
            for result in &others {
                let mut state = start;
                observe(&mut state, result);
                assert_eq!(state, start, "{result:?}");
            }
        }
    }

    #[test]
    fn torque_write_ok_gives_wanted() {
        let mut state = TorqueState::Off;
        after_torque_write(&mut state, TorqueState::On, &Ok(()));
        assert_eq!(state, TorqueState::On);
        after_torque_write(&mut state, TorqueState::Off, &Ok(()));
        assert_eq!(state, TorqueState::Off);
    }

    #[test]
    fn torque_write_hardware_fault_faults() {
        for start in ALL {
            let mut state = start;
            after_torque_write(&mut state, TorqueState::On, &p2(true, None));
            assert_eq!(state, TorqueState::Faulted);
        }
    }

    #[test]
    fn torque_write_other_error_disables_and_keeps_a_fault() {
        let errors: [Result<(), Error>; 3] = [
            Err(TransportError::Timeout { id: Some(id(1)) }.into()),
            Err(ProtocolError::Malformed.into()),
            p2(false, Some(P2Code::Access)),
        ];
        for error in &errors {
            for (start, end) in [
                (TorqueState::On, TorqueState::Off),
                (TorqueState::Off, TorqueState::Off),
                (TorqueState::Faulted, TorqueState::Faulted),
            ] {
                let mut state = start;
                after_torque_write(&mut state, TorqueState::On, error);
                assert_eq!(state, end, "{start:?}, {error:?}");
            }
        }
    }

    #[cfg(any(feature = "x-series", feature = "ax"))]
    mod clear {
        use super::*;
        use crate::actuator::ModelNumber;
        #[cfg(feature = "ax")]
        use crate::protocol::P1;
        #[cfg(feature = "x-series")]
        use crate::protocol::P2;
        use crate::transport::MockTime;

        #[cfg(feature = "x-series")]
        fn faulted_xm430(time: &MockTime) -> TestBus<'_, P2> {
            let mut bus = test_bus(time);
            xm430_p2(line(&mut bus), 1, 1, 0, 4095, 3);
            bus.declare([(id(1), ModelNumber(XM430))]).unwrap();
            line(&mut bus).reply(&p2_status(1, 0x80, &[]));
            let _ = bus.actuator(id(1)).unwrap().enable_torque();
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Faulted));
            bus
        }

        #[cfg(feature = "x-series")]
        #[test]
        fn clean_hardware_error_status_clears_to_off() {
            let time = MockTime::new();
            let mut bus = faulted_xm430(&time);
            let mark = mark(&bus);
            line(&mut bus).reply(&p2_status(1, 0, &[0]));
            assert_eq!(bus.actuator(id(1)).unwrap().clear_fault(), Ok(()));
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Off));
            assert_eq!(sent_since(&bus, mark), p2_read(1, 70, 1));
        }

        #[cfg(feature = "x-series")]
        #[test]
        fn alert_reply_keeps_the_fault() {
            let time = MockTime::new();
            let mut bus = faulted_xm430(&time);
            line(&mut bus).reply(&p2_status(1, 0x80, &[0x20]));
            assert_eq!(
                bus.actuator(id(1)).unwrap().clear_fault(),
                Err(StatusError {
                    id: id(1),
                    report: StatusReport::P2 {
                        alert: true,
                        code: None
                    }
                }
                .into())
            );
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Faulted));
        }

        #[cfg(feature = "x-series")]
        #[test]
        fn non_zero_hardware_error_status_keeps_the_fault() {
            let time = MockTime::new();
            let mut bus = faulted_xm430(&time);
            line(&mut bus).reply(&p2_status(1, 0, &[0x20]));
            assert_eq!(
                bus.actuator(id(1)).unwrap().clear_fault(),
                Err(StatusError {
                    id: id(1),
                    report: StatusReport::P2 {
                        alert: true,
                        code: None
                    }
                }
                .into())
            );
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Faulted));
        }

        #[cfg(feature = "x-series")]
        #[test]
        fn failed_check_keeps_the_fault() {
            let time = MockTime::new();
            let mut bus = faulted_xm430(&time);
            assert!(matches!(
                bus.actuator(id(1)).unwrap().clear_fault(),
                Err(Error::Transport(TransportError::Timeout { .. }))
            ));
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Faulted));
        }

        #[cfg(feature = "x-series")]
        #[test]
        fn clear_fault_on_an_off_slot_still_asks() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            xm430_p2(line(&mut bus), 1, 1, 0, 4095, 3);
            bus.declare([(id(1), ModelNumber(XM430))]).unwrap();
            let mark = mark(&bus);
            line(&mut bus).reply(&p2_status(1, 0, &[0]));
            assert_eq!(bus.actuator(id(1)).unwrap().clear_fault(), Ok(()));
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Off));
            assert_eq!(sent_since(&bus, mark), p2_read(1, 70, 1));
        }

        #[cfg(feature = "ax")]
        fn faulted_ax12(time: &MockTime) -> TestBus<'_, P1> {
            let mut bus = test_bus(time);
            ax12_p1(line(&mut bus), 1, 1, 0, 1023);
            bus.declare([(id(1), ModelNumber(AX12))]).unwrap();
            line(&mut bus).reply(&p1_status(1, 0x20, &[]));
            let _ = bus.actuator(id(1)).unwrap().enable_torque();
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Faulted));
            bus
        }

        #[cfg(feature = "ax")]
        #[test]
        fn clean_p1_ping_clears_to_off() {
            let time = MockTime::new();
            let mut bus = faulted_ax12(&time);
            let mark = mark(&bus);
            line(&mut bus).reply(&p1_status(1, 0, &[]));
            assert_eq!(bus.actuator(id(1)).unwrap().clear_fault(), Ok(()));
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Off));
            assert_eq!(sent_since(&bus, mark), p1_instruction(1, PING, &[]));
        }

        #[cfg(feature = "ax")]
        #[test]
        fn p1_ping_with_overheating_keeps_the_fault() {
            let time = MockTime::new();
            let mut bus = faulted_ax12(&time);
            line(&mut bus).reply(&p1_status(1, 0x04, &[]));
            assert!(matches!(
                bus.actuator(id(1)).unwrap().clear_fault(),
                Err(Error::Status(_))
            ));
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Faulted));
        }
    }
}
