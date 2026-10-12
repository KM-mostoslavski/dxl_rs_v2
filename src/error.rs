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
}

/// A value rejected before anything was sent on the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ValidationError {
    /// The ID is outside `0..=252`; see [`Id::MAX`](crate::Id::MAX).
    #[error("invalid actuator id {0}: must be in 0..=252")]
    InvalidId(u8),
}
