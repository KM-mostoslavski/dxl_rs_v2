//! What every actuator model has in common.

use crate::Id;

/// Common interface of every actuator model.
///
/// Implemented by each generated model type and by [`ActuatorKind`], so code
/// that only needs the common features works with either.
///
/// [`ActuatorKind`]: crate::ActuatorKind
pub trait Actuator {
    /// Firmware ID of this actuator.
    fn id(&self) -> Id;

    /// Model number reported by the actuator (register 0).
    fn model_number(&self) -> u16;

    /// Display name, e.g. `"XM430-W350"`.
    fn name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActuatorKind, Error};

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    #[test]
    #[cfg(feature = "x-series")]
    fn builds_model_from_number() {
        let kind = ActuatorKind::new(1020, 45, id(3)).unwrap();
        assert!(matches!(kind, ActuatorKind::Xm430W350(_)));
        assert_eq!(kind.id(), id(3));
        assert_eq!(kind.model_number(), 1020);
        assert_eq!(kind.name(), "XM430-W350");
    }

    #[test]
    #[cfg(feature = "x-series")]
    fn picks_table_by_firmware() {
        let old = ActuatorKind::new(1190, 52, id(1)).unwrap();
        let new = ActuatorKind::new(1190, 53, id(1)).unwrap();
        assert!(matches!(old, ActuatorKind::Xl330M077Fw52(_)));
        assert!(matches!(new, ActuatorKind::Xl330M077(_)));
        assert_eq!(old.name(), new.name());
    }

    #[test]
    #[cfg(feature = "ax")]
    fn concrete_types_implement_actuator() {
        let ActuatorKind::Ax12a(ax) = ActuatorKind::new(12, 0, id(9)).unwrap() else {
            panic!("model 12 is an AX-12A");
        };
        assert_eq!(
            (ax.id(), ax.model_number(), ax.name()),
            (id(9), 12, "AX-12A")
        );
    }

    #[test]
    fn unknown_model_is_unsupported() {
        assert_eq!(
            ActuatorKind::new(9999, 0, id(1)),
            Err(Error::UnsupportedModel {
                model_number: 9999,
                feature: None
            })
        );
    }

    #[test]
    fn unsupported_model_message_names_the_feature() {
        extern crate std;
        use std::string::ToString;
        let with = Error::UnsupportedModel {
            model_number: 1020,
            feature: Some("x-series"),
        };
        let without = Error::UnsupportedModel {
            model_number: 9999,
            feature: None,
        };
        assert_eq!(
            with.to_string(),
            "unsupported model number 1020 (enable the `x-series` feature)"
        );
        assert_eq!(without.to_string(), "unsupported model number 9999");
    }
}
