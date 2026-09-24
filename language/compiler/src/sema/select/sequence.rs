use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{CheckState, FlowPointId, Origin, Protocol, SignatureRejection, Value};

impl CheckState<'_> {
    /// Select one sequence pattern.
    pub(in crate::sema) fn select_sequence_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        scrutinee: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        let module = node.module_id;
        self.report_duplicate_pattern_bindings(module, fields)?;

        // require a well formed rest field
        if !self.report_pattern_rest_fields(module, fields) {
            return self.commit_rejected_pattern(node);
        }

        // tuple sources destructure positionally like written tuple patterns
        let subject = self.shallow_resolve(scrutinee)?;
        if matches!(self.ty(subject)?, dir::Type::Tuple(_)) {
            return self.select_tuple_pattern(node, origin, flow, scope, subject, fields);
        }

        // reject non-sequence sources before projecting fields
        let Some((sequence, _length)) = self.select_sequence_length(origin, scrutinee)? else {
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
                    let Some(projection) = self.select_sequence_element(
                        field.into_global_any(module),
                        origin,
                        scrutinee,
                        position,
                    )?
                    else {
                        continue;
                    };
                    let element = projection.ty();

                    self.check_pattern_projection(
                        flow,
                        scope,
                        element,
                        pattern.into_global_any(module),
                    )?;
                    projected.push(dir::PatternFieldResolution {
                        source: field.into_global_any(module),
                        projection,
                        pattern: Some(pattern.into_global_any(module)),
                    });
                    position += 1;
                }
                dir::PatternField::Rest { pattern } => {
                    let pattern = *pattern;
                    let Some(call) = self.select_sequence_rest(
                        field.into_global_any(module),
                        origin,
                        scrutinee,
                        &sequence,
                        position,
                    )?
                    else {
                        continue;
                    };
                    let rest_type = call.return_type();

                    if let Some(pattern) = pattern {
                        self.check_pattern_projection(
                            flow,
                            scope,
                            rest_type,
                            pattern.into_global_any(module),
                        )?;
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

        // commit the sequence form behind the scrutinee
        self.commit_pattern(
            node,
            dir::PatternDecision::Destructure(Box::new(
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
    pub(in crate::sema) fn select_assign_sequence_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        scrutinee: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> CompilerResult<bool> {
        let module = node.module_id;
        if !self.report_assign_rest_fields(module, fields) {
            self.commit_decision(node.into_any(), dir::Decision::Rejected)?;

            return Ok(false);
        }

        // read the position the rest field starts at
        let rest_start = self.assign_sequence_rest_start(module, fields);

        // reject non-sequence sources before projecting fields
        let Some((sequence, _length)) = self.select_sequence_length(origin, scrutinee)? else {
            self.report_pattern_source_not_sequence_shaped(origin, scrutinee)?;
            self.commit_decision(node.into_any(), dir::Decision::Rejected)?;

            return Ok(false);
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
                    let Some(projection) = self.select_sequence_element(
                        field.into_global_any(module),
                        origin,
                        scrutinee,
                        position,
                    )?
                    else {
                        continue;
                    };
                    let element = projection.ty();

                    self.check_pattern_projection(
                        flow,
                        scope,
                        element,
                        pattern.into_global_any(module),
                    )?;
                    projected.push(dir::AssignPatternFieldResolution {
                        source: field.into_global_any(module),
                        projection,
                        pattern: Some(pattern.into_global_any(module)),
                    });
                    position += 1;
                }
                dir::AssignPatternField::Rest { pattern } => {
                    let pattern = *pattern;
                    let Some(call) = self.select_sequence_rest(
                        field.into_global_any(module),
                        origin,
                        scrutinee,
                        &sequence,
                        position,
                    )?
                    else {
                        continue;
                    };
                    let rest_type = call.return_type();

                    if let Some(pattern) = pattern {
                        self.check_pattern_projection(
                            flow,
                            scope,
                            rest_type,
                            pattern.into_global_any(module),
                        )?;
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

        // commit the sequence assignment pattern
        let () = self.commit_assign_pattern(
            node,
            dir::AssignPatternDecision::Sequence(dir::AssignPatternSequenceResolution {
                arity,
                fields: projected,
                rest,
            }),
        )?;

        Ok(true)
    }

    /// Return the `length` member for a sequence receiver.
    fn select_sequence_length(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(Protocol, dir::MemberDecision)>> {
        let key = self.static_name("length");
        let Some((sequence, member)) = self.select_language_protocol_member(
            origin,
            receiver,
            receiver,
            dir::MemberSpace::Instance,
            key,
            dir::LanguageItem::Sequence,
            &[],
            &[],
        )?
        else {
            return Ok(None);
        };

        Ok(Some((sequence, member.resolution)))
    }

    /// Return the `Index.index` call for a sequence receiver.
    fn select_sequence_element(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        index: usize,
    ) -> CompilerResult<Option<dir::ProjectionResolution>> {
        // rebase the origin at this element field
        let origin = self.origin_at(origin, node)?;

        // read the element at this exact index
        let index = self.static_usize_type(node, index)?;
        let Some(selection) = self.select_subscript_read_source(
            origin,
            Value {
                ty: receiver,
                node: None,
                place: None,
                is_fresh: false,
            },
            receiver,
            dir::ArgumentSource::Static(index),
            index,
        )?
        else {
            return Ok(None);
        };
        let Some(projection) = selection.into_read_projection() else {
            return Ok(None);
        };

        Ok(Some(projection))
    }

    /// Return the `Sequence.rest` call for a sequence receiver.
    fn select_sequence_rest(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        sequence: &Protocol,
        start: usize,
    ) -> CompilerResult<Option<dir::CallDecision>> {
        // rebase the origin at this rest field
        let origin = self.origin_at(origin, node)?;

        // read the rest slice from this start
        let start_type = self.static_usize_type(node, start)?;
        let key = self.static_name("rest");
        let sources = [dir::ArgumentSource::Static(start_type)];
        let call = self.select_protocol_call(
            origin,
            Value {
                ty: receiver,
                node: None,
                place: None,
                is_fresh: false,
            },
            receiver,
            dir::MemberSpace::Instance,
            key,
            sequence,
            &sources,
        )?;
        match call {
            Ok(call) => Ok(Some(call.resolution)),
            // report a sequence without a rest method as unsupported
            Err(SignatureRejection::Inapplicable) => {
                self.report_sequence_rest_unsupported(node);

                Ok(None)
            }
            // report the rejection of a selected rest method
            Err(rejection) => {
                self.report_signature_rejection(origin, rejection)?;

                Ok(None)
            }
        }
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
        _node: dir::GlobalNodeIdAny,
        value: usize,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_type(dir::Type::Literal(dir::Literal::Integer(value as i64)))
    }

    /// Return one interned static name key.
    fn static_name(&self, name: &str) -> dir::StaticKey {
        dir::StaticKey::Name(self.strings().intern(name))
    }
}
