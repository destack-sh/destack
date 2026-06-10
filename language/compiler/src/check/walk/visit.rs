use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{CheckState, WalkState};

impl CheckState<'_> {
    /// Declare DIR headers needed before body checking.
    ///
    /// Example:
    /// ```ds
    /// class Box<T> {}
    /// ```
    pub(in crate::check) fn declare_module_headers(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        let input = self.module(module);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));

        let mut walk = WalkState::new(module, tree, self);

        // declare generic headers before bodies
        for root in &expanded.roots {
            walk.declare_expression_header(*root, tree.get(*root))?;
        }

        Ok(())
    }

    /// Visit DIR and collect check constraints and obligations.
    ///
    /// Example:
    /// ```ds
    /// export function value(): number { 1 }
    /// ```
    pub(in crate::check) fn walk_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        let input = self.module(module);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));

        let mut walk = WalkState::new(module, tree, self);

        // walk expanded roots
        for root in &expanded.roots {
            walk.walk_expression(*root, tree.get(*root))?;
        }

        Ok(())
    }
}
