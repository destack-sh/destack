use destack_dir as dir;

use crate::check::{
    ExpectedType, FlowPredicate, Obligation, Origin, PatternCoverage, PatternCoverageObligation,
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

        let widening = declarator_widening(self.tree, declarator, binding_kind);
        self.walk_pattern(
            declarator.pattern,
            self.tree.get(declarator.pattern),
            Some(widening),
        )?;

        // walk the declared pattern type
        let matched = match declarator.ty {
            Some(annotation) => {
                let ty = match is_ambient {
                    true => self.walk_static_type_expression(annotation)?,
                    false => self.walk_type_expression(annotation)?,
                };
                let source = annotation.into_global_any(self.module);
                let origin = Origin::Node(source, self.flow().template_scope());

                Some(self.check.storage_type(origin, ty)?)
            }
            None => None,
        };

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
            let key = dir::StaticKey::Symbol(dir::SymbolKey::Unique(symbol));
            let value = self.intern_type(dir::Type::Key(key))?;
            self.commit_static_value(symbol, value)?;
        }

        // walk matched value
        if let Some(value) = declarator.value {
            self.walk_declarator_initializer(id, value)?;
        }

        // record pattern checking from the initializer or annotation
        if let Some(value) = declarator.value {
            // non-matching positions must always succeed
            if self.is_irrefutable_declarator_pattern_required(id) {
                self.check.push_obligation(
                    Obligation::PatternCoverage(PatternCoverageObligation {
                        source: declarator.pattern.into_global_any(self.module),
                        value: ExpectedType::Node(value.into_global_any(self.module)),
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

    /// Walk one direct declarator initializer.
    fn walk_declarator_initializer(
        &mut self,
        id: dir::LocalNodeId<dir::Declarator>,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(value, self.tree.get(value))?;

        // initializing a binding may consume an identifier source
        let target = self
            .check
            .module(self.module)
            .declaration_symbol(self.tree.get(id).pattern.into_any());
        self.mark_moved_source(value, None, target);

        Ok(())
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
        let Some(path) = self.lexical_access_path(value) else {
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
        path: dir::AccessPath,
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

                self.flow_mut().apply_narrowing(path, predicate);
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
                    self.flow_mut().apply_narrowing(path.clone(), predicate);
                    self.narrow_pattern_field_match(path, &fields)?;
                } else {
                    self.flow_mut().apply_narrowing(path, predicate);
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
        path: dir::AccessPath,
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
                    field_path.push(name.static_key());

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

/// Return the widening policy for one inferred declarator initializer.
pub(in crate::check) fn declarator_widening(
    tree: dir::View<'_>,
    declarator: &dir::Declarator,
    binding_kind: Option<dir::LetKind>,
) -> Widening {
    if declarator.ty.is_some() {
        return Widening::Never;
    }
    let Some(value) = declarator.value else {
        return Widening::Never;
    };

    match tree.get(value) {
        // value satisfies T
        dir::Expression::Satisfies { .. } => Widening::Never,
        // value as const
        dir::Expression::As { target_type, .. }
            if matches!(tree.get(*target_type), dir::TypeExpression::Const) =>
        {
            Widening::Never
        }
        // mutable bindings widen their initialized value
        _ if binding_kind != Some(dir::LetKind::Const) => Widening::Always,
        // immutable aggregate bindings keep mutable contents usable
        dir::Expression::ArrayExpression { .. }
        | dir::Expression::TupleExpression { .. }
        | dir::Expression::ObjectExpression { .. } => Widening::Always,
        // immutable scalar bindings stay literal
        _ => Widening::Never,
    }
}
