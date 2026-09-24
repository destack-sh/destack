use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{
    CheckState, ExpectedType, FlowPredicate, Obligation, PatternCoverage,
    PatternCoverageObligation, WalkState,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one declarator.
    ///
    /// Example:
    /// ```ds
    /// value: number = 1
    /// ```
    pub(in crate::sema) fn walk_declarator(
        &mut self,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
        is_ambient: bool,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }

        // walk the pattern and its annotation
        let matched = self.walk_declarator_pattern(declarator, is_ambient)?;

        // walk matched value
        if let Some(value) = declarator.value {
            self.walk_expression(value, self.tree.get(value))?;
        }

        // select pattern coverage from the initializer or annotation
        let expected = match (declarator.value, matched) {
            (Some(value), _) => Some(ExpectedType::Node(value.into_global_any(self.module))),
            (None, Some(matched)) => Some(ExpectedType::Type(matched)),
            (None, None) => None,
        };

        // require non-matching positions to always succeed
        if let Some(value) = expected
            && !self.check.is_refutable_pattern_position(self.module, id)
        {
            self.check.push_obligation(
                Obligation::PatternCoverage(PatternCoverageObligation {
                    source: declarator.pattern.into_global_any(self.module),
                    value,
                    coverage: PatternCoverage::Binding {
                        pattern: declarator.pattern.into_global(self.module),
                    },
                }),
                self.flow().template_scope(),
            )?;
        }

        Ok(())
    }

    /// Walk one let declaration's annotated and exported bindings.
    pub(in crate::sema) fn walk_let_bindings(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let dir::Expression::Let {
            declarators,
            is_ambient,
            ..
        } = self.tree.get(expression).clone()
        else {
            return Err(CompilerError::Internal {
                message: format!("let binding walk on a non-let expression {expression:?}"),
            });
        };

        // walk each declarator with a declared type
        for id in declarators {
            let declarator = self.tree.get(id).clone();
            if !self.declare_decorators(id.into_any())? {
                continue;
            }

            // read whether the binding leaves this module
            let exported = match self.declared_symbol(declarator.pattern.into_any()) {
                Some(symbol) => self
                    .check
                    .binding_table(symbol.module_id)?
                    .get_symbol(symbol.local_id)
                    .export_kind
                    .is_some(),
                None => false,
            };

            // annotations that infer resolve at their initializers
            let infers = !is_ambient
                && declarator.ty.is_some_and(|annotation| {
                    self.check.is_inferring_annotation(self.module, annotation)
                });
            if (declarator.ty.is_none() || infers) && !exported {
                continue;
            }
            if infers {
                self.check
                    .report_missing_export_binding_type(self.module, declarator.pattern.into_any());

                continue;
            }

            // walk the pattern and its annotation
            self.walk_declarator_pattern(&declarator, is_ambient)?;

            // exported bindings without annotations keep literal values only
            if declarator.ty.is_none()
                && let Some(value) = declarator.value
            {
                if self.check.is_literal_initializer(self.module, value) {
                    self.walk_expression(value, self.tree.get(value))?;
                } else {
                    self.check.report_missing_export_binding_type(
                        self.module,
                        declarator.pattern.into_any(),
                    );
                }
            }
        }

        Ok(())
    }

    /// Walk one declarator's pattern and annotation, returning the annotated type.
    fn walk_declarator_pattern(
        &mut self,
        declarator: &dir::Declarator,
        is_ambient: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // walk the bound pattern itself
        self.walk_pattern(declarator.pattern, self.tree.get(declarator.pattern), true)?;

        // walk the declared pattern type
        let matched = match declarator.ty {
            Some(annotation) => Some(self.walk_binding_type(annotation, is_ambient)?),
            None => None,
        };

        Ok(matched)
    }
}

