use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// The condition name of an export case that always applies.
pub const DEFAULT_EXPORT_CONDITION: &str = "default";

/// One package export: a package-relative path, or paths chosen by condition name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum Export {
    /// One package-relative path.
    Path(String),
    /// Package-relative paths by condition name, the first active condition selected.
    Conditions(IndexMap<String, String>),
}
