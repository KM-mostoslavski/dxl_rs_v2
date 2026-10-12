//! Crate errors, split by where they come from.

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
        model_number: u16,
        feature: Option<&'static str>,
    },
}

/// Formats the "enable this feature" hint of [`Error::UnsupportedModel`].
struct FeatureHint(Option<&'static str>);

impl core::fmt::Display for FeatureHint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
}
