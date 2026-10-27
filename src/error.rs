//! Crate errors, split by where they come from.

use core::fmt;

use crate::Id;

/// Every error this crate returns.
///
/// Each variant is one category from `docs/ARCHITECTURE.md`; match on the
/// category first, then on its inner error for the detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A value was rejected before anything was sent on the bus.
    #[error(transparent)]
    Validation(#[from] ValidationError),

    /// An actuator reported a model number this build can't drive.
    ///
    /// `feature` names the Cargo feature that would add the model, or is
    /// `None` if no feature does.
    #[error("unsupported model number {model_number}{}", FeatureHint(*feature))]
    UnsupportedModel {
        /// The model number the actuator reported.
        model_number: u16,
        /// The Cargo feature that adds this model, if any.
        feature: Option<&'static str>,
    },

    /// The serial line failed, or a reply did not arrive in time.
    #[error(transparent)]
    Transport(#[from] TransportError),

    /// Bytes arrived but did not form the expected status packet.
    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    /// An actuator answered, and its status packet reports an error.
    #[error(transparent)]
    Status(#[from] StatusError),
}

/// Formats the "enable this feature" hint of [`Error::UnsupportedModel`].
struct FeatureHint(Option<&'static str>);

impl fmt::Display for FeatureHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(feature) => write!(f, " (enable the `{feature}` feature)"),
            None => Ok(()),
        }
    }
}

/// A value rejected before anything was sent on the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ValidationError {
    /// The ID is outside `0..=252`; see [`Id::MAX`](crate::Id::MAX).
    #[error("invalid actuator id {0}: must be in 0..=252")]
    InvalidId(u8),

    /// Protocol 1 addresses are one byte; `addr` does not fit.
    #[error("address {addr} does not fit Protocol 1's one-byte address field")]
    AddressOutOfRange {
        /// The address that was asked for.
        addr: u16,
    },

    /// The packet's length field cannot describe `len` bytes; `max` is the
    /// protocol's limit.
    #[error("packet of {len} bytes exceeds the protocol limit of {max}")]
    PacketTooLarge {
        /// The length that was asked for, in bytes.
        len: usize,
        /// The largest length the protocol allows, in bytes.
        max: usize,
    },

    /// A data slice does not have the size the instruction declares.
    #[error("data is {got} bytes, expected {expected}")]
    DataLength {
        /// The size the instruction declares, in bytes.
        expected: usize,
        /// The size of the slice that was passed, in bytes.
        got: usize,
    },
}

/// The serial line failed, or a reply did not arrive in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TransportError {
    /// The transport returned an error other than a timeout.
    #[error("serial transport error: {0}")]
    Io(embedded_io::ErrorKind),

    /// No complete reply arrived before the deadline. `id` is the actuator
    /// that was expected to answer, when there was exactly one.
    #[error("timed out waiting for {}", WaitingFor(*id))]
    Timeout {
        /// The actuator that was expected to answer, if only one was.
        id: Option<Id>,
    },
}

/// Formats who a [`TransportError::Timeout`] was waiting for.
struct WaitingFor(Option<Id>);

impl fmt::Display for WaitingFor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(id) => write!(f, "actuator {}", id.get()),
            None => f.write_str("a reply"),
        }
    }
}

/// Bytes arrived but did not form the expected status packet.
///
/// IDs are raw bytes here: a corrupted packet may carry an ID that is not a
/// valid [`Id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    /// A Protocol 1 status packet failed its checksum.
    #[error("checksum mismatch in status packet from id {id}")]
    Checksum {
        /// The raw ID byte of the packet.
        id: u8,
    },

    /// A Protocol 2 status packet failed its CRC.
    #[error("CRC mismatch in status packet from id {id}")]
    Crc {
        /// The raw ID byte of the packet.
        id: u8,
    },

    /// A valid status packet came from another actuator than the one asked.
    #[error("expected a reply from actuator {}, got id {got}", expected.get())]
    WrongId {
        /// The actuator the instruction was sent to.
        expected: Id,
        /// The raw ID byte of the reply.
        got: u8,
    },

    /// A valid status packet carried a different amount of data than asked.
    #[error("expected {expected} data bytes in the reply, got {got}")]
    WrongLength {
        /// The number of data bytes asked for.
        expected: usize,
        /// The number of data bytes the reply carried.
        got: usize,
    },

    /// A length field that no valid packet can have, or one longer than
    /// the expected reply can be (a corrupted length that would otherwise
    /// swallow the packets after it).
    #[error("malformed status packet")]
    Malformed,
}

