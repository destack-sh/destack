use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::{CheckModuleState, PatternRelation, TypeOperationTerm, TypeTerm};

impl CheckModuleState {
    /// Walk one declarator.
    pub(in crate::check) fn walk_declarator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Declarator, id.id);

        // define the direct binding type from its annotation or initializer
        let symbol = self.declaration_symbol_maybe(declarator.pattern.into_any());
        if let Some(symbol) = symbol {
            let variable = self.intern_symbol_type_variable(symbol);

            // let x: T
            let term = if let Some(ty) = declarator.ty {
                Some(TypeTerm::Variable(self.intern_local_type_variable(ty)))
            }
            // let x = value
            else if let Some(value) = declarator.value {
                let source = self.intern_local_type_variable(value);

                // keep const bindings exact
                if Self::declarator_is_const_binding(tree, id) {
                    Some(TypeTerm::Variable(source))
                }
                // widen mutable bindings
                else {
                    Some(TypeTerm::Operation(TypeOperationTerm::Widen { source }))
                }
            }
            // let x
            else {
                None
            };

            if let Some(term) = term {
                self.define_type_term(variable, term);
            }
        }

        // constrain the declared pattern against its initializer
        if let Some(value) = declarator.value
            && let Some(pattern) = self.pattern_term(declarator.pattern, tree)
        {
            let value = self.intern_local_type_variable(value);

            self.relate_pattern(
                PatternRelation::Match(pattern),
                declarator.pattern.into_any(),
                value,
            );
        }

        self.walk_pattern(tree, declarator.pattern, tree.get(declarator.pattern));

        if let Some(ty) = declarator.ty {
            self.walk_type_expression(tree, ty, tree.get(ty));
        }

        if let Some(value) = declarator.value {
            self.walk_expression(tree, value, tree.get(value));
        }
    }

    /// Return whether one declarator belongs to a `const` binding.
    fn declarator_is_const_binding(
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
    ) -> bool {
        // require a parent expression
        let Some(parent) = tree.get_parent(id.id) else {
            return false;
        };
        if parent.ty != dir::NodeType::Expression {
            return false;
        }

        // recognize const binding forms
        let expression = tree.get(dir::LocalNodeId::<dir::Expression>::new(parent.id));
        matches!(
            expression,
            dir::Expression::Let {
                kind: dir::LetKind::Const,
                ..
            } | dir::Expression::LetElse {
                kind: dir::LetKind::Const,
                ..
            }
        )
    }
}
