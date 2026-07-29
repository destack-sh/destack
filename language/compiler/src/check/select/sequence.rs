use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{Answer, BodyState, Decision, FlowPointId, Origin, Protocol, Value, answer};

impl BodyState<'_, '_> {
    /// Select one sequence pattern.
    pub(in crate::check) fn select_sequence_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        scrutinee: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        self.check_pattern_bindings(module, fields)?;

        if !self.check_pattern_rest_fields(module, fields) {
            return self.commit_rejected_pattern(node);
        }

        // reject non-sequence sources before projecting fields
        let Some((sequence, _length)) = answer!(self.select_sequence_length(origin, scrutinee)?)
        else {
            self.report_pattern_source_not_sequence_shaped(origin, scrutinee)?;

            return self.commit_rejected_pattern(node);
        };

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
                dir::PatternField::Rest { .. }
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
                    let Some(projection) = answer!(self.select_sequence_element(
                        field.into_global_any(module),
                        origin,
                        scrutinee,
                        position,
                    )?) else {
                        continue;
                    };
                    let element = projection.ty();

                    answer!(self.check_pattern_projection(
                        flow,
                        scope,
                        element,
                        pattern.into_global_any(module),
                    )?);
                    projected.push(dir::PatternFieldResolution {
                        source: field.into_global_any(module),
                        projection,
                        pattern: Some(pattern.into_global_any(module)),
                    });
                    position += 1;
                }
                dir::PatternField::Rest { pattern } => {
                    let pattern = *pattern;
                    let Some(call) = answer!(self.select_sequence_rest(
                        field.into_global_any(module),
                        origin,
                        scrutinee,
                        &sequence,
                        position,
                    )?) else {
                        continue;
                    };
                    let rest_type = call.return_type();

                    if let Some(pattern) = pattern {
                        answer!(self.check_pattern_projection(
                            flow,
                            scope,
                            rest_type,
                            pattern.into_global_any(module),
                        )?);
                    }
                    rest = Some(Box::new(dir::PatternFieldResolution {
                        source: field.into_global_any(module),
                        projection: call.into(),
                        pattern: pattern.map(|pattern| pattern.into_global_any(module)),
                    }));
                }
                dir::PatternField::Elision => {
                    position += 1;
                }
                _ => {}
            }
        }

        // record the sequence form behind the scrutinee
        self.commit_pattern(
            node,
            dir::PatternResolution::Destructure(Box::new(
                dir::PatternDestructureResolution::Sequence(
                    dir::PatternSequenceDestructureResolution {
                        arity,
                        fields: projected,
                        rest,
                    },
                ),
            )),
        )
    }

    /// Select one sequence assignment pattern.
    pub(in crate::check) fn select_assign_sequence_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        scrutinee: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> CompilerResult<Answer<bool>> {
        let module = node.module_id;
        if !self.check_assign_pattern_rest_fields(module, fields) {
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(false));
        }

        let rest_start = self.assign_sequence_rest_start(module, fields);

        // reject non-sequence sources before projecting fields
        let Some((sequence, _length)) = answer!(self.select_sequence_length(origin, scrutinee)?)
        else {
            self.report_pattern_source_not_sequence_shaped(origin, scrutinee)?;
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(false));
        };

        // compute the arity requirement introduced by the target
        let fixed_positions = fields
            .iter()
            .filter(|field| {
                matches!(
                    self.module(module).view().get(**field),
                    dir::AssignPatternField::Positional { .. } | dir::AssignPatternField::Elision
                )
            })
            .count();
        let arity = dir::PatternSequenceArity {
            minimum: fixed_positions,
            maximum: rest_start.is_none().then_some(fixed_positions),
        };

        // project elements and the rest target
        let mut projected = Vec::with_capacity(fields.len());
        let mut rest = None;
        let mut position = 0usize;
        for field in fields {
            match self.module(module).view().get(*field) {
                dir::AssignPatternField::Positional { pattern } => {
                    let pattern = *pattern;
                    let Some(projection) = answer!(self.select_sequence_element(
                        field.into_global_any(module),
                        origin,
                        scrutinee,
                        position,
                    )?) else {
                        continue;
                    };
                    let element = projection.ty();

                    answer!(self.check_pattern_projection(
                        flow,
                        scope,
                        element,
                        pattern.into_global_any(module),
                    )?);
                    projected.push(dir::AssignPatternFieldResolution {
                        source: field.into_global_any(module),
                        projection,
                        pattern: Some(pattern.into_global_any(module)),
                    });
                    position += 1;
                }
                dir::AssignPatternField::Rest { pattern } => {
                    let pattern = *pattern;
                    let Some(call) = answer!(self.select_sequence_rest(
                        field.into_global_any(module),
                        origin,
                        scrutinee,
                        &sequence,
                        position,
                    )?) else {
                        continue;
                    };
                    let rest_type = call.return_type();

                    if let Some(pattern) = pattern {
                        answer!(self.check_pattern_projection(
                            flow,
                            scope,
                            rest_type,
                            pattern.into_global_any(module),
                        )?);
                    }
                    rest = Some(Box::new(dir::AssignPatternFieldResolution {
                        source: field.into_global_any(module),
                        projection: call.into(),
                        pattern: pattern.map(|pattern| pattern.into_global_any(module)),
                    }));
                }
                dir::AssignPatternField::Elision => {
                    position += 1;
                }
                _ => {}
            }
        }

        let () = answer!(self.commit_assign_pattern(
            node,
            dir::AssignPatternResolution::Sequence(dir::AssignPatternSequenceResolution {
                arity,
                fields: projected,
                rest,
            }),
        )?);

        Ok(Answer::Ready(true))
    }

    /// Return the `length` member for a sequence receiver.
    fn select_sequence_length(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<(Protocol, dir::MemberResolution)>>> {
        let key = self.static_name("length");
        let Some((sequence, member)) = answer!(self.select_language_protocol_member(
            origin,
            receiver,
            receiver,
            dir::MemberSpace::Instance,
            key,
            dir::LanguageItem::Sequence,
            &[],
            &[],
        )?) else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some((sequence, member.resolution))))
    }

    /// Return the `Index.index` call for a sequence receiver.
    fn select_sequence_element(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        index: usize,
    ) -> CompilerResult<Answer<Option<dir::ProjectionResolution>>> {
        let index = self.static_usize_type(node, index)?;
        let Some(selection) = answer!(self.select_subscript_read_source(
            origin,
            Value {
                ty: receiver,
                place: None,
            },
            receiver,
            dir::ArgumentSource::Static(index),
            index,
        )?) else {
            return Ok(Answer::Ready(None));
        };
        let Some(projection) = selection.into_read_projection() else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some(projection)))
    }

    /// Return the `Sequence.rest` call for a sequence receiver.
    fn select_sequence_rest(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        sequence: &Protocol,
        start: usize,
    ) -> CompilerResult<Answer<Option<dir::CallResolution>>> {
        let start_type = self.static_usize_type(node, start)?;
        let key = self.static_name("rest");
        let sources = [dir::ArgumentSource::Static(start_type)];
        let Some(call) = answer!(self.select_protocol_call(
            origin,
            Value {
                ty: receiver,
                place: None,
            },
            receiver,
            dir::MemberSpace::Instance,
            key,
            sequence,
            &sources,
        )?) else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some(call.resolution)))
    }

    /// Return the rest start position of one sequence assignment pattern.
    fn assign_sequence_rest_start(
        &self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> Option<usize> {
        let mut position = 0usize;
        for field in fields {
            match self.module(module).view().get(*field) {
                dir::AssignPatternField::Positional { .. } | dir::AssignPatternField::Elision => {
                    position += 1;
                }
                dir::AssignPatternField::Rest { .. } => return Some(position),
                _ => {}
            }
        }

        None
    }

    /// Return one generated usize literal type.
    fn static_usize_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        value: usize,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_type(
            node.module_id,
            dir::Type::Literal(dir::ScalarLiteral::Integer(value as i64)),
        )
    }

    /// Return one interned static name key.
    fn static_name(&self, name: &str) -> dir::StaticKey {
        dir::StaticKey::Name(self.strings().intern(name))
    }
}