/// An actuator's status packet reported an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("actuator {} reported {report}", id.get())]
#[non_exhaustive]
pub struct StatusError {
    /// The actuator that reported it.
    pub id: Id,
    /// What its error byte says, decoded per protocol.
    pub report: StatusReport,
}

impl StatusError {
    /// Whether the report is a hardware fault, after which the actuator
    /// should be treated as faulted and its torque as off.
    ///
    /// - Protocol 2: the alert bit (bit 7) is set. The e-Manual defines it as
    ///   "a problem with the device", detailed in Hardware Error Status
    ///   (<https://emanual.robotis.com/docs/en/dxl/protocol2/#error>).
    /// - Protocol 1: Overload, Overheating or Input Voltage is set. The other
    ///   flags report a rejected instruction, not the hardware
    ///   (<https://emanual.robotis.com/docs/en/dxl/protocol1/#error>).
    pub fn is_hardware_fault(&self) -> bool {
        match self.report {
            StatusReport::P1(flags) => {
                flags.contains(P1Flag::Overload)
                    || flags.contains(P1Flag::Overheating)
                    || flags.contains(P1Flag::InputVoltage)
            }
            StatusReport::P2 { alert, .. } => alert,
        }
    }
}

/// The error byte of a status packet, decoded per protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StatusReport {
    /// Protocol 1: one flag per bit.
    P1(P1Flags),
    /// Protocol 2: the alert bit and an optional error code.
    P2 {
        /// Bit 7: a hardware fault; Hardware Error Status says which.
        alert: bool,
        /// Bits 0 to 6, or `None` when they are all zero.
        code: Option<P2Code>,
    },
}

impl fmt::Display for StatusReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            StatusReport::P1(flags) => write!(f, "{flags}"),
            StatusReport::P2 { alert, code } => match (alert, code) {
                (true, Some(code)) => write!(f, "a hardware alert and {code}"),
                (true, None) => f.write_str("a hardware alert"),
                (false, Some(code)) => write!(f, "{code}"),
                (false, None) => f.write_str("no error"),
            },
        }
    }
}

/// A Protocol 2 error code (bits 0..=6 of the error byte).
///
/// Source: <https://emanual.robotis.com/docs/en/dxl/protocol2/#error>.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum P2Code {
    /// 0x01: the instruction packet could not be processed.
    ResultFail,
    /// 0x02: undefined instruction, or Action without Reg Write.
    Instruction,
    /// 0x03: the actuator saw a CRC mismatch in our packet.
    Crc,
    /// 0x04: the value is outside the register's range.
    DataRange,
    /// 0x05: the data is shorter than the register.
    DataLength,
    /// 0x06: the value is outside the configured limit.
    DataLimit,
    /// 0x07: read-only, write-only or undefined address, or an EEPROM write
    /// while torque is on.
    Access,
    /// A code the e-Manual does not define.
    Unknown(u8),
}

impl P2Code {
    /// Decodes the error code from a Protocol 2 error byte, ignoring the
    /// alert bit. `None` when the code is 0.
    pub const fn from_bits(byte: u8) -> Option<Self> {
        Some(match byte & 0x7F {
            0 => return None,
            1 => Self::ResultFail,
            2 => Self::Instruction,
            3 => Self::Crc,
            4 => Self::DataRange,
            5 => Self::DataLength,
            6 => Self::DataLimit,
            7 => Self::Access,
            other => Self::Unknown(other),
        })
    }
}

