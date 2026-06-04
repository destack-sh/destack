use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CallDecision, CallResolution, CallTargetResolution, CheckState, ConstructDecision,
    ConstructTargetResolution, MemberDecision, MemberTargetResolution, OperatorDecision,
    OperatorResolution, OperatorTermKind, PatternDecision, PatternFieldResolution,
    PatternFieldTargetResolution, PatternResolution, PatternSequenceResolution,
    PatternTargetResolution, StaticOperand, TypeOperand,
};
use crate::{CompilerError, CompilerResult};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit checked resolutions into one DIR resolution segment.
    pub(super) fn commit_resolution_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<dir::ResolutionSegment> {
        let mut resolutions = dir::ResolutionSegment::new(module);

        self.write_call_resolutions(module, output, environment, &mut resolutions)?;
        self.write_construct_resolutions(module, output, environment, &mut resolutions)?;
        self.write_operator_resolutions(module, output, environment, &mut resolutions)?;
        self.write_name_resolutions(module, &mut resolutions);
        self.write_receiver_resolutions(module, output, environment, &mut resolutions)?;
        self.write_member_resolutions(module, output, environment, &mut resolutions);
        self.write_pattern_resolutions(module, output, environment, &mut resolutions);

        Ok(resolutions)
    }

    /// Write resolved calls into the resolution segment.
    fn write_call_resolutions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        resolutions: &mut dir::ResolutionSegment,
    ) -> CompilerResult<()> {
        let calls = self
            .inference
            .calls()
            .into_iter()
            .filter_map(|(_, decision)| match decision {
                CallDecision::Resolved(call) => Some(call),
                CallDecision::Rejected(_) => None,
            })
            .collect::<Vec<_>>();

        // write calls resolved by solve
        for call in calls {
            if call.source.module_id != module {
                continue;
            }
            let Some(parameters) = self.commit_function_parameter_type_ids(
                module,
                output,
                environment,
                &call.function.parameters,
                call.source.local_id,
            ) else {
                continue;
            };
            let Some(return_type) = self.commit_function_return_type(
                module,
                output,
                environment,
                call.function.return_type,
                call.source.local_id,
            ) else {
                continue;
            };
            let target = self.commit_call_target(module, output, environment, &call);
            let resolution = dir::CallResolution::new(target, parameters, return_type);

            resolutions.set_call_resolution(call.source, resolution);
        }

        Ok(())
    }

    /// Write resolved construct expressions into the resolution segment.
    fn write_construct_resolutions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        resolutions: &mut dir::ResolutionSegment,
    ) -> CompilerResult<()> {
        let constructs = self
            .inference
            .constructs()
            .into_iter()
            .filter_map(|(_, decision)| match decision {
                ConstructDecision::Resolved(construct) => Some(construct),
                ConstructDecision::Rejected(_) => None,
            })
            .collect::<Vec<_>>();

        // write construct expressions resolved by solve
        for construct in constructs {
            if construct.source.module_id != module {
                continue;
            }
            let Some(parameters) = self.commit_function_parameter_type_ids(
                module,
                output,
                environment,
                &construct.function.parameters,
                construct.source.local_id,
            ) else {
                continue;
            };
            let Some(return_type) = self.commit_function_return_type(
                module,
                output,
                environment,
                construct.function.return_type,
                construct.source.local_id,
            ) else {
                continue;
            };
            let Some(target) =
                self.commit_construct_target(module, output, environment, &construct)
            else {
                continue;
            };
            let resolution = dir::ConstructResolution::new(target, parameters, return_type);

            resolutions.set_construct_resolution(construct.source, resolution);
        }

        Ok(())
    }

    /// Commit one function return type for call-like resolution metadata.
    fn commit_function_return_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        return_type: Option<TypeOperand>,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        let Some(return_type) = return_type else {
            let type_id = self.intern_type(module, output, dir::Type::Void, source);

            return Some(type_id.into_global(module));
        };

        self.commit_type_operand(module, output, environment, return_type, source)
    }

    /// Write accepted operators into the resolution segment.
    fn write_operator_resolutions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        resolutions: &mut dir::ResolutionSegment,
    ) -> CompilerResult<()> {
        let operators = self
            .inference
            .operators()
            .into_iter()
            .filter_map(|(_, decision)| match decision {
                OperatorDecision::Resolved(operator) => Some(operator),
                OperatorDecision::Rejected(_) => None,
            })
            .collect::<Vec<_>>();

        // write operators resolved by solve
        for operator in operators {
            match operator {
                OperatorResolution::Builtin {
                    source,
                    kind,
                    receiver,
                    argument,
                    result,
                } => {
                    if source.module_id != module {
                        continue;
                    }
                    let Some(receiver) = self.commit_type_operand(
                        module,
                        output,
                        environment,
                        receiver,
                        source.local_id,
                    ) else {
                        continue;
                    };
                    let Some(return_type) =
                        self.commit_variable_type(module, output, environment, result)
                    else {
                        continue;
                    };
                    let Some((target, parameters)) = self.commit_builtin_operator_call(
                        module,
                        output,
                        environment,
                        source.local_id,
                        kind,
                        receiver,
                        argument,
                    ) else {
                        continue;
                    };
                    let resolution = dir::CallResolution::new(target, parameters, return_type);

                    resolutions.set_call_resolution(source, resolution);
                }
                OperatorResolution::Method {
                    source,
                    symbol,
                    receiver,
                    function,
                } => {
                    if source.module_id != module {
                        continue;
                    }
                    let Some(receiver) = self.commit_type_operand(
                        module,
                        output,
                        environment,
                        receiver,
                        source.local_id,
                    ) else {
                        continue;
                    };
                    let Some(parameters) = self.commit_function_parameter_type_ids(
                        module,
                        output,
                        environment,
                        &function.parameters,
                        source.local_id,
                    ) else {
                        continue;
                    };
                    let Some(return_type) = self.commit_function_return_type(
                        module,
                        output,
                        environment,
                        function.return_type,
                        source.local_id,
                    ) else {
                        continue;
                    };
                    let target = dir::CallTarget::Symbol(dir::CallCandidate {
                        receiver: Some(receiver),
                        symbol,
                        instance: None,
                    });
                    let resolution = dir::CallResolution::new(target, parameters, return_type);

                    resolutions.set_call_resolution(source, resolution);

                    let target = dir::MemberTarget::Symbol(dir::MemberCandidate {
                        receiver,
                        symbol,
                        instance: None,
                    });
                    let resolution = dir::MemberResolution::new(receiver, target);

                    resolutions.set_member_resolution(source, resolution);
                }
            }
        }

        Ok(())
    }

    /// Write collected name resolutions into the resolution segment.
    fn write_name_resolutions(&self, module: ModuleId, resolutions: &mut dir::ResolutionSegment) {
        let names = self.inference.names();

        // write lexical resolutions selected by check
        for (source, resolution) in names {
            if source.module_id != module {
                continue;
            }

            resolutions.set_name_resolution(source, resolution);
        }
    }

    /// Write collected receiver resolutions into the resolution segment.
    fn write_receiver_resolutions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        resolutions: &mut dir::ResolutionSegment,
    ) -> CompilerResult<()> {
        let receivers = self.inference.receivers();

        // write contextual receiver resolutions
        for receiver in receivers {
            if receiver.source.module_id != module {
                continue;
            }
            let Some(ty) = self.commit_type_operand(
                module,
                output,
                environment,
                receiver.ty,
                receiver.source.local_id,
            ) else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "receiver {:?} selected uncommittable type {:?}",
                        receiver.source, receiver.ty
                    ),
                });
            };
            let resolution = dir::ReceiverResolution {
                kind: receiver.kind,
                owner: receiver.owner,
                ty,
            };

            resolutions.set_receiver_resolution(receiver.source, resolution);
        }

        Ok(())
    }

    /// Write member solutions into the resolution segment.
    fn write_member_resolutions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        resolutions: &mut dir::ResolutionSegment,
    ) {
        let members = self
            .inference
            .members()
            .into_iter()
            .filter_map(|(_, decision)| match decision {
                MemberDecision::Resolved(member) if member.source.module_id == module => {
                    Some(member)
                }
                MemberDecision::Rejected(_) => None,
                MemberDecision::Resolved(_) => None,
            })
            .collect::<Vec<_>>();

        // write member solutions selected by solve
        for member in members {
            if resolutions.member_resolution(member.source).is_some() {
                continue;
            }
            let Some(receiver) = self.commit_type_operand(
                module,
                output,
                environment,
                member.receiver,
                member.source.local_id,
            ) else {
                continue;
            };
            let target = match member.target {
                MemberTargetResolution::Builtin(member) => dir::MemberTarget::Builtin(member),
                MemberTargetResolution::Field(key) => dir::MemberTarget::Field(key),
                MemberTargetResolution::Symbol { symbol, instance } => {
                    let instance = instance.as_ref().and_then(|instance| {
                        self.commit_generic_instance(
                            module,
                            output,
                            environment,
                            member.source,
                            instance,
                        )
                    });
                    let candidate = dir::MemberCandidate {
                        receiver,
                        symbol,
                        instance,
                    };

                    dir::MemberTarget::Symbol(candidate)
                }
                MemberTargetResolution::Union(candidates) => {
                    let candidates = candidates
                        .into_iter()
                        .map(|candidate| {
                            let instance = candidate.instance.as_ref().and_then(|instance| {
                                self.commit_generic_instance(
                                    module,
                                    output,
                                    environment,
                                    member.source,
                                    instance,
                                )
                            });

                            dir::MemberCandidate {
                                receiver,
                                symbol: candidate.symbol,
                                instance,
                            }
                        })
                        .collect();

                    dir::MemberTarget::Union(candidates)
                }
            };
            let resolution = dir::MemberResolution::new(receiver, target);

            resolutions.set_member_resolution(member.source, resolution);
        }
    }

    /// Write collected pattern resolutions into the resolution segment.
    fn write_pattern_resolutions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        resolutions: &mut dir::ResolutionSegment,
    ) {
        let patterns = self
            .inference
            .patterns()
            .into_iter()
            .filter_map(|decision| match decision {
                PatternDecision::Resolved(pattern) if pattern.source.module_id == module => {
                    Some(pattern)
                }
                PatternDecision::Rejected(_) => None,
                PatternDecision::Resolved(_) => None,
            })
            .collect::<Vec<_>>();

        // write pattern decisions selected by check
        for pattern in patterns {
            let Some(resolution) =
                self.commit_pattern_selection(module, output, environment, &pattern)
            else {
                continue;
            };

            resolutions.set_pattern_resolution(pattern.source, resolution);
        }
    }
}

