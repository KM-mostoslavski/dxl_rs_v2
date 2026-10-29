//! Register locations inside a control table.

use crate::error::ValidationError;

/// Whether a register can be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(any(
        feature = "ax",
        feature = "mx",
        feature = "rx",
        feature = "x-series",
        feature = "y-series",
        feature = "pro"
    )),
    allow(dead_code, reason = "only generated models construct it")
)]
pub enum Access {
    /// Read only.
    R,
    /// Read and write.
    RW,
}

/// Which memory area of the control table a register sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(any(
        feature = "ax",
        feature = "mx",
        feature = "rx",
        feature = "x-series",
        feature = "y-series",
        feature = "pro"
    )),
    allow(dead_code, reason = "only generated models construct it")
)]
pub enum Area {
    /// Kept across power cycles; writable only with torque off.
    Eeprom,
    /// Lost at power off; resets to its default at power on.
    Ram,
    /// Kept across power cycles and writable with torque on (Y series, e-Manual
    /// <https://emanual.robotis.com/docs/en/dxl/y/ym070-210-m001-rh/>).
    #[cfg_attr(
        not(feature = "y-series"),
        allow(dead_code, reason = "built only by the Y-series generated models")
    )]
    Hybrid,
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

    /// XM430-W350 Goal Position(116) (e-Manual).
    const GOAL_POSITION: Register = Register {
        addr: 116,
        size: 4,
        access: Access::RW,
        area: Area::Ram,
    };

    /// XM430-W350 Torque Enable(64).
    const TORQUE_ENABLE: Register = Register {
        addr: 64,
        size: 1,
        access: Access::RW,
        area: Area::Ram,
    };

    /// XM430-W350 Present Position(132).
    const PRESENT_POSITION: Register = Register {
        addr: 132,
        size: 4,
        access: Access::R,
        area: Area::Ram,
    };

    /// XM430-W350 Present Temperature(146).
    const PRESENT_TEMPERATURE: Register = Register {
        addr: 146,
        size: 1,
        access: Access::R,
        area: Area::Ram,
    };

    /// XM430-W350 PWM Limit(36).
    const PWM_LIMIT: Register = Register {
        addr: 36,
        size: 2,
        access: Access::RW,
        area: Area::Eeprom,
    };

    /// AX-12A Goal Position(30).
    const AX_GOAL_POSITION: Register = Register {
        addr: 30,
        size: 2,
        access: Access::RW,
        area: Area::Ram,
    };

    /// A register `size` bytes wide; `encode` and `decode` read only the size.
    const fn sized(size: u8) -> Register {
        Register {
            addr: 0,
            size,
            access: Access::RW,
            area: Area::Ram,
        }
    }

    #[test]
    fn literal_keeps_every_field() {
        let Register {
            addr,
            size,
            access,
            area,
        } = GOAL_POSITION;
        assert_eq!((addr, size, access, area), (116, 4, Access::RW, Area::Ram));
        assert_ne!(GOAL_POSITION, PRESENT_POSITION);
        assert_eq!(sized(4).size, 4);
    }

    #[test]
    fn access_and_area_are_comparable() {
        assert_ne!(Access::R, Access::RW);
        assert_ne!(Area::Eeprom, Area::Ram);
        assert_ne!(Area::Hybrid, Area::Ram);
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
            GOAL_POSITION.encode(2500, &mut out),
            Ok(&[0xC4, 0x09, 0x00, 0x00][..])
        );
    }

    #[test]
    fn refuses_values_that_do_not_fit() {
        let mut out = [0; 4];
        assert_eq!(sized(2).encode(70_000, &mut out), does_not_fit(70_000, 2));
        for raw in [-1, 256, i32::MIN, i32::MAX] {
            assert_eq!(TORQUE_ENABLE.encode(raw, &mut out), does_not_fit(raw, 1));
        }
        for raw in [-1, 65_536] {
            assert_eq!(AX_GOAL_POSITION.encode(raw, &mut out), does_not_fit(raw, 2));
        }
        for size in [0, 3, 5, 8] {
            for raw in [0, 1, -1] {
                assert_eq!(sized(size).encode(raw, &mut out), does_not_fit(raw, size));
            }
        }
    }

    #[test]
    fn encode_writes_size_bytes() {
        let mut out = [0; 4];
        for size in [1, 2, 4] {
            assert_eq!(
                sized(size).encode(1, &mut out).map(<[u8]>::len),
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
            assert_eq!(sized(size).encode(raw, &mut out), Ok(bytes));
        }
    }

    #[test]
    fn decode_sign_rules() {
        assert_eq!(sized(2).decode(&[0xFF, 0xFF]), Ok(65_535));
        assert_eq!(PRESENT_POSITION.decode(&[0xFF, 0xFF, 0xFF, 0xFF]), Ok(-1));
        assert_eq!(PRESENT_TEMPERATURE.decode(&[0x80]), Ok(128));
        assert_eq!(PRESENT_POSITION.decode(&[0xC4, 0x09, 0x00, 0x00]), Ok(2500));
    }

    #[test]
    fn decode_checks_length() {
        assert_eq!(
            PRESENT_POSITION.decode(&[0; 2]),
            Err(ValidationError::DataLength {
                expected: 4,
                got: 2
            })
        );
        assert_eq!(
            PWM_LIMIT.decode(&[0; 3]),
            Err(ValidationError::DataLength {
                expected: 2,
                got: 3
            })
        );
    }

    #[test]
    fn decode_refuses_other_sizes() {
        assert_eq!(
            sized(3).decode(&[0x01, 0x02, 0x03]),
            Err(ValidationError::DoesNotFit {
                raw: 0x03_0201,
                size: 3
            })
        );
        assert!(matches!(
            sized(8).decode(&[0xFF; 8]),
            Err(ValidationError::DoesNotFit { size: 8, .. })
        ));
        assert!(matches!(
            sized(0).decode(&[]),
            Err(ValidationError::DoesNotFit { raw: 0, size: 0 })
        ));
    }

    proptest! {
        #[test]
        fn round_trip_when_it_fits(
            raw in any::<i32>(),
            size in prop::sample::select(std::vec![1u8, 2, 4]),
        ) {
            let register = sized(size);
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
            let _ = sized(size).decode(&bytes);
        }
    }
}
