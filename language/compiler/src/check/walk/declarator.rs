use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, ExpectedType, FlowPredicate, Obligation, Origin, PatternCoverage,
    PatternCoverageObligation, WalkState, Widening,
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

        let matched = self.walk_declarator_pattern(declarator, binding_kind, is_ambient)?;

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

    /// Walk one let declaration's annotated and exported bindings.
    ///
    /// Every other declarator belongs to the checking pass.
    pub(in crate::check) fn walk_let_bindings(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let dir::Expression::Let {
            kind,
            declarators,
            is_ambient,
            ..
        } = self.tree.get(expression).clone()
        else {
            return Err(CompilerError::Internal {
                message: format!("let binding walk on a non-let expression {expression:?}"),
            });
        };

        for id in declarators {
            let declarator = self.tree.get(id).clone();
            if !self.declare_decorators(id.into_any())? {
                continue;
            }

            let exported = self
                .check
                .module(self.module)
                .declaration_symbol(declarator.pattern.into_any())
                .is_some_and(|symbol| {
                    self.check
                        .binding_table(symbol.module_id)
                        .get_symbol(symbol.local_id)
                        .export_kind
                        .is_some()
                });

            // annotations that infer resolve at their initializers
            let infers = !is_ambient
                && declarator.ty.is_some_and(|annotation| {
                    self.check.annotation_infers(self.module, annotation)
                });
            if (declarator.ty.is_none() || infers) && !exported {
                continue;
            }
            if infers {
                self.check
                    .report_missing_export_binding_type(self.module, declarator.pattern.into_any());
                continue;
            }

            self.walk_declarator_pattern(&declarator, Some(kind), is_ambient)?;

            // exported bindings without annotations keep literal values only
            if declarator.ty.is_none()
                && let Some(value) = declarator.value
            {
                if self.check.is_transcribable_literal(self.module, value) {
                    self.walk_declarator_initializer(id, value)?;
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
        binding_kind: Option<dir::LetKind>,
        is_ambient: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let widening = self
            .check
            .declarator_widening(self.module, declarator, binding_kind);
        self.walk_pattern(
            declarator.pattern,
            self.tree.get(declarator.pattern),
            Some(widening),
        )?;

        // the declared layer already carries transcribed annotation rows
        let declared_row = self
            .check
            .module(self.module)
            .declaration_symbol(declarator.pattern.into_any())
            .is_some_and(|symbol| {
                let module = self.check.module(self.module);
                module.declared.is_some() && module.types.get_symbol_type_id(symbol).is_some()
            });

        // walk the declared pattern type
        let matched = match declarator.ty {
            Some(_) if declared_row && !self.check.is_declaration() => None,
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

        Ok(matched)
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

impl CheckState<'_> {
    /// Return the widening policy for one inferred declarator initializer.
    pub(in crate::check) fn declarator_widening(
        &self,
        module: ModuleId,
        declarator: &dir::Declarator,
        binding_kind: Option<dir::LetKind>,
    ) -> Widening {
        // annotated and valueless declarators never widen
        if declarator.ty.is_some() {
            return Widening::Never;
        }
        let Some(value) = declarator.value else {
            return Widening::Never;
        };

        let tree = self.module(module).view();
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

    /// Return whether one annotation writes a hole or elides a borrow lifetime.
    pub(in crate::check) fn annotation_infers(
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

    /// Return whether an initializer's type transcribes without inference.
    pub(in crate::check) fn is_transcribable_literal(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let tree = self.module(module).view();
        match tree.get(expression) {
            // 1, "text", true
            dir::Expression::ScalarLiteral(_) => true,
            dir::Expression::TemplateExpression {
                value: dir::TemplateLiteral::String { .. },
            } => true,
            // signed number literals
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Negate | dir::UnaryOperator::Plus,
                right,
            } => matches!(tree.get(*right), dir::Expression::ScalarLiteral(_)),
            // value as const
            dir::Expression::As {
                expression,
                target_type,
            } if matches!(tree.get(*target_type), dir::TypeExpression::Const) => {
                self.is_transcribable_literal(module, *expression)
            }
            // [1, 2], (1, "a")
            dir::Expression::ArrayExpression { elements }
            | dir::Expression::TupleExpression { elements } => {
                elements.iter().all(|element| match tree.get(*element) {
                    dir::Argument::Positional { value } => {
                        self.is_transcribable_literal(module, *value)
                    }
                    _ => false,
                })
            }
            // [0; 16]
            dir::Expression::FixedArrayExpression { value, length } => {
                self.is_transcribable_literal(module, *value)
                    && self.is_transcribable_literal(module, *length)
            }
            // { name: "a" }
            dir::Expression::ObjectExpression { properties } => {
                properties.iter().all(|property| match tree.get(*property) {
                    dir::Property::Field {
                        key: dir::Key::Name(_),
                        value,
                        ..
                    } => self.is_transcribable_literal(module, *value),
                    _ => false,
                })
            }
            _ => false,
        }
    }
}