impl CheckState<'_> {
    /// Commit one call target.
    fn commit_call_target(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        call: &CallResolution,
    ) -> dir::CallTarget {
        match &call.target {
            CallTargetResolution::Expression { instance } => {
                let instance = instance.as_ref().and_then(|instance| {
                    self.commit_generic_instance(module, output, environment, call.source, instance)
                });

                dir::CallTarget::Expression { instance }
            }
            CallTargetResolution::Symbol {
                symbol,
                instance,
                receiver,
            } => {
                let receiver = receiver.and_then(|receiver| {
                    self.commit_type_operand(
                        module,
                        output,
                        environment,
                        receiver,
                        call.source.local_id,
                    )
                });
                let instance = instance.as_ref().and_then(|instance| {
                    self.commit_generic_instance(module, output, environment, call.source, instance)
                });
                let candidate = dir::CallCandidate {
                    receiver,
                    symbol: *symbol,
                    instance,
                };

                dir::CallTarget::Symbol(candidate)
            }
            CallTargetResolution::Union {
                candidates,
                receiver,
            } => {
                let receiver = receiver.and_then(|receiver| {
                    self.commit_type_operand(
                        module,
                        output,
                        environment,
                        receiver,
                        call.source.local_id,
                    )
                });
                let candidates = candidates
                    .iter()
                    .map(|candidate| {
                        let instance = candidate.instance.as_ref().and_then(|instance| {
                            self.commit_generic_instance(
                                module,
                                output,
                                environment,
                                call.source,
                                instance,
                            )
                        });

                        dir::CallCandidate {
                            receiver,
                            symbol: candidate.symbol,
                            instance,
                        }
                    })
                    .collect();

                dir::CallTarget::Union(candidates)
            }
        }
    }

    /// Commit one construct target.
    fn commit_construct_target(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        construct: &crate::check::ConstructResolution,
    ) -> Option<dir::ConstructTarget> {
        match &construct.target {
            ConstructTargetResolution::Class {
                symbol,
                constructor,
                instance,
            } => {
                let candidate = self.commit_class_construct_candidate(
                    module,
                    output,
                    environment,
                    construct.source,
                    *symbol,
                    *constructor,
                    instance.as_ref(),
                );

                Some(dir::ConstructTarget::Class(candidate))
            }
            ConstructTargetResolution::Newtype { symbol, instance } => {
                let candidate = self.commit_construct_candidate(
                    module,
                    output,
                    environment,
                    construct.source,
                    *symbol,
                    instance.as_ref(),
                );

                Some(dir::ConstructTarget::Newtype(candidate))
            }
        }
    }

    /// Commit one class construct candidate.
    fn commit_class_construct_candidate(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        constructor: Option<dir::GlobalSymbolId>,
        instance: Option<&crate::check::GenericInstance>,
    ) -> dir::ClassConstructCandidate {
        let instance = instance.and_then(|instance| {
            self.commit_generic_instance(module, output, environment, source, instance)
        });

        dir::ClassConstructCandidate {
            symbol,
            constructor,
            instance,
        }
    }

    /// Commit one newtype construct candidate.
    fn commit_construct_candidate(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        instance: Option<&crate::check::GenericInstance>,
    ) -> dir::NewtypeConstructCandidate {
        let instance = instance.and_then(|instance| {
            self.commit_generic_instance(module, output, environment, source, instance)
        });

        dir::NewtypeConstructCandidate { symbol, instance }
    }

    /// Commit one builtin operator as a builtin call.
    fn commit_builtin_operator_call(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        source: dir::LocalNodeIdAny,
        kind: OperatorTermKind,
        receiver: dir::GlobalTypeId,
        argument: Option<TypeOperand>,
    ) -> Option<(dir::CallTarget, Vec<dir::GlobalTypeId>)> {
        match kind {
            OperatorTermKind::Unary(operator) => {
                let target = dir::CallTarget::Builtin(dir::BuiltinCall::UnaryOperator { operator });

                Some((target, vec![receiver]))
            }
            OperatorTermKind::Binary(operator) => {
                let argument = argument?;
                let argument =
                    self.commit_type_operand(module, output, environment, argument, source)?;
                let target =
                    dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator });

                Some((target, vec![receiver, argument]))
            }
        }
    }

    /// Commit one pattern selection.
    fn commit_pattern_selection(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        pattern: &PatternResolution,
    ) -> Option<dir::PatternResolution> {
        match &pattern.target {
            PatternTargetResolution::Wildcard => Some(dir::PatternResolution::Wildcard),
            PatternTargetResolution::Binding(binding) => Some(dir::PatternResolution::Binding(
                dir::PatternBindingResolution {
                    symbol: binding.symbol,
                    pattern: binding.pattern,
                },
            )),
            PatternTargetResolution::Literal(literal) => {
                let value =
                    self.commit_static_operand(module, output, environment, literal.value)?;

                Some(dir::PatternResolution::Literal(
                    dir::PatternLiteralResolution { value },
                ))
            }
            PatternTargetResolution::Range(range) => {
                let domain = self.commit_type_operand(
                    module,
                    output,
                    environment,
                    range.domain,
                    pattern.source.local_id,
                )?;
                let start =
                    self.commit_optional_pattern_static(module, output, environment, range.start)?;
                let end =
                    self.commit_optional_pattern_static(module, output, environment, range.end)?;

                Some(dir::PatternResolution::Range(dir::PatternRangeResolution {
                    domain,
                    start,
                    end,
                    end_bound: range.end_bound,
                }))
            }
            PatternTargetResolution::Tuple(tuple) => {
                let fields = self.commit_pattern_field_selections(&tuple.fields);

                Some(dir::PatternResolution::Tuple(dir::PatternTupleResolution {
                    fields,
                }))
            }
            PatternTargetResolution::Sequence(sequence) => {
                self.commit_pattern_sequence_selection(module, output, environment, sequence)
            }
            PatternTargetResolution::Shape(shape) => {
                let fields = self.commit_pattern_field_selections(&shape.fields);

                Some(dir::PatternResolution::Shape(dir::PatternShapeResolution {
                    fields,
                }))
            }
            PatternTargetResolution::Nominal(nominal) => {
                let instance = nominal.instance.as_ref().and_then(|instance| {
                    self.commit_generic_instance(
                        module,
                        output,
                        environment,
                        pattern.source,
                        instance,
                    )
                });
                let fields = self.commit_pattern_field_selections(&nominal.fields);

                Some(dir::PatternResolution::Nominal(
                    dir::PatternNominalResolution {
                        symbol: nominal.symbol,
                        instance,
                        fields,
                    },
                ))
            }
            PatternTargetResolution::Newtype(newtype) => {
                let instance = newtype.instance.as_ref().and_then(|instance| {
                    self.commit_generic_instance(
                        module,
                        output,
                        environment,
                        pattern.source,
                        instance,
                    )
                });

                Some(dir::PatternResolution::Newtype(
                    dir::PatternNewtypeResolution {
                        symbol: newtype.symbol,
                        instance,
                        value: newtype.value,
                    },
                ))
            }
            PatternTargetResolution::Variant(variant) => {
                let instance = variant.instance.as_ref().and_then(|instance| {
                    self.commit_generic_instance(
                        module,
                        output,
                        environment,
                        pattern.source,
                        instance,
                    )
                });
                let discriminant = self.commit_optional_pattern_static(
                    module,
                    output,
                    environment,
                    variant.discriminant,
                )?;
                let fields = self.commit_pattern_field_selections(&variant.fields);

                Some(dir::PatternResolution::Variant(
                    dir::PatternVariantResolution {
                        symbol: variant.symbol,
                        instance,
                        discriminant,
                        fields,
                    },
                ))
            }
            PatternTargetResolution::Union(union) => {
                Some(dir::PatternResolution::Union(dir::PatternUnionResolution {
                    alternatives: union.alternatives.clone(),
                }))
            }
            PatternTargetResolution::Borrow(borrow) => Some(dir::PatternResolution::Borrow(
                dir::PatternBorrowResolution {
                    access: borrow.access,
                    pattern: borrow.pattern,
                },
            )),
            PatternTargetResolution::Move(move_) => {
                Some(dir::PatternResolution::Move(dir::PatternMoveResolution {
                    access: move_.access,
                    pattern: move_.pattern,
                }))
            }
            PatternTargetResolution::Dereference(dereference) => Some(
                dir::PatternResolution::Dereference(dir::PatternDereferenceResolution {
                    pattern: dereference.pattern,
                }),
            ),
        }
    }

    /// Commit one sequence pattern selection.
    fn commit_pattern_sequence_selection(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        sequence: &PatternSequenceResolution,
    ) -> Option<dir::PatternResolution> {
        let sequence = match sequence {
            PatternSequenceResolution::Array { fields, rest } => {
                dir::PatternSequenceResolution::Array {
                    fields: self.commit_pattern_field_selections(fields),
                    rest: rest.map(|rest| dir::PatternRestResolution {
                        source: rest.source,
                        pattern: rest.pattern,
                    }),
                }
            }
            PatternSequenceResolution::Slice { fields, rest } => {
                dir::PatternSequenceResolution::Slice {
                    fields: self.commit_pattern_field_selections(fields),
                    rest: rest.map(|rest| dir::PatternRestResolution {
                        source: rest.source,
                        pattern: rest.pattern,
                    }),
                }
            }
            PatternSequenceResolution::FixedArray { fields, length } => {
                let length = self.commit_static_operand(module, output, environment, *length)?;

                dir::PatternSequenceResolution::FixedArray {
                    fields: self.commit_pattern_field_selections(fields),
                    length,
                }
            }
        };

        Some(dir::PatternResolution::Sequence(sequence))
    }

    /// Commit pattern field selections.
    fn commit_pattern_field_selections(
        &self,
        fields: &[PatternFieldResolution],
    ) -> Vec<dir::PatternFieldResolution> {
        fields
            .iter()
            .map(|field| {
                let target = match field.target {
                    PatternFieldTargetResolution::Key(key) => dir::PatternFieldTarget::Key(key),
                    PatternFieldTargetResolution::Index(index) => {
                        dir::PatternFieldTarget::Index(index)
                    }
                };

                dir::PatternFieldResolution {
                    source: field.source,
                    target,
                    pattern: field.pattern,
                }
            })
            .collect()
    }

    /// Commit one optional pattern static operand.
    fn commit_optional_pattern_static(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        operand: Option<StaticOperand>,
    ) -> Option<Option<dir::GlobalStaticId>> {
        let Some(operand) = operand else {
            return Some(None);
        };
        let value = self.commit_static_operand(module, output, environment, operand)?;

        Some(Some(value))
    }
}
