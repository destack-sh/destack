use destack_dir as dir;

use crate::CompilerResult;
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
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        if let Some(symbol) = self.declarator_binding_symbol(declarator) {
            self.walk_name_declarator(symbol, declarator)?;
        } else {
            self.walk_pattern_declarator(id, declarator)?;
        }

        Ok(())
    }

    /// Walk one declarator that binds a plain name.
    ///
    /// Example:
    /// ```ds
    /// value = 1
    /// ```
    fn walk_name_declarator(
        &mut self,
        symbol: dir::GlobalSymbolId,
        declarator: &dir::Declarator,
    ) -> CompilerResult<()> {
        // walk declared sources before reading operands
        if let Some(ty) = declarator.ty {
            self.walk_type_expression(ty, self.tree.get(ty))?;
        }
        if let Some(value) = declarator.value {
            self.walk_expression(value, self.tree.get(value))?;
        }

        let condition = self.active_static_guard();

        // set binding type from annotation or initializer
        if let Some(ty) = declarator.ty {
            let operand = self.node_type_operand(ty)?;
            self.constrain_symbol_type(symbol, operand, condition)?;
        } else if let Some(value) = declarator.value {
            let operand = self.node_type_operand(value)?;
            if let Some(term) = self.widen_declarator_initializer_type(symbol, value, operand)? {
                self.constrain_symbol_type_term(symbol, term, condition)?;
            } else {
                self.constrain_symbol_type(symbol, operand, condition)?;
            }
        }

        // check initializers against explicit annotations
        if let (Some(ty), Some(value)) = (declarator.ty, declarator.value) {
            let origin = Origin::Node(value.into_global_any(self.module));
            let value = self.node_type_operand(value)?;
            let target = self.node_type_operand(ty)?;
            let condition = self.active_static_guard();
            self.check
                .constrain_type(origin, TypeRelation::Assignable, value, target, condition);
        }

        Ok(())
    }

    /// Walk one declarator that destructures or matches a value.
    ///
    /// Example:
    /// ```ds
    /// { name } = user
    /// ```
    fn walk_pattern_declarator(
        &mut self,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) -> CompilerResult<()> {
        self.walk_pattern(declarator.pattern, self.tree.get(declarator.pattern))?;

        // walk declared type
        if let Some(ty) = declarator.ty {
            self.walk_type_expression(ty, self.tree.get(ty))?;
        }

        // walk matched value
        if let Some(value) = declarator.value {
            self.walk_expression(value, self.tree.get(value))?;
        }

        let matched_value = if let Some(value) = declarator.value {
            Some(self.node_type_operand(value)?)
        } else if let Some(ty) = declarator.ty {
            Some(self.node_type_operand(ty)?)
        } else {
            None
        };
        let pattern = self.pattern_term(self.module, declarator.pattern)?;

        if let (Some(value), Some(pattern)) = (matched_value, pattern) {
            let condition = self.active_static_guard();

            if self.is_irrefutable_declarator_pattern_required(id) {
                self.check.constrain_irrefutable_pattern(
                    self.module,
                    declarator.pattern.into_any(),
                    pattern,
                    value,
                    condition.clone(),
                );
            }

            self.check.constrain_pattern(
                self.module,
                PatternRelation::Match(pattern),
                declarator.pattern.into_any(),
                value,
                condition,
            );
        }

        Ok(())
    }

    /// Return the symbol bound by a plain name declarator.
    fn declarator_binding_symbol(
        &self,
        declarator: &dir::Declarator,
    ) -> Option<dir::GlobalSymbolId> {
        match self.tree.get(declarator.pattern) {
            dir::Pattern::Binding { pattern: None, .. } => self
                .check
                .module(self.module)
                .declaration_symbol(declarator.pattern.into_any()),
            _ => None,
        }
    }

    /// Return the widened inferred type term for one declarator initializer.
    ///
    /// Example:
    /// ```ds
    /// let value = 1
    /// ```
    fn widen_declarator_initializer_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::LocalNodeId<dir::Expression>,
        source: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        // keep precise initializer types when widening is not requested
        if !self.should_widen_declarator_initializer(symbol, value) {
            return Ok(None);
        }

        // widen known scalar literals without an operation term
        let literal = match self.check.resolved_type_operand(source) {
            Some(TypeOperand::Term(term)) => match self.check.inference.term(term) {
                TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)) => Some(literal.clone()),
                _ => None,
            },
            Some(TypeOperand::Type(ty)) => {
                match TypeLiteralTerm::from_type(self.check.r#type(ty)) {
                    Some(TypeLiteralTerm::Scalar(literal)) => Some(literal),
                    _ => None,
                }
            }
            Some(TypeOperand::Variable(_)) | None => None,
        };
        if let Some(literal) = literal {
            let literal = CheckState::widen_scalar_literal(literal);

            return Ok(Some(TypeTerm::Literal(literal)));
        }

        // defer non scalar widening to reduction
        let operation = self
            .check
            .inference
            .push_term(TypeOperationTerm::Widen { source });

        Ok(Some(TypeTerm::Operation(operation)))
    }

    /// Return whether one declarator should widen its inferred initializer type.
    fn should_widen_declarator_initializer(
        &self,
        symbol: dir::GlobalSymbolId,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let bindings = self.check.module(symbol.module_id).binding_table();
        let binding = bindings.get_symbol(symbol.local_id);

        match self.tree.get(value) {
            // (value)
            dir::Expression::Parenthesized { expression } => {
                self.should_widen_declarator_initializer(symbol, *expression)
            }
            // value satisfies T
            dir::Expression::Satisfies { .. } => false,
            // value as const
            dir::Expression::As { target_type, .. }
                if matches!(self.tree.get(*target_type), dir::TypeExpression::Const) =>
            {
                false
            }
            // mutable bindings widen initializers
            _ if binding.binding_mutability != Some(dir::Mutability::Immutable) => true,
            // immutable aggregate bindings keep mutable contents usable
            dir::Expression::ArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::ObjectExpression { .. } => true,
            // immutable scalar bindings stay literal
            _ => false,
        }
    }

    /// Return whether one declarator is outside a matching context.
    fn is_irrefutable_declarator_pattern_required(
        &self,
        id: dir::LocalNodeId<dir::Declarator>,
    ) -> bool {
        // require a parent expression (otherwise this smells like damaged syntax)
        let Some(parent) = self.tree.get_parent(id.id) else {
            return true;
        };
        if parent.ty != dir::NodeType::Expression {
            return true;
        }

        // allow matching binding forms
        let expression = self
            .tree
            .get(dir::LocalNodeId::<dir::Expression>::new(parent.id));
        match expression {
            dir::Expression::LetElse { declarator, .. }
            | dir::Expression::If {
                condition: dir::IfCondition::Let { declarator, .. },
                ..
            } if declarator == &id => false,
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
        id: dir::LocalNodeId<dir::Declarator>,
    ) -> CompilerResult<()> {
        let declarator = self.tree.get(id);
        let Some(value) = declarator.value else {
            return Ok(());
        };
        let Some(path) = self.flow_path(value) else {
            return Ok(());
        };
        self.narrow_pattern_success(path, declarator.pattern)
    }

    /// Narrow one flow path from a successful pattern.
    ///
    /// Example:
    /// ```ds
    /// value is T
    /// ```
    pub(in crate::check) fn narrow_pattern_success(
        &mut self,
        path: FlowPath,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<()> {
        match self.tree.get(pattern) {
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
                self.narrow_pattern_success(path, *pattern)?;
            }
            // value
            dir::Pattern::Expression { value } => {
                let ty = self.node_type_operand(*value)?;
                self.narrow_flow_path(path, ty);
            }
            // value is T
            dir::Pattern::TypeExpression { value } => {
                let ty = self.node_type_operand(*value)?;
                self.narrow_flow_path(path, ty);
            }
            // T(a, b), T { name }
            dir::Pattern::Newtype { ty, fields }
            | dir::Pattern::NominalObject { ty, fields } => {
                let narrowed = self.node_type_operand(*ty)?;
                self.narrow_flow_path(path.clone(), narrowed);
                self.narrow_pattern_fields_success(path, fields)?;
            }
            // { name }
            dir::Pattern::Object { fields } => {
                self.narrow_pattern_fields_success(path, fields)?;
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

        Ok(())
    }

    /// Narrow object field paths from successful patterns.
    ///
    /// Example:
    /// ```ds
    /// { name: string }
    /// ```
    fn narrow_pattern_fields_success(
        &mut self,
        path: FlowPath,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        for field in fields {
            match self.tree.get(*field) {
                // { name: pattern }
                dir::PatternField::Named {
                    name,
                    pattern: Some(pattern),
                    ..
                } => {
                    let mut field_path = path.clone();
                    field_path.push_segment(name.static_key());

                    self.narrow_pattern_success(field_path, *pattern)?;
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

        Ok(())
    }
}
