use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CallDecision, CallResolution, CallResolutionTarget, CheckState, ConstructDecision,
    MemberDecision, MemberResolutionTarget, NameDecision, OperatorDecision, OperatorResolution,
    OperatorTermKind, ReceiverDecision, VariableId,
};

impl CheckState<'_> {
    /// Commit resolved calls into the checked resolution table.
    pub(super) fn commit_call_resolution_table(&mut self) -> CompilerResult<()> {
        let environment = self.environment.clone();
        let calls = self
            .solutions
            .call
            .values()
            .filter_map(|decision| match decision {
                CallDecision::Resolved(call) => Some(call.clone()),
                CallDecision::Rejected(_) => None,
            })
            .collect::<Vec<_>>();

        // write calls resolved by solve
        for call in calls {
            let module = call.source.module_id;
            let Some(parameters) = self.commit_function_parameter_type_ids(
                module,
                environment.as_ref(),
                &call.function.parameters,
                call.source.local_id,
            ) else {
                continue;
            };
            let return_type = call
                .function
                .return_type
                .and_then(|ty| self.commit_variable_type(environment.as_ref(), ty));
            let (target, candidate) = build_call_target(self, environment.as_ref(), &call);
            let resolution = dir::CallResolution::new(target, parameters, return_type);

            self.output_mut(module)
                .resolutions
                .set_call_resolution(call.source, resolution);

            if call.member_source.is_some()
                && let Some(candidate) = candidate
                && let Some(member_node) = call.member_source
            {
                let receiver = candidate.receiver;
                let target = dir::MemberTarget::Symbol(dir::MemberCandidate {
                    receiver,
                    symbol: candidate.symbol,
                    instance: candidate.instance,
                });
                let resolution = dir::MemberResolution::new(receiver, target);

                self.output_mut(module)
                    .resolutions
                    .set_member_resolution(member_node, resolution);
            }
        }

        Ok(())
    }

    /// Commit resolved construct expressions into the checked resolution table.
    pub(super) fn commit_construct_resolution_table(&mut self) -> CompilerResult<()> {
        let environment = self.environment.clone();
        let constructs = self
            .solutions
            .construct
            .values()
            .filter_map(|decision| match decision {
                ConstructDecision::Resolved(construct) => Some(construct.clone()),
                ConstructDecision::Rejected(_) => None,
            })
            .collect::<Vec<_>>();

        // write construct expressions resolved by solve
        for construct in constructs {
            let module = construct.source.module_id;
            let Some(parameters) = self.commit_function_parameter_type_ids(
                module,
                environment.as_ref(),
                &construct.function.parameters,
                construct.source.local_id,
            ) else {
                continue;
            };
            let return_type = construct
                .function
                .return_type
                .and_then(|ty| self.commit_variable_type(environment.as_ref(), ty));
            let target = match construct.symbol {
                Some(symbol) => {
                    let instance = construct.instance.as_ref().and_then(|instance| {
                        self.commit_generic_instance(
                            environment.as_ref(),
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

            self.output_mut(module)
                .resolutions
                .set_call_resolution(construct.source, resolution);
        }

        Ok(())
    }

    /// Commit accepted operators into the checked resolution table.
    pub(super) fn commit_operator_resolution_table(&mut self) -> CompilerResult<()> {
        let environment = self.environment.clone();
        let operators = self
            .solutions
            .operator
            .values()
            .filter_map(|decision| match decision {
                OperatorDecision::Resolved(operator) => Some(operator.clone()),
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
                    let Some(receiver) = self.commit_variable_type(environment.as_ref(), receiver)
                    else {
                        continue;
                    };
                    let Some(return_type) = self.commit_variable_type(environment.as_ref(), result)
                    else {
                        continue;
                    };
                    let Some((target, parameters)) = build_builtin_operator_call(
                        self,
                        environment.as_ref(),
                        kind,
                        receiver,
                        argument,
                    ) else {
                        continue;
                    };
                    let resolution =
                        dir::CallResolution::new(target, parameters, Some(return_type));

                    self.output_mut(source.module_id)
                        .resolutions
                        .set_call_resolution(source, resolution);
                }
                OperatorResolution::Method {
                    source,
                    symbol,
                    receiver,
                    function,
                } => {
                    let Some(receiver) = self.commit_variable_type(environment.as_ref(), receiver)
                    else {
                        continue;
                    };
                    let Some(parameters) = self.commit_function_parameter_type_ids(
                        source.module_id,
                        environment.as_ref(),
                        &function.parameters,
                        source.local_id,
                    ) else {
                        continue;
                    };
                    let return_type = function
                        .return_type
                        .and_then(|ty| self.commit_variable_type(environment.as_ref(), ty));
                    let candidate = dir::CallCandidate {
                        receiver: Some(receiver),
                        symbol,
                        instance: None,
                    };
                    let target = dir::CallTarget::Symbol(candidate.clone());
                    let resolution = dir::CallResolution::new(target, parameters, return_type);

                    self.output_mut(source.module_id)
                        .resolutions
                        .set_call_resolution(source, resolution);

                    let target = dir::MemberTarget::Symbol(dir::MemberCandidate {
                        receiver: Some(receiver),
                        symbol,
                        instance: None,
                    });
                    let resolution = dir::MemberResolution::new(Some(receiver), target);

                    self.output_mut(source.module_id)
                        .resolutions
                        .set_member_resolution(source, resolution);
                }
            }
        }

        Ok(())
    }
}

impl CheckState<'_> {
    /// Commit collected name resolutions into the checked resolution table.
    pub(super) fn commit_name_resolution_table(&mut self, module: destack_source::ModuleId) {
        let names = self.solutions.name.clone();

        // write lexical resolutions recorded by check
        for (source, decision) in names {
            if source.module_id != module {
                continue;
            }
            let NameDecision::Resolved(resolution) = decision;
            let resolution = dir::NameResolution::from_symbols(resolution.symbols);

            self.output_mut(module)
                .resolutions
                .set_name_resolution(source, resolution);
        }
    }

    /// Commit collected receiver resolutions into the checked resolution table.
    pub(super) fn commit_receiver_resolution_table(
        &mut self,
        module: destack_source::ModuleId,
        environment: &GlobalEnvironment,
    ) {
        let receivers = self.solutions.receiver.clone();

        // write contextual receiver resolutions
        for receiver in receivers.into_values() {
            let ReceiverDecision::Resolved(receiver) = receiver;
            if receiver.source.module_id != module {
                continue;
            }
            let ty = self.commit_variable_type(environment, receiver.ty);
            let resolution = dir::ReceiverResolution {
                kind: receiver.kind,
                owner: receiver.owner,
                ty,
            };

            self.output_mut(module)
                .resolutions
                .set_receiver_resolution(receiver.source, resolution);
        }
    }

    /// Commit member solutions into the checked resolution table.
    pub(super) fn commit_member_resolution_table(
        &mut self,
        module: destack_source::ModuleId,
        environment: &GlobalEnvironment,
    ) {
        let members = self
            .solutions
            .member
            .values()
            .filter_map(|decision| match decision {
                MemberDecision::Resolved(member) if member.source.module_id == module => {
                    Some(member)
                }
                MemberDecision::Rejected(_) => None,
                MemberDecision::Resolved(_) => None,
            })
            .cloned()
            .collect::<Vec<_>>();

        // write member solutions recorded by solve
        for member in members {
            if self
                .output(module)
                .resolutions
                .member_resolution(member.source)
                .is_some()
            {
                continue;
            }
            let Some(receiver) = self.commit_variable_type(environment, member.receiver) else {
                continue;
            };
            let target = match member.target {
                MemberResolutionTarget::Field(key) => dir::MemberTarget::Field(key),
                MemberResolutionTarget::Symbol { symbol, instance } => {
                    let instance = instance.as_ref().and_then(|instance| {
                        self.commit_generic_instance(environment, member.source, instance)
                    });
                    let candidate = dir::MemberCandidate {
                        receiver: Some(receiver),
                        symbol,
                        instance,
                    };

                    dir::MemberTarget::Symbol(candidate)
                }
            };
            let resolution = dir::MemberResolution::new(Some(receiver), target);

            self.output_mut(module)
                .resolutions
                .set_member_resolution(member.source, resolution);
        }
    }
}

/// Build one call target and its reusable symbol candidate.
fn build_call_target(
    check: &mut CheckState<'_>,
    environment: &GlobalEnvironment,
    call: &CallResolution,
) -> (dir::CallTarget, Option<dir::CallCandidate>) {
    let target = match &call.target {
        CallResolutionTarget::Value => (dir::CallTarget::Value, None),
        CallResolutionTarget::Symbol {
            symbol,
            instance,
            receiver,
        } => {
            let receiver =
                receiver.and_then(|receiver| check.commit_variable_type(environment, receiver));
            let instance = instance.as_ref().and_then(|instance| {
                check.commit_generic_instance(environment, call.source, instance)
            });
            let candidate = dir::CallCandidate {
                receiver,
                symbol: *symbol,
                instance,
            };

            (dir::CallTarget::Symbol(candidate.clone()), Some(candidate))
        }
        CallResolutionTarget::Constructor { symbol, instance } => {
            let instance = instance.as_ref().and_then(|instance| {
                check.commit_generic_instance(environment, call.source, instance)
            });
            let candidate = dir::CallCandidate {
                receiver: None,
                symbol: *symbol,
                instance,
            };

            (
                dir::CallTarget::Construct(candidate.clone()),
                Some(candidate),
            )
        }
    };

    target
}

/// Build the builtin call shape for one operator.
fn build_builtin_operator_call(
    check: &mut CheckState<'_>,
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
            let argument = check.commit_variable_type(environment, argument)?;
            let target = dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator });

            Some((target, vec![receiver, argument]))
        }
    }
}
