use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CallOutcome, CallResolution, CallResolutionTarget, CheckComponentState, CheckModuleState,
    ConstructOutcome, MemberOutcome, MemberResolutionTarget, OperatorOutcome, OperatorResolution,
    OperatorTermKind, VariableId,
};

impl CheckComponentState<'_> {
    /// Commit resolved calls into the checked resolution table.
    pub(super) fn commit_call_resolution_table(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // write calls resolved by solve
        for module in modules {
            let environment = self.environment.clone();
            let calls = self
                .module_mut(module)?
                .decisions
                .call
                .values()
                .filter_map(|outcome| match outcome {
                    CallOutcome::Resolved(call) => Some(call),
                    CallOutcome::Rejected(_) => None,
                })
                .cloned()
                .collect::<Vec<_>>();
            let check_module = self.module_mut(module)?;

            for call in calls {
                let Some(parameters) = check_module.commit_function_parameter_type_ids(
                    environment.as_ref(),
                    &call.function.parameters,
                ) else {
                    continue;
                };
                let return_type = call
                    .function
                    .return_type
                    .and_then(|ty| check_module.commit_variable_type(environment.as_ref(), ty));
                let (target, candidate) =
                    build_call_target(check_module, environment.as_ref(), &call);
                let resolution = dir::CallResolution::new(target, parameters, return_type);

                check_module
                    .output
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

                    check_module
                        .output
                        .resolutions
                        .set_member_resolution(member_node, resolution);
                }
            }
        }

        Ok(())
    }

    /// Commit resolved constructs into the checked resolution table.
    pub(super) fn commit_construct_resolution_table(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // write constructs resolved by solve
        for module in modules {
            let environment = self.environment.clone();
            let constructs = self
                .module_mut(module)?
                .decisions
                .construct
                .values()
                .filter_map(|outcome| match outcome {
                    ConstructOutcome::Resolved(construct) => Some(construct),
                    ConstructOutcome::Rejected(_) => None,
                })
                .cloned()
                .collect::<Vec<_>>();
            let check_module = self.module_mut(module)?;

            for construct in constructs {
                let Some(parameters) = check_module.commit_function_parameter_type_ids(
                    environment.as_ref(),
                    &construct.function.parameters,
                ) else {
                    continue;
                };
                let return_type = construct
                    .function
                    .return_type
                    .and_then(|ty| check_module.commit_variable_type(environment.as_ref(), ty));
                let target = match construct.symbol {
                    Some(symbol) => {
                        let instance = construct.instance.as_ref().and_then(|instance| {
                            check_module.commit_generic_instance(
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

                check_module
                    .output
                    .resolutions
                    .set_call_resolution(construct.source, resolution);
            }
        }

        Ok(())
    }

    /// Commit accepted operators into the checked resolution table.
    pub(super) fn commit_operator_resolution_table(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // write operators resolved by solve
        for module in modules {
            let environment = self.environment.clone();
            let operators = self
                .module_mut(module)?
                .decisions
                .operator
                .values()
                .filter_map(|outcome| match outcome {
                    OperatorOutcome::Resolved(operator) => Some(operator),
                    OperatorOutcome::Rejected(_) => None,
                })
                .cloned()
                .collect::<Vec<_>>();
            let check_module = self.module_mut(module)?;

            for operator in operators {
                match operator {
                    OperatorResolution::Builtin {
                        source,
                        kind,
                        receiver,
                        argument,
                        result,
                    } => {
                        let Some(receiver) =
                            check_module.commit_variable_type(environment.as_ref(), receiver)
                        else {
                            continue;
                        };
                        let Some(return_type) =
                            check_module.commit_variable_type(environment.as_ref(), result)
                        else {
                            continue;
                        };
                        let Some((target, parameters)) = build_builtin_operator_call(
                            check_module,
                            environment.as_ref(),
                            kind,
                            receiver,
                            argument,
                        ) else {
                            continue;
                        };
                        let resolution =
                            dir::CallResolution::new(target, parameters, Some(return_type));

                        check_module
                            .output
                            .resolutions
                            .set_call_resolution(source, resolution);
                    }
                    OperatorResolution::Method {
                        source,
                        symbol,
                        receiver,
                        function,
                    } => {
                        let Some(receiver) =
                            check_module.commit_variable_type(environment.as_ref(), receiver)
                        else {
                            continue;
                        };
                        let Some(parameters) = check_module.commit_function_parameter_type_ids(
                            environment.as_ref(),
                            &function.parameters,
                        ) else {
                            continue;
                        };
                        let return_type = function.return_type.and_then(|ty| {
                            check_module.commit_variable_type(environment.as_ref(), ty)
                        });
                        let candidate = dir::CallCandidate {
                            receiver: Some(receiver),
                            symbol,
                            instance: None,
                        };
                        let target = dir::CallTarget::Symbol(candidate.clone());
                        let resolution = dir::CallResolution::new(target, parameters, return_type);

                        check_module
                            .output
                            .resolutions
                            .set_call_resolution(source, resolution);

                        let target = dir::MemberTarget::Symbol(dir::MemberCandidate {
                            receiver: Some(receiver),
                            symbol,
                            instance: None,
                        });
                        let resolution = dir::MemberResolution::new(Some(receiver), target);

                        check_module
                            .output
                            .resolutions
                            .set_member_resolution(source, resolution);
                    }
                }
            }
        }

        Ok(())
    }
}

impl CheckModuleState {
    /// Commit collected name resolutions into the checked resolution table.
    pub(super) fn commit_name_resolution_table(&mut self) {
        let names = std::mem::take(&mut self.decisions.name);

        // write lexical resolutions recorded by walk
        for (source, resolution) in names {
            self.output
                .resolutions
                .set_name_resolution(source, resolution);
        }
    }

    /// Commit collected receiver resolutions into the checked resolution table.
    pub(super) fn commit_receiver_resolution_table(&mut self, environment: &GlobalEnvironment) {
        let receivers = std::mem::take(&mut self.decisions.receiver);

        // write contextual receiver resolutions
        for receiver in receivers.into_values() {
            let ty = self.commit_variable_type(environment, receiver.ty);
            let resolution = dir::ReceiverResolution {
                kind: receiver.kind,
                owner: receiver.owner,
                ty,
            };

            self.output
                .resolutions
                .set_receiver_resolution(receiver.source, resolution);
        }
    }

    /// Commit member outcomes into the checked resolution table.
    pub(super) fn commit_member_resolution_table(&mut self, environment: &GlobalEnvironment) {
        let members = self
            .decisions
            .member
            .values()
            .filter_map(|outcome| match outcome {
                MemberOutcome::Resolved(member) => Some(member),
                MemberOutcome::Rejected(_) => None,
            })
            .cloned()
            .collect::<Vec<_>>();

        // write member outcomes recorded by solve
        for member in members {
            if self
                .output
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

            self.output
                .resolutions
                .set_member_resolution(member.source, resolution);
        }
    }
}

/// Build one call target and its reusable symbol candidate.
fn build_call_target(
    check_module: &mut CheckModuleState,
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
            let receiver = receiver
                .and_then(|receiver| check_module.commit_variable_type(environment, receiver));
            let instance = instance.as_ref().and_then(|instance| {
                check_module.commit_generic_instance(environment, call.source, instance)
            });
            let candidate = dir::CallCandidate {
                receiver,
                symbol: *symbol,
                instance,
            };

            (dir::CallTarget::Symbol(candidate.clone()), Some(candidate))
        }
        CallResolutionTarget::Construct { symbol, instance } => {
            let instance = instance.as_ref().and_then(|instance| {
                check_module.commit_generic_instance(environment, call.source, instance)
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
    check_module: &mut CheckModuleState,
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
            let argument = check_module.commit_variable_type(environment, argument)?;
            let target = dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator });

            Some((target, vec![receiver, argument]))
        }
    }
}
