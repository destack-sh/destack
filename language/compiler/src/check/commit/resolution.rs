use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CallResolution, CallResolutionTarget, CallSelection, CheckState, ConstructSelection,
    MemberResolutionTarget, MemberSelection, NameSelection, OperatorResolution, OperatorSelection,
    OperatorTermKind, ReceiverSelection, VariableId,
};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit resolved calls into the checked resolution table.
    pub(super) fn commit_call_resolution_table(
        &mut self,
        module: destack_source::ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<()> {
        let calls = self
            .solutions
            .call
            .values()
            .filter_map(|decision| match decision {
                CallSelection::Resolved(call) => Some(call.clone()),
                CallSelection::Rejected(_) => None,
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
            let return_type = call.function.return_type.and_then(|ty| {
                self.commit_type_operand(module, output, environment, ty, call.source.local_id)
            });
            let target = build_call_target(self, module, output, environment, &call);
            let resolution = dir::CallResolution::new(target, parameters, return_type);

            output
                .resolutions
                .set_call_resolution(call.source, resolution);
        }

        Ok(())
    }

    /// Commit resolved construct expressions into the checked resolution table.
    pub(super) fn commit_construct_resolution_table(
        &mut self,
        module: destack_source::ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<()> {
        let constructs = self
            .solutions
            .construct
            .values()
            .filter_map(|decision| match decision {
                ConstructSelection::Resolved(construct) => Some(construct.clone()),
                ConstructSelection::Rejected(_) => None,
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
            let return_type = construct.function.return_type.and_then(|ty| {
                self.commit_type_operand(module, output, environment, ty, construct.source.local_id)
            });
            let target = match construct.symbol {
                Some(symbol) => {
                    let instance = construct.instance.as_ref().and_then(|instance| {
                        self.commit_generic_instance(
                            module,
                            output,
                            environment,
                            construct.source,
                            instance,
                        )
                    });
                    let candidate = dir::CallCandidate {
                        receiver: None,
                        symbol,
                        instance,
                    };

                    dir::CallTarget::Construct(candidate)
                }
                None => dir::CallTarget::Value,
            };
            let resolution = dir::CallResolution::new(target, parameters, return_type);

            output
                .resolutions
                .set_call_resolution(construct.source, resolution);
        }

        Ok(())
    }

    /// Commit accepted operators into the checked resolution table.
    pub(super) fn commit_operator_resolution_table(
        &mut self,
        module: destack_source::ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<()> {
        let operators = self
            .solutions
            .operator
            .values()
            .filter_map(|decision| match decision {
                OperatorSelection::Resolved(operator) => Some(operator.clone()),
                OperatorSelection::Rejected(_) => None,
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
                    let Some(receiver) =
                        self.commit_variable_type(module, output, environment, receiver)
                    else {
                        continue;
                    };
                    let Some(return_type) =
                        self.commit_variable_type(module, output, environment, result)
                    else {
                        continue;
                    };
                    let Some((target, parameters)) = build_builtin_operator_call(
                        self,
                        module,
                        output,
                        environment,
                        kind,
                        receiver,
                        argument,
                    ) else {
                        continue;
                    };
                    let resolution =
                        dir::CallResolution::new(target, parameters, Some(return_type));

                    output.resolutions.set_call_resolution(source, resolution);
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
                    let Some(receiver) =
                        self.commit_variable_type(module, output, environment, receiver)
                    else {
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
                    let return_type = function.return_type.and_then(|ty| {
                        self.commit_type_operand(module, output, environment, ty, source.local_id)
                    });
                    let candidate = dir::CallCandidate {
                        receiver: Some(receiver),
                        symbol,
                        instance: None,
                    };
                    let target = dir::CallTarget::Symbol(candidate.clone());
                    let resolution = dir::CallResolution::new(target, parameters, return_type);

                    output.resolutions.set_call_resolution(source, resolution);

                    let target = dir::MemberTarget::Symbol(dir::MemberCandidate {
                        receiver: Some(receiver),
                        symbol,
                        instance: None,
                    });
                    let resolution = dir::MemberResolution::new(Some(receiver), target);

                    output.resolutions.set_member_resolution(source, resolution);
                }
            }
        }

        Ok(())
    }
}

impl CheckState<'_> {
    /// Commit collected name resolutions into the checked resolution table.
    pub(super) fn commit_name_resolution_table(
        &mut self,
        module: destack_source::ModuleId,
        output: &mut CheckModuleOutput,
    ) {
        let names = self.solutions.name.clone();

        // write lexical resolutions selected by check
        for (source, decision) in names {
            if source.module_id != module {
                continue;
            }
            let NameSelection::Resolved(resolution) = decision;
            let resolution = dir::NameResolution::from_symbols(resolution.symbols);

            output.resolutions.set_name_resolution(source, resolution);
        }
    }

    /// Commit collected receiver resolutions into the checked resolution table.
    pub(super) fn commit_receiver_resolution_table(
        &mut self,
        module: destack_source::ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) {
        let receivers = self.solutions.receiver.clone();

        // write contextual receiver resolutions
        for receiver in receivers.into_values() {
            let ReceiverSelection::Resolved(receiver) = receiver;
            if receiver.source.module_id != module {
                continue;
            }
            let ty = self.commit_variable_type(module, output, environment, receiver.ty);
            let resolution = dir::ReceiverResolution {
                kind: receiver.kind,
                owner: receiver.owner,
                ty,
            };

            output
                .resolutions
                .set_receiver_resolution(receiver.source, resolution);
        }
    }

    /// Commit member solutions into the checked resolution table.
    pub(super) fn commit_member_resolution_table(
        &mut self,
        module: destack_source::ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) {
        let members = self
            .solutions
            .member
            .values()
            .filter_map(|decision| match decision {
                MemberSelection::Resolved(member) if member.source.module_id == module => {
                    Some(member)
                }
                MemberSelection::Rejected(_) => None,
                MemberSelection::Resolved(_) => None,
            })
            .cloned()
            .collect::<Vec<_>>();

        // write member solutions selected by solve
        for member in members {
            if output
                .resolutions
                .member_resolution(member.source)
                .is_some()
            {
                continue;
            }
            let Some(receiver) =
                self.commit_variable_type(module, output, environment, member.receiver)
            else {
                continue;
            };
            let target = match member.target {
                MemberResolutionTarget::Builtin(member) => dir::MemberTarget::Builtin(member),
                MemberResolutionTarget::Field(key) => dir::MemberTarget::Field(key),
                MemberResolutionTarget::Symbol { symbol, instance } => {
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
                        receiver: Some(receiver),
                        symbol,
                        instance,
                    };

                    dir::MemberTarget::Symbol(candidate)
                }
                MemberResolutionTarget::Select(candidates) => {
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
                                receiver: Some(receiver),
                                symbol: candidate.symbol,
                                instance,
                            }
                        })
                        .collect();

                    dir::MemberTarget::Select(candidates)
                }
            };
            let resolution = dir::MemberResolution::new(Some(receiver), target);

            output
                .resolutions
                .set_member_resolution(member.source, resolution);
        }
    }
}