impl fmt::Display for P2Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResultFail => f.write_str("result fail"),
            Self::Instruction => f.write_str("instruction error"),
            Self::Crc => f.write_str("CRC error"),
            Self::DataRange => f.write_str("data range error"),
            Self::DataLength => f.write_str("data length error"),
            Self::DataLimit => f.write_str("data limit error"),
            Self::Access => f.write_str("access error"),
            Self::Unknown(code) => write!(f, "unknown error code {code}"),
        }
    }
}

/// One Protocol 1 error flag.
///
/// Source: <https://emanual.robotis.com/docs/en/dxl/protocol1/#error>.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum P1Flag {
    /// Bit 0: input voltage out of range.
    InputVoltage,
    /// Bit 1: goal position outside the angle limits.
    AngleLimit,
    /// Bit 2: internal temperature too high.
    Overheating,
    /// Bit 3: instruction value out of range.
    Range,
    /// Bit 4: the actuator saw a checksum mismatch in our packet.
    Checksum,
    /// Bit 5: the load cannot be controlled with the set torque.
    Overload,
    /// Bit 6: undefined instruction, or Action without Reg Write.
    Instruction,
}

impl P1Flag {
    const ALL: [Self; 7] = [
        Self::InputVoltage,
        Self::AngleLimit,
        Self::Overheating,
        Self::Range,
        Self::Checksum,
        Self::Overload,
        Self::Instruction,
    ];

    const fn bit(self) -> u8 {
        1 << self as u8
    }

    const fn name(self) -> &'static str {
        match self {
            Self::InputVoltage => "input voltage error",
            Self::AngleLimit => "angle limit error",
            Self::Overheating => "overheating",
            Self::Range => "range error",
            Self::Checksum => "checksum error",
            Self::Overload => "overload",
            Self::Instruction => "instruction error",
        }
    }
}

