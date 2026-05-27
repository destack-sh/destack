use destack_source::ModuleId;
use std::sync::Arc;

use crate::check::CheckState;

impl CheckState<'_> {
    /// Visit DIR and collect check constraints and obligations.
    pub(in crate::check) fn walk_module(&mut self, module: ModuleId) {
        let input = self.input(module);
        let parsed = Arc::clone(&input.parsed);
        let expanded = Arc::clone(&input.expanded);
        let tree = &parsed.tree;

        // walk expanded roots in semantic context
        for root in &expanded.roots {
            self.walk_expression(tree, *root, tree.get(*root));
        }
    }
}
