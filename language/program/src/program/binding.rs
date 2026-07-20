use destack_core::{SectionEntry, fnv1a_128};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Stable identifier for a runtime binding name.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct BindingId(pub u128);

impl BindingId {
    /// Build a binding id from a static binding name.
    pub const fn from_static_name(name: &'static str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }

    /// Build a binding id from a binding name.
    pub fn from_name(name: &str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }
}
