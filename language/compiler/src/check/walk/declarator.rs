use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    FlowPath, Obligation, Origin, PatternObligation, Relation, WalkState, Widening,
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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
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
        // walk declared sources before reading their types
        if let Some(ty) = declarator.ty {
            self.walk_type_expression(ty)?;
        }
        if let Some(value) = declarator.value {
            self.walk_expression(value, self.tree.get(value))?;
        }

        // bind annotated declarators to their spelled types directly
        if let Some(ty) = declarator.ty {
            let spelled = self.walk_type_expression(ty)?;
            self.declare_symbol_type(symbol, spelled)?;

            // check initializers against explicit annotations
            if let Some(value) = declarator.value {
                self.expect_assignable(value, spelled)?;
            }

            return Ok(());
        }

        // settle the widening policy from the initializer shape
        let widening = match declarator.value {
            Some(value) if self.should_widen_declarator_initializer(symbol, value) => {
                Widening::Widen
            }
            _ => Widening::Preserve,
        };

        // bind closed initializers directly (widened)
        if let Some(value) = declarator.value {
            let initializer = self.node_type(value)?;
            if self.check.type_variables(initializer)?.is_empty() {
                let source = value.into_any();
                let bound = match widening {
                    Widening::Widen => self.check.widen_type(self.module, source, initializer)?,
                    Widening::Preserve => initializer,
                };
                self.declare_symbol_type(symbol, bound)?;

                // the binding consumes the value at its widened shape
                let node = value.into_global_any(self.module);
                self.check.record_coercion(node, initializer, bound)?;

                return Ok(());
            }

            // open initializers flow into a binding variable
            let binding = self.binding_type(symbol, widening)?;
            let origin = Origin::Node(value.into_global_any(self.module));
            self.relate_type(origin, Relation::Assignable, initializer, binding);

            return Ok(());
        }

        // uninitialized bindings keep a variable for their writes
        self.binding_type(symbol, widening)?;

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
            self.walk_type_expression(ty)?;
        }

        // walk matched value
        if let Some(value) = declarator.value {
            self.walk_expression(value, self.tree.get(value))?;
        }

        // flow the matched value into the pattern holes
        let matched = if let Some(value) = declarator.value {
            Some(self.node_type(value)?)
        } else if let Some(ty) = declarator.ty {
            Some(self.walk_type_expression(ty)?)
        } else {
            None
        };
        if let Some(matched) = matched {
            let origin = Origin::Node(declarator.pattern.into_global_any(self.module));
            let pattern = self.node_type(declarator.pattern)?;
            self.relate_type(origin, Relation::Assignable, matched, pattern);

            // non-matching positions must always succeed
            if self.is_irrefutable_declarator_pattern_required(id) {
                let condition = self.active_static_guard();
                self.check
                    .push_obligation(Obligation::Pattern(PatternObligation {
                        source: declarator.pattern.into_global_any(self.module),
                        condition,
                        pattern: declarator.pattern.into_global(self.module),
                        value: matched,
                    }));
            }
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

    /// Narrow flow from one matched declarator pattern.
    ///
    /// Example:
    /// ```ds
    /// if let Some(value) = option { value }
    /// ```
    pub(in crate::check) fn narrow_declarator_match(
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

        self.narrow_pattern_match(path, declarator.pattern)
    }

    /// Narrow one flow path from one matched pattern.
    ///
    /// Example:
    /// ```ds
    /// value is T
    /// ```
    pub(in crate::check) fn narrow_pattern_match(
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
                let pattern = *pattern;
                self.narrow_pattern_match(path, pattern)?;
            }
            // value
            dir::Pattern::Expression { value } => {
                let value = *value;
                let ty = self.node_type(value)?;
                self.narrow_flow_path(path, ty);
            }
            // value is T
            dir::Pattern::TypeExpression { value } => {
                let value = *value;
                let ty = self.walk_type_expression(value)?;
                self.narrow_flow_path(path, ty);
            }
            // T(a, b), T { name }
            dir::Pattern::Newtype { ty, fields }
            | dir::Pattern::NominalObject { ty, fields } => {
                let (ty, fields) = (*ty, fields.clone());
                let narrowed = self.walk_type_expression(ty)?;
                self.narrow_flow_path(path.clone(), narrowed);
                self.narrow_pattern_field_match(path, &fields)?;
            }
            // { name }
            dir::Pattern::Object { fields } => {
                let fields = fields.clone();

                self.narrow_pattern_field_match(path, &fields)?;
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

    /// Narrow object field paths from matched field patterns.
    ///
    /// Example:
    /// ```ds
    /// { name: string }
    /// ```
    fn narrow_pattern_field_match(
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
                    let (name, pattern) = (*name, *pattern);
                    let mut field_path = path.clone();
                    field_path.push_segment(name.static_key());

                    self.narrow_pattern_match(field_path, pattern)?;
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
