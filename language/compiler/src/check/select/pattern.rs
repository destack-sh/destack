use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, AssignmentSelection, BodyState, Cause, CauseKind, Decision, Expectation, FlowPointId,
    FlowSite, InferMode, Obligation, Origin, PlaceUse, Relation, ValueUse, Widening,
    WritableTargetObligation, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Check one pattern against its input type.
    pub(in crate::check) fn check_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        self.commit_node_type(node.into_any(), input)?;

        self.select_pattern(node, flow, scope, input)
    }

    /// Check one assignment pattern against its input type.
    pub(in crate::check) fn check_assign_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        origin: Origin,
    ) -> CompilerResult<Answer<()>> {
        self.commit_node_type(node.into_any(), input)?;
        answer!(self.select_assign_pattern(node, flow, scope, input, origin)?);

        Ok(Answer::Ready(()))
    }

    /// Select the assignment meaning of one assignment pattern node.
    pub(in crate::check) fn select_assign_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        input_origin: Origin,
    ) -> CompilerResult<Answer<bool>> {
        let module = node.module_id;
        let pattern_origin = Origin::Node(node.into_any(), scope);

        // read each assignment target shape once and dispatch with it
        let pattern = self.module(module).view().get(node.local_id).clone();
        let needs_reduced_input = matches!(
            &pattern,
            dir::AssignPattern::Sequence { .. }
                | dir::AssignPattern::Tuple { .. }
                | dir::AssignPattern::Object { .. }
        );
        let input = if needs_reduced_input {
            answer!(self.reduce_type_head(pattern_origin, input)?)
        } else {
            input
        };

        match pattern {
            // x = value, obj.x = value
            dir::AssignPattern::Place { expression: value } => {
                let Some(place) = answer!(self.select_assignment(
                    FlowSite {
                        node: value.into_global_any(module),
                        flow,
                        scope,
                    },
                    value,
                    PlaceUse::Write
                )?) else {
                    self.commit_decision(node.into_any(), Decision::Rejected)?;

                    return Ok(Answer::Ready(false));
                };
                let target =
                    answer!(self.commit_assign_pattern_place(input_origin, node, place)?);
                let pattern_cause = self.intern_cause(Cause::root(
                    input_origin,
                    CauseKind::Pattern {
                        pattern: node.into_any(),
                    },
                ));
                let pattern_site = FlowSite {
                    node: node.into_any(),
                    flow,
                    scope,
                };
                let expectation = Expectation::assignable(target, pattern_cause, ValueUse::Store);
                answer!(self.check_value(pattern_site, input, expectation)?);

                Ok(Answer::Ready(true))
            }
            // x = default
            dir::AssignPattern::Default { pattern, value } => {
                let value_node = value.into_global_any(module);
                let default = answer!(self.infer_node_type(
                    FlowSite {
                        node: value_node,
                        flow,
                        scope,
                    },
                    PlaceUse::Read
                )?);
                let input = answer!(self.defaulted_pattern_type(pattern_origin, input, default)?);

                // flow the defaulted input into the nested target
                answer!(self.check_pattern_projection(
                    flow,
                    scope,
                    input,
                    pattern.into_global_any(module)
                )?);

                let () = answer!(self.commit_assign_pattern(
                    node,
                    dir::AssignPatternResolution::Default(dir::AssignPatternDefaultResolution {
                        pattern: pattern.into_global_any(module),
                        value: value.into_global_any(module),
                    }),
                )?);

                Ok(Answer::Ready(true))
            }
            // [a, ...rest] = values
            dir::AssignPattern::Sequence { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_assign_sequence_pattern(
                    node,
                    pattern_origin,
                    flow,
                    scope,
                    input,
                    &fields,
                )
            }
            // (a, b) = point
            dir::AssignPattern::Tuple { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_assign_tuple_pattern(node, pattern_origin, flow, scope, input, &fields)
            }
            // { x, y: z } = point
            dir::AssignPattern::Object { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_assign_object_pattern(node, pattern_origin, flow, scope, input, &fields)
            }
        }
    }

    /// Commit one assignment pattern that writes into a selected place.
    pub(in crate::check) fn commit_assign_pattern_place(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        place: AssignmentSelection,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // commit the selected place occurrence
        let target_type = place.write.ty();
        let resolution = place.clone().resolution();
        let source_type = resolution
            .read
            .as_ref()
            .map_or_else(|| resolution.write.ty(), dir::ReadResolution::ty);
        let source = resolution.target;
        self.commit_decision(source, Decision::Assignment(resolution))?;
        self.commit_node_type(source, source_type)?;

        // require the written place to be writable
        let scope = self.origin_scope(origin)?;
        self.push_obligation(
            Obligation::WritableTarget(Box::new(WritableTargetObligation {
                target: place,
                ty: target_type,
            })),
            scope,
        );

        // commit the assignment pattern resolution
        let () = answer!(self.commit_assign_pattern(node, dir::AssignPatternResolution::Place)?);

        Ok(Answer::Ready(target_type))
    }

    /// Select the pattern meaning of one pattern node.
    pub(in crate::check) fn select_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let origin = Origin::Node(node.into_any(), scope);

        // reduce inputs only for patterns that inspect their value
        let pattern = self.module(module).view().get(node.local_id).clone();
        let needs_reduced_input = !matches!(
            &pattern,
            dir::Pattern::Wildcard | dir::Pattern::Binding { .. }
        );
        let input = if needs_reduced_input {
            answer!(self.reduce_type_head(origin, input)?)
        } else {
            input
        };
        let input = answer!(self.filter_destructuring_source(node, origin, input)?);

        match &pattern {
            // _
            dir::Pattern::Wildcard => self.commit_pattern(node, dir::PatternResolution::Ignore),

            // name, name: pattern
            dir::Pattern::Binding { pattern, .. } => {
                self.select_binding_pattern(node, flow, scope, input, *pattern)
            }

            // pattern!, pattern = value
            dir::Pattern::Must(pattern) => {
                let pattern = *pattern;

                self.commit_pattern(
                    node,
                    dir::PatternResolution::Must(dir::PatternMustResolution {
                        pattern: pattern.into_global_any(module),
                    }),
                )
            }
            dir::Pattern::Default { pattern, value } => {
                let (pattern, value) = (*pattern, *value);

                // transcribe the written input while declaring, infer the default while checking
                let input = if self.is_declaration() {
                    input
                } else {
                    let value_node = value.into_global_any(module);
                    let default = answer!(self.infer_node_type(
                        FlowSite {
                            node: value_node,
                            flow,
                            scope,
                        },
                        PlaceUse::Read
                    )?);

                    answer!(self.defaulted_pattern_type(origin, input, default)?)
                };

                // flow the defaulted input into the nested pattern
                answer!(self.check_pattern_projection(
                    flow,
                    scope,
                    input,
                    pattern.into_global_any(module)
                )?);

                self.commit_pattern(
                    node,
                    dir::PatternResolution::Default(dir::PatternDefaultResolution {
                        pattern: pattern.into_global_any(module),
                        value: value.into_global_any(module),
                    }),
                )
            }

            // &pattern, ^pattern, *pattern
            dir::Pattern::BorrowOf { mutability, right } => {
                self.select_borrow_pattern(node, flow, scope, input, *right, *mutability)
            }
            dir::Pattern::MoveOf { mutability, right } => {
                self.select_move_pattern(node, flow, scope, input, *right, *mutability)
            }
            dir::Pattern::DereferenceOf { right } => {
                self.select_dereference_pattern(node, origin, flow, scope, input, *right)
            }

            // 1, "ready"
            dir::Pattern::Expression { value } => {
                let value = *value;

                self.select_literal_pattern(node, origin, flow, scope, input, value)
            }

            // start..end
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => {
                let (start, end, end_kind) = (*start, *end, *end_kind);

                self.select_range_pattern(node, origin, flow, scope, input, start, end, end_kind)
            }

            // (a, b)
            dir::Pattern::Tuple { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_tuple_pattern(node, origin, flow, scope, input, &fields)
            }

            // [a, b, ...rest]
            dir::Pattern::Sequence { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_sequence_pattern(node, origin, flow, scope, input, &fields)
            }

            // { name, nested: pattern }
            dir::Pattern::Object { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_object_pattern(node, origin, flow, scope, input, &fields)
            }

            // T(value), T { name }
            dir::Pattern::NominalTuple { ty, fields } => {
                let (ty, fields) = (*ty, fields.iter().copied().collect::<SmallVec<[_; 4]>>());

                self.select_newtype_pattern(node, origin, flow, scope, ty, &fields)
            }
            dir::Pattern::NominalObject { ty, fields } => {
                let (ty, fields) = (*ty, fields.iter().copied().collect::<SmallVec<[_; 4]>>());

                self.select_nominal_pattern(node, origin, flow, scope, ty, &fields)
            }

            // a | b
            dir::Pattern::Union { patterns } => {
                let patterns = patterns.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_union_pattern(node, flow, scope, input, &patterns)
            }
        }
    }

    /// Select one symbol binding and its optional nested pattern.
    fn select_binding_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        pattern: Option<dir::LocalNodeId<dir::Pattern>>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let origin = Origin::Node(node.into_any(), scope);
        let symbol = self
            .module(module)
            .declaration_symbol(node.local_id.into_any())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("binding pattern {node:?} has no symbol"),
            })?;
        let binding = self.check.symbol_type_maybe(symbol);
        let input = if binding == Some(input) {
            input
        } else {
            let input = self.pattern_binding_type(symbol, input)?;
            if let Some(binding) = binding {
                let cause = self.check.intern_cause(Cause::root(
                    origin,
                    CauseKind::Pattern {
                        pattern: node.into_any(),
                    },
                ));
                answer!(
                    self.check
                        .relate(origin, cause, Relation::Equal, input, binding,)?
                );
            } else {
                self.check.commit_binding_type(symbol, input)?;
            }

            input
        };

        // project the captured input into the nested pattern
        if let Some(pattern) = pattern {
            answer!(self.check_pattern_projection(
                flow,
                scope,
                input,
                pattern.into_global_any(module)
            )?);
        }

        self.commit_pattern(
            node,
            dir::PatternResolution::Bind(dir::PatternBindingResolution {
                symbol: Some(symbol),
                pattern: pattern.map(|pattern| pattern.into_global_any(module)),
            }),
        )
    }

    /// Select one or-pattern, sharing the input across branches.
    fn select_union_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        patterns: &[dir::LocalNodeId<dir::Pattern>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;

        // match every branch against the same input
        for pattern in patterns {
            answer!(self.check_pattern_projection(
                flow,
                scope,
                input,
                (*pattern).into_global_any(module)
            )?);
        }

        self.commit_pattern(
            node,
            dir::PatternResolution::Or(dir::PatternOrResolution {
                patterns: patterns
                    .iter()
                    .map(|pattern| pattern.into_global_any(module))
                    .collect(),
            }),
        )
    }

    /// Commit one pattern decision.
    pub(in crate::check) fn commit_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        resolution: dir::PatternResolution,
    ) -> CompilerResult<Answer<()>> {
        self.commit_decision(node.into_any(), Decision::Pattern(resolution))?;

        Ok(Answer::Ready(()))
    }

    /// Commit one assignment pattern decision.
    pub(in crate::check) fn commit_assign_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        resolution: dir::AssignPatternResolution,
    ) -> CompilerResult<Answer<()>> {
        self.commit_decision(node.into_any(), Decision::AssignPattern(resolution))?;

        Ok(Answer::Ready(()))
    }

    /// Reject one pattern whose tag is not nominal.
    pub(in crate::check) fn reject_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        tag: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        self.report_invalid_pattern_tag(origin, tag)?;

        self.commit_rejected_pattern(node)
    }

    /// Commit one rejected pattern, poisoning the bindings it would introduce.
    pub(in crate::check) fn commit_rejected_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<Answer<()>> {
        self.poison_pattern(node)?;
        self.commit_decision(node.into_any(), Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }

    /// Bind every binding beneath one rejected pattern to the error type.
    fn poison_pattern(&mut self, node: dir::GlobalNodeId<dir::Pattern>) -> CompilerResult<()> {
        let module = node.module_id;
        let error = self.intern_type(dir::Type::Error)?;

        self.poison_pattern_bindings(module, error, node.local_id)
    }

    /// Bind every binding beneath one rejected pattern field to the error type.
    pub(in crate::check) fn poison_pattern_field(
        &mut self,
        field: dir::GlobalNodeId<dir::PatternField>,
    ) -> CompilerResult<()> {
        let module = field.module_id;
        let error = self.intern_type(dir::Type::Error)?;

        self.poison_pattern_field_bindings(module, error, field.local_id)
    }

    /// Bind one rejected pattern's subtree to the error type.
    fn poison_pattern_bindings(
        &mut self,
        module: ModuleId,
        error: dir::GlobalTypeId,
        node: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<()> {
        if let Some(symbol) = self.module(module).declaration_symbol(node.into_any()) {
            self.bind_symbol_type(symbol, error)?;
        }

        // walk pattern binding shape
        match self.module(module).view().get(node).clone() {
            // name: pattern
            dir::Pattern::Binding {
                pattern: Some(pattern),
                ..
            }
            // pattern!
            | dir::Pattern::Must(pattern)
            // &pattern
            | dir::Pattern::BorrowOf { right: pattern, .. }
            // move pattern
            | dir::Pattern::MoveOf { right: pattern, .. }
            // *pattern
            | dir::Pattern::DereferenceOf { right: pattern }
            // pattern = value
            | dir::Pattern::Default { pattern, .. } => {
                self.poison_pattern_bindings(module, error, pattern)?;
            }
            // [a, b]
            dir::Pattern::Tuple { fields }
            // [...items]
            | dir::Pattern::Sequence { fields }
            // { name }
            | dir::Pattern::Object { fields }
            // T(a, b)
            | dir::Pattern::NominalTuple { fields, .. }
            // T { name }
            | dir::Pattern::NominalObject { fields, .. } => {
                for field in fields {
                    self.poison_pattern_field_bindings(module, error, field)?;
                }
            }
            // a | b
            dir::Pattern::Union { patterns } => {
                for pattern in patterns {
                    self.poison_pattern_bindings(module, error, pattern)?;
                }
            }
            // name
            dir::Pattern::Binding { pattern: None, .. }
            // _
            | dir::Pattern::Wildcard
            // value
            | dir::Pattern::Expression { .. }
            // start..end
            | dir::Pattern::Range { .. } => {}
        }

        Ok(())
    }

    /// Bind one rejected pattern field's subtree to the error type.
    fn poison_pattern_field_bindings(
        &mut self,
        module: ModuleId,
        error: dir::GlobalTypeId,
        field: dir::LocalNodeId<dir::PatternField>,
    ) -> CompilerResult<()> {
        if let Some(symbol) = self.module(module).declaration_symbol(field.into_any()) {
            self.bind_symbol_type(symbol, error)?;
        }

        // walk pattern field binding shape
        match self.module(module).view().get(field).clone() {
            // { name: pattern }
            dir::PatternField::Named {
                pattern: Some(pattern),
                ..
            }
            // { [key]: pattern }
            | dir::PatternField::Computed { pattern, .. }
            // T(pattern)
            | dir::PatternField::Positional { pattern }
            // { ...pattern }
            | dir::PatternField::Rest {
                pattern: Some(pattern),
            } => {
                self.poison_pattern_bindings(module, error, pattern)?;
            }
            // { name }, ...rest without a pattern and holes bind nothing nested
            dir::PatternField::Named { pattern: None, .. }
            | dir::PatternField::Rest { pattern: None }
            | dir::PatternField::Elision => {}
        }

        Ok(())
    }
}

impl BodyState<'_, '_> {
    /// Return the type bound by one pattern binding.
    pub(in crate::check) fn pattern_binding_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        input: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut slot = self.check.binding_type_maybe(symbol);
        while let Some(ty) = slot
            && let dir::Type::Form(form) = self.ty(ty)?
        {
            slot = Some(form.value);
        }
        let widening = match slot {
            Some(slot) => match self.check.root_variable(slot)? {
                Some(variable) => self.check.solver.variable(variable)?.widening,
                None => Widening::Never,
            },
            None => Widening::Never,
        };
        let input = match widening {
            Widening::Always => self.widen_type(input)?,
            Widening::Never | Widening::Aggregate | Widening::Multiple => input,
        };

        self.check.place_binding_type(symbol, input)
    }

    /// Return whether one pattern has a default branch.
    pub(in crate::check) fn is_defaulted_pattern(
        &self,
        module: ModuleId,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> bool {
        matches!(
            self.module(module).view().get(pattern),
            dir::Pattern::Default { .. }
        )
    }

    /// Return whether one assignment pattern has a default branch.
    pub(in crate::check) fn is_defaulted_assign_pattern(
        &self,
        module: ModuleId,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
    ) -> bool {
        matches!(
            self.module(module).view().get(pattern),
            dir::AssignPattern::Default { .. }
        )
    }

    /// Filter one union to alternatives satisfying a destructuring requirement.
    fn filter_destructuring_source(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let Some(requirement) = self.destructuring_requirement(node)? else {
            return Ok(Answer::Ready(source));
        };
        if !matches!(self.ty(source)?, dir::Type::Union(_)) {
            return Ok(Answer::Ready(source));
        }

        let operation = dir::TypeOperation::Narrow(dir::NarrowType {
            source,
            target: requirement,
            is_positive: true,
        });
        let narrowed = answer!(self.reduce_operation_type(origin, operation)?);

        // preserve the rejected source for the pattern diagnostic
        if matches!(self.ty(narrowed)?, dir::Type::Never) {
            Ok(Answer::Ready(source))
        } else {
            Ok(Answer::Ready(narrowed))
        }
    }

    /// Return one destructuring pattern's structural requirement.
    fn destructuring_requirement(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let module = node.module_id;
        let pattern = self.module(module).view().get(node.local_id).clone();
        let requirement = match pattern {
            dir::Pattern::Tuple { fields } => Some(self.tuple_requirement(module, &fields)?),
            dir::Pattern::Object { fields } => Some(self.object_requirement(module, &fields)?),
            _ => None,
        };

        Ok(requirement)
    }

    /// Return one tuple pattern's structural requirement.
    fn tuple_requirement(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let unknown = self.intern_type(dir::Type::Unknown)?;
        let mut elements = Vec::with_capacity(fields.len());
        for field in fields {
            let field = self.module(module).view().get(*field);
            match field {
                dir::PatternField::Positional { pattern }
                | dir::PatternField::Named {
                    pattern: Some(pattern),
                    ..
                } => {
                    let mut element = dir::TypeElement::new(unknown);
                    element.is_optional = self.is_defaulted_pattern(module, *pattern);
                    elements.push(element);
                }
                dir::PatternField::Named { pattern: None, .. } | dir::PatternField::Elision => {
                    elements.push(dir::TypeElement::new(unknown));
                }
                dir::PatternField::Computed { .. } | dir::PatternField::Rest { .. } => {}
            }
        }
        let elements = self.intern_elements(module, &elements)?;
        let tuple = dir::TupleType {
            form: dir::TupleForm::Tuple,
            elements,
        };

        self.intern_type(dir::Type::Tuple(tuple))
    }

    /// Return one object pattern's structural requirement.
    fn object_requirement(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let unknown = self.intern_type(dir::Type::Unknown)?;
        let mut required = Vec::new();
        for field in fields {
            let Some((key, pattern)) = self.object_field_requirement(module, *field)? else {
                continue;
            };
            if pattern.is_some_and(|pattern| self.is_defaulted_pattern(module, pattern)) {
                continue;
            }
            required.push(dir::TypeProperty {
                key,
                access: dir::PropertyAccess::Read(unknown),
                is_optional: false,
            });
        }
        let fields = self.intern_properties(module, &required)?;
        let shape = dir::ShapeType {
            properties: fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures: dir::TypeListId::EMPTY,
        };

        // the required keys constrain the matched value, they are not a class
        let ty = self.intern_type(dir::Type::Shape(shape))?;

        Ok(ty)
    }

    /// Return the static key named by one object pattern field.
    fn object_field_requirement(
        &mut self,
        module: ModuleId,
        field: dir::LocalNodeId<dir::PatternField>,
    ) -> CompilerResult<Option<(dir::StaticKey, Option<dir::LocalNodeId<dir::Pattern>>)>> {
        let field = self.module(module).view().get(field).clone();
        let key = match field {
            dir::PatternField::Named { name, pattern, .. } => Some((name.static_key(), pattern)),
            dir::PatternField::Computed { key, pattern } => self
                .evaluate_static_key(module, key)?
                .map(|key| (key, Some(pattern))),
            dir::PatternField::Positional { .. }
            | dir::PatternField::Rest { .. }
            | dir::PatternField::Elision => None,
        };

        Ok(key)
    }

    /// Flow one projected value into a nested pattern hole.
    pub(in crate::check) fn check_pattern_projection(
        &mut self,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        pattern: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<()>> {
        let site = FlowSite {
            node: pattern,
            flow,
            scope,
        };
        let expectation = Expectation {
            target: input,
            relation: Relation::Assignable,
            cause: self.check.intern_cause(Cause::root(
                Origin::Node(pattern, scope),
                CauseKind::Pattern { pattern },
            )),
            use_: ValueUse::Store,
            mode: InferMode::Exact,
        };
        answer!(self.attempt_node(site, PlaceUse::Read, Some(expectation))?);

        Ok(Answer::Ready(()))
    }

    /// Return the type produced by one defaulted pattern.
    pub(in crate::check) fn defaulted_pattern_type(
        &mut self,
        origin: Origin,
        input: dir::GlobalTypeId,
        default: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let input = answer!(self.reduce_type_head(origin, input)?);
        let mut kept = Vec::new();
        let mut has_undefined = false;

        // split the undefined arm that triggers the default
        match self.ty(input)? {
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 4]> =
                    SmallVec::from_slice(self.type_ids(input.module_id, union.elements)?);
                for element in elements {
                    if self.ty(element)?.is_undefined() {
                        has_undefined = true;
                    } else {
                        kept.push(element);
                    }
                }
            }
            _ if self.ty(input)?.is_undefined() => {
                has_undefined = true;
            }
            _ => {
                kept.push(input);
            }
        }

        // add the default only when it can actually run
        if has_undefined {
            kept.push(default);
        }

        let ty = self.normalized_union_type(kept)?;

        Ok(Answer::Ready(ty))
    }
}
