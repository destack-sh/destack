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
        let tree = &parsed.tree;

        let mut walk = WalkState::new(module, tree, self);

        // predeclare hoisted symbol types before bodies can reference them
        for root in &expanded.roots {
            walk.predeclare_hoisted_symbol_types(*root)?;
        }

        // walk expanded roots in semantic context
        for root in &expanded.roots {
            walk.walk_expression(*root, tree.get(*root))?;
        }

        Ok(())
    }
}

impl WalkState<'_, '_> {
    /// Predeclare hoisted symbol types visible before source order.
    ///
    /// Example:
    /// ```ds
    /// function later(): number { 1 }
    /// ```
    fn predeclare_hoisted_symbol_types(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        match self.tree.get(expression) {
            dir::Expression::Declaration(declaration) => {
                self.predeclare_hoisted_symbol_type(*declaration, self.tree.get(*declaration))?;
            }
            dir::Expression::Block(block) => {
                for expression in self.tree.get(*block).iter_expressions() {
                    self.predeclare_hoisted_symbol_types(expression)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Predeclare one hoisted symbol type.
    ///
    /// Example:
    /// ```ds
    /// struct Box { value: number }
    /// ```
    fn predeclare_hoisted_symbol_type(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> CompilerResult<()> {
        match declaration {
            dir::Declaration::Global(declaration) => {
                for expression in &declaration.expressions {
                    self.predeclare_hoisted_symbol_types(*expression)?;
                }
            }
            dir::Declaration::Module(declaration) => {
                for expression in &declaration.expressions {
                    self.predeclare_hoisted_symbol_types(*expression)?;
                }
            }
            dir::Declaration::Type(declaration) if !declaration.is_nominal => {}
            dir::Declaration::Extension(_) => {}
            dir::Declaration::Type(_)
            | dir::Declaration::Struct(_)
            | dir::Declaration::Class(_)
            | dir::Declaration::Enum(_)
            | dir::Declaration::Interface(_)
            | dir::Declaration::Function(_) => {
                let Some(symbol) = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any())
                else {
                    return Ok(());
                };
                if self.check.inputs.symbol_type(symbol).is_some() {
                    return Ok(());
                }

                self.allocate_symbol_type_variable(symbol)?;
            }
        }

        Ok(())
    }
}
