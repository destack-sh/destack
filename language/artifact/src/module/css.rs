use destack_css::print::print_stylesheet;
use destack_css::{LocalNodeId, Stylesheet, Tree};
use serde::{Deserialize, Serialize};

/// One parsed CSS module payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Css {
    /// The CSS tree.
    pub tree: Tree,
    /// The root stylesheet node.
    pub stylesheet: LocalNodeId<Stylesheet>,
}

impl Css {
    /// Return this stylesheet as canonical CSS source.
    pub fn to_source(&self) -> String {
        print_stylesheet(&self.tree, self.stylesheet)
    }
}
