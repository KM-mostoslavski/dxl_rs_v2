//! Actuator firmware IDs.

use crate::error::{Error, ValidationError};

/// Firmware ID of an actuator, guaranteed to be in `0..=252`.
///
/// This is the ID set in the actuator's EEPROM, not a position in the bus.
/// See [`Id::MAX`] for why the range stops at 252.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u8);

impl Id {
    /// Highest ID an actuator can have on either protocol.
    ///
    /// Protocol 2 allows 0..=252; Protocol 1 allows 0..=253. 254 is broadcast
    /// on both. One range for both protocols keeps `Id` free of a protocol
    /// type parameter, at the cost of ID 253 on Protocol 1.
    ///
    /// Sources: <https://emanual.robotis.com/docs/en/dxl/protocol2/>,
    /// <https://emanual.robotis.com/docs/en/dxl/protocol1/>
    pub const MAX: u8 = 252;

    /// Creates an `Id`, or returns [`ValidationError::InvalidId`] if `id` is
    /// above [`Id::MAX`].
    ///
    /// Usable in `const` context, so IDs known at compile time are checked
    /// at compile time.
    pub const fn new(id: u8) -> Result<Self, Error> {
        if id <= Self::MAX {
            Ok(Self(id))
        } else {
            Err(Error::Validation(ValidationError::InvalidId(id)))
        }
    }

    /// Returns the ID as a plain `u8`.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Id {
    type Error = Error;

    /// Same as [`Id::new`].
    fn try_from(id: u8) -> Result<Self, Error> {
        Self::new(id)
    }
}

impl From<Id> for u8 {
    /// Same as [`Id::get`].
    fn from(id: Id) -> u8 {
        id.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_full_valid_range() {
        for raw in 0..=Id::MAX {
            assert_eq!(Id::new(raw).unwrap().get(), raw);
        }
    }

    #[test]
    fn rejects_reserved_and_out_of_range() {
        for raw in [253, 254, 255] {
            assert_eq!(
                Id::new(raw),
                Err(Error::Validation(ValidationError::InvalidId(raw)))
            );
        }
    }

    #[test]
    fn converts_both_ways() {
        let id = Id::try_from(7).unwrap();
        assert_eq!(u8::from(id), 7);
        assert!(Id::try_from(253).is_err());
    }

    #[test]
    fn checks_at_compile_time() {
        const ID: Id = match Id::new(42) {
            Ok(id) => id,
            Err(_) => panic!("invalid id"),
        };
        assert_eq!(ID.get(), 42);
    }

    #[test]
    fn error_message_names_the_id() {
        extern crate std;
        use std::string::ToString;
        let err = Id::new(254).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid actuator id 254: must be in 0..=252"
        );
    }
}
