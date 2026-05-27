use destack_dir as dir;

use crate::check::{
    CheckState, ConstraintOrigin, FlowPath, PatternRelation, TypeOperationTerm, TypeRelation,
    TypeTerm,
};

impl CheckState<'_> {
    /// Walk one declarator.
    pub(in crate::check) fn walk_declarator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }

        // define the direct binding type from its annotation or initializer
        let symbol = self.declaration_symbol(tree.module_id, declarator.pattern.into_any());
        let binding_type =
            symbol.map(|symbol| self.intern_symbol_type_variable(tree.module_id, symbol));
        if let Some(variable) = binding_type {
            // let x: T
            let term = if let Some(ty) = declarator.ty {
                Some(TypeTerm::Variable(
                    self.intern_local_type_variable(tree.module_id, ty),
                ))
            }
            // let x = value
            else if let Some(value) = declarator.value {
                let source = self.intern_local_type_variable(tree.module_id, value);

                // keep const bindings exact
                if Self::declarator_is_const_binding(tree, id) {
                    Some(TypeTerm::Variable(source))
                }
                // widen mutable bindings
                else {
                    let operation = self.terms.push(TypeOperationTerm::Widen { source });

                    Some(TypeTerm::Operation(operation))
                }
            }
            // let x
            else {
                None
            };

            // : T
            if let Some(term) = term {
                self.define_type(tree.module_id, variable, term);
            }

            // check initializers against explicit annotations
            if let (Some(ty), Some(value)) = (declarator.ty, declarator.value) {
                let origin = ConstraintOrigin::Node(value.into_global_any(tree.module_id));
                let value = self.intern_local_type_variable(tree.module_id, value);
                let target = self.intern_local_type_variable(tree.module_id, ty);

                self.constrain_type(origin, TypeRelation::Assignable, value, target);
            }
        }

        // constrain the declared pattern against its initializer
        let matched_value = binding_type
            .or_else(|| {
                declarator
                    .value
                    .map(|value| self.intern_local_type_variable(tree.module_id, value))
            })
            .or_else(|| {
                declarator
                    .ty
                    .map(|ty| self.intern_local_type_variable(tree.module_id, ty))
            });

        if let Some(value) = matched_value
            && let Some(pattern) = self.build_pattern_term(tree.module_id, declarator.pattern, tree)
        {
            if Self::declarator_requires_irrefutable_pattern(tree, id) {
                self.require_irrefutable_pattern(
                    tree.module_id,
                    declarator.pattern.into_any(),
                    pattern,
                    value,
                );
            }

            self.constrain_pattern(
                tree.module_id,
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

        self.pop_static_condition(tree.module_id);
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

    /// Return whether one declarator is outside a matching context.
    fn declarator_requires_irrefutable_pattern(
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
    ) -> bool {
        // require a parent expression
        let Some(parent) = tree.get_parent(id.id) else {
            return true;
        };
        if parent.ty != dir::NodeType::Expression {
            return true;
        }

        // allow matching binding forms
        let expression = tree.get(dir::LocalNodeId::<dir::Expression>::new(parent.id));
        !matches!(
            expression,
            dir::Expression::LetElse { declarator, .. } if *declarator == id
        ) && !matches!(
            expression,
            dir::Expression::If {
                condition: dir::IfCondition::Let { declarator, .. },
                ..
            } if *declarator == id
        )
    }

    /// Apply successful pattern facts from one matching declarator.
    pub(in crate::check) fn apply_declarator_pattern_narrowings(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
    ) {
        let declarator = tree.get(id);
        let Some(value) = declarator.value else {
            return;
        };
        let Some(path) = self.flow_path(tree, value) else {
            return;
        };

        self.apply_pattern_success_narrowings(tree, path, declarator.pattern);
    }

    /// Apply successful pattern facts to one flow path.
    pub(in crate::check) fn apply_pattern_success_narrowings(
        &mut self,
        tree: &dir::Tree,
        path: FlowPath,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) {
        match tree.get(pattern) {
            // pattern!
            dir::Pattern::Must(pattern)
            // pattern = value
            | dir::Pattern::Assign { pattern, .. }
            // &pattern
            | dir::Pattern::BorrowOf { right: pattern, .. }
            // move pattern
            | dir::Pattern::MoveOf { right: pattern, .. }
            // *pattern
            | dir::Pattern::DereferenceOf { right: pattern }
            // name: pattern
            | dir::Pattern::Binding {
                pattern: Some(pattern),
                ..
            } => {
                self.apply_pattern_success_narrowings(tree, path, *pattern);
            }
            // value
            dir::Pattern::Expression { value } => {
                let ty = self.intern_local_type_variable(tree.module_id, *value);

                self.narrow_flow_path(path, ty);
            }
            // value is T
            dir::Pattern::TypeExpression { value } => {
                let ty = self.intern_local_type_variable(tree.module_id, *value);

                self.narrow_flow_path(path, ty);
            }
            // T(a, b), T { name }
            dir::Pattern::TaggedTuple { ty, fields }
            | dir::Pattern::TaggedObject { ty, fields } => {
                let narrowed = self.intern_local_type_variable(tree.module_id, *ty);

                self.narrow_flow_path(path.clone(), narrowed);
                self.apply_pattern_field_success_narrowings(tree, path, fields);
            }
            // { name }
            dir::Pattern::Object { fields } => {
                self.apply_pattern_field_success_narrowings(tree, path, fields);
            }
            // _, name
            dir::Pattern::Wildcard | dir::Pattern::Binding { pattern: None, .. } => {}
            // start..end
            dir::Pattern::Range { .. } => {}
            // [a, b], [...items], a | b
            dir::Pattern::Tuple { .. }
            | dir::Pattern::Sequence { .. }
            | dir::Pattern::Union { .. } => {}
        }
    }

    /// Apply successful object field pattern facts.
    fn apply_pattern_field_success_narrowings(
        &mut self,
        tree: &dir::Tree,
        path: FlowPath,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) {
        for field in fields {
            match tree.get(*field) {
                // { name: pattern }
                dir::PatternField::Named {
                    name,
                    pattern: Some(pattern),
                    ..
                } => {
                    let mut field_path = path.clone();
                    field_path.push_segment(name.static_key());

                    self.apply_pattern_success_narrowings(tree, field_path, *pattern);
                }
                // { [key]: pattern }
                dir::PatternField::Computed { .. }
                // [pattern]
                | dir::PatternField::Positional { .. }
                // { ...pattern }
                | dir::PatternField::Spread { .. }
                // { name }
                | dir::PatternField::Named { pattern: None, .. }
                // [,]
                | dir::PatternField::Elision => {}
            }
        }
    }
}
