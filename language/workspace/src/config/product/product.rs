use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Policy;

use super::App;

/// Product assembled from one or more build targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    /// Target names keyed by product role.
    pub targets: BTreeMap<String, String>,
    /// App declaration used for host integration.
    pub app: App,
    /// Product policy declarations and rules.
    pub policy: Policy,
}
