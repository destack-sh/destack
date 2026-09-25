use serde::{Deserialize, Serialize};
use tspp_core::StableHasher;
use tspp_serde::Reflect;

use crate::{ModuleId, ProfileId};

const COMPONENT_DOMAIN: &[u8] = b"tspp.source.component.v1";

/// Stable identifier for one source component.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
pub struct ComponentId(pub u128);

impl std::fmt::Debug for ComponentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "c{:032x}", self.0)
    }
}

impl std::fmt::Display for ComponentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "c{:032x}", self.0)
    }
}

impl ComponentId {
    /// Create a ComponentId from a raw hash value.
    pub const fn new(id: u128) -> Self {
        Self(id)
    }

    /// Create a ComponentId from one profile scoped module set.
    pub fn from_modules(profile: ProfileId, modules: impl IntoIterator<Item = ModuleId>) -> Self {
        let mut modules = modules.into_iter().collect::<Vec<_>>();
        modules.sort_unstable();
        modules.dedup();

        Self::from_sorted_modules(profile, modules.into_iter())
    }

    /// Create a ComponentId from one sorted profile scoped module set.
    pub fn from_sorted_modules(
        profile: ProfileId,
        modules: impl ExactSizeIterator<Item = ModuleId>,
    ) -> Self {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(COMPONENT_DOMAIN);
        hasher.update(&profile.raw().to_le_bytes());
        hasher.update(&(modules.len() as u64).to_le_bytes());

        // hash modules in canonical order
        for module in modules {
            hasher.update(&module.package_id.raw().to_le_bytes());
            hasher.update(&module.module_key.raw().to_le_bytes());
        }

        Self(hasher.finish_u128())
    }

    /// Return the raw stable id value.
    pub const fn raw(self) -> u128 {
        self.0
    }
}