impl CheckState<'_> {
    /// Return whether one declarator position permits a refutable pattern.
    pub(in crate::sema) fn is_refutable_pattern_position(
        &self,
        module: ModuleId,
        declarator: dir::LocalNodeId<dir::Declarator>,
    ) -> bool {
        let view = self.module(module).view();
        let Some(parent) = view.get_parent_for(declarator) else {
            return false;
        };

        match parent.ty {
            // recognize expression conditions and let-else bindings
            dir::NodeType::Expression => {
                let expression = view.get(dir::LocalNodeId::<dir::Expression>::new(parent.id));

                matches!(
                    expression,
                    dir::Expression::LetElse { declarator: binding, .. }
                        if *binding == declarator
                ) || matches!(
                    expression,
                    dir::Expression::If { condition, .. }
                        | dir::Expression::While { condition, .. }
                        if condition.binds(declarator)
                )
            }
            // recognize match guard bindings
            dir::NodeType::MatchArm => {
                let arm = view.get(dir::LocalNodeId::<dir::MatchArm>::new(parent.id));

                arm.guard()
                    .is_some_and(|condition| condition.binds(declarator))
            }
            _ => false,
        }
    }

    /// Narrow flow from one matched declarator pattern.
    ///
    /// Example:
    /// ```ds
    /// if let Some(value) = option { value }
    /// ```
    pub(in crate::sema) fn narrow_declarator_match(
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
    pub(in crate::sema) fn narrow_declarator_pattern(
        &mut self,
        id: dir::LocalNodeId<dir::Declarator>,
        is_positive: bool,
    ) -> CompilerResult<()> {
        let declarator = self.module(self.module_id).view().get(id);
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
    pub(in crate::sema) fn narrow_pattern(
        &mut self,
        path: dir::AccessPath,
        pattern: dir::LocalNodeId<dir::Pattern>,
        is_positive: bool,
    ) -> CompilerResult<()> {
        match self.module(self.module_id).view().get(pattern) {
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
                    pattern: pattern.into_global(self.module_id),
                    is_positive,
                };

                self.flow.apply_narrowing(path, predicate);
            }
            // T(a, b), T { name }
            dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::NominalObject { fields, .. } => {
                let fields = fields.clone();
                let predicate = FlowPredicate::Pattern {
                    pattern: pattern.into_global(self.module_id),
                    is_positive,
                };

                if is_positive {
                    self.flow.apply_narrowing(path.clone(), predicate);
                    self.narrow_pattern_field_match(path, &fields)?;
                } else {
                    self.flow.apply_narrowing(path, predicate);
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
            match self.module(self.module_id).view().get(*field) {
                // { name: pattern }
                dir::PatternField::Named {
                    name,
                    pattern: Some(pattern),
                    ..
                } => {
                    let (name, pattern) = (*name, *pattern);
                    let mut field_path = path.clone();
                    field_path.push(name.into());

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

impl CheckState<'_> {
    /// Return whether one annotation writes a hole or elides a borrow lifetime.
    pub(in crate::sema) fn is_inferring_annotation(
        &self,
        module: ModuleId,
        annotation: dir::LocalNodeId<dir::TypeExpression>,
    ) -> bool {
        let tree = self.module(module).view();

        // find elided borrows and type holes written inside the annotation
        for (id, node) in tree.iter_nodes::<dir::TypeExpression>() {
            if matches!(
                node,
                dir::TypeExpression::BorrowedOf { lifetime: None, .. }
                    | dir::TypeExpression::Infer { .. }
            ) && tree.is_inside(id.into_any(), annotation.into_any())
            {
                return true;
            }
        }

        // find value holes like fixed-array lengths
        for (id, node) in tree.iter_nodes::<dir::Expression>() {
            if matches!(node, dir::Expression::Infer { .. })
                && tree.is_inside(id.into_any(), annotation.into_any())
            {
                return true;
            }
        }

        false
    }

    /// Return the newtype declaration one call head names, when one does.
    pub(in crate::sema) fn written_newtype_head(
        &self,
        module: ModuleId,
        callee: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        // resolve the head to one bound declaration
        let state = self.module(module);
        let reference = state
            .resolved
            .references
            .get(callee.into_global_any(module))?;
        let dir::Reference::Bound(symbols) = reference else {
            return None;
        };
        let symbols = self.present_symbols(symbols);
        let [symbol] = symbols.as_slice() else {
            return None;
        };

        // require a newtype declaration in the walked module
        let definition = self.module(symbol.module_id).definition(*symbol)?;
        matches!(*definition, dir::Definition::Newtype(_)).then_some(*symbol)
    }

    /// Return whether an initializer's type reads off its written form without inference.
    pub(in crate::sema) fn is_literal_initializer(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let tree = self.module(module).view();
        match tree.get(expression) {
            // 1, "text", true
            dir::Expression::Literal(_) => true,
            // `text`
            dir::Expression::TemplateExpression {
                value: dir::TemplateLiteral::String { .. },
            } => true,
            // signed number literals
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Negate | dir::UnaryOperator::Plus,
                right,
            } => matches!(tree.get(*right), dir::Expression::Literal(_)),
            // 1 << 2, "a" + "b"
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                dir::StaticBinaryOperator::try_from(*operator).is_ok()
                    && self.is_literal_initializer(module, *left)
                    && self.is_literal_initializer(module, *right)
            }
            // Color.Red, this.width
            dir::Expression::Member {
                left,
                name: Some(_),
                is_optional: false,
            } => matches!(
                tree.get(*left),
                dir::Expression::Identifier { .. } | dir::Expression::This
            ),
            // value as const
            dir::Expression::As {
                expression,
                target_type,
            } if matches!(tree.get(*target_type), dir::TypeExpression::Const) => {
                self.is_literal_initializer(module, *expression)
            }
            // [1, 2], (1, "a")
            dir::Expression::ArrayExpression { elements }
            | dir::Expression::TupleExpression { elements } => {
                elements.iter().all(|element| match tree.get(*element) {
                    dir::Argument::Positional { value } => {
                        self.is_literal_initializer(module, *value)
                    }
                    _ => false,
                })
            }
            // [0; 16]
            dir::Expression::FixedArrayExpression { value, length } => {
                self.is_literal_initializer(module, *value)
                    && self.is_literal_initializer(module, *length)
            }
            // { name: "a" }
            dir::Expression::ObjectExpression { properties } => {
                properties.iter().all(|property| match tree.get(*property) {
                    dir::Property::Field { value, .. } => {
                        self.is_literal_initializer(module, *value)
                    }
                    _ => false,
                })
            }
            // SocketFlags(1), the head names a newtype over literal initializers
            dir::Expression::Call {
                position: dir::PostfixPosition::Direct,
                left,
                generic_arguments,
                arguments,
                is_optional: false,
            } if generic_arguments.is_empty() => {
                self.written_newtype_head(module, *left).is_some()
                    && arguments.iter().all(|argument| match tree.get(*argument) {
                        dir::Argument::Positional { value } => {
                            self.is_literal_initializer(module, *value)
                        }
                        _ => false,
                    })
            }
            // Position { x: 1, y: 2 }, the head names the type
            dir::Expression::StructExpression { properties, .. } => {
                properties.iter().all(|property| match tree.get(*property) {
                    dir::Property::Field { value, .. } => {
                        self.is_literal_initializer(module, *value)
                    }
                    _ => false,
                })
            }
            _ => false,
        }
    }
}
