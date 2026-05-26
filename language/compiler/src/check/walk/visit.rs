use destack_dir as dir;
use std::sync::Arc;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Visit DIR and collect check constraints and obligations.
    pub(in crate::check) fn walk(&mut self) {
        let parsed = Arc::clone(&self.input.parsed);
        let expanded = Arc::clone(&self.input.expanded);
        let tree = &parsed.tree;

        // walk expanded roots in semantic context
        for root in &expanded.roots {
            self.walk_expression(tree, *root, tree.get(*root));
        }
    }
}

impl dir::NodeVisitor for CheckModuleState {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.input.options
    }
}
