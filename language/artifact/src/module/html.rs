use destack_html::{Document, LocalNodeId, NodeTree};
use serde::{Deserialize, Serialize};

/// One parsed HTML module payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Html {
    /// The HTML node tree.
    pub tree: NodeTree,
    /// The root document node.
    pub document: LocalNodeId<Document>,
}
