use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    AssignmentSelection, Cause, CauseKind, CheckState, Expectation, FlowPointId, FlowSite,
    InferMode, Obligation, Origin, PlaceUse, Relation, StoreTarget, ValueUse,
    WritableTargetObligation,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check one pattern against its input type.
    pub(in crate::sema) fn check_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.commit_node_type(node.into_any(), input)?;

        self.select_pattern(node, flow, scope, input)
    }

    /// Check one assignment pattern against its input type.
    pub(in crate::sema) fn check_assign_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        origin: Origin,
    ) -> CompilerResult<()> {
        self.commit_node_type(node.into_any(), input)?;
        self.select_assign_pattern(node, flow, scope, input, origin)?;

        Ok(())
    }

    /// Select the assignment meaning of one assignment pattern node.
    pub(in crate::sema) fn select_assign_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        input_origin: Origin,
    ) -> CompilerResult<bool> {
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
            self.normalize(pattern_origin, input)?
        } else {
            input
        };

        // select by the assignment pattern's own syntax
        match pattern {
            // x = value, obj.x = value
            dir::AssignPattern::Place { expression: value } => {
                let Some(place) = self.select_assignment(
                    FlowSite {
                        node: value.into_global_any(module),
                        flow,
                        scope,
                    },
                    value,
                    PlaceUse::Write,
                )?
                else {
                    self.commit_decision(node.into_any(), dir::Decision::Rejected)?;

                    return Ok(false);
                };
                let target = self.commit_assign_pattern_place(input_origin, node, place)?;
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
                self.check_value(pattern_site, input, expectation)?;

                Ok(true)
            }
            // x = default
            dir::AssignPattern::Default { pattern, value } => {
                let value_node = value.into_global_any(module);
                let default = self.infer_node_type(
                    FlowSite {
                        node: value_node,
                        flow,
                        scope,
                    },
                    PlaceUse::Read,
                )?;
                let origin = Origin::Node(value_node, scope);
                let input = self.defaulted_pattern_type(origin, input, default)?;

                // flow the defaulted input into the nested target
                self.check_pattern_projection(flow, scope, input, pattern.into_global_any(module))?;

                let () = self.commit_assign_pattern(
                    node,
                    dir::AssignPatternDecision::Default(dir::AssignPatternDefaultResolution {
                        pattern: pattern.into_global_any(module),
                        value: value.into_global_any(module),
                    }),
                )?;

                Ok(true)
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
    pub(in crate::sema) fn commit_assign_pattern_place(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        place: AssignmentSelection,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // commit the selected place occurrence
        let target_type = place.write.ty();
        let resolution = place.clone().resolution();
        let source_type = resolution
            .read
            .as_ref()
            .map_or_else(|| resolution.write.ty(), dir::ReadResolution::ty);
        let source = resolution.target;
        self.commit_decision(source, dir::Decision::Assignment(Box::new(resolution)))?;
        self.commit_access_use(source, dir::BindingUse::WRITE);
        self.commit_node_type(source, source_type)?;

        // commit the pattern's place
        let site = self.visit_site(source)?;
        self.commit_expression_place(site, source_type)?;

        // require the written place to be writable
        let scope = self.origin_scope(origin)?;
        self.push_obligation(
            Obligation::WritableTarget(Box::new(WritableTargetObligation {
                target: place,
                ty: target_type,
            })),
            scope,
        )?;

        // commit the assignment pattern resolution
        let () = self.commit_assign_pattern(node, dir::AssignPatternDecision::Place)?;

        Ok(target_type)
    }

    /// Return whether one pattern destructures its input.
    pub(in crate::sema) fn is_destructuring_pattern(pattern: &dir::Pattern) -> bool {
        !matches!(
            pattern,
            dir::Pattern::Wildcard | dir::Pattern::Binding { .. }
        )
    }

    /// Return whether one assignment pattern destructures its input.
    pub(in crate::sema) fn is_destructuring_assign_pattern(pattern: &dir::AssignPattern) -> bool {
        !matches!(pattern, dir::AssignPattern::Place { .. })
    }

    /// Select the pattern meaning of one pattern node.
    pub(in crate::sema) fn select_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let module = node.module_id;
        let origin = Origin::Node(node.into_any(), scope);

        // reduce inputs only for patterns that inspect their value
        let pattern = self.module(module).view().get(node.local_id).clone();
        let needs_reduced_input = Self::is_destructuring_pattern(&pattern);
        let input = if needs_reduced_input {
            self.normalize(origin, input)?
        } else {
            input
        };
        let input = self.filter_destructuring_source(node, origin, input)?;

        // select by the pattern's own syntax
        match &pattern {
            // _
            dir::Pattern::Wildcard => self.commit_pattern(node, dir::PatternDecision::Ignore),

            // name, name: pattern
            dir::Pattern::Binding { pattern, .. } => {
                self.select_binding_pattern(node, flow, scope, input, *pattern)
            }

            // pattern!
            dir::Pattern::Must(pattern) => {
                let pattern = *pattern;

                // flow the present input into the nested pattern
                let present = self.required_pattern_type(origin, input)?;
                self.check_pattern_projection(
                    flow,
                    scope,
                    present,
                    pattern.into_global_any(module),
                )?;

                self.commit_pattern(
                    node,
                    dir::PatternDecision::Must(dir::PatternMustResolution {
                        pattern: pattern.into_global_any(module),
                        ty: present,
                    }),
                )
            }
            // pattern = value
            dir::Pattern::Default { pattern, value } => {
                let (pattern, value) = (*pattern, *value);

                // read the annotated input while declaring, infer the default while checking
                let input = if self.is_declaring() {
                    input
                } else {
                    let value_node = value.into_global_any(module);
                    let default = self.infer_node_type(
                        FlowSite {
                            node: value_node,
                            flow,
                            scope,
                        },
                        PlaceUse::Read,
                    )?;

                    let origin = Origin::Node(value_node, scope);

                    self.defaulted_pattern_type(origin, input, default)?
                };

                // flow the defaulted input into the nested pattern
                self.check_pattern_projection(flow, scope, input, pattern.into_global_any(module))?;

                self.commit_pattern(
                    node,
                    dir::PatternDecision::Default(dir::PatternDefaultResolution {
                        pattern: pattern.into_global_any(module),
                        value: value.into_global_any(module),
                    }),
                )
            }

            // &pattern
            dir::Pattern::BorrowOf { access, right } => {
                self.select_borrow_pattern(node, flow, scope, input, *right, *access)
            }
            // ^pattern
            dir::Pattern::MoveOf { mutability, right } => {
                self.select_move_pattern(node, flow, scope, input, *right, *mutability)
            }
            // *pattern
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

                self.select_range_pattern(node, flow, scope, input, start, end, end_kind)
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

            // T(value)
            dir::Pattern::NominalTuple { ty, fields } => {
                let (ty, fields) = (*ty, fields.iter().copied().collect::<SmallVec<[_; 4]>>());

                self.select_newtype_pattern(node, origin, flow, scope, ty, &fields)
            }
            // T { name }
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
    ) -> CompilerResult<()> {
        let module = node.module_id;
        let origin = Origin::Node(node.into_any(), scope);
        let symbol = self
            .module(module)
            .declaration_symbol(node.local_id.into_any())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("binding pattern {node:?} has no symbol"),
            })?;

        // store the captured input in the binding's handle
        let binding = self.symbol_type_maybe(symbol)?;
        let input = if binding == Some(input) {
            input
        } else {
            let cause = self.intern_cause(Cause::root(
                origin,
                CauseKind::Pattern {
                    pattern: node.into_any(),
                },
            ));
            if let Some(binding) = binding {
                self.relate(origin, cause, Relation::Equal, input, binding)?;
            } else {
                self.commit_binding_type(symbol, input)?;
            }

            input
        };

        // project the captured input into the nested pattern
        if let Some(pattern) = pattern {
            self.check_pattern_projection(flow, scope, input, pattern.into_global_any(module))?;
        }

        // commit the binding
        self.commit_pattern(
            node,
            dir::PatternDecision::Bind(dir::PatternBindingResolution {
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
    ) -> CompilerResult<()> {
        let module = node.module_id;

        // match every branch against the same input
        for pattern in patterns {
            self.check_pattern_projection(flow, scope, input, (*pattern).into_global_any(module))?;
        }

        // commit the or pattern
        self.commit_pattern(
            node,
            dir::PatternDecision::Or(dir::PatternOrResolution {
                patterns: patterns
                    .iter()
                    .map(|pattern| pattern.into_global_any(module))
                    .collect(),
            }),
        )
    }

    /// Commit one pattern decision.
    pub(in crate::sema) fn commit_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        resolution: dir::PatternDecision,
    ) -> CompilerResult<()> {
        self.commit_decision(node.into_any(), dir::Decision::Pattern(resolution))?;

        Ok(())
    }

    /// Commit one assignment pattern decision.
    pub(in crate::sema) fn commit_assign_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        resolution: dir::AssignPatternDecision,
    ) -> CompilerResult<()> {
        self.commit_decision(node.into_any(), dir::Decision::AssignPattern(resolution))?;

        Ok(())
    }

    /// Report one pattern whose tag names no nominal, committing the rejection.
    pub(in crate::sema) fn report_rejected_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        tag: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // report an invalid tag only when its type has not already failed
        if !self.has_error_operand(&[tag])? {
            self.report_invalid_pattern_tag(origin, tag)?;
        }

        self.commit_rejected_pattern(node)
    }

    /// Commit one rejected pattern, poisoning the bindings it would introduce.
    pub(in crate::sema) fn commit_rejected_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<()> {
        self.poison_pattern(node)?;
        self.commit_decision(node.into_any(), dir::Decision::Rejected)?;

        Ok(())
    }

    /// Bind every binding beneath one rejected pattern to the error type.
    fn poison_pattern(&mut self, node: dir::GlobalNodeId<dir::Pattern>) -> CompilerResult<()> {
        let module = node.module_id;
        let error = self.intern_type(dir::Type::Error)?;

        self.poison_pattern_bindings(module, error, node.local_id)
    }

    /// Bind every binding beneath one rejected pattern field to the error type.
    pub(in crate::sema) fn poison_pattern_field(
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
            self.commit_symbol_type(symbol, error)?;
        }

        // recurse into the nested patterns each shape holds
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
            self.commit_symbol_type(symbol, error)?;
        }

        // recurse into the nested pattern each field shape holds
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
            // bare names, bare rests, and elisions bind nothing nested
            dir::PatternField::Named { pattern: None, .. }
            | dir::PatternField::Rest { pattern: None }
            | dir::PatternField::Elision => {}
        }

        Ok(())
    }

    /// Return the borrow form a destructured input reads its fields through.
    pub(in crate::sema) fn pattern_binding_form(
        &mut self,
        origin: Origin,
        input: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Form>> {
        let chain = self.form_chain(origin, input)?;
        let borrowed = chain
            .forms()
            .iter()
            .find(|entry| matches!(entry.form, dir::Form::Borrowed(_)))
            .map(|entry| entry.form);

        Ok(borrowed)
    }

    /// Return one projected field type bound through the binding form of its input.
    pub(in crate::sema) fn bound_through(
        &mut self,
        origin: Origin,
        form: Option<dir::Form>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(form) = form else {
            return Ok(ty);
        };
        if self.decide_copy(origin, ty, &mut SmallVec::new())?.holds() {
            return Ok(ty);
        }

        self.intern_type(dir::Type::Form(dir::FormType { form, value: ty }))
    }

    /// Return whether one pattern has a default branch.
    pub(in crate::sema) fn is_defaulted_pattern(
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
    pub(in crate::sema) fn is_defaulted_assign_pattern(
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
    ) -> CompilerResult<dir::GlobalTypeId> {
        // select alternatives for destructuring patterns over a union only
        let Some(requirement) = self.destructuring_requirement(node)? else {
            return Ok(source);
        };
        if !matches!(self.ty(source)?, dir::Type::Union(_)) {
            return Ok(source);
        }

        // keep the alternatives that satisfy the pattern's shape
        let operation = dir::TypeOperation::Narrow(dir::NarrowType {
            source,
            target: requirement,
            is_positive: true,
        });
        let narrowed = self.reduce_operation_type(origin, operation)?;

        // preserve the rejected source for the pattern diagnostic
        if matches!(self.ty(narrowed)?, dir::Type::Never) {
            Ok(source)
        } else {
            Ok(narrowed)
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
        let elements = self.intern_elements(&elements)?;
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

        // constrain the matched value through the required keys
        self.intern_object(&required)
    }

    /// Return the static key named by one object pattern field.
    fn object_field_requirement(
        &mut self,
        module: ModuleId,
        field: dir::LocalNodeId<dir::PatternField>,
    ) -> CompilerResult<Option<(dir::StaticKey, Option<dir::LocalNodeId<dir::Pattern>>)>> {
        // read the key and nested pattern each field form names
        let field = self.module(module).view().get(field).clone();
        let key = match field {
            dir::PatternField::Named { name, pattern, .. } => Some((name.into(), pattern)),
            dir::PatternField::Computed { key, pattern } => self
                .evaluate_static_key(module, key)?
                .map(|key| (key, Some(pattern))),
            dir::PatternField::Rest { .. } => None,
            invalid @ (dir::PatternField::Positional { .. } | dir::PatternField::Elision) => {
                return Err(CompilerError::Internal {
                    message: format!("invalid object pattern field: {invalid:?}"),
                });
            }
        };

        Ok(key)
    }

    /// Flow one projected value into a nested pattern hole.
    pub(in crate::sema) fn check_pattern_projection(
        &mut self,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        pattern: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let site = FlowSite {
            node: pattern,
            flow,
            scope,
        };
        let expectation = Expectation {
            target: input,
            relation: Relation::Storable,
            cause: self.intern_cause(Cause::root(
                Origin::Node(pattern, scope),
                CauseKind::Pattern { pattern },
            )),
            use_: ValueUse::Store,
            mode: InferMode::Regular,
            store: StoreTarget::Exact,
        };
        self.attempt_node(site, PlaceUse::Read, Some(expectation))?;

        Ok(())
    }

    /// Return the type produced by one defaulted pattern.
    pub(in crate::sema) fn defaulted_pattern_type(
        &mut self,
        origin: Origin,
        input: dir::GlobalTypeId,
        default: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
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
            let default = match kept.is_empty() {
                true => self.widen_fresh(origin, default)?,
                false => default,
            };
            kept.push(default);
        }

        let ty = self.normalized_union_type(kept)?;

        Ok(ty)
    }

    /// Return the non-nullish type one required pattern accepts.
    pub(in crate::sema) fn required_pattern_type(
        &mut self,
        origin: Origin,
        input: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.without_union_members(origin, input, |member| {
            matches!(member, dir::Type::Null | dir::Type::Undefined)
        })
    }
}
