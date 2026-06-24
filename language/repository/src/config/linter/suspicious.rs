use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Suspicious-category linter options.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LinterSuspiciousOptions {}
