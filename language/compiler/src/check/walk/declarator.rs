use destack_dir as dir;

use crate::check::{
    ExpectedType, FlowPath, FlowPredicate, Obligation, PatternCoverage, PatternCoverageObligation,
    WalkState, Widening,
};
use crate::{CompilerError, CompilerResult};

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
        is_ambient: bool,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }

        let widening = self.declarator_widening(declarator, binding_kind);
        self.walk_pattern(
            declarator.pattern,
            self.tree.get(declarator.pattern),
            Some(widening),
        )?;

        // walk the declared pattern type
        let matched = declarator
            .ty
            .map(|ty| match is_ambient {
                true => self.walk_static_type_expression(ty),
                false => self.walk_type_expression(ty),
            })
            .transpose()?;

        // create declaration identity for const unique symbols
        if binding_kind == Some(dir::LetKind::Const)
            && let Some(matched) = matched
            && matches!(
                self.check.ty(matched)?,
                dir::Type::Primitive(dir::PrimitiveType::UniqueSymbol)
            )
            && matches!(
                self.tree.get(declarator.pattern),
                dir::Pattern::Binding { .. }
            )
        {
            let symbol = self
                .check
                .module(self.module)
                .declaration_symbol(declarator.pattern.into_any())
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("declaration pattern {:?} has no symbol", declarator.pattern),
                })?;
            let arguments = self.intern_type_ids(&[])?;
            let value = self.intern_type(dir::Type::Instance(dir::GenericInstance {
                symbol,
                arguments,
            }))?;
            self.commit_static_value(symbol, value)?;
        }

        // walk matched value
        if let Some(value) = declarator.value {
            self.walk_expression(value, self.tree.get(value))?;
        }

        // record pattern checking from the initializer or annotation
        if let Some(value) = declarator.value {
            let value_site = self.node_site(value)?;
            // non-matching positions must always succeed
            if self.is_irrefutable_declarator_pattern_required(id) {
                self.check.push_obligation(
                    Obligation::PatternCoverage(PatternCoverageObligation {
                        source: declarator.pattern.into_global_any(self.module),
                        value: ExpectedType::Node(value_site),
                        coverage: PatternCoverage::Binding {
                            pattern: declarator.pattern.into_global(self.module),
                        },
                    }),
                    self.flow().template_scope(),
                );
            }
        } else if let Some(matched) = matched {
            // non-matching positions must always succeed
            if self.is_irrefutable_declarator_pattern_required(id) {
                self.check.push_obligation(
                    Obligation::PatternCoverage(PatternCoverageObligation {
                        source: declarator.pattern.into_global_any(self.module),
                        value: ExpectedType::Type(matched),
                        coverage: PatternCoverage::Binding {
                            pattern: declarator.pattern.into_global(self.module),
                        },
                    }),
                    self.flow().template_scope(),
                );
            }
        }

        Ok(())
    }

    /// Return the widening policy for one inferred declarator initializer.
    fn declarator_widening(
        &self,
        declarator: &dir::Declarator,
        binding_kind: Option<dir::LetKind>,
    ) -> Widening {
        if declarator.ty.is_some() {
            return Widening::Never;
        }
        let Some(value) = declarator.value else {
            return Widening::Never;
        };

        match self.tree.get(value) {
            // value satisfies T
            dir::Expression::Satisfies { .. } => Widening::Never,
            // value as const
            dir::Expression::As { target_type, .. }
                if matches!(self.tree.get(*target_type), dir::TypeExpression::Const) =>
            {
                Widening::Never
            }
            // mutable bindings widen initializers
            _ if binding_kind != Some(dir::LetKind::Const) => Widening::Always,
            // immutable aggregate bindings keep mutable contents usable
            dir::Expression::ArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::ObjectExpression { .. } => Widening::Always,
            // immutable scalar bindings stay literal
            _ => Widening::Never,
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
            // value, start..end, a | b
            dir::Pattern::Expression { .. }
            | dir::Pattern::Range { .. }
            | dir::Pattern::Union { .. } => {
                let predicate = FlowPredicate::Pattern {
                    pattern: pattern.into_global(self.module),
                    is_positive,
                };

                self.apply_flow_predicate(path, predicate);
            }
            // T(a, b), T { name }
            dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::NominalObject { fields, .. } => {
                let fields = fields.clone();
                let predicate = FlowPredicate::Pattern {
                    pattern: pattern.into_global(self.module),
                    is_positive,
                };

                if is_positive {
                    self.apply_flow_predicate(path.clone(), predicate);
                    self.narrow_pattern_field_match(path, &fields)?;
                } else {
                    self.apply_flow_predicate(path, predicate);
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
            // [a, b], [...items]
            dir::Pattern::Tuple { .. } | dir::Pattern::Sequence { .. } => {}
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
                | dir::PatternField::Rest { .. }
                // { name }
                | dir::PatternField::Named { pattern: None, .. }
                // [,]
                | dir::PatternField::Elision => {}
            }
        }

        Ok(())
    }
}
