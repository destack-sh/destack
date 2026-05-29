use destack_source::ModuleId;
use std::sync::Arc;

use crate::check::{CheckState, FlowState};

impl CheckState<'_> {
    /// Visit DIR and collect check constraints and obligations.
    pub(in crate::check) fn walk_module(&mut self, module: ModuleId) {
        let input = self.module(module);
        let parsed = Arc::clone(&input.parsed);
        let expanded = Arc::clone(&input.expanded);
        let tree = &parsed.tree;

        self.flow.insert(module, FlowState::default());

        // walk expanded roots in semantic context
        for root in &expanded.roots {
            self.walk_expression(tree, *root, tree.get(*root));
        }

        self.flow.shift_remove(&module);
    }
}
