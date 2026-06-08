use std::sync::Arc;

use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{CheckState, WalkState};

impl CheckState<'_> {
    /// Visit DIR and collect check constraints and obligations.
    ///
    /// Example:
    /// ```ds
    /// export function value(): number { 1 }
    /// ```
    pub(in crate::check) fn walk_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        let input = self.module(module);
        let parsed = Arc::clone(&input.parsed);
        let expanded = Arc::clone(&input.expanded);
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));

        let mut walk = WalkState::new(module, tree, self);

        // walk visible roots in semantic context
        for root in &expanded.roots {
            walk.walk_expression(*root, tree.get(*root))?;
        }

        Ok(())
    }
}