impl fmt::Display for P1Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A Protocol 1 error byte: any combination of [`P1Flag`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct P1Flags(pub u8);

impl P1Flags {
    /// Whether `flag` is set.
    pub const fn contains(self, flag: P1Flag) -> bool {
        self.0 & flag.bit() != 0
    }

    /// Every set flag, lowest bit first.
    pub fn iter(self) -> impl Iterator<Item = P1Flag> {
        P1Flag::ALL
            .into_iter()
            .filter(move |&flag| self.contains(flag))
    }
}

impl fmt::Display for P1Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unknown = self.0 & !P1Flag::ALL.iter().fold(0, |bits, flag| bits | flag.bit());
        let mut flags = self.iter();
        match flags.next() {
            None if unknown == 0 => return f.write_str("no error"),
            None => return write!(f, "unknown error bits {unknown:#04X}"),
            Some(first) => {
                write!(f, "{first}")?;
                flags.try_for_each(|flag| write!(f, ", {flag}"))?;
            }
        }
        if unknown != 0 {
            write!(f, ", unknown error bits {unknown:#04X}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::string::ToString;

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    #[test]
    fn validation_messages() {
        let cases = [
            (
                ValidationError::AddressOutOfRange { addr: 300 },
                "address 300 does not fit Protocol 1's one-byte address field",
            ),
            (
                ValidationError::PacketTooLarge { len: 300, max: 255 },
                "packet of 300 bytes exceeds the protocol limit of 255",
            ),
            (
                ValidationError::DataLength {
                    expected: 4,
                    got: 2,
                },
                "data is 2 bytes, expected 4",
            ),
        ];
        for (err, msg) in cases {
            assert_eq!(Error::from(err).to_string(), msg);
        }
    }

    #[test]
    fn transport_messages() {
        assert_eq!(
            Error::from(TransportError::Timeout { id: Some(id(3)) }).to_string(),
            "timed out waiting for actuator 3"
        );
        assert_eq!(
            TransportError::Timeout { id: None }.to_string(),
            "timed out waiting for a reply"
        );
        assert!(
            TransportError::Io(embedded_io::ErrorKind::BrokenPipe)
                .to_string()
                .starts_with("serial transport error: ")
        );
    }

    #[test]
    fn protocol_messages() {
        let cases = [
            (
                ProtocolError::Checksum { id: 1 },
                "checksum mismatch in status packet from id 1",
            ),
            (
                ProtocolError::Crc { id: 2 },
                "CRC mismatch in status packet from id 2",
            ),
            (
                ProtocolError::WrongId {
                    expected: id(1),
                    got: 7,
                },
                "expected a reply from actuator 1, got id 7",
            ),
            (
                ProtocolError::WrongLength {
                    expected: 4,
                    got: 0,
                },
                "expected 4 data bytes in the reply, got 0",
            ),
            (ProtocolError::Malformed, "malformed status packet"),
        ];
        for (err, msg) in cases {
            assert_eq!(Error::from(err).to_string(), msg);
        }
    }

    #[test]
    fn p2_codes_decode_and_ignore_alert() {
        assert_eq!(P2Code::from_bits(0), None);
        assert_eq!(P2Code::from_bits(0x80), None);
        assert_eq!(P2Code::from_bits(0x01), Some(P2Code::ResultFail));
        assert_eq!(P2Code::from_bits(0x85), Some(P2Code::DataLength));
        assert_eq!(P2Code::from_bits(0x07), Some(P2Code::Access));
        assert_eq!(P2Code::from_bits(0x09), Some(P2Code::Unknown(9)));
    }

    #[test]
    fn p1_flags_iterate_every_set_bit() {
        let flags = P1Flags(0x24);
        let set: std::vec::Vec<_> = flags.iter().collect();
        assert_eq!(set, [P1Flag::Overheating, P1Flag::Overload]);
        assert_eq!(P1Flags(0).iter().count(), 0);
        assert_eq!(P1Flags(0x7F).iter().count(), 7);
    }

    #[test]
    fn hardware_faults_per_protocol() {
        let p1 = |byte| StatusError {
            id: id(1),
            report: StatusReport::P1(P1Flags(byte)),
        };
        assert!(p1(0x20).is_hardware_fault());
        assert!(p1(0x04).is_hardware_fault());
        assert!(p1(0x01).is_hardware_fault());
        assert!(!p1(0x08).is_hardware_fault());
        assert!(!p1(0x40).is_hardware_fault());

        let p2 = |alert, code| StatusError {
            id: id(1),
            report: StatusReport::P2 { alert, code },
        };
        assert!(p2(true, None).is_hardware_fault());
        assert!(!p2(false, Some(P2Code::Access)).is_hardware_fault());
    }

    #[test]
    fn status_messages_name_id_and_every_condition() {
        let p1 = StatusError {
            id: id(1),
            report: StatusReport::P1(P1Flags(0x24)),
        };
        assert_eq!(
            Error::from(p1).to_string(),
            "actuator 1 reported overheating, overload"
        );
        let p2 = |alert, code| {
            StatusError {
                id: id(5),
                report: StatusReport::P2 { alert, code },
            }
            .to_string()
        };
        assert_eq!(
            p2(true, Some(P2Code::DataLength)),
            "actuator 5 reported a hardware alert and data length error"
        );
        assert_eq!(p2(true, None), "actuator 5 reported a hardware alert");
        assert_eq!(
            p2(false, Some(P2Code::Unknown(9))),
            "actuator 5 reported unknown error code 9"
        );
        assert_eq!(p2(false, None), "actuator 5 reported no error");
        assert_eq!(P1Flags(0).to_string(), "no error");
        assert_eq!(P1Flags(0x80).to_string(), "unknown error bits 0x80");
        assert_eq!(
            P1Flags(0xA0).to_string(),
            "overload, unknown error bits 0x80"
        );
        for (code, name) in [
            (P2Code::ResultFail, "result fail"),
            (P2Code::Instruction, "instruction error"),
            (P2Code::Crc, "CRC error"),
            (P2Code::DataRange, "data range error"),
            (P2Code::DataLimit, "data limit error"),
            (P2Code::Access, "access error"),
        ] {
            assert_eq!(code.to_string(), name);
        }
        for flag in P1Flag::ALL {
            assert_eq!(flag.to_string(), flag.name());
        }
    }
}
