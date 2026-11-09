//! Raw register access on one actuator (`raw-registers`; research R-21):
//! what is sent bypasses the bus's checks, what comes back still passes the
//! fault check.

use crate::Transport;
use crate::actuator::Actuator;
use crate::error::Error;
use crate::protocol::{Clock, Protocol};

use super::actuator_ref::ActuatorRef;
use super::diagnose::diagnosed;
use super::observe::{observe, reached};

/// Any register, by address (`raw-registers` feature).
impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> {
    /// Reads `out.len()` bytes at `addr`: one Read, in every torque state.
    ///
    /// No guard and no check of `addr` against the model's control table.
    /// Retried like any unicast read ([`Config::retries`]). A reply
    /// reporting a hardware fault is measured and returned as
    /// [`Error::Fault`] and marks
    /// the actuator [`Faulted`](crate::TorqueState::Faulted) on this call,
    /// like every other reply. `out` holds the bytes only on `Ok`.
    ///
    /// # Errors
    ///
    /// - A request the protocol cannot send, with nothing sent: `out` too
    ///   long for one reply, or on Protocol 1 an address above 255.
    /// - Any transport, protocol or status error of the read.
    ///
    /// [`Config::retries`]: crate::Config::retries
    pub fn read_raw(&mut self, addr: u16, out: &mut [u8]) -> Result<(), Error> {
        let result = self.wire.read_at(self.kind.id(), addr, out);
        let result = diagnosed(&mut self.wire, self.kind, result);
        observe(self.state, &result);
        result
    }

    /// Writes `data` at `addr`: one acknowledged Write, in every torque
    /// state.
    ///
    /// No torque guard, no limit check, no operating-mode check, no check
    /// of `addr` against the model's control table: the bytes are sent as
    /// given, even while torque is off. Retried like any unicast write
    /// ([`Config::retries`]). A reply reporting a hardware fault is
    /// measured and returned as
    /// [`Error::Fault`] and marks the actuator
    /// [`Faulted`](crate::TorqueState::Faulted), like every other reply.
    ///
    /// The write never updates the bus's view of the actuator:
    /// - a write to Torque Enable leaves
    ///   [`torque_state`](ActuatorRef::torque_state) as it was, until the
    ///   next [`enable_torque`](ActuatorRef::enable_torque) or
    ///   [`disable_torque`](ActuatorRef::disable_torque);
    /// - a write to Operating Mode, or to a position limit, leaves the
    ///   limits and mode read at Setup in force for goal checks until the
    ///   next Setup. [`write_limit`](ActuatorRef::write_limit) and the
    ///   limit setters write a limit and keep them true.
    ///
    /// Any address may hold Torque Enable, so once the write is sent,
    /// whatever its reply, torque is no longer confirmed off (plan
    /// 2026-11-09 D8): limit writes are refused with
    /// [`TorqueUnconfirmed`](crate::ValidationError::TorqueUnconfirmed)
    /// until an acknowledged [`disable_torque`](ActuatorRef::disable_torque).
    ///
    /// # Errors
    ///
    /// - A request the protocol cannot send, with nothing sent (on
    ///   Protocol 1 an address above 255).
    /// - Any transport, protocol or status error of the write.
    ///
    /// [`Config::retries`]: crate::Config::retries
    pub fn write_raw(&mut self, addr: u16, data: &[u8]) -> Result<(), Error> {
        let result = self.wire.write_at(self.kind.id(), addr, data);
        if reached(&result) {
            self.marks.forget();
        }
        let result = diagnosed(&mut self.wire, self.kind, result);
        observe(self.state, &result);
        result
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "x-series")]
    mod x_series {
        extern crate std;

        use crate::bus::testing::*;
        use crate::error::{
            Error, FaultError, Instruction, Request, TransportError, ValidationError,
        };
        use crate::protocol::P2;
        use crate::transport::{MockTime, MockTransport};
        use crate::units::HardwareErrors;
        use crate::{Bus, Config};

        /// An XM430-W350 at ID 1, limits 1000..=3000, torque `state`.
        fn xm430(time: &MockTime, state: crate::TorqueState) -> TestBus<'_, P2> {
            let mut bus = test_bus(time);
            hold(&mut bus, XM430, 1, (1000, 3000), BOUNDED, state);
            bus
        }

        /// The raw `instruction` at `addr` of `len` bytes on actuator 1.
        fn raw(instruction: Instruction, addr: u16, len: u16) -> Request {
            Request::NONE
                .instruction(instruction)
                .id(id(1))
                .addr(addr)
                .len(len)
        }

        /// The overload actuator 1 reports in reply to `request`, measured.
        fn overload(request: Request) -> Error {
            FaultError::Overload {
                id: id(1),
                faults: HardwareErrors::from_raw(0x20),
                request,
            }
            .into()
        }

        #[test]
        fn raw_write_while_off_is_sent() {
            let time = MockTime::new();
            let mut bus = xm430(&time, OFF);
            ack(&mut bus, 1);
            let mut xm = bus.actuator(id(1)).unwrap();
            assert_eq!(xm.write_raw(116, &[0x00, 0x08, 0x00, 0x00]), Ok(()));
            assert_eq!(xm.torque_state(), OFF);
            assert_eq!(sent(&bus), p2_write(1, 116, &[0x00, 0x08, 0x00, 0x00]));
            assert_done(&bus);
        }

        #[test]
        fn raw_write_skips_the_limits() {
            let time = MockTime::new();
            let mut bus = xm430(&time, ON);
            ack(&mut bus, 1);
            let outside = 5000i32.to_le_bytes();
            assert_eq!(
                bus.actuator(id(1)).unwrap().write_raw(116, &outside),
                Ok(())
            );
            assert_eq!(sent(&bus), p2_write(1, 116, &outside));
            assert_eq!(
                bus.actuator(id(1)).unwrap().set_goal_position_raw(5000),
                Err(ValidationError::OutOfLimits {
                    id: id(1),
                    raw: 5000,
                    min: 1000,
                    max: 3000
                }
                .into())
            );
        }

        #[test]
        fn raw_torque_write_does_not_change_state() {
            let time = MockTime::new();
            let mut bus = xm430(&time, OFF);
            ack(&mut bus, 1);
            assert_eq!(bus.actuator(id(1)).unwrap().write_raw(64, &[1]), Ok(()));
            assert_eq!(sent(&bus), p2_write(1, 64, &[1]));
            assert_eq!(bus.torque_state(id(1)), Some(OFF));
            assert_eq!(
                bus.actuator(id(1)).unwrap().set_goal_position_raw(2000),
                Err(ValidationError::ActuatorIsOff { id: id(1) }.into())
            );
        }

        #[test]
        fn raw_read_returns_bytes() {
            let time = MockTime::new();
            let mut bus = xm430(&time, FAULTED);
            line(&mut bus).reply(&p2_status(1, 0, &[0x46, 0x09, 0x00, 0x00]));
            let mut out = [0; 4];
            assert_eq!(bus.actuator(id(1)).unwrap().read_raw(132, &mut out), Ok(()));
            assert_eq!(out, [0x46, 0x09, 0x00, 0x00]);
            assert_eq!(sent(&bus), p2_read(1, 132, 4));
            assert_eq!(bus.torque_state(id(1)), Some(FAULTED));
        }

        #[test]
        fn raw_alert_reply_faults() {
            let time = MockTime::new();
            let mut bus = xm430(&time, ON);
            line(&mut bus)
                .reply_to(1, &p2_status(1, 0x80, &[0; 4]))
                .reply_to(2, &p2_status(1, 0x80, &[0x20]));
            let mut xm = bus.actuator(id(1)).unwrap();
            assert_eq!(
                xm.read_raw(132, &mut [0; 4]),
                Err(overload(raw(Instruction::Read, 132, 4)))
            );
            assert_eq!(xm.torque_state(), FAULTED);

            let time = MockTime::new();
            let mut bus = xm430(&time, OFF);
            line(&mut bus)
                .reply_to(1, &p2_status(1, 0x80, &[]))
                .reply_to(2, &p2_status(1, 0x80, &[0x20]));
            let mut xm = bus.actuator(id(1)).unwrap();
            assert_eq!(
                xm.write_raw(65, &[1]),
                Err(overload(raw(Instruction::Write, 65, 1)))
            );
            assert_eq!(xm.torque_state(), FAULTED);
        }

        #[test]
        fn raw_errors_that_are_not_faults_leave_the_state() {
            let time = MockTime::new();
            let mut bus = xm430(&time, ON);
            let mut xm = bus.actuator(id(1)).unwrap();
            assert_eq!(
                xm.write_raw(65, &[1]),
                Err(TransportError::NoReply {
                    request: raw(Instruction::Write, 65, 1),
                    timeout_us: 10_000,
                }
                .into())
            );
            assert_eq!(xm.torque_state(), ON);
        }

        #[test]
        fn raw_calls_are_retried_like_any_unicast() {
            let time = MockTime::new();
            let config = Config::new().retries(1);
            let mut bus: TestBus<'_, P2> =
                Bus::new(MockTransport::new(&time), Ticker(&time), config);
            hold(&mut bus, XM430, 1, (1000, 3000), BOUNDED, OFF);
            line(&mut bus)
                .reply_to(2, &p2_status(1, 0, &[7]))
                .reply_to(4, &p2_status(1, 0, &[]));
            let mut xm = bus.actuator(id(1)).unwrap();
            let mut out = [0; 1];
            assert_eq!(xm.read_raw(11, &mut out), Ok(()));
            assert_eq!(out, [7]);
            assert_eq!(xm.write_raw(11, &[4]), Ok(()));
            let read = p2_read(1, 11, 1);
            let write = p2_write(1, 11, &[4]);
            assert_eq!(
                sent(&bus),
                concat(&[read.clone(), read, write.clone(), write])
            );
        }
    }

    #[cfg(feature = "ax")]
    mod ax {
        use crate::bus::testing::*;
        use crate::error::{Error, Instruction, Request, ValidationError};
        use crate::protocol::P1;
        use crate::transport::MockTime;
        use crate::{IdSet, ModelNumber};

        /// FR-2.7, FR-6.5, FR-8.1: an actuator held `Faulted` at Setup is
        /// reached with `write_raw`, cleared, then enabled.
        #[test]
        fn faulted_at_setup_is_fixed_with_write_raw() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p1_status(1, 0x01, &[]))
                .reply_to(2, &p1_status(1, 0x01, &AX12.to_le_bytes()))
                .reply_to(3, &p1_status(1, 0x01, &0u16.to_le_bytes()))
                .reply_to(4, &p1_status(1, 0x01, &1023u16.to_le_bytes()))
                .reply_to(5, &p1_status(1, 0, &[]))
                .reply_to(6, &p1_status(1, 0, &[]))
                .reply_to(7, &p1_status(1, 0, &[]));
            let report = bus.declare([(id(1), ModelNumber(AX12))]).unwrap();
            assert_eq!(report.faulted, [id(1)].into_iter().collect::<IdSet>());
            let mark = mark(&bus);
            let mut ax = bus.actuator(id(1)).unwrap();
            assert_eq!(
                ax.enable_torque(),
                Err(ValidationError::ActuatorFaulted { id: id(1) }.into())
            );
            assert_eq!(ax.write_raw(12, &[95]), Ok(()));
            assert_eq!(ax.torque_state(), FAULTED);
            assert_eq!(ax.clear_fault(), Ok(()));
            assert_eq!(ax.torque_state(), OFF);
            assert_eq!(ax.enable_torque(), Ok(()));
            assert_eq!(ax.torque_state(), ON);
            assert_eq!(
                sent_since(&bus, mark),
                concat(&[
                    p1_write(1, 12, &[95]),
                    p1_instruction(1, PING, &[]),
                    p1_write(1, 24, &[1]),
                ])
            );
        }

        #[test]
        fn p1_raw_round_trip_and_short_addresses() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            hold(&mut bus, AX12, 1, (0, 1023), BOUNDED, OFF);
            line(&mut bus)
                .reply_to(1, &p1_status(1, 0, &[]))
                .reply_to(2, &p1_status(1, 0, &[0x00, 0x02]));
            let mut ax = bus.actuator(id(1)).unwrap();
            assert_eq!(ax.write_raw(30, &[0x00, 0x02]), Ok(()));
            let mut out = [0; 2];
            assert_eq!(ax.read_raw(30, &mut out), Ok(()));
            assert_eq!(out, [0x00, 0x02]);
            assert_eq!(ax.torque_state(), OFF);
            let mark = mark(&bus);
            let mut ax = bus.actuator(id(1)).unwrap();
            let far = |instruction, len| -> Result<(), Error> {
                Err(ValidationError::AddressOutOfRange {
                    addr: 256,
                    request: Request::NONE
                        .instruction(instruction)
                        .id(id(1))
                        .addr(256)
                        .len(len),
                }
                .into())
            };
            assert_eq!(ax.write_raw(256, &[0]), far(Instruction::Write, 1));
            assert_eq!(ax.read_raw(256, &mut out), far(Instruction::Read, 2));
            assert_quiet(&bus, mark);
            assert_eq!(
                sent(&bus),
                concat(&[p1_write(1, 30, &[0x00, 0x02]), p1_read(1, 30, 2)])
            );
        }
    }
}
