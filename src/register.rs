//! Register locations inside a control table.

use crate::error::ValidationError;

/// Whether a register can be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "RW and Ram are built by X1's generated reg() calls"
)]
pub enum Access {
    /// Read only.
    R,
    /// Read and write.
    RW,
}

/// Which memory area of the control table a register sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "RW and Ram are built by X1's generated reg() calls"
)]
pub enum Area {
    /// Kept across power cycles; writable only with torque off.
    Eeprom,
    /// Lost at power off; resets to its default at power on.
    Ram,
}

/// Where a register lives in an actuator's control table.
///
/// Generated model types expose one of these per register as an associated
/// `const`. Users never handle them: wrapper methods do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code, reason = "fields read by bus-1")]
pub struct Register {
    /// Control-table address.
    pub(crate) addr: u16,
    /// Width in bytes: 1, 2 or 4 for every register the core code encodes.
    pub(crate) size: u8,
    /// Whether the register can be written.
    pub(crate) access: Access,
    /// The memory area the register sits in.
    pub(crate) area: Area,
}

impl Register {
    /// Register at byte `addr`, `size` bytes wide, with placeholder access
    /// ([`Access::R`]) and area ([`Area::Eeprom`]).
    ///
    /// Kept only so today's generated code builds; generated code builds
    /// registers with its own `reg()` helper instead.
    #[doc(hidden)]
    #[allow(
        dead_code,
        reason = "unused once X1 regenerates with reg(); removed in core-2"
    )]
    pub(crate) const fn new(addr: u16, size: u8) -> Self {
        Self {
            addr,
            size,
            access: Access::R,
            area: Area::Eeprom,
        }
    }

    /// Writes `raw` little-endian into `out[..size]` and returns those
    /// bytes. A value is never truncated.
    ///
    /// Size 1 takes `0..=255`, size 2 `0..=65535`, size 4 any `i32`.
    ///
    /// # Errors
    ///
    /// [`ValidationError::DoesNotFit`] if `raw` is outside the register's
    /// range, or the register is not 1, 2 or 4 bytes wide.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) fn encode(self, raw: i32, out: &mut [u8; 4]) -> Result<&[u8], ValidationError> {
        let does_not_fit = ValidationError::DoesNotFit {
            raw: raw.into(),
            size: self.size,
        };
        let fits = match self.size {
            1 => u8::try_from(raw).is_ok(),
            2 => u16::try_from(raw).is_ok(),
            4 => true,
            _ => false,
        };
        if !fits {
            return Err(does_not_fit);
        }
        *out = raw.to_le_bytes();
        out.get(..usize::from(self.size)).ok_or(does_not_fit)
    }

    /// Reads the register's value from its little-endian `bytes`.
    ///
    /// Sizes 1 and 2 are unsigned; size 4 is a two's-complement `i32`.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::DataLength`] if `bytes` is not `size` long.
    /// - [`ValidationError::DoesNotFit`] if the register is not 1, 2 or 4
    ///   bytes wide; `raw` is then the bytes read as an unsigned
    ///   little-endian number, saturated to `i64`.
    #[allow(dead_code, reason = "used by bus-1")]
    pub(crate) fn decode(self, bytes: &[u8]) -> Result<i32, ValidationError> {
        let expected = usize::from(self.size);
        if bytes.len() != expected {
            return Err(ValidationError::DataLength {
                expected,
                got: bytes.len(),
            });
        }
        match *bytes {
            [b0] => Ok(i32::from(b0)),
            [b0, b1] => Ok(i32::from(u16::from_le_bytes([b0, b1]))),
            [b0, b1, b2, b3] => Ok(i32::from_le_bytes([b0, b1, b2, b3])),
            _ => Err(ValidationError::DoesNotFit {
                raw: bytes.iter().rev().fold(0, |value: i64, &byte| {
                    value.saturating_mul(256).saturating_add(i64::from(byte))
                }),
                size: self.size,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use proptest::prelude::*;

    use super::*;

    #[test]
    fn shim_keeps_address_and_size() {
        let reg = Register::new(116, 4);
        assert_eq!(reg.addr, 116);
        assert_eq!(reg.size, 4);
        assert_eq!(reg.access, Access::R);
        assert_eq!(reg.area, Area::Eeprom);
    }

    #[test]
    fn access_and_area_are_comparable() {
        assert_ne!(Access::R, Access::RW);
        assert_ne!(Area::Eeprom, Area::Ram);
    }

    fn does_not_fit(raw: i32, size: u8) -> Result<&'static [u8], ValidationError> {
        Err(ValidationError::DoesNotFit {
            raw: raw.into(),
            size,
        })
    }

    #[test]
    fn encodes_2500_in_four_bytes() {
        let mut out = [0; 4];
        assert_eq!(
            Register::new(116, 4).encode(2500, &mut out),
            Ok(&[0xC4, 0x09, 0x00, 0x00][..])
        );
    }

    #[test]
    fn refuses_values_that_do_not_fit() {
        let mut out = [0; 4];
        assert_eq!(
            Register::new(116, 2).encode(70_000, &mut out),
            does_not_fit(70_000, 2)
        );
        for raw in [-1, 256, i32::MIN, i32::MAX] {
            assert_eq!(
                Register::new(64, 1).encode(raw, &mut out),
                does_not_fit(raw, 1)
            );
        }
        for raw in [-1, 65_536] {
            assert_eq!(
                Register::new(30, 2).encode(raw, &mut out),
                does_not_fit(raw, 2)
            );
        }
        for size in [0, 3, 5, 8] {
            for raw in [0, 1, -1] {
                assert_eq!(
                    Register::new(0, size).encode(raw, &mut out),
                    does_not_fit(raw, size)
                );
            }
        }
    }

    #[test]
    fn encode_writes_size_bytes() {
        let mut out = [0; 4];
        for size in [1, 2, 4] {
            assert_eq!(
                Register::new(0, size).encode(1, &mut out).map(<[u8]>::len),
                Ok(usize::from(size))
            );
        }
        let cases: [(u8, i32, &[u8]); 6] = [
            (1, 255, &[0xFF]),
            (1, 0, &[0x00]),
            (2, 65_535, &[0xFF, 0xFF]),
            (2, 1023, &[0xFF, 0x03]),
            (4, i32::MIN, &[0x00, 0x00, 0x00, 0x80]),
            (4, i32::MAX, &[0xFF, 0xFF, 0xFF, 0x7F]),
        ];
        for (size, raw, bytes) in cases {
            assert_eq!(Register::new(0, size).encode(raw, &mut out), Ok(bytes));
        }
    }

    #[test]
    fn decode_sign_rules() {
        assert_eq!(Register::new(132, 2).decode(&[0xFF, 0xFF]), Ok(65_535));
        assert_eq!(
            Register::new(132, 4).decode(&[0xFF, 0xFF, 0xFF, 0xFF]),
            Ok(-1)
        );
        assert_eq!(Register::new(146, 1).decode(&[0x80]), Ok(128));
        assert_eq!(
            Register::new(132, 4).decode(&[0xC4, 0x09, 0x00, 0x00]),
            Ok(2500)
        );
    }

    #[test]
    fn decode_checks_length() {
        assert_eq!(
            Register::new(132, 4).decode(&[0; 2]),
            Err(ValidationError::DataLength {
                expected: 4,
                got: 2
            })
        );
        assert_eq!(
            Register::new(36, 2).decode(&[0; 3]),
            Err(ValidationError::DataLength {
                expected: 2,
                got: 3
            })
        );
    }

    #[test]
    fn decode_refuses_other_sizes() {
        assert_eq!(
            Register::new(0, 3).decode(&[0x01, 0x02, 0x03]),
            Err(ValidationError::DoesNotFit {
                raw: 0x03_0201,
                size: 3
            })
        );
        assert!(matches!(
            Register::new(0, 8).decode(&[0xFF; 8]),
            Err(ValidationError::DoesNotFit { size: 8, .. })
        ));
        assert!(matches!(
            Register::new(0, 0).decode(&[]),
            Err(ValidationError::DoesNotFit { raw: 0, size: 0 })
        ));
    }

    proptest! {
        #[test]
        fn round_trip_when_it_fits(
            raw in any::<i32>(),
            size in prop::sample::select(std::vec![1u8, 2, 4]),
        ) {
            let register = Register::new(0, size);
            let mut out = [0; 4];
            if let Ok(bytes) = register.encode(raw, &mut out) {
                prop_assert_eq!(register.decode(bytes), Ok(raw));
            }
        }

        #[test]
        fn decode_never_panics(
            bytes in prop::collection::vec(any::<u8>(), 0..12),
            size in any::<u8>(),
        ) {
            let _ = Register::new(0, size).decode(&bytes);
        }
    }
}
