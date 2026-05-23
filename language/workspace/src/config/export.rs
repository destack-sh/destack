use serde::{Deserialize, Serialize};

use super::{ConditionPredicate, ConditionSet};

/// Public package material declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Export {
    /// Exported material kind.
    pub kind: ExportKind,
    /// Package relative material path.
    pub path: String,
    /// Condition predicate required for this export.
    pub when: Option<ConditionPredicate>,
}

impl Export {
    /// Return whether this export is active for one condition set.
    pub fn matches(&self, conditions: &ConditionSet) -> bool {
        self.when
            .as_ref()
            .is_none_or(|predicate| predicate.matches(conditions))
    }
}

impl Default for Export {
    fn default() -> Self {
        Self {
            kind: ExportKind::Module,
            path: String::new(),
            when: None,
        }
    }
}

/// Public package material kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ExportKind {
    /// Source module.
    #[default]
    Module,
    /// Static or generated asset.
    Asset,
    /// Template material.
    Template,
    /// Reflect-derived schema material.
    Schema,
    /// Simulation scenario or model material.
    Simulation,
    /// Service definition material.
    Service,
    /// Application definition material.
    App,
}
