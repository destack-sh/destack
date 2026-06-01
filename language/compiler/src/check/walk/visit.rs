use destack_source::ModuleId;
use std::sync::Arc;

use destack_dir as dir;

use crate::check::{CheckState, WalkState};

impl CheckState<'_> {
    /// Visit DIR and collect check constraints and obligations.
    ///
    /// Example:
    /// ```ds
    /// export function value(): number { 1 }
    /// ```
    pub(in crate::check) fn walk_module(&mut self, module: ModuleId) {
        let input = self.module(module);
        let parsed = Arc::clone(&input.parsed);
        let expanded = Arc::clone(&input.expanded);
        let tree = &parsed.tree;

        let mut walk = WalkState::new(module, self);

        // predeclare hoisted symbol types before bodies can reference them
        for root in &expanded.roots {
            walk.predeclare_hoisted_symbol_types(tree, *root);
        }

        // walk expanded roots in semantic context
        for root in &expanded.roots {
            walk.walk_expression(tree, *root, tree.get(*root));
        }
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
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
    ) {
        match tree.get(expression) {
            dir::Expression::Declaration(declaration) => {
                self.predeclare_hoisted_symbol_type(tree, *declaration, tree.get(*declaration));
            }
            dir::Expression::Block(block) => {
                for expression in tree.get(*block).iter_expressions() {
                    self.predeclare_hoisted_symbol_types(tree, expression);
                }
            }
            _ => {}
        }
    }

    /// Predeclare one hoisted symbol type.
    ///
    /// Example:
    /// ```ds
    /// struct Box { value: number }
    /// ```
    fn predeclare_hoisted_symbol_type(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        match declaration {
            dir::Declaration::Global(declaration) => {
                for expression in &declaration.expressions {
                    self.predeclare_hoisted_symbol_types(tree, *expression);
                }
            }
            dir::Declaration::Module(declaration) => {
                for expression in &declaration.expressions {
                    self.predeclare_hoisted_symbol_types(tree, *expression);
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
                let Some(symbol) = self.check.declaration_symbol(tree.module_id, id.into_any())
                else {
                    return;
                };
                if self.check.operands.symbol_types.contains_key(&symbol) {
                    return;
                }

                self.check
                    .bind_symbol_type_variable_if_missing(tree.module_id, symbol);
            }
        }
    }
}
