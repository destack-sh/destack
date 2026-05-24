use destack_artifact::{DirCheckedComponentEntry, GlobalEnvironment};
use destack_dir as dir;
use destack_source::DiagnosticCollection;

use crate::CompilerResult;
use crate::check::solve::Decision;
use crate::check::{
    CallOutcome, CallResolution, CallResolutionTarget, CheckComponentState, CheckModuleState,
    Constraint, ConstraintOrigin, ConstructOutcome, OperatorOutcome, OperatorResolution,
    OperatorTermKind, TypeRelation, VariableId,
};

impl CheckComponentState<'_> {
    /// Commit solved check state into checked DIR tables and diagnostics.
    pub(in crate::check) fn commit(
        mut self,
    ) -> CompilerResult<(Vec<DirCheckedComponentEntry>, DiagnosticCollection)> {
        let diagnostics = self.commit_diagnostics()?;

        self.commit_coercions()?;
        self.commit_call_resolutions()?;
        self.commit_construct_resolutions()?;
        self.commit_operator_resolutions()?;

        let modules = self.commit_modules()?;

        Ok((modules, diagnostics))
    }

    /// Commit accepted type changes into coercion tables.
    fn commit_coercions(&mut self) -> CompilerResult<()> {
        let constraints = self.constraints();

        // write coercions for accepted value type changes
        for constraint in constraints {
            let Constraint::RelateType {
                relation,
                left,
                right,
                origin: ConstraintOrigin::Node(node),
            } = constraint
            else {
                continue;
            };

            let Some(origin) = Self::coercion_origin(relation) else {
                continue;
            };
            if self.decide_type_relation(relation, left, right)? != Decision::Yes {
                continue;
            }

            let environment = self.environment.clone();
            let check_module = self.module_mut(node.module_id)?;
            let Some(source) = check_module.commit_variable_type(environment.as_ref(), left) else {
                continue;
            };
            let Some(target) = check_module.commit_variable_type(environment.as_ref(), right)
            else {
                continue;
            };
            if source == target {
                continue;
            }

            check_module
                .output
                .coercions
                .set_coercion(node, dir::Coercion::new(source, target, origin));
        }

        Ok(())
    }

    /// Return the coercion origin for one recorded relation.
    fn coercion_origin(relation: TypeRelation) -> Option<dir::CastOrigin> {
        match relation {
            TypeRelation::Assignable => Some(dir::CastOrigin::Implicit),
            TypeRelation::Castable => Some(dir::CastOrigin::Explicit),
            TypeRelation::Equal
            | TypeRelation::Satisfies
            | TypeRelation::Extends
            | TypeRelation::Implements => None,
        }
    }

    /// Commit resolved calls into resolution tables.
    fn commit_call_resolutions(&mut self) -> CompilerResult<()> {
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
                let Some(parameters) = check_module
                    .commit_type_variables(environment.as_ref(), &call.function.parameters)
                else {
                    continue;
                };
                let return_type = call
                    .function
                    .return_type
                    .and_then(|ty| check_module.commit_variable_type(environment.as_ref(), ty));
                let (target, candidate) =
                    Self::commit_call_target(check_module, environment.as_ref(), &call);
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

    /// Commit resolved constructs into resolution tables.
    fn commit_construct_resolutions(&mut self) -> CompilerResult<()> {
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
                let Some(parameters) = check_module
                    .commit_type_variables(environment.as_ref(), &construct.function.parameters)
                else {
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

    /// Commit one call target and its reusable symbol candidate.
    fn commit_call_target(
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

    /// Commit accepted operators into resolution tables.
    fn commit_operator_resolutions(&mut self) -> CompilerResult<()> {
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
                        let Some((target, parameters)) = Self::builtin_operator_call(
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
                        let Some(parameters) = check_module
                            .commit_type_variables(environment.as_ref(), &function.parameters)
                        else {
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

    /// Return the builtin call shape for one operator.
    fn builtin_operator_call(
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
                let target =
                    dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator });

                Some((target, vec![receiver, argument]))
            }
        }
    }

    /// Commit checked DIR tables for every loaded module.
    fn commit_modules(&mut self) -> CompilerResult<Vec<DirCheckedComponentEntry>> {
        let modules = std::mem::take(&mut self.modules);
        let mut entries = Vec::with_capacity(modules.len());

        // commit modules in stable load order
        for (module, check_module) in modules {
            let checked = check_module.commit(self.environment.as_ref());

            entries.push(DirCheckedComponentEntry { module, checked });
        }

        Ok(entries)
    }
}
