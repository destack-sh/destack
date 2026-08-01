use std::slice::from_ref;

use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Obligation, PlaceUse, Task, TemplatePass, WalkState, WellFormedTypeObligation,
};

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
        let tree = dir::View::with_patches(&parsed.tree, from_ref(&expanded.patch));

        let mut walk = WalkState::new(module, tree, self);

        // visit declaration templates before any body walks
        for root in &expanded.roots {
            walk.visit_expression_templates(*root, tree.get(*root), pass)?;
        }
        walk.commit()?;

        Ok(())
    }

    /// Walk one foreign member declaration on demand.
    pub(in crate::check) fn demand_module_declaration(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // demand needs every template walked; unloaded modules are sealed;
        //  declaration-level cycles resolve inside the demanded frame
        if !self.templates_ready || !self.is_own_module(symbol.module_id) {
            return Ok(());
        }

        // skip symbols the declared stage already declares
        if self.current_stage_declares(symbol) {
            return Ok(());
        }

        let input = self.module(symbol.module_id);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, from_ref(&expanded.patch));

        let mut walk = WalkState::new(symbol.module_id, tree, self);
        walk.demand_symbol_declaration(symbol)?;
        walk.commit()?;

        Ok(())
    }

    /// Visit DIR and collect check constraints and obligations.
    ///
    /// Example:
    /// ```ds
    /// export function value(): number { 1 }
    /// ```
    pub(in crate::check) fn walk_module_bodies(&mut self, module: ModuleId) -> CompilerResult<()> {
        let input = self.module(module);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, from_ref(&expanded.patch));

        // walk and queue each root's bodies, inducing body-written lifetimes
        let mut walk = WalkState::new(module, tree, self);
        for root in &expanded.roots {
            walk.visit_body_expression(*root)?;
            walk.queue_module_expression(*root)?;
            walk.check.induce_signature_lifetimes()?;
        }
        walk.commit()?;
        self.queue_written_type_obligations(module)?;

        Ok(())
    }

    /// Queue obligations for written index and placed types.
    fn queue_written_type_obligations(&mut self, module: ModuleId) -> CompilerResult<()> {
        let mut written = Vec::new();
        if let Some(declared) = &self.module(module).declared {
            written.extend(declared.types.node_types());
        }
        written.extend(self.node_types.iter().map(|(node, ty)| (*node, *ty)));

        for (source, ty) in written {
            if source.try_into_typed::<dir::TypeExpression>().is_err() {
                continue;
            }

            let checked = matches!(self.operation_head(ty)?, Some(dir::TypeOperation::Index(_)))
                || matches!(
                    self.ty(ty)?,
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Placed { .. },
                        ..
                    })
                );
            if !checked {
                continue;
            }

            self.push_obligation(
                Obligation::WellFormedType(WellFormedTypeObligation { source, ty }),
                None,
            );
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
        let tree = dir::View::with_patches(&parsed.tree, from_ref(&expanded.patch));

        // walk and queue expanded module roots
        let mut walk = WalkState::new(module, tree, self);
        for root in &expanded.roots {
            match tree.get(*root) {
                dir::Expression::Declaration(declaration) => {
                    let declaration = *declaration;
                    walk.enter_node(*root)?;
                    let void = walk.intern_type(dir::Type::Void)?;
                    walk.commit_node_type(*root, void)?;
                    walk.walk_declaration(declaration, &tree.get(declaration).clone())?;
                }
                dir::Expression::Let { .. } => {
                    walk.enter_node(*root)?;
                    walk.walk_let_bindings(*root)?;
                }
                _ => continue,
            }

            walk.queue_module_expression(*root)?;
            walk.check.induce_signature_lifetimes()?;
        }
        walk.commit()?;

        Ok(())
    }
}

impl WalkState<'_, '_> {
    /// Queue one module-scope expression for inference.
    fn queue_module_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let node = expression.into_global_any(self.module);
        if self.check.is_absent(node) {
            return Ok(());
        }

        // transcribe declarations / bindings only
        if self.check.is_declaration()
            && !matches!(
                self.tree.get(expression),
                dir::Expression::Declaration(_) | dir::Expression::Let { .. }
            )
        {
            return Ok(());
        }

        let expressions = match self.tree.get(expression) {
            dir::Expression::Declaration(declaration) => match self.tree.get(*declaration) {
                dir::Declaration::Global(declaration) => Some(declaration.expressions.clone()),
                dir::Declaration::Module(declaration) => Some(declaration.expressions.clone()),
                _ => None,
            },
            _ => None,
        };

        // queue expressions nested by global and module declarations
        if let Some(expressions) = expressions {
            for expression in expressions {
                self.queue_module_expression(expression)?;
            }

            return Ok(());
        }

        let site = self.node_site(expression)?;
        self.check.queue_task(Task::Infer {
            site,
            use_: PlaceUse::Read,
        });

        Ok(())
    }
}
