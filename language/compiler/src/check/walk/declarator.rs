use destack_dir as dir;

use crate::check::{
    CheckState, FlowPath, Origin, PatternRelation, TypeLiteralTerm, TypeOperand, TypeOperationTerm,
    TypeRelation, TypeTerm, WalkState,
};

impl WalkState<'_, '_> {
    /// Walk one declarator.
    ///
    /// Example:
    /// ```ds
    /// value: number = 1
    /// ```
    pub(in crate::check) fn walk_declarator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }

        if let Some(symbol) = self.find_declarator_binding_symbol(tree, declarator) {
            self.walk_name_declarator(tree, symbol, declarator);
        } else {
            self.walk_pattern_declarator(tree, id, declarator);
        }

        self.pop_static_guard();
    }

    /// Walk one declarator that binds a plain name.
    ///
    /// Example:
    /// ```ds
    /// value = 1
    /// ```
    fn walk_name_declarator(
        &mut self,
        tree: &dir::Tree,
        symbol: dir::GlobalSymbolId,
        declarator: &dir::Declarator,
    ) {
        // walk sources before reading their checked types
        if let Some(ty) = declarator.ty {
            self.walk_type_expression(tree, ty, tree.get(ty));
        }
        if let Some(value) = declarator.value {
            self.walk_expression(tree, value, tree.get(value));
        }

        let condition = self.active_static_guard();

        // set binding type from annotation or initializer
        if let Some(ty) = declarator.ty {
            let operand = self.check.require_local_node_type(tree.module_id, ty);

            self.check
                .output_symbol_type_operand(tree.module_id, symbol, operand, condition);
        } else if let Some(value) = declarator.value {
            let operand = self.check.require_local_node_type(tree.module_id, value);

            if let Some(term) =
                self.lower_name_declarator_widened_type(symbol, value, operand, tree)
            {
                self.check
                    .output_symbol_type(tree.module_id, symbol, term, condition);
            } else {
                self.check
                    .output_symbol_type_operand(tree.module_id, symbol, operand, condition);
            }
        }

        // check initializers against explicit annotations
        if let (Some(ty), Some(value)) = (declarator.ty, declarator.value) {
            let origin = Origin::Node(value.into_global_any(tree.module_id));
            let value = self.check.require_local_node_type(tree.module_id, value);
            let target = self.check.require_local_node_type(tree.module_id, ty);
            let condition = self.active_static_guard();

            self.check
                .relate_type(origin, TypeRelation::Assignable, value, target, condition);
        }
    }

    /// Walk one declarator that destructures or matches a value.
    ///
    /// Example:
    /// ```ds
    /// { name } = user
    /// ```
    fn walk_pattern_declarator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) {
        self.walk_pattern(tree, declarator.pattern, tree.get(declarator.pattern));

        // walk declared type
        if let Some(ty) = declarator.ty {
            self.walk_type_expression(tree, ty, tree.get(ty));
        }

        // walk matched value
        if let Some(value) = declarator.value {
            self.walk_expression(tree, value, tree.get(value));
        }

        let matched_value = declarator
            .value
            .map(|value| self.check.require_local_node_type(tree.module_id, value))
            .or_else(|| {
                declarator
                    .ty
                    .map(|ty| self.check.require_local_node_type(tree.module_id, ty))
            });

        if let Some(value) = matched_value
            && let Some(pattern) = self.lower_pattern_term(tree.module_id, declarator.pattern, tree)
        {
            let condition = self.active_static_guard();

            if Self::is_irrefutable_declarator_pattern_required(tree, id) {
                self.check.require_irrefutable_pattern(
                    tree.module_id,
                    declarator.pattern.into_any(),
                    pattern,
                    value,
                    condition.clone(),
                );
            }

            self.check.relate_pattern(
                tree.module_id,
                PatternRelation::Match(pattern),
                declarator.pattern.into_any(),
                value,
                condition,
            );
        }
    }

    /// Return the symbol bound by a plain name declarator.
    fn find_declarator_binding_symbol(
        &self,
        tree: &dir::Tree,
        declarator: &dir::Declarator,
    ) -> Option<dir::GlobalSymbolId> {
        match tree.get(declarator.pattern) {
            dir::Pattern::Binding { pattern: None, .. } => self
                .check
                .declaration_symbol(tree.module_id, declarator.pattern.into_any()),
            _ => None,
        }
    }

    /// Lower the widened inferred type term for one name declarator value.
    ///
    /// Example:
    /// ```ds
    /// let value = 1
    /// ```
    fn lower_name_declarator_widened_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::LocalNodeId<dir::Expression>,
        source: TypeOperand,
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        if !self.is_declarator_initializer_widened(symbol, value, tree) {
            return None;
        }
        let source_term = source.to_type_term(self.check);
        if let TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)) = source_term {
            return Some(TypeTerm::Literal(CheckState::widen_scalar_literal(literal)));
        }

        let operation = self
            .check
            .inference
            .terms
            .push(TypeOperationTerm::Widen { source });

        Some(TypeTerm::Operation(operation))
    }

    /// Return whether one declarator widens its inferred initializer type.
    fn is_declarator_initializer_widened(
        &self,
        symbol: dir::GlobalSymbolId,
        value: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) -> bool {
        let bindings = self.check.module(symbol.module_id).binding_table();
        let binding = bindings.get_symbol(symbol.local_id);

        match tree.get(value) {
            dir::Expression::Parenthesized { expression } => {
                return self.is_declarator_initializer_widened(symbol, *expression, tree);
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
    fn is_irrefutable_declarator_pattern_required(
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

    /// Narrow flow from one successful matching declarator.
    ///
    /// Example:
    /// ```ds
    /// if let Some(value) = option { value }
    /// ```
    pub(in crate::check) fn narrow_declarator_pattern_success(
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

        self.narrow_pattern_success(tree, path, declarator.pattern);
    }

    /// Narrow one flow path from a successful pattern.
    ///
    /// Example:
    /// ```ds
    /// value is T
    /// ```
    pub(in crate::check) fn narrow_pattern_success(
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
                self.narrow_pattern_success(tree, path, *pattern);
            }
            // value
            dir::Pattern::Expression { value } => {
                let ty = self.check.require_local_node_type(tree.module_id, *value);

                self.narrow_flow_path(path, ty);
            }
            // value is T
            dir::Pattern::TypeExpression { value } => {
                let ty = self.check.require_local_node_type(tree.module_id, *value);

                self.narrow_flow_path(path, ty);
            }
            // T(a, b), T { name }
            dir::Pattern::Newtype { ty, fields }
            | dir::Pattern::NominalObject { ty, fields } => {
                let narrowed = self.check.require_local_node_type(tree.module_id, *ty);

                self.narrow_flow_path(path.clone(), narrowed);
                self.narrow_pattern_fields_success(tree, path, fields);
            }
            // { name }
            dir::Pattern::Object { fields } => {
                self.narrow_pattern_fields_success(tree, path, fields);
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

    /// Narrow object field paths from successful patterns.
    ///
    /// Example:
    /// ```ds
    /// { name: value is string }
    /// ```
    fn narrow_pattern_fields_success(
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

                    self.narrow_pattern_success(tree, field_path, *pattern);
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
