use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tspp_artifact::Stability;

use crate::Policy;

use super::App;

/// Product assembled from one or more build targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    /// Stability promised by this product.
    pub stability: Option<Stability>,
    /// Active source graph modes for this product.
    pub modes: Vec<String>,
    /// Active source graph roles for this product.
    pub roles: Vec<String>,
    /// Active source graph features for this product.
    pub features: Vec<String>,
    /// Active source graph tags for this product.
    pub tags: Vec<String>,
    /// Target names keyed by product role.
    pub targets: BTreeMap<String, String>,
    /// App declaration used for host integration.
    pub app: App,
    /// Product policy declarations and rules.
    pub policy: Policy,
}
