use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    AssignPatternField, AssignPatternTerm, GenericInstance, Origin, PatternBindingResolution,
    PatternBorrowResolution, PatternDecision, PatternDereferenceResolution, PatternField,
    PatternFieldResolution, PatternFieldTargetResolution, PatternLiteralResolution,
    PatternMoveResolution, PatternNewtypeResolution, PatternNominalResolution,
    PatternRangeResolution, PatternResolution, PatternShapeResolution, PatternTarget,
    PatternTargetResolution, PatternTerm, PatternTupleResolution, PatternUnionResolution,
    PatternVariantResolution, TermId, TypeOperand, TypeTerm, WalkState,
};

impl WalkState<'_, '_> {
    /// Walk one pattern.
    ///
    /// Example:
    /// ```ds
    /// Some({ name }) if name is string
    /// ```
    pub(in crate::check) fn walk_pattern(
        &mut self,
        id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        match pattern {
            // _
            dir::Pattern::Wildcard => {}
            // pattern!
            dir::Pattern::Must(pattern)
            // &pattern
            | dir::Pattern::BorrowOf { right: pattern, .. }
            // move pattern
            | dir::Pattern::MoveOf { right: pattern, .. }
            // *pattern
            | dir::Pattern::DereferenceOf { right: pattern } => {
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // pattern = value
            dir::Pattern::Assign { pattern, value } => {
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;

                // check pattern default in selector context
                let before_value = self.fork_flow();
                self.walk_expression(*value, self.tree.get(*value))?;
                self.restore_flow(before_value);
            }
            // name: pattern
            dir::Pattern::Binding {
                pattern: Some(pattern),
                ..
            } => {
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // name
            dir::Pattern::Binding { pattern: None, .. } => {}
            // value
            dir::Pattern::Expression { value } => {
                // check value pattern in selector context
                let before_value = self.fork_flow();
                self.walk_expression(*value, self.tree.get(*value))?;
                self.restore_flow(before_value);
            }
            // start..end
            dir::Pattern::Range { start, end, .. } => {
                if let Some(start) = start {
                    // check range bound in selector context
                    let before_start = self.fork_flow();
                    self.walk_expression(*start, self.tree.get(*start))?;
                    self.restore_flow(before_start);
                }

                if let Some(end) = end {
                    // check range bound in selector context
                    let before_end = self.fork_flow();
                    self.walk_expression(*end, self.tree.get(*end))?;
                    self.restore_flow(before_end);
                }
            }
            // value is T
            dir::Pattern::TypeExpression { value } => {
                self.walk_type_expression(*value, self.tree.get(*value))?;
            }
            // [a, b]
            dir::Pattern::Tuple { fields } => {
                for field in fields {
                    self.walk_pattern_field(*field, self.tree.get(*field))?;
                }
            }
            // [...items]
            dir::Pattern::Sequence { fields } => {
                for field in fields {
                    self.walk_pattern_field(*field, self.tree.get(*field))?;
                }
            }
            // { name }
            dir::Pattern::Object { fields } => {
                for field in fields {
                    self.walk_pattern_field(*field, self.tree.get(*field))?;
                }
            }
            // T(a, b)
            dir::Pattern::Newtype { ty, fields } => {
                self.walk_type_expression(*ty, self.tree.get(*ty))?;

                for field in fields {
                    self.walk_pattern_field(*field, self.tree.get(*field))?;
                }
            }
            // T { name }
            dir::Pattern::NominalObject { ty, fields } => {
                self.walk_type_expression(*ty, self.tree.get(*ty))?;

                for field in fields {
                    self.walk_pattern_field(*field, self.tree.get(*field))?;
                }
            }
            // a | b
            dir::Pattern::Union { patterns } => {
                for pattern in patterns {
                    self.walk_pattern(*pattern, self.tree.get(*pattern))?;
                }
            }
        };

        Ok(())
    }

    /// Walk one pattern field.
    ///
    /// Example:
    /// ```ds
    /// { name: pattern }
    /// ```
    pub(in crate::check) fn walk_pattern_field(
        &mut self,
        id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        match pattern_field {
            // { name: pattern }
            dir::PatternField::Named {
                pattern: Some(pattern),
                ..
            }
            // { ...pattern }
            | dir::PatternField::Spread {
                pattern: Some(pattern),
            } => {
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // { [key]: pattern }
            dir::PatternField::Computed { key, pattern } => {
                // check computed key in selector context
                let before_key = self.fork_flow();
                self.walk_expression(*key, self.tree.get(*key))?;
                self.restore_flow(before_key);

                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // [pattern]
            dir::PatternField::Positional { pattern } => {
                self.walk_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // { name }
            dir::PatternField::Named { pattern: None, .. }
            // { ... }
            | dir::PatternField::Spread { pattern: None }
            // [,]
            | dir::PatternField::Elision => {}
        };

        Ok(())
    }

    /// Walk one assignment pattern.
    ///
    /// Example:
    /// ```ds
    /// { name } = value
    /// ```
    pub(in crate::check) fn walk_assign_pattern(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPattern>,
        assign_pattern: &dir::AssignPattern,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        match assign_pattern {
            // target
            dir::AssignPattern::Expression { value } => {
                self.walk_assignment_target(*value)?;
            }
            // target = value
            dir::AssignPattern::Assign { pattern, value } => {
                self.walk_assign_pattern(*pattern, self.tree.get(*pattern))?;

                // check destructuring default in conditional assignment context
                let before_value = self.fork_flow();
                self.walk_expression(*value, self.tree.get(*value))?;
                self.restore_flow(before_value);
            }
            // [a, b]
            dir::AssignPattern::Sequence { fields }
            // { a, b }
            | dir::AssignPattern::Object { fields } => {
                for field in fields {
                    self.walk_assign_pattern_field(*field, self.tree.get(*field))?;
                }
            }
        };

        Ok(())
    }

    /// Walk one assignment pattern field.
    ///
    /// Example:
    /// ```ds
    /// { name: target }
    /// ```
    pub(in crate::check) fn walk_assign_pattern_field(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        assign_pattern_field: &dir::AssignPatternField,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        match assign_pattern_field {
            // { name: pattern }
            dir::AssignPatternField::Named {
                pattern: Some(pattern),
                ..
            }
            // { ...pattern }
            | dir::AssignPatternField::Spread {
                pattern: Some(pattern),
            } => {
                self.walk_assign_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // { [key]: pattern }
            dir::AssignPatternField::Computed { key, pattern } => {
                self.walk_expression(*key, self.tree.get(*key))?;
                self.walk_assign_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // [pattern]
            dir::AssignPatternField::Positional { pattern } => {
                self.walk_assign_pattern(*pattern, self.tree.get(*pattern))?;
            }
            // { name }
            dir::AssignPatternField::Named { pattern: None, .. }
            // { ... }
            | dir::AssignPatternField::Spread { pattern: None }
            // [,]
            | dir::AssignPatternField::Elision => {}
        };

        Ok(())
    }

    /// Return one pattern term from syntax.
    ///
    /// Example:
    /// ```ds
    /// Some(value)
    /// ```
    pub(in crate::check) fn pattern_term(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<Option<TermId<PatternTerm>>> {
        let term = match self.tree.get(id) {
            // _
            dir::Pattern::Wildcard => PatternTarget::Wildcard,
            // pattern!
            dir::Pattern::Must(pattern) => {
                let Some(pattern) = self.pattern_term(module, *pattern)? else {
                    return Ok(None);
                };

                PatternTarget::Must { pattern }
            }
            // pattern = value
            dir::Pattern::Assign { pattern, value } => {
                let Some(pattern) = self.pattern_term(module, *pattern)? else {
                    return Ok(None);
                };
                let value = self.node_type_operand(*value)?;

                PatternTarget::Assign { pattern, value }
            }
            // &pattern
            dir::Pattern::BorrowOf { mutability, right } => {
                let Some(pattern) = self.pattern_term(module, *right)? else {
                    return Ok(None);
                };

                PatternTarget::BorrowOf {
                    mutability: *mutability,
                    pattern,
                }
            }
            // ^pattern
            dir::Pattern::MoveOf { mutability, right } => {
                let Some(pattern) = self.pattern_term(module, *right)? else {
                    return Ok(None);
                };

                PatternTarget::MoveOf {
                    mutability: *mutability,
                    pattern,
                }
            }
            // *pattern
            dir::Pattern::DereferenceOf { right } => {
                let Some(pattern) = self.pattern_term(module, *right)? else {
                    return Ok(None);
                };

                PatternTarget::DereferenceOf { pattern }
            }
            // name, name: pattern
            dir::Pattern::Binding { pattern, .. } => PatternTarget::Binding {
                symbol: self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any()),
                pattern: pattern
                    .map(|pattern| self.pattern_term(module, pattern))
                    .transpose()?
                    .flatten(),
            },
            // value
            dir::Pattern::Expression { value } => PatternTarget::Expression {
                value: self.node_type_operand(*value)?,
            },
            // start..end
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => PatternTarget::Range {
                start: start
                    .map(|start| self.node_type_operand(start))
                    .transpose()?,
                end: end.map(|end| self.node_type_operand(end)).transpose()?,
                end_kind: *end_kind,
            },
            // value is T
            dir::Pattern::TypeExpression { value } => PatternTarget::Type {
                ty: self.node_type_operand(*value)?,
            },
            // [a, b]
            dir::Pattern::Tuple { fields } => PatternTarget::Tuple {
                fields: fields
                    .iter()
                    .map(|field| self.pattern_field_term(module, *field))
                    .collect::<CompilerResult<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect(),
            },
            // T(a, b)
            dir::Pattern::Newtype { ty, fields } => PatternTarget::Newtype {
                ty: self.node_type_operand(*ty)?,
                fields: fields
                    .iter()
                    .map(|field| self.pattern_field_term(module, *field))
                    .collect::<CompilerResult<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect(),
            },
            // [...items]
            dir::Pattern::Sequence { fields } => PatternTarget::Sequence {
                fields: fields
                    .iter()
                    .map(|field| self.pattern_field_term(module, *field))
                    .collect::<CompilerResult<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect(),
            },
            // { name }
            dir::Pattern::Object { fields } => PatternTarget::Object {
                fields: fields
                    .iter()
                    .map(|field| self.pattern_field_term(module, *field))
                    .collect::<CompilerResult<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect(),
            },
            // T { name }
            dir::Pattern::NominalObject { ty, fields } => PatternTarget::NominalObject {
                ty: self.node_type_operand(*ty)?,
                fields: fields
                    .iter()
                    .map(|field| self.pattern_field_term(module, *field))
                    .collect::<CompilerResult<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect(),
            },
            // a | b
            dir::Pattern::Union { patterns } => PatternTarget::Union {
                patterns: patterns
                    .iter()
                    .map(|pattern| self.pattern_term(module, *pattern))
                    .collect::<CompilerResult<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect(),
            },
        };

        // immediately decide patterns when possible
        let source = id.into_global_any(module);
        let term = self
            .check
            .inference
            .push_term(PatternTerm::node(source, term));
        if let Some(target) = self.pattern_resolution(module, id)? {
            let selection = PatternResolution { source, target };
            self.check
                .inference
                .select_pattern(source, PatternDecision::Resolved(selection))?;
        }

        Ok(Some(term))
    }

    /// Return one pattern selection when the pattern determines it.
    ///
    /// Example:
    /// ```ds
    /// { name, age }
    /// ```
    fn pattern_resolution(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<Option<PatternTargetResolution>> {
        match self.tree.get(id) {
            // _
            dir::Pattern::Wildcard => Ok(Some(PatternTargetResolution::Wildcard)),
            // name, name: pattern
            dir::Pattern::Binding { pattern, .. } => {
                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());
                let pattern = pattern.map(|pattern| pattern.into_global_any(module));

                Ok(Some(PatternTargetResolution::Binding(
                    PatternBindingResolution { symbol, pattern },
                )))
            }
            // &pattern
            dir::Pattern::BorrowOf { mutability, right } => {
                let access = mutability.map(dir::Mutability::access);
                let pattern = right.into_global_any(module);

                Ok(Some(PatternTargetResolution::Borrow(
                    PatternBorrowResolution { access, pattern },
                )))
            }
            // ^pattern
            dir::Pattern::MoveOf { mutability, right } => {
                let access = mutability.map(dir::Mutability::access);
                let pattern = right.into_global_any(module);

                Ok(Some(PatternTargetResolution::Move(PatternMoveResolution {
                    access,
                    pattern,
                })))
            }
            // *pattern
            dir::Pattern::DereferenceOf { right } => {
                let pattern = right.into_global_any(module);

                Ok(Some(PatternTargetResolution::Dereference(
                    PatternDereferenceResolution { pattern },
                )))
            }
            // value
            dir::Pattern::Expression { value } => {
                let value = value.into_global_any(module);
                let Some(value) = self.check.inputs.node_static(value) else {
                    return Ok(None);
                };

                Ok(Some(PatternTargetResolution::Literal(
                    PatternLiteralResolution { value },
                )))
            }
            // static scalar range
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => {
                // range domain
                let Some(domain) = self.pattern_range_domain(*start, *end)? else {
                    return Ok(None);
                };

                // start
                let start = if let Some(start) = start {
                    let node = start.into_global_any(module);
                    let Some(operand) = self.check.inputs.node_static(node) else {
                        return Ok(None);
                    };

                    Some(operand)
                } else {
                    None
                };

                // end
                let end = if let Some(end) = end {
                    let node = end.into_global_any(module);
                    let Some(operand) = self.check.inputs.node_static(node) else {
                        return Ok(None);
                    };

                    Some(operand)
                } else {
                    None
                };

                Ok(Some(PatternTargetResolution::Range(
                    PatternRangeResolution {
                        domain,
                        start,
                        end,
                        end_bound: *end_kind,
                    },
                )))
            }
            // [a, b]
            dir::Pattern::Tuple { fields } => {
                let Some(fields) = self.indexed_pattern_field_resolutions(module, fields) else {
                    return Ok(None);
                };

                Ok(Some(PatternTargetResolution::Tuple(
                    PatternTupleResolution { fields },
                )))
            }
            // T(a, b)
            dir::Pattern::Newtype { ty, fields } => {
                // target
                let Some((symbol, instance)) = self.pattern_reference_target(*ty)? else {
                    return Ok(None);
                };

                // fields
                let Some(fields) = self.indexed_pattern_field_resolutions(module, fields) else {
                    return Ok(None);
                };
                let value = fields.first().and_then(|field| field.pattern);

                Ok(Some(PatternTargetResolution::Newtype(
                    PatternNewtypeResolution {
                        symbol,
                        instance,
                        value,
                    },
                )))
            }
            // { name }
            dir::Pattern::Object { fields } => {
                // fields
                let Some(fields) = self.keyed_pattern_field_resolutions(module, fields) else {
                    return Ok(None);
                };

                Ok(Some(PatternTargetResolution::Shape(
                    PatternShapeResolution { fields },
                )))
            }
            // T { name }
            dir::Pattern::NominalObject { ty, fields } => {
                // target
                let Some((symbol, instance)) = self.pattern_reference_target(*ty)? else {
                    return Ok(None);
                };

                // fields
                let Some(fields) = self.keyed_pattern_field_resolutions(module, fields) else {
                    return Ok(None);
                };

                // enum variant
                if self.check.symbol_kind(symbol) == dir::SymbolKind::EnumField {
                    let discriminant = Some(self.check.symbol_static_operand(module, symbol)?);

                    return Ok(Some(PatternTargetResolution::Variant(
                        PatternVariantResolution {
                            symbol,
                            instance,
                            discriminant,
                            fields,
                        },
                    )));
                }

                // nominal object
                Ok(Some(PatternTargetResolution::Nominal(
                    PatternNominalResolution {
                        symbol,
                        instance,
                        fields,
                    },
                )))
            }
            // a | b
            dir::Pattern::Union { patterns } => {
                let alternatives = patterns
                    .iter()
                    .map(|pattern| pattern.into_global_any(module))
                    .collect();

                Ok(Some(PatternTargetResolution::Union(
                    PatternUnionResolution { alternatives },
                )))
            }
            // value dependent pattern
            dir::Pattern::Must(_)
            | dir::Pattern::Assign { .. }
            | dir::Pattern::TypeExpression { .. }
            | dir::Pattern::Sequence { .. } => Ok(None),
        }
    }

    /// Return a range domain from one static range bound.
    fn pattern_range_domain(
        &mut self,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Option<TypeOperand>> {
        // start
        if let Some(start) = start {
            return self.node_type_operand(start).map(Some);
        }

        // end
        if let Some(end) = end {
            return self.node_type_operand(end).map(Some);
        }

        Ok(None)
    }

    /// Return one direct symbol target from a type pattern.
    fn pattern_reference_target(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<(dir::GlobalSymbolId, Option<GenericInstance>)>> {
        // type expression
        let ty = self.node_type_operand(id)?;
        let Some(ty) = self.check.resolved_type_operand(ty) else {
            return Ok(None);
        };

        // reference
        let (symbol, arguments) = match ty {
            TypeOperand::Term(term) => {
                let TypeTerm::Reference {
                    origin: _,
                    symbol,
                    arguments,
                } = self.check.inference.term(term)
                else {
                    return Ok(None);
                };

                (*symbol, arguments.clone())
            }
            TypeOperand::Type(ty) => {
                let dir::Type::Reference(reference) = self.check.r#type(ty) else {
                    return Ok(None);
                };
                if !reference.arguments.is_empty() {
                    return Ok(None);
                }

                (reference.symbol, Default::default())
            }
            TypeOperand::Variable(_) => return Ok(None),
        };

        // nongeneric reference
        if arguments.is_empty() {
            return Ok(Some((symbol, None)));
        }

        // generic reference
        let Some(template) = self.check.inference.generic_template_by_symbol(symbol) else {
            return Ok(None);
        };
        let instance = GenericInstance::new(template, arguments.into());

        Ok(Some((symbol, Some(instance))))
    }

    /// Return positional field selections for one tuple pattern.
    ///
    /// Example:
    /// ```ds
    /// [first, second]
    /// ```
    fn indexed_pattern_field_resolutions(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> Option<Vec<PatternFieldResolution>> {
        let mut next_index = 0;
        let mut selections = Vec::with_capacity(fields.len());

        // preserve tuple field positions
        for field in fields {
            let source = field.into_global_any(module);
            match self.tree.get(*field) {
                // [pattern]
                dir::PatternField::Positional { pattern } => {
                    let target = PatternFieldTargetResolution::Index(next_index);
                    let pattern = Some(pattern.into_global_any(module));
                    selections.push(PatternFieldResolution {
                        source,
                        target,
                        pattern,
                    });
                    next_index += 1;
                }
                // [,]
                dir::PatternField::Elision => {
                    let target = PatternFieldTargetResolution::Index(next_index);
                    selections.push(PatternFieldResolution {
                        source,
                        target,
                        pattern: None,
                    });
                    next_index += 1;
                }
                // unsupported tuple field forms need solved pattern logic
                dir::PatternField::Named { .. }
                | dir::PatternField::Computed { .. }
                | dir::PatternField::Spread { .. } => return None,
            }
        }

        Some(selections)
    }

    /// Return keyed field selections for one object pattern.
    ///
    /// Example:
    /// ```ds
    /// { name: value }
    /// ```
    fn keyed_pattern_field_resolutions(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> Option<Vec<PatternFieldResolution>> {
        let mut selections = Vec::with_capacity(fields.len());

        // preserve statically named object fields
        for field in fields {
            let source = field.into_global_any(module);
            let dir::PatternField::Named { name, pattern, .. } = self.tree.get(*field) else {
                return None;
            };
            let target = PatternFieldTargetResolution::Key(name.static_key());
            let pattern = pattern.map(|pattern| pattern.into_global_any(module));

            selections.push(PatternFieldResolution {
                source,
                target,
                pattern,
            });
        }

        Some(selections)
    }

    /// Return one pattern field term from syntax.
    ///
    /// Example:
    /// ```ds
    /// { name: pattern }
    /// ```
    fn pattern_field_term(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::PatternField>,
    ) -> CompilerResult<Option<PatternField>> {
        let source = id.into_global_any(module);
        let term = match self.tree.get(id) {
            // { name: pattern }
            dir::PatternField::Named { name, pattern, .. } => {
                let pattern = pattern
                    .map(|pattern| self.pattern_term(module, pattern))
                    .transpose()?
                    .flatten();
                let binding = if pattern.is_none() {
                    self.check
                        .module(self.module)
                        .declaration_symbol(id.into_any())
                } else {
                    None
                };
                let value = pattern.map(|_| {
                    self.check
                        .push_type_variable(module, Origin::Node(id.into_global_any(module)))
                });

                PatternField::Named {
                    source,
                    key: name.static_key(),
                    binding,
                    value,
                    pattern,
                }
            }
            // { [key]: pattern }
            dir::PatternField::Computed { key, pattern } => {
                let key = self.node_type_operand(*key)?;
                let Some(pattern) = self.pattern_term(module, *pattern)? else {
                    return Ok(None);
                };

                PatternField::Computed {
                    source,
                    key,
                    pattern,
                }
            }
            // [pattern]
            dir::PatternField::Positional { pattern } => {
                let Some(pattern) = self.pattern_term(module, *pattern)? else {
                    return Ok(None);
                };

                PatternField::Positional { source, pattern }
            }
            // { ...pattern }
            dir::PatternField::Spread { pattern } => PatternField::Spread {
                source,
                pattern: pattern
                    .map(|pattern| self.pattern_term(module, pattern))
                    .transpose()?
                    .flatten(),
            },
            // [,]
            dir::PatternField::Elision => PatternField::Elision { source },
        };

        Ok(Some(term))
    }

    /// Return one assignment pattern term from syntax.
    ///
    /// Example:
    /// ```ds
    /// [first, ...rest]
    /// ```
    pub(in crate::check) fn assign_pattern_term(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<Option<TermId<AssignPatternTerm>>> {
        let term = match self.tree.get(id) {
            // target
            dir::AssignPattern::Expression { value } => {
                let Some(place) = self.place_term(*value)? else {
                    return Ok(None);
                };

                AssignPatternTerm::Expression { target: place.ty }
            }
            // target = value
            dir::AssignPattern::Assign { pattern, value } => {
                let Some(pattern) = self.assign_pattern_term(module, *pattern)? else {
                    return Ok(None);
                };
                let value = self.node_type_operand(*value)?;

                AssignPatternTerm::Assign { pattern, value }
            }
            // [a, b]
            dir::AssignPattern::Sequence { fields } => AssignPatternTerm::Sequence {
                fields: fields
                    .iter()
                    .map(|field| self.assign_pattern_field_term(module, *field))
                    .collect::<CompilerResult<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect(),
            },
            // { a, b }
            dir::AssignPattern::Object { fields } => AssignPatternTerm::Object {
                fields: fields
                    .iter()
                    .map(|field| self.assign_pattern_field_term(module, *field))
                    .collect::<CompilerResult<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect(),
            },
        };

        Ok(Some(self.check.inference.push_term(term)))
    }

    /// Return one assignment pattern field term from syntax.
    ///
    /// Example:
    /// ```ds
    /// { name: target }
    /// ```
    fn assign_pattern_field_term(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::AssignPatternField>,
    ) -> CompilerResult<Option<AssignPatternField>> {
        let term = match self.tree.get(id) {
            // { name: pattern }
            dir::AssignPatternField::Named { name, pattern, .. } => {
                let pattern = pattern
                    .map(|pattern| self.assign_pattern_term(module, pattern))
                    .transpose()?
                    .flatten();
                let value = pattern.map(|_| {
                    self.check
                        .push_type_variable(module, Origin::Node(id.into_global_any(module)))
                });

                AssignPatternField::Named {
                    key: name.static_key(),
                    value,
                    pattern,
                }
            }
            // { [key]: pattern }
            dir::AssignPatternField::Computed { key, pattern } => {
                let key = self.node_type_operand(*key)?;
                let Some(pattern) = self.assign_pattern_term(module, *pattern)? else {
                    return Ok(None);
                };

                AssignPatternField::Computed { key, pattern }
            }
            // [pattern]
            dir::AssignPatternField::Positional { pattern } => {
                let Some(pattern) = self.assign_pattern_term(module, *pattern)? else {
                    return Ok(None);
                };

                AssignPatternField::Positional { pattern }
            }
            // { ...pattern }
            dir::AssignPatternField::Spread { pattern } => AssignPatternField::Spread {
                pattern: pattern
                    .map(|pattern| self.assign_pattern_term(module, pattern))
                    .transpose()?
                    .flatten(),
            },
            // [,]
            dir::AssignPatternField::Elision => AssignPatternField::Elision,
        };

        Ok(Some(term))
    }
}
