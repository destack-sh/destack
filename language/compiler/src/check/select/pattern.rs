use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::select::protocol::Protocol;
use crate::check::{
    Answer, CheckState, Condition, Constraint, ConstraintCause, Decision, Dependency, MemberLookup,
    Origin, Relation, answer,
};
use crate::{CheckError, CompilerResult};

impl CheckState<'_> {
    /// Select the pattern meaning of one pattern node.
    ///
    /// Every pattern node carries its own scrutinee component: the match
    /// site flowed the value into the root, and each parent selection
    /// projects components into its nested pattern holes.
    pub(in crate::check) fn select_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let origin = Origin::Node(node.into_any());

        // read each shape's payload once and dispatch with it
        let view = self.module(module).view();
        match view.get(node.local_id) {
            // _
            dir::Pattern::Wildcard => self.record_pattern(node, dir::PatternResolution::Ignore),

            // name, name: pattern
            dir::Pattern::Binding { pattern, .. } => {
                let pattern = *pattern;
                let symbol = self
                    .module(module)
                    .declaration_symbol(node.local_id.into_any());

                self.record_pattern(
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

                self.record_pattern(
                    node,
                    dir::PatternResolution::Must(dir::PatternMustResolution {
                        pattern: pattern.into_global_any(module),
                    }),
                )
            }
            dir::Pattern::Assign { pattern, value } => {
                let (pattern, value) = (*pattern, *value);

                self.record_pattern(
                    node,
                    dir::PatternResolution::Default(dir::PatternDefaultResolution {
                        pattern: pattern.into_global_any(module),
                        value: value.into_global_any(module),
                    }),
                )
            }

            // &pattern, ^pattern, *pattern
            dir::Pattern::BorrowOf { right, .. } => {
                let right = *right;
                let ty = answer!(self.node_type_answer(right.into_global_any(module))?);

                self.record_pattern(
                    node,
                    dir::PatternResolution::Project(dir::PatternProjectionResolution {
                        projection: dir::Projection::Borrow { access: None, ty },
                        pattern: Some(right.into_global_any(module)),
                    }),
                )
            }
            dir::Pattern::MoveOf { right, .. } => {
                let right = *right;
                let ty = answer!(self.node_type_answer(right.into_global_any(module))?);

                self.record_pattern(
                    node,
                    dir::PatternResolution::Project(dir::PatternProjectionResolution {
                        projection: dir::Projection::Move { access: None, ty },
                        pattern: Some(right.into_global_any(module)),
                    }),
                )
            }
            dir::Pattern::DereferenceOf { right } => {
                let right = *right;
                let ty = answer!(self.node_type_answer(right.into_global_any(module))?);

                self.record_pattern(
                    node,
                    dir::PatternResolution::Project(dir::PatternProjectionResolution {
                        projection: dir::Projection::Dereference { ty },
                        pattern: Some(right.into_global_any(module)),
                    }),
                )
            }

            // 1, "ready"
            dir::Pattern::Expression { value } => {
                let value = *value;

                self.select_literal_pattern(node, origin, value)
            }

            // start..end
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => {
                let (start, end, end_kind) = (*start, *end, *end_kind);

                self.select_range_pattern(node, origin, start, end, end_kind)
            }

            // (a, b)
            dir::Pattern::Tuple { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_tuple_pattern(node, origin, &fields)
            }

            // [a, b, ...rest]
            dir::Pattern::Sequence { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_sequence_pattern(node, origin, &fields)
            }

            // { name, nested: pattern }
            dir::Pattern::Object { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_object_pattern(node, origin, &fields)
            }

            // T(value), T { name }
            dir::Pattern::NominalTuple { ty, fields } => {
                let (ty, fields) = (*ty, fields.iter().copied().collect::<SmallVec<[_; 4]>>());

                self.select_newtype_pattern(node, origin, ty, &fields)
            }
            dir::Pattern::NominalObject { ty, fields } => {
                let (ty, fields) = (*ty, fields.iter().copied().collect::<SmallVec<[_; 4]>>());

                self.select_nominal_pattern(node, origin, ty, &fields)
            }

            // a | b
            dir::Pattern::Union { patterns } => {
                let patterns = patterns.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_union_pattern(node, origin, &patterns)
            }
        }
    }

    /// Select one literal pattern from its closed expression value.
    fn select_literal_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let value_node = value.into_global_any(module);
        let ty = answer!(self.node_type_answer(value_node)?);
        let ty = answer!(self.evaluate_root(origin, ty)?);

        // closed literal values select literal patterns
        let literal = match self.ty(ty)? {
            dir::Type::Literal(literal) => Some(*literal),
            dir::Type::Null => Some(dir::ScalarLiteral::Null),
            dir::Type::Undefined => Some(dir::ScalarLiteral::Undefined),
            _ => None,
        };
        match literal {
            Some(value) => {
                let input = answer!(self.pattern_component(node, origin)?);
                let predicate = dir::Predicate::unary(
                    dir::Projection::Identity { ty: input },
                    dir::PredicateCondition::Literal(value),
                )
                .with_success(dir::Projection::Identity { ty });

                self.record_pattern(
                    node,
                    dir::PatternResolution::Test(dir::PatternPredicateResolution { predicate }),
                )
            }
            // non-literal comparisons match through their value types
            None => {
                self.report_expression_pattern_not_literal(module, node.local_id.into_any());

                self.record_pattern(node, dir::PatternResolution::Ignore)
            }
        }
    }

    /// Select one range pattern from its closed bounds.
    fn select_range_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;

        // close both written bounds to literals
        let mut bounds = [None, None];
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for (slot, bound) in [start, end].into_iter().enumerate() {
            let Some(bound) = bound else {
                continue;
            };
            let bound_node = bound.into_global_any(module);
            let ty = answer!(self.node_type_answer(bound_node)?);
            let ty = match self.evaluate_root(origin, ty)? {
                Answer::Ready(ty) => ty,
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            };
            if let dir::Type::Literal(literal) = self.ty(ty)? {
                bounds[slot] = Some(*literal);
            }
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // the matched scalar domain is the pattern's own component
        let domain = answer!(self.node_type_answer(node.into_any())?);
        let predicate = dir::Predicate::unary(
            dir::Projection::Identity { ty: domain },
            dir::PredicateCondition::Range(dir::PredicateRange {
                domain,
                start: bounds[0],
                end: bounds[1],
                end_bound: end_kind,
            }),
        )
        .with_success(dir::Projection::Identity { ty: domain });

        self.record_pattern(
            node,
            dir::PatternResolution::Test(dir::PatternPredicateResolution { predicate }),
        )
    }

    /// Select one tuple pattern, projecting elements by position.
    fn select_tuple_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let scrutinee = answer!(self.pattern_component(node, origin)?);

        // project tuple elements into positional holes
        let elements = match self.ty(scrutinee)? {
            dir::Type::Tuple(tuple) => tuple
                .elements
                .iter()
                .map(|element| element.ty)
                .collect::<SmallVec<[_; 4]>>(),
            _ => SmallVec::new(),
        };

        let mut projected = Vec::with_capacity(fields.len());
        let mut position = 0usize;
        for field in fields {
            let pattern = match self.module(module).view().get(*field) {
                dir::PatternField::Positional { pattern }
                | dir::PatternField::Named {
                    pattern: Some(pattern),
                    ..
                } => Some(pattern.into_global_any(module)),
                dir::PatternField::Named { pattern: None, .. } => {
                    Some(field.into_global_any(module))
                }
                dir::PatternField::Elision => {
                    position += 1;

                    continue;
                }
                _ => continue,
            };

            // flow the positional component into the nested hole
            if let (Some(pattern), Some(component)) = (pattern, elements.get(position).copied()) {
                if pattern.local_id.ty == dir::NodeType::Pattern {
                    let pattern = dir::LocalNodeId::<dir::Pattern>::new(pattern.local_id.id);
                    self.project_pattern_component(origin, module, component, pattern)?;
                } else {
                    let hole = self.require_node_type(pattern)?;
                    self.push_constraint(Constraint {
                        relation: Relation::Equal,
                        left: component,
                        right: hole,
                        origin,
                        condition: Condition::Always,
                        cause: ConstraintCause::Inference,
                    });
                }
            }
            let ty = elements
                .get(position)
                .copied()
                .or_else(|| pattern.and_then(|pattern| self.node_type_maybe(pattern)))
                .unwrap_or(scrutinee);
            projected.push(dir::PatternFieldResolution {
                source: field.into_global_any(module),
                projection: dir::Projection::FieldGet {
                    field: dir::ProjectionField::Key(dir::StaticKey::Index(position)),
                    ty,
                },
                pattern,
            });
            position += 1;
        }

        self.record_pattern(
            node,
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Tuple(
                dir::PatternTupleDestructureResolution { fields: projected },
            )),
        )
    }

    /// Select one sequence pattern.
    fn select_sequence_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let scrutinee = answer!(self.pattern_component(node, origin)?);

        // reject non-sequence sources before projecting fields
        let Some(sequence) = answer!(self.sequence_protocol(node, origin, scrutinee, fields)?)
        else {
            let source = self.format_type(scrutinee);
            let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
            let error = CheckError::PatternSourceNotSequenceShaped {
                anchor,
                module,
                source,
            };
            self.module_mut(module).diagnostics.push(error.into());
            self.record_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        };
        let element = sequence.element_at.as_ref().map(|call| call.return_type);
        let rest_type = sequence.view.as_ref().map(|view| view.return_type);

        // compute the arity requirement introduced by the pattern
        let fixed_positions = fields
            .iter()
            .filter(|field| {
                matches!(
                    self.module(module).view().get(**field),
                    dir::PatternField::Positional { .. } | dir::PatternField::Elision
                )
            })
            .count();
        let has_rest = fields.iter().any(|field| {
            matches!(
                self.module(module).view().get(*field),
                dir::PatternField::Spread { .. }
            )
        });
        let arity = dir::PatternSequenceArity {
            minimum: fixed_positions,
            maximum: (!has_rest).then_some(fixed_positions),
        };

        // project elements and the rest binding
        let mut projected = Vec::with_capacity(fields.len());
        let mut rest = None;
        let mut position = 0usize;
        for field in fields {
            match self.module(module).view().get(*field) {
                dir::PatternField::Positional { pattern } => {
                    let pattern = *pattern;
                    let Some(element) = element else {
                        continue;
                    };
                    self.project_pattern_component(origin, module, element, pattern)?;
                    projected.push(dir::PatternSequenceElementResolution {
                        source: field.into_global_any(module),
                        index: position,
                        ty: element,
                        pattern: pattern.into_global_any(module),
                    });
                    position += 1;
                }
                dir::PatternField::Spread { pattern } => {
                    let pattern = *pattern;
                    let Some(rest_type) = rest_type else {
                        continue;
                    };
                    if let Some(pattern) = pattern {
                        self.project_pattern_component(origin, module, rest_type, pattern)?;
                    }
                    rest = Some(dir::PatternSequenceRestResolution {
                        source: field.into_global_any(module),
                        start: position,
                        end: None,
                        ty: rest_type,
                        pattern: pattern.map(|pattern| pattern.into_global_any(module)),
                    });
                }
                dir::PatternField::Elision => {
                    position += 1;
                }
                _ => {}
            }
        }

        // record the sequence form behind the scrutinee
        self.record_pattern(
            node,
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Sequence(
                dir::PatternSequenceDestructureResolution {
                    sequence,
                    arity,
                    fields: projected,
                    rest,
                },
            )),
        )
    }

    /// Select the sequence protocol operations needed by one pattern.
    fn sequence_protocol(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<Option<dir::SequenceProtocol>>> {
        let module = node.module_id;
        let sequence = self.language_protocol(dir::LanguageItem::Sequence, Vec::new());

        // select the protocol operations used by destructuring
        let Some(length) = answer!(self.sequence_length_resolution(origin, receiver, &sequence)?)
        else {
            return Ok(Answer::Ready(None));
        };
        let has_elements = fields.iter().any(|field| {
            matches!(
                self.module(module).view().get(*field),
                dir::PatternField::Positional { .. }
            )
        });
        let element_at = if has_elements {
            let Some(element_at) =
                answer!(self.sequence_element_resolution(node, origin, receiver, &sequence)?)
            else {
                return Ok(Answer::Ready(None));
            };

            Some(element_at)
        } else {
            None
        };

        let has_rest = fields.iter().any(|field| {
            matches!(
                self.module(module).view().get(*field),
                dir::PatternField::Spread { .. }
            )
        });
        let view = if has_rest {
            let Some(view) =
                answer!(self.sequence_view_resolution(node, origin, receiver, &sequence, fields)?)
            else {
                return Ok(Answer::Ready(None));
            };

            Some(view)
        } else {
            None
        };

        Ok(Answer::Ready(Some(dir::SequenceProtocol {
            length,
            element_at,
            view,
        })))
    }

    /// Return the selected `length` member of a sequence receiver.
    fn sequence_length_resolution(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        sequence: &Protocol,
    ) -> CompilerResult<Answer<Option<dir::MemberResolution>>> {
        let module = origin.module();
        let key = self.static_name(module, "length");
        let Some(member) = answer!(self.select_protocol_member(origin, receiver, key, sequence)?)
        else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some(member.resolution)))
    }

    /// Return the selected `Index.index` call of a sequence receiver.
    fn sequence_element_resolution(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        sequence: &Protocol,
    ) -> CompilerResult<Answer<Option<dir::CallResolution>>> {
        let module = node.module_id;
        let index = self.static_usize_type(node, 0)?;
        let key = self.static_name(module, "index");
        let arguments = [index];
        let sources = [dir::ArgumentSource::Static(index)];
        let Some(call) = answer!(
            self.select_protocol_call(origin, receiver, key, sequence, &arguments, &sources)?
        ) else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some(call.resolution)))
    }

    /// Return the selected `Sequence.view` call of a sequence receiver.
    fn sequence_view_resolution(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        sequence: &Protocol,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<Option<dir::CallResolution>>> {
        let module = node.module_id;
        let Some(start) = self.sequence_rest_start(module, fields) else {
            return Ok(Answer::Ready(None));
        };
        let start_type = self.static_usize_type(node, start)?;
        let key = self.static_name(module, "view");
        let arguments = [start_type];
        let sources = [dir::ArgumentSource::Static(start_type)];
        let Some(call) = answer!(
            self.select_protocol_call(origin, receiver, key, sequence, &arguments, &sources)?
        ) else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some(call.resolution)))
    }

    /// Return the rest start position of one sequence pattern.
    fn sequence_rest_start(
        &self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> Option<usize> {
        let mut position = 0usize;
        for field in fields {
            match self.module(module).view().get(*field) {
                dir::PatternField::Positional { .. } | dir::PatternField::Elision => {
                    position += 1;
                }
                dir::PatternField::Spread { .. } => return Some(position),
                _ => {}
            }
        }

        None
    }

    /// Return one generated usize literal type.
    fn static_usize_type(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        value: usize,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.push_type(
            node.module_id,
            dir::Type::Literal(dir::ScalarLiteral::Integer(value as i64)),
            node.local_id.into_any(),
        )
    }

    /// Return one interned static name key.
    fn static_name(&mut self, module: ModuleId, name: &str) -> dir::StaticKey {
        dir::StaticKey::Name(self.module_mut(module).strings.intern(name))
    }

    /// Select one object pattern, projecting fields by key.
    fn select_object_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let scrutinee = answer!(self.pattern_component(node, origin)?);
        let projected = answer!(self.project_named_fields(node, origin, scrutinee, fields)?);

        self.record_pattern(
            node,
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Object(
                dir::PatternObjectDestructureResolution { fields: projected },
            )),
        )
    }

    /// Select one newtype pattern, unwrapping the substituted backing.
    fn select_newtype_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;

        // close the written nominal tag
        let tag_node = ty.into_global_any(module);
        let tag = answer!(self.node_type_answer(tag_node)?);
        let tag = answer!(self.evaluate_root(origin, tag)?);
        let instance = match self.ty(tag)? {
            dir::Type::Instance(instance) => instance.clone(),
            _ => return self.reject_pattern(node, origin, tag),
        };

        // unwrap the substituted newtype backing
        let backing = match self.definition(instance.symbol) {
            Some(dir::Definition::Newtype(definition)) => definition.value,
            _ => return self.reject_pattern(node, origin, tag),
        };
        let substitution = self.parameter_substitution(&instance)?.with_receiver(tag);
        let source = self.origin_source_node(origin)?;
        let backing = if substitution.is_empty() {
            backing
        } else {
            self.fold_type(module, source, backing, substitution.rewrite())?
        };

        // flow the backing into the wrapped hole
        let value = fields
            .first()
            .and_then(|field| match self.module(module).view().get(*field) {
                dir::PatternField::Positional { pattern } => Some(*pattern),
                _ => None,
            });
        if let Some(value) = value {
            self.project_pattern_component(origin, module, backing, value)?;
        }

        self.record_pattern(
            node,
            dir::PatternResolution::Project(dir::PatternProjectionResolution {
                projection: dir::Projection::NewtypePayload {
                    symbol: instance.symbol,
                    generic_arguments: self
                        .symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?,
                    ty: backing,
                },
                pattern: value.map(|value| value.into_global_any(module)),
            }),
        )
    }

    /// Select one nominal object pattern, projecting declared fields.
    fn select_nominal_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;

        // close the written nominal tag
        let tag_node = ty.into_global_any(module);
        let tag = answer!(self.node_type_answer(tag_node)?);
        let tag = answer!(self.evaluate_root(origin, tag)?);
        let instance = match self.ty(tag)? {
            dir::Type::Instance(instance) => instance.clone(),
            _ => return self.reject_pattern(node, origin, tag),
        };

        // project the declared fields off the matched declaration
        let fields = answer!(self.project_named_fields(node, origin, tag, fields)?);
        self.record_pattern(
            node,
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Nominal(
                dir::PatternNominalDestructureResolution {
                    symbol: instance.symbol,
                    generic_arguments: self
                        .symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?,
                    fields,
                },
            )),
        )
    }

    /// Select one or-pattern, sharing the component across branches.
    fn select_union_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        patterns: &[dir::LocalNodeId<dir::Pattern>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let component = answer!(self.node_type_answer(node.into_any())?);

        // match every branch against the same component
        for pattern in patterns {
            self.project_pattern_component(origin, module, component, *pattern)?;
        }

        self.record_pattern(
            node,
            dir::PatternResolution::Or(dir::PatternOrResolution {
                patterns: patterns
                    .iter()
                    .map(|pattern| pattern.into_global_any(module))
                    .collect(),
            }),
        )
    }

    /// Project named pattern fields off one closed owner component.
    fn project_named_fields(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        owner: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<Vec<dir::PatternFieldResolution>>> {
        let module = node.module_id;

        let mut projected = Vec::with_capacity(fields.len());
        for field in fields {
            let (name, pattern) = match self.module(module).view().get(*field) {
                dir::PatternField::Named { name, pattern, .. } => (*name, *pattern),
                // spreads and computed keys keep the whole component
                _ => continue,
            };
            let key = name.static_key();

            // look the component up on the owner type
            let lookup =
                self.lookup_member(origin, module, owner, dir::MemberSpace::Instance, key)?;
            let projection = match lookup {
                MemberLookup::Field(ty) => Some((ty, dir::ProjectionField::Key(key))),
                MemberLookup::Found(candidates) => candidates.first().map(|candidate| {
                    let field = candidate
                        .symbol
                        .map(dir::ProjectionField::Member)
                        .unwrap_or(dir::ProjectionField::Key(key));

                    (candidate.ty, field)
                }),
                MemberLookup::Missing => None,
                MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            let Some((component, projection_field)) = projection else {
                let key = self.format_static_key(&key);
                self.report_missing_member(origin, owner, key)?;

                continue;
            };

            // flow the component into the nested or shorthand hole
            match pattern {
                Some(pattern) => {
                    self.project_pattern_component(origin, module, component, pattern)?;
                }
                // shorthand fields bind through their own field node
                None => {
                    let field_node = field.into_global_any(module);
                    if let Some(hole) = self.node_type_maybe(field_node) {
                        self.push_constraint(Constraint {
                            relation: Relation::Equal,
                            left: component,
                            right: hole,
                            origin,
                            condition: Condition::Always,
                            cause: ConstraintCause::Inference,
                        });
                    }
                }
            }
            projected.push(dir::PatternFieldResolution {
                source: field.into_global_any(module),
                projection: dir::Projection::FieldGet {
                    field: projection_field,
                    ty: component,
                },
                pattern: pattern.map(|pattern| pattern.into_global_any(module)),
            });
        }

        Ok(Answer::Ready(projected))
    }

    /// Return one pattern node's closed scrutinee component.
    fn pattern_component(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let ty = answer!(self.node_type_answer(node.into_any())?);

        self.evaluate_root(origin, ty)
    }

    /// Flow one projected component into a nested pattern hole.
    fn project_pattern_component(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        component: dir::GlobalTypeId,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<()> {
        let pattern_node = pattern.into_global_any(module);
        let hole = self.require_node_type(pattern_node)?;
        let relation = match self.module(module).view().get(pattern) {
            dir::Pattern::Binding { pattern: None, .. } => Relation::Equal,
            _ => Relation::Assignable,
        };

        self.push_constraint(Constraint {
            relation,
            left: component,
            right: hole,
            origin,
            condition: Condition::Always,
            cause: ConstraintCause::Inference,
        });

        Ok(())
    }

    /// Record one pattern decision.
    fn record_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        resolution: dir::PatternResolution,
    ) -> CompilerResult<Answer<()>> {
        self.record_decision(node.into_any(), Decision::Pattern(resolution))?;

        Ok(Answer::Ready(()))
    }

    /// Reject one pattern whose tag is not nominal.
    fn reject_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        tag: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let ty = self.format_type(tag);
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::InvalidPatternTag { anchor, module, ty };
        self.module_mut(module).diagnostics.push(error.into());
        self.record_decision(node.into_any(), Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }
}
