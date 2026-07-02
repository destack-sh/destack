use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{CheckState, PlaceUse, TemplatePass, WalkState};

impl CheckState<'_> {
    /// Declare every declaration template in one module.
    ///
    /// Example:
    /// ```ds
    /// class Box<T> {}
    /// ```
    pub(in crate::check) fn declare_module_templates(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // bind nominal references before templates can read them
        self.bind_module_reference_types(module)?;

        self.visit_module_templates(module, TemplatePass::Declare)
    }

    /// Walk every declared template's bounds, defaults, and predicates.
    pub(in crate::check) fn walk_module_templates(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        self.visit_module_templates(module, TemplatePass::Walk)
    }

    /// Visit every declaration template in one module.
    fn visit_module_templates(
        &mut self,
        module: ModuleId,
        pass: TemplatePass,
    ) -> CompilerResult<()> {
        let input = self.module(module);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));

        let mut walk = WalkState::new(module, tree, self);

        // visit declaration templates before any body walks
        for root in &expanded.roots {
            walk.visit_expression_templates(*root, tree.get(*root), pass)?;
        }
        walk.commit();

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
            walk.walk_value_expression(*root, PlaceUse::Read)?;
        }
        walk.commit();

        Ok(())
    }
}