/// Build one call target and its reusable symbol candidate.
fn build_call_target(
    check: &mut CheckState<'_>,
    module: destack_source::ModuleId,
    output: &mut CheckModuleOutput,
    environment: &GlobalEnvironment,
    call: &CallResolution,
) -> dir::CallTarget {
    match &call.target {
        CallResolutionTarget::Value => dir::CallTarget::Value,
        CallResolutionTarget::Symbol {
            symbol,
            instance,
            receiver,
        } => {
            let receiver = receiver.and_then(|receiver| {
                check.commit_variable_type(module, output, environment, receiver)
            });
            let instance = instance.as_ref().and_then(|instance| {
                check.commit_generic_instance(module, output, environment, call.source, instance)
            });
            let candidate = dir::CallCandidate {
                receiver,
                symbol: *symbol,
                instance,
            };

            dir::CallTarget::Symbol(candidate)
        }
        CallResolutionTarget::Select {
            candidates,
            receiver,
        } => {
            let receiver = receiver.and_then(|receiver| {
                check.commit_variable_type(module, output, environment, receiver)
            });
            let candidates = candidates
                .iter()
                .map(|candidate| {
                    let instance = candidate.instance.as_ref().and_then(|instance| {
                        check.commit_generic_instance(
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

            dir::CallTarget::Select(candidates)
        }
        CallResolutionTarget::Constructor { symbol, instance } => {
            let instance = instance.as_ref().and_then(|instance| {
                check.commit_generic_instance(module, output, environment, call.source, instance)
            });
            let candidate = dir::CallCandidate {
                receiver: None,
                symbol: *symbol,
                instance,
            };

            dir::CallTarget::Construct(candidate)
        }
    }
}

/// Build the builtin call shape for one operator.
fn build_builtin_operator_call(
    check: &mut CheckState<'_>,
    module: destack_source::ModuleId,
    output: &mut CheckModuleOutput,
    environment: &GlobalEnvironment,
    kind: OperatorTermKind,
    receiver: dir::LocalTypeId,
    argument: Option<VariableId>,
) -> Option<(dir::CallTarget, Vec<dir::LocalTypeId>)> {
    match kind {
        OperatorTermKind::Unary(operator) => {
            let target = dir::CallTarget::Builtin(dir::BuiltinCall::UnaryOperator { operator });

            Some((target, vec![receiver]))
        }
        OperatorTermKind::Binary(operator) => {
            let argument = argument?;
            let argument = check.commit_variable_type(module, output, environment, argument)?;
            let target = dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator });

            Some((target, vec![receiver, argument]))
        }
    }
}
