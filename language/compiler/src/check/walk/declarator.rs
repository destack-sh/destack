use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Expectation, ExpectedType, FlowNarrowing, FlowPath, Obligation, Origin, PatternCoverage,
    PatternCoverageObligation, ValueUse, WalkState, Widening,
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
        decorated: Option<dir::LocalNodeIdAny>,
    ) -> CompilerResult<()> {
        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(());
        }

        if let Some(symbol) = self.direct_declarator_symbol(declarator) {
            self.walk_direct_declarator(id, symbol, declarator, binding_kind, decorated)?;
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
    fn walk_direct_declarator(
        &mut self,
        id: dir::LocalNodeId<dir::Declarator>,
        symbol: dir::GlobalSymbolId,
        declarator: &dir::Declarator,
        binding_kind: Option<dir::LetKind>,
        decorated: Option<dir::LocalNodeIdAny>,
    ) -> CompilerResult<()> {
        // bind annotated declarators before checking their initializers
        if let Some(ty) = declarator.ty {
            let written = self.walk_type_expression(ty)?;
            self.queue_bind_type(symbol, written);

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
                self.commit_static_value(symbol, value)?;
            }

            // check annotated initializers before ordinary inference can claim them
            if let Some(value) = declarator.value {
                let expectation = Expectation::assignable(
                    written,
                    Origin::Node(value.into_global_any(self.module)),
                    ValueUse::Store,
                );
                self.walk_declarator_initializer(id, decorated, value)?;
                self.queue_node_check(value, expectation)?;
            }

            return Ok(());
        }

        // choose the TS initializer widening rule before walking the value
        let widening = self.declarator_initializer_widening(symbol, declarator.value);

        // walk the initializer as its own expression
        if let Some(value) = declarator.value {
            self.walk_declarator_initializer(id, decorated, value)?;
        }

        // bind inferred declarations from their initializer
        if let Some(value) = declarator.value {
            self.queue_bind_initializer(symbol, value, widening)?;

            return Ok(());
        }

        // uninitialized bindings keep a variable for their writes
        self.binding_type_slot(symbol, widening)?;

        Ok(())
    }

    /// Walk one direct declarator initializer.
    fn walk_declarator_initializer(
        &mut self,
        id: dir::LocalNodeId<dir::Declarator>,
        decorated: Option<dir::LocalNodeIdAny>,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let decorated = decorated.unwrap_or_else(|| id.into_any());
        let directive = self
            .check
            .capture_directive_for_source(self.module, decorated)?;
        self.with_capture_directive(directive, |state| {
            state.walk_expression(value, state.tree.get(value))
        })
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

        // queue pattern checking from the initializer or annotation
        if let Some(value) = declarator.value {
            let value_site = self.node_site(value)?;
            let expectation = Expectation::assignable_node(
                value_site,
                Origin::Node(value_site.node),
                ValueUse::Store,
            );
            self.queue_node_check(declarator.pattern, expectation)?;

            // non-matching positions must always succeed
            if self.is_irrefutable_declarator_pattern_required(id) {
                self.check.push_obligation(Obligation::PatternCoverage(
                    PatternCoverageObligation {
                        source: declarator.pattern.into_global_any(self.module),
                        value: ExpectedType::Node(value_site),
                        coverage: PatternCoverage::Binding {
                            pattern: declarator.pattern.into_global(self.module),
                        },
                    },
                ));
            }
        } else if let Some(ty) = declarator.ty {
            let matched = self.walk_type_expression(ty)?;
            let expectation = Expectation::assignable(
                matched,
                Origin::Node(ty.into_global_any(self.module)),
                ValueUse::Store,
            );
            self.queue_node_check(declarator.pattern, expectation)?;

            // non-matching positions must always succeed
            if self.is_irrefutable_declarator_pattern_required(id) {
                self.check.push_obligation(Obligation::PatternCoverage(
                    PatternCoverageObligation {
                        source: declarator.pattern.into_global_any(self.module),
                        value: ExpectedType::Type(matched),
                        coverage: PatternCoverage::Binding {
                            pattern: declarator.pattern.into_global(self.module),
                        },
                    },
                ));
            }
        }

        Ok(())
    }

    /// Return the single symbol bound directly by one declarator.
    fn direct_declarator_symbol(
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
        self.narrow_declarator_pattern(id, true)
    }

    /// Narrow flow from one declarator pattern.
    ///
    /// Example:
    /// ```ds
    /// if let Some(value) = option { value } else { option }
    /// ```
    pub(in crate::check) fn narrow_declarator_pattern(
        &mut self,
        id: dir::LocalNodeId<dir::Declarator>,
        is_positive: bool,
    ) -> CompilerResult<()> {
        let declarator = self.tree.get(id);
        let Some(value) = declarator.value else {
            return Ok(());
        };
        let Some(path) = self.flow_path(value) else {
            return Ok(());
        };

        self.narrow_pattern(path, declarator.pattern, is_positive)
    }

    /// Narrow one flow path from one pattern predicate.
    ///
    /// Example:
    /// ```ds
    /// if let 1 | 2 = value { value } else { value }
    /// ```
    pub(in crate::check) fn narrow_pattern(
        &mut self,
        path: FlowPath,
        pattern: dir::LocalNodeId<dir::Pattern>,
        is_positive: bool,
    ) -> CompilerResult<()> {
        match self.tree.get(pattern) {
            // pattern!
            dir::Pattern::Must(pattern)
            // pattern = value
            | dir::Pattern::Default { pattern, .. }
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
                self.narrow_pattern(path, pattern, is_positive)?;
            }
            // value
            dir::Pattern::Expression { .. } => {
                let narrowing = FlowNarrowing::Pattern {
                    pattern: pattern.into_global(self.module),
                    is_positive,
                };

                self.narrow_flow_path(path, narrowing);
            }
            // T(a, b), T { name }
            dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::NominalObject { fields, .. } => {
                let fields = fields.clone();
                let narrowing = FlowNarrowing::Pattern {
                    pattern: pattern.into_global(self.module),
                    is_positive,
                };

                if is_positive {
                    self.narrow_flow_path(path.clone(), narrowing);
                    self.narrow_pattern_field_match(path, &fields)?;
                } else {
                    self.narrow_flow_path(path, narrowing);
                }
            }
            // { name }
            dir::Pattern::Object { fields } => {
                let fields = fields.clone();

                if is_positive {
                    self.narrow_pattern_field_match(path, &fields)?;
                }
            }
            // _, name
            dir::Pattern::Wildcard | dir::Pattern::Binding { pattern: None, .. } => {}
            // start..end
            dir::Pattern::Range { .. } => {
                let narrowing = FlowNarrowing::Pattern {
                    pattern: pattern.into_global(self.module),
                    is_positive,
                };

                self.narrow_flow_path(path, narrowing);
            }
            // a | b
            dir::Pattern::Union { .. } => {
                let narrowing = FlowNarrowing::Pattern {
                    pattern: pattern.into_global(self.module),
                    is_positive,
                };

                self.narrow_flow_path(path, narrowing);
            }
            // [a, b], [...items]
            dir::Pattern::Tuple { .. }
            | dir::Pattern::Sequence { .. } => {}
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

                    self.narrow_pattern(field_path, pattern, true)?;
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
