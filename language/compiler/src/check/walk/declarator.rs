use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    ConstraintRole, FlowPath, Obligation, Origin, PatternCoverage, PatternCoverageObligation,
    Relation, WalkState, Widening,
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
        binding_kind: Option<dir::LetKind>,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(());
        };

        if let Some(symbol) = self.plain_declarator_symbol(declarator) {
            self.walk_plain_declarator(symbol, declarator, binding_kind)?;
        } else {
            self.walk_pattern_declarator(id, declarator)?;
        }

        Ok(())
    }

    /// Walk one declarator that binds one symbol directly.
    ///
    /// Example:
    /// ```ds
    /// value = 1
    /// ```
    fn walk_plain_declarator(
        &mut self,
        symbol: dir::GlobalSymbolId,
        declarator: &dir::Declarator,
        binding_kind: Option<dir::LetKind>,
    ) -> CompilerResult<()> {
        // bind annotated declarators before checking their initializers
        if let Some(ty) = declarator.ty {
            let written = self.walk_type_expression(ty)?;
            self.bind_symbol_type(symbol, written)?;

            // const unique symbols carry their declaration identity as a static value
            if binding_kind == Some(dir::LetKind::Const)
                && matches!(
                    self.check.ty(written)?,
                    dir::Type::Primitive(dir::PrimitiveType::UniqueSymbol)
                )
            {
                let value = self.push_type(
                    dir::Type::Instance(dir::GenericInstance {
                        symbol,
                        arguments: Vec::new(),
                    }),
                    ty.into_any(),
                )?;
                self.set_static_value(symbol, value)?;
            }

            // check initializers against explicit annotations
            if let Some(value) = declarator.value {
                self.walk_expression_expected(value, self.tree.get(value), written)?;
            }

            return Ok(());
        }

        // choose the TS initializer widening rule before walking the value
        let widening = self.declarator_initializer_widening(symbol, declarator.value);

        // walk the initializer as its own expression
        if let Some(value) = declarator.value {
            self.walk_expression(value, self.tree.get(value))?;
        }

        // bind closed initializers directly (widened)
        if let Some(value) = declarator.value {
            let initializer = self.node_type(value)?;
            if self.check.type_variables(initializer)?.is_empty() {
                let source = value.into_any();
                let bound = match widening {
                    Widening::Widen => self.check.widen_type(self.module, source, initializer)?,
                    Widening::Preserve => initializer,
                };
                self.bind_symbol_type(symbol, bound)?;

                return Ok(());
            }

            // open initializers flow into a binding variable
            let binding = self.binding_type(symbol, widening)?;
            let origin = Origin::Node(value.into_global_any(self.module));
            self.push_relation(
                origin,
                ConstraintRole::Value,
                Relation::Assignable,
                initializer,
                binding,
            );

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

        // flow the matched value into the pattern type
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
            self.push_relation(
                origin,
                ConstraintRole::Value,
                Relation::Assignable,
                matched,
                pattern,
            );

            // non-matching positions must always succeed
            if self.is_irrefutable_declarator_pattern_required(id) {
                let condition = self.active_static_guard();
                self.check.push_obligation(Obligation::PatternCoverage(
                    PatternCoverageObligation {
                        source: declarator.pattern.into_global_any(self.module),
                        condition,
                        value: matched,
                        coverage: PatternCoverage::Binding {
                            pattern: declarator.pattern.into_global(self.module),
                        },
                    },
                ));
            }
        }

        Ok(())
    }

    /// Return the single symbol bound by a plain declarator.
    fn plain_declarator_symbol(&self, declarator: &dir::Declarator) -> Option<dir::GlobalSymbolId> {
        match self.tree.get(declarator.pattern) {
            dir::Pattern::Binding { pattern: None, .. } => self
                .check
                .module(self.module)
                .declaration_symbol(declarator.pattern.into_any()),
            _ => None,
        }
    }

    /// Return the widening policy for one inferred declarator initializer.
    fn declarator_initializer_widening(
        &self,
        symbol: dir::GlobalSymbolId,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> Widening {
        let Some(value) = value else {
            return Widening::Preserve;
        };
        let bindings = self.check.module(symbol.module_id).binding_table();
        let binding = bindings.get_symbol(symbol.local_id);

        match self.tree.get(value) {
            // (value)
            dir::Expression::Parenthesized { expression } => {
                self.declarator_initializer_widening(symbol, Some(*expression))
            }
            // value satisfies T
            dir::Expression::Satisfies { .. } => Widening::Preserve,
            // value as const
            dir::Expression::As { target_type, .. }
                if matches!(self.tree.get(*target_type), dir::TypeExpression::Const) =>
            {
                Widening::Preserve
            }
            // mutable bindings widen initializers
            _ if binding.binding_mutability != Some(dir::Mutability::Immutable) => Widening::Widen,
            // immutable aggregate bindings keep mutable contents usable
            dir::Expression::ArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::ObjectExpression { .. } => Widening::Widen,
            // immutable scalar bindings stay literal
            _ => Widening::Preserve,
        }
    }

    /// Return whether one declarator is outside a matching context.
    fn is_irrefutable_declarator_pattern_required(
        &self,
        id: dir::LocalNodeId<dir::Declarator>,
    ) -> bool {
        // require a parent expression for destructuring context
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
        !matches!(
            expression,
            dir::Expression::LetElse { declarator, .. }
                if declarator == &id
        ) && !matches!(
            expression,
            dir::Expression::If { condition, .. }
                if condition.as_binding().is_some_and(|(_, _, declarator)| declarator == id)
        )
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
            // T(a, b), T { name }
            dir::Pattern::NominalTuple { ty, fields }
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
