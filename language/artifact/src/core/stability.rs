use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Stability promised by one published package or product.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Stability {
    /// Experimental surface with no compatibility guarantee.
    Experimental,
    /// Alpha surface intended for early use.
    Alpha,
    /// Beta surface approaching compatibility guarantees.
    Beta,
    /// Stable surface governed by compatibility guarantees.
    Stable,
}

impl Stability {
    /// Return the canonical stability name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Alpha => "alpha",
            Self::Beta => "beta",
            Self::Stable => "stable",
        }
    }
}
