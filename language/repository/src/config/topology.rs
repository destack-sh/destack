use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Package topology definition.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    /// Declared topology entities.
    pub entities: IndexMap<String, TopologyEntity>,
    /// Declared topology edges.
    pub edges: IndexMap<String, TopologyEdge>,
}

/// One declared topology entity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct TopologyEntity {
    /// Stable entity kind identifier.
    pub kind: String,
    /// Stable entity name.
    pub name: Option<String>,
    /// Entity labels.
    pub labels: IndexMap<String, String>,
}

/// One declared topology edge.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct TopologyEdge {
    /// Stable edge kind identifier.
    pub kind: String,
    /// Stable edge name.
    pub name: Option<String>,
    /// Source node name.
    pub from: String,
    /// Target node name.
    pub to: String,
    /// Edge labels.
    pub labels: IndexMap<String, String>,
}
