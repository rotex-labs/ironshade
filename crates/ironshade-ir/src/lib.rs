//! Ironshade Graph IR. M0 exposes an empty module for HAL integration.

/// A module of Ironshade Graph IR.
#[derive(Clone, Debug, Default)]
pub struct Module {
    _private: (),
}

impl Module {
    /// Constructs an empty module.
    pub fn empty() -> Self {
        Self::default()
    }
}
