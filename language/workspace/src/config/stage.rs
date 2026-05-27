use serde::{Deserialize, Serialize};

/// Package release stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    /// Experimental package surface.
    Experimental,
    /// Alpha package surface.
    Alpha,
    /// Beta package surface.
    Beta,
    /// Stable package surface.
    Stable,
}

impl Stage {
    /// Return the canonical stage name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Alpha => "alpha",
            Self::Beta => "beta",
            Self::Stable => "stable",
        }
    }
}
