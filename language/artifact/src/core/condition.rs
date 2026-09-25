use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{Host, Platform, Runtime};

/// Active source graph and runtime selection conditions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConditionSet {
    /// Active source graph modes.
    pub modes: IndexSet<String>,
    /// Active source graph roles.
    pub roles: IndexSet<String>,
    /// Active optional features.
    pub features: IndexSet<String>,
    /// Active source graph tags.
    pub tags: IndexSet<String>,
    /// Active build target.
    pub target: Option<String>,
    /// Active product.
    pub product: Option<String>,
    /// Active product role.
    pub role: Option<String>,
    /// Labels contributed by active source graph conditions.
    pub labels: BTreeMap<String, Vec<String>>,
    /// Active target platform.
    pub platform: Platform,
    /// Active host environment.
    pub host: Host,
    /// Active runtime.
    pub runtime: Runtime,
}

impl Hash for ConditionSet {
    /// Hash the active condition axes in stable order.
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_condition_names(&self.modes, state);
        hash_condition_names(&self.roles, state);
        hash_condition_names(&self.features, state);
        hash_condition_names(&self.tags, state);
        self.target.hash(state);
        self.product.hash(state);
        self.role.hash(state);
        self.labels.hash(state);
        self.platform.hash(state);
        self.host.hash(state);
        self.runtime.hash(state);
    }
}

impl ConditionSet {
    /// Return whether this set contains one mode.
    pub fn contains_mode(&self, name: &str) -> bool {
        self.modes.contains(name)
    }

    /// Return whether this set contains one role.
    pub fn contains_role(&self, name: &str) -> bool {
        self.roles.contains(name)
    }

    /// Return whether this set contains one feature.
    pub fn contains_feature(&self, name: &str) -> bool {
        self.features.contains(name)
    }

    /// Return whether this set contains one tag.
    pub fn contains_tag(&self, name: &str) -> bool {
        self.tags.contains(name)
    }
}

/// Hash condition names in active order.
fn hash_condition_names<H: Hasher>(names: &IndexSet<String>, state: &mut H) {
    names.len().hash(state);
    for name in names {
        name.hash(state);
    }
}
