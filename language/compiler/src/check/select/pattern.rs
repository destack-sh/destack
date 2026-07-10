use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Constraint, Decision, ExpectedType, FlowPointId, FlowSite, Obligation,
    Origin, PlaceUse, Relation, Task, ValueUse, WritablePlaceObligation, WriteTarget, answer,
};

impl CheckState<'_> {
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

        let accepted = answer!(self.select_assign_pattern(node, flow, scope, input, origin)?);
        if !accepted {
            return Ok(Answer::Ready(()));
        }

        Ok(Answer::Ready(()))
    }

    /// Select the assignment meaning of one assignment pattern node.
    ///
    /// Assignment patterns decompose their input like match patterns, then
    /// flow leaf inputs into writable places.
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
        match pattern {
            // x = value, obj.x = value
            dir::AssignPattern::Place { expression: value } => {
                let Some(place) = answer!(self.select_assign_place(
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
                self.push_constraint(Constraint::value(
                    Relation::Assignable,
                    input,
                    target,
                    input_origin,
                    ValueUse::Store,
                ));

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
                let input =
                    answer!(self.defaulted_pattern_input(pattern_origin, input, default)?);

                // flow the defaulted input into the nested target
                self.project_pattern_input(flow, scope, input, pattern.into_global_any(module))?;

                let () = answer!(self.commit_assign_pattern(
                    node,
                    dir::AssignPatternResolution::Default(dir::AssignPatternDefaultResolution {
                        pattern: pattern.into_global_any(module),
                        value: value.into_global_any(module),
                    }),
                )?);

                Ok(Answer::Ready(true))
            }
            // [a, , ...rest] = values
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
        place: WriteTarget,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // commit the selected place occurrence
        let target_type = place.ty;
        let resolution = place.clone().resolution();
        self.commit_node_type(resolution.source, target_type)?;

        // require the written place to be writable
        let scope = self.origin_scope(origin);
        self.push_obligation(
            Obligation::WritablePlace(WritablePlaceObligation {
                place,
                ty: target_type,
            }),
            scope,
        );

        // commit the assignment pattern resolution
        let () = answer!(
            self.commit_assign_pattern(node, dir::AssignPatternResolution::Place(resolution))?
        );

        Ok(Answer::Ready(target_type))
    }

    /// Select the pattern meaning of one pattern node.
    ///
    /// Every pattern node carries its own input: the match
    /// site flowed the value into the root, and each parent selection
    /// projects values into its nested pattern holes.
    pub(in crate::check) fn select_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let origin = Origin::Node(node.into_any(), scope);
        let input = answer!(self.reduce_type_head(origin, input)?);
        let input = answer!(self.accepted_pattern_input(node, origin, input)?);

        // read each shape's payload once and dispatch with it
        let view = self.module(module).view();
        match view.get(node.local_id) {
            // _
            dir::Pattern::Wildcard => self.commit_pattern(node, dir::PatternResolution::Ignore),

            // name, name: pattern
            dir::Pattern::Binding { pattern, .. } => {
                let pattern = *pattern;
                let symbol = self
                    .module(module)
                    .declaration_symbol(node.local_id.into_any());
                if let Some(symbol) = symbol {
                    let input = self.pattern_binding_type(symbol, input)?;

                    self.bind_symbol_type(symbol, input)?;
                }
                if let Some(pattern) = pattern {
                    self.project_pattern_input(
                        flow,
                        scope,
                        input,
                        pattern.into_global_any(module),
                    )?;
                }

                self.commit_pattern(
                    node,
                    dir::PatternResolution::Bind(dir::PatternBindingResolution {
                        symbol,
                        pattern: pattern.map(|pattern| pattern.into_global_any(module)),
                    }),
                )
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
                let value_node = value.into_global_any(module);
                let default = answer!(self.infer_node_type(
                    FlowSite {
                        node: value_node,
                        flow,
                        scope,
                    },
                    PlaceUse::Read
                )?);
                let input = answer!(self.defaulted_pattern_input(origin, input, default)?);

                // flow the defaulted input into the nested pattern
                self.project_pattern_input(flow, scope, input, pattern.into_global_any(module))?;

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
            self.project_pattern_input(flow, scope, input, (*pattern).into_global_any(module))?;
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
        self.commit_decision(node.into_any(), Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }
}

impl CheckState<'_> {
    /// Return the input type accepted by one pattern on its success branch.
    pub(in crate::check) fn accepted_pattern_input(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        input: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let Some(target) = answer!(self.pattern_target_type(node)?) else {
            return Ok(Answer::Ready(input));
        };
        let should_narrow = matches!(
            self.ty(input)?,
            dir::Type::Any
                | dir::Type::Unknown
                | dir::Type::Object
                | dir::Type::Union(_)
                | dir::Type::Dynamic(_)
        );
        if !should_narrow {
            return Ok(Answer::Ready(input));
        }

        let operation = dir::TypeOperation::Narrow(dir::NarrowType {
            source: input,
            target,
            is_positive: true,
        });
        let accepted = answer!(self.reduce_operation_type(origin, operation)?);
        if matches!(self.ty(accepted)?, dir::Type::Never) {
            return Ok(Answer::Ready(input));
        }

        Ok(Answer::Ready(accepted))
    }

    /// Return the type bound by one pattern binding.
    pub(in crate::check) fn pattern_binding_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        input: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let binding = self
            .module(symbol.module_id)
            .bindings
            .get_symbol(symbol.local_id);
        if binding.binding_mutability == Some(dir::Mutability::Immutable) {
            Ok(input)
        } else {
            self.widen_type(input)
        }
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

    /// Flow one projected value into a nested pattern hole.
    pub(in crate::check) fn project_pattern_input(
        &mut self,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        pattern: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        self.queue_task(Task::Check {
            site: FlowSite {
                node: pattern,
                flow,
                scope,
            },
            expected: ExpectedType::Type(input),
            relation: Relation::Assignable,
            origin: Origin::Node(pattern, scope),
            use_: ValueUse::Store,
        });

        Ok(())
    }

    /// Return the value produced by one defaulted pattern.
    pub(in crate::check) fn defaulted_pattern_input(
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

        let ty = self.normalized_union_type(origin.module(), kept)?;

        Ok(Answer::Ready(ty))
    }

    /// Return the broad runtime type tested by one destructuring pattern.
    fn pattern_target_type(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = node.module_id;
        let target = match self.module(module).view().get(node.local_id).clone() {
            dir::Pattern::Tuple { fields } => {
                let target = self.tuple_pattern_target_type(module, &fields)?;

                Some(target)
            }
            dir::Pattern::Object { fields } => {
                let target = answer!(self.object_pattern_target_type(module, &fields)?);

                Some(target)
            }
            _ => None,
        };

        Ok(Answer::Ready(target))
    }

    /// Return a tuple type broad enough for one tuple pattern.
    fn tuple_pattern_target_type(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let unknown = self.intern_type(module, dir::Type::Unknown)?;
        let mut elements = Vec::with_capacity(fields.len());
        for field in fields {
            let field = self.module(module).view().get(*field);
            match field {
                dir::PatternField::Positional { pattern } => {
                    let mut element = dir::TypeElement::new(unknown);
                    element.is_optional = self.is_defaulted_pattern(module, *pattern);
                    elements.push(element);
                }
                dir::PatternField::Named {
                    pattern: Some(pattern),
                    ..
                } => {
                    let mut element = dir::TypeElement::new(unknown);
                    element.is_optional = self.is_defaulted_pattern(module, *pattern);
                    elements.push(element);
                }
                dir::PatternField::Named { pattern: None, .. } | dir::PatternField::Elision => {
                    elements.push(dir::TypeElement::new(unknown))
                }
                _ => {}
            }
        }
        let elements = self.intern_elements(module, &elements)?;
        let tuple = dir::TupleType {
            form: dir::TupleForm::Tuple,
            elements,
        };

        self.intern_type(module, dir::Type::Tuple(tuple))
    }

    /// Return an object shape broad enough for one object pattern.
    fn object_pattern_target_type(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let unknown = self.intern_type(module, dir::Type::Unknown)?;
        let mut fields_target = Vec::new();
        for field in fields {
            let Some((key, pattern)) = self.pattern_field_target_key(module, *field)? else {
                continue;
            };
            if pattern.is_some_and(|pattern| self.is_defaulted_pattern(module, pattern)) {
                continue;
            }
            fields_target.push(dir::TypeField {
                key,
                ty: unknown,
                is_optional: false,
                is_readonly: false,
            });
        }
        let fields_target = self.intern_fields(module, &fields_target)?;
        let shape = dir::ShapeType {
            fields: fields_target,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures: dir::TypeListId::EMPTY,
        };
        let target = self.intern_type(module, dir::Type::Shape(shape))?;

        Ok(Answer::Ready(target))
    }

    /// Return the static key locally named by one pattern field.
    fn pattern_field_target_key(
        &self,
        module: ModuleId,
        field: dir::LocalNodeId<dir::PatternField>,
    ) -> CompilerResult<Option<(dir::StaticKey, Option<dir::LocalNodeId<dir::Pattern>>)>> {
        let key = match self.module(module).view().get(field).clone() {
            dir::PatternField::Named { name, pattern, .. } => Some((name.static_key(), pattern)),
            dir::PatternField::Computed { key, pattern } => self
                .static_key_from_expression(module, key)?
                .map(|key| (key, Some(pattern))),
            dir::PatternField::Rest { .. }
            | dir::PatternField::Elision
            | dir::PatternField::Positional { .. } => None,
        };

        Ok(key)
    }
}
