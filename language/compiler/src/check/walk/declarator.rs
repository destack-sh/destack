use destack_dir as dir;

use crate::check::{
    CheckState, FlowPath, Origin, PatternRelation, TypeOperationTerm, TypeRelation, TypeTerm,
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
            symbol.map(|symbol| self.intern_local_symbol_type_variable(tree.module_id, symbol));
        if let Some(variable) = binding_type {
            // let x: T
            let term = if let Some(ty) = declarator.ty {
                Some(TypeTerm::Variable(
                    self.intern_local_node_type_variable(tree.module_id, ty),
                ))
            }
            // let x = value
            else if let Some(value) = declarator.value {
                let source = self.intern_local_node_type_variable(tree.module_id, value);

                // widen mutable bindings and fresh aggregate literals
                if symbol
                    .is_some_and(|symbol| self.declarator_widens_inferred_type(symbol, value, tree))
                {
                    let operation = self.terms.push(TypeOperationTerm::Widen { source });

                    Some(TypeTerm::Operation(operation))
                }
                // preserve exact type
                else {
                    Some(TypeTerm::Variable(source))
                }
            }
            // let x
            else {
                None
            };

            // : T
            if let Some(term) = term {
                let condition = self.active_static_condition(tree.module_id);

                self.add_type_definition(variable, term, condition);
            }

            // check initializers against explicit annotations
            if let (Some(ty), Some(value)) = (declarator.ty, declarator.value) {
                let origin = Origin::Node(value.into_global_any(tree.module_id));
                let value = self.intern_local_node_type_variable(tree.module_id, value);
                let target = self.intern_local_node_type_variable(tree.module_id, ty);
                let condition = self.active_static_condition(tree.module_id);

                self.add_type_constraint(
                    origin,
                    TypeRelation::Assignable,
                    value,
                    target,
                    condition,
                );
            }
        }

        // constrain the declared pattern against its initializer
        let matched_value = binding_type
            .or_else(|| {
                declarator
                    .value
                    .map(|value| self.intern_local_node_type_variable(tree.module_id, value))
            })
            .or_else(|| {
                declarator
                    .ty
                    .map(|ty| self.intern_local_node_type_variable(tree.module_id, ty))
            });

        if let Some(value) = matched_value
            && let Some(pattern) = self.build_pattern_term(tree.module_id, declarator.pattern, tree)
        {
            let condition = self.active_static_condition(tree.module_id);

            if Self::declarator_requires_irrefutable_pattern(tree, id) {
                self.require_irrefutable_pattern(
                    tree.module_id,
                    declarator.pattern.into_any(),
                    pattern,
                    value,
                    condition.clone(),
                );
            }

            self.constrain_pattern(
                tree.module_id,
                PatternRelation::Match(pattern),
                declarator.pattern.into_any(),
                value,
                condition,
            );
        }
        self.walk_pattern(tree, declarator.pattern, tree.get(declarator.pattern));

        // type
        if let Some(ty) = declarator.ty {
            self.walk_type_expression(tree, ty, tree.get(ty));
        }

        // value
        if let Some(value) = declarator.value {
            self.walk_expression(tree, value, tree.get(value));
        }

        self.pop_static_condition(tree.module_id);
    }

    /// Return whether one declarator widens its inferred initializer type.
    fn declarator_widens_inferred_type(
        &self,
        symbol: dir::GlobalSymbolId,
        value: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) -> bool {
        let bindings = self.module(symbol.module_id).binding_table();
        let binding = bindings.get_symbol(symbol.local_id);

        match tree.get(value) {
            dir::Expression::Parenthesized { expression } => {
                return self.declarator_widens_inferred_type(symbol, *expression, tree);
            }
            dir::Expression::Satisfies { .. } => {
                return false;
            }
            dir::Expression::As { target_type, .. }
                if matches!(tree.get(*target_type), dir::TypeExpression::Const) =>
            {
                return false;
            }
            _ => {}
        }

        if binding.binding_mutability != Some(dir::Mutability::Immutable) {
            return true;
        }

        match tree.get(value) {
            dir::Expression::ArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::ObjectExpression { .. } => true,
            _ => false,
        }
    }

    /// Return whether one declarator is outside a matching context.
    fn declarator_requires_irrefutable_pattern(
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
    ) -> bool {
        // require a parent expression (otherwise this smells like damaged syntax)
        let Some(parent) = tree.get_parent(id.id) else {
            return true;
        };
        if parent.ty != dir::NodeType::Expression {
            return true;
        }

        // allow matching binding forms
        let expression = tree.get(dir::LocalNodeId::<dir::Expression>::new(parent.id));
        match expression {
            dir::Expression::LetElse { declarator, .. }
            | dir::Expression::If {
                condition: dir::IfCondition::Let { declarator, .. },
                ..
            } if *declarator == id => false,
            _ => true,
        }
    }

    /// Apply successful pattern narrowings from one matching declarator.
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

    /// Apply successful pattern narrowings to one flow path.
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
                let ty = self.intern_local_node_type_variable(tree.module_id, *value);

                self.narrow_flow_path(path, ty);
            }
            // value is T
            dir::Pattern::TypeExpression { value } => {
                let ty = self.intern_local_node_type_variable(tree.module_id, *value);

                self.narrow_flow_path(path, ty);
            }
            // T(a, b), T { name }
            dir::Pattern::Newtype { ty, fields }
            | dir::Pattern::NominalObject { ty, fields } => {
                let narrowed = self.intern_local_node_type_variable(tree.module_id, *ty);

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

    /// Apply successful object field pattern narrowings.
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
