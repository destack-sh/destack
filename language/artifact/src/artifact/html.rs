use destack_html::{Document, LocalNodeId, Tree};
use serde::{Deserialize, Serialize};

/// One parsed HTML module payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Html {
    /// The HTML tree.
    pub tree: Tree,
    /// The root document node.
    pub document: LocalNodeId<Document>,
}
