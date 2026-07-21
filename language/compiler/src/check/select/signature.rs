use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::infer::InferMode;
use crate::check::{
    Answer, BodyState, CandidateOutcome, Cause, CauseKind, CheckOutcome, Constraint,
    ConstraintState, Dependency, Expectation, MemoryRank, Origin, PlaceUse, ReceiverSteps,
    Relation, TypeConstraint, TypeSubstitution, ValueCheck, ValueSource, ValueUse, VariableDomain,
    answer,
};
use crate::{CompilerError, CompilerResult};

/// Callable signature accepted for an invocation.
pub(in crate::check) struct SignatureSelection {
    /// The callable type after substitution.
    pub(in crate::check) callable: dir::GlobalTypeId,
    /// The parameter types after substitution.
    pub(in crate::check) parameters: SmallVec<[dir::FunctionParameterType; 4]>,
    /// The return type after substitution.
    pub(in crate::check) return_type: dir::GlobalTypeId,
    /// The solved generic argument bindings.
    pub(in crate::check) generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The projection steps when the declared this adjusted the receiver.
    pub(in crate::check) receiver_steps: Option<ReceiverSteps>,
}

/// Result of matching one callable signature.
pub(in crate::check) enum SignatureMatch {
    /// The invocation satisfies the selected signature.
    Selected(SignatureSelection),
    /// The selected signature rejects one invocation judgment.
    Invalid {
        /// The selected signature.
        selection: SignatureSelection,
        /// The rejected invocation judgment.
        rejection: SignatureRejection,
        /// The inference variables owned by the rejected invocation.
        variables: VariableDomain,
    },
    /// The selected return type does not satisfy its surrounding context.
    ReturnMismatch(SignatureSelection),
    /// No coherent signature can be instantiated for the invocation.
    Inapplicable(SignatureRejection),
}

/// Result of one invocation constraint judgment.
enum InvocationJudgment {
    /// The constraint holds.
    Holds {
        /// The completed constraint.
        constraint: Constraint,
        /// The concrete target selected by a value check.
        value_target: Option<dir::GlobalTypeId>,
    },
    /// The constraint awaits solver progress.
    Pending(Constraint),
    /// The constraint rejects the invocation.
    Rejects(SignatureRejection),
}

impl InvocationJudgment {
    /// Separate a surviving constraint from an invocation rejection.
    fn into_constraint(
        self,
    ) -> Result<(Constraint, ConstraintState, Option<dir::GlobalTypeId>), SignatureRejection> {
        match self {
            Self::Holds {
                constraint,
                value_target,
            } => Ok((constraint, ConstraintState::Holds, value_target)),
            Self::Pending(constraint) => Ok((constraint, ConstraintState::Pending, None)),
            Self::Rejects(rejection) => Err(rejection),
        }
    }
}

impl SignatureMatch {
    /// Convert this match into an overload candidate outcome.
    pub(in crate::check) fn into_candidate(
        self,
    ) -> CandidateOutcome<SignatureSelection, SignatureRejection> {
        match self {
            Self::Selected(selection) => CandidateOutcome::Accepted(selection),
            Self::Invalid { rejection, .. } | Self::Inapplicable(rejection) => {
                CandidateOutcome::Rejected(rejection)
            }
            Self::ReturnMismatch(_) => CandidateOutcome::Rejected(SignatureRejection::Inapplicable),
        }
    }
}

impl SignatureSelection {
    /// Return the greatest memory rank required by the accepted arguments.
    pub(in crate::check) fn memory_rank(
        &self,
        origin: Origin,
        arguments: &[CallableArgument],
        state: &mut BodyState<'_, '_>,
    ) -> CompilerResult<Answer<MemoryRank>> {
        let mut rank = MemoryRank::Exact;

        for (index, argument) in arguments.iter().copied().enumerate() {
            let parameter = self
                .parameters
                .get(index)
                .or_else(|| self.parameters.last().filter(|parameter| parameter.is_rest));
            let Some(parameter) = parameter else {
                return Err(CompilerError::Internal {
                    message: format!("accepted signature has no parameter for argument {index}"),
                });
            };
            let argument = match argument.ty {
                Some(ty) => ty,
                None => {
                    let site = state.node_site(argument.source)?;

                    answer!(state.node_type_at(site)?)
                }
            };
            let required = state.memory_rank(origin, argument, parameter.ty)?;
            rank = rank.max(required);
        }

        Ok(Answer::Ready(rank))
    }
}

/// Argument matched against one callable signature parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct CallableArgument {
    /// Source node used for origins and diagnostics.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// Known argument type, or none when the source expression must be checked.
    pub(in crate::check) ty: Option<dir::GlobalTypeId>,
    /// Relation selected from the authored argument expression.
    pub(in crate::check) relation: Relation,
}

/// Reason one callable signature rejected an invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum SignatureRejection {
    /// Candidate cannot accept this invocation.
    Inapplicable,
    /// Argument count outside the accepted range.
    Arity {
        /// Required positional argument count.
        required: usize,
        /// Total declared parameter count.
        total: usize,
        /// Whether the signature has a rest parameter.
        has_rest: bool,
        /// Supplied argument count.
        supplied: usize,
    },
    /// Argument type not assignable to the parameter type.
    Argument {
        /// Argument index in source order.
        index: usize,
        /// Relation selected for the argument expression.
        relation: Relation,
        /// Supplied argument type.
        source: dir::GlobalTypeId,
        /// Parameter type.
        target: dir::GlobalTypeId,
    },
    /// Generic argument does not satisfy its parameter bound.
    Bound {
        /// Source occurrence that induced or supplied the argument.
        source_node: dir::GlobalNodeIdAny,
        /// Applied argument type.
        source: dir::GlobalTypeId,
        /// Required parameter bound.
        target: dir::GlobalTypeId,
    },
    /// Receiver type not assignable to the declared this parameter.
    Receiver {
        /// Supplied receiver type.
        source: dir::GlobalTypeId,
        /// Declared this parameter type.
        target: dir::GlobalTypeId,
    },
    /// Generic argument lacks writable index support required by its bound.
    WritableIndex {
        /// Source occurrence that induced or supplied the argument.
        source_node: dir::GlobalNodeIdAny,
        /// Applied argument type.
        source: dir::GlobalTypeId,
        /// Required index key type.
        key: dir::GlobalTypeId,
        /// Required index value type.
        value: dir::GlobalTypeId,
    },
}

impl SignatureRejection {
    /// Return whether this rejection identifies a precise call failure.
    pub(in crate::check) fn is_precise(&self) -> bool {
        !matches!(self, Self::Inapplicable)
    }
}

#[allow(clippy::too_many_arguments)]
impl BodyState<'_, '_> {
    /// Return the reduced signature type carried by one callable type.
    pub(in crate::check) fn callable_signature_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<(dir::GlobalTypeId, dir::FunctionSignatureType)>>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);
        let signature = match self.ty(ty)? {
            dir::Type::FunctionSignature(signature) => {
                Some((ty, self.type_signature(ty.module_id, signature)?))
            }
            dir::Type::Function(function) => {
                return self.callable_signature_type(origin, function.signature);
            }
            dir::Type::FunctionPointer(function) => {
                return self.callable_signature_type(origin, function.signature);
            }
            _ => None,
        };

        Ok(Answer::Ready(signature))
    }

    /// Create one substituted function signature type.
    fn instantiate_signature_type(
        &mut self,
        source: ModuleId,
        target: ModuleId,
        signature: &dir::FunctionSignatureType,
        substitution: &TypeSubstitution,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let this_parameter = match signature.this_parameter {
            Some(this_parameter) => {
                let this_parameter = self.substitute_type(target, this_parameter, substitution)?;

                Some(this_parameter)
            }
            None => None,
        };
        let parameters = self
            .signature_parameters(source, signature.parameters)?
            .to_vec()
            .iter()
            .map(|parameter| {
                let ty = self.substitute_type(target, parameter.ty, substitution)?;
                let ty = self.erase_inference_barriers(target, ty)?;

                Ok(dir::FunctionParameterType {
                    ty,
                    is_optional: parameter.is_optional,
                    is_rest: parameter.is_rest,
                })
            })
            .collect::<CompilerResult<Vec<_>>>()?;
        let parameters = self.intern_parameters(target, &parameters)?;

        self.intern_signature(
            target,
            dir::FunctionSignatureType {
                asynchrony: signature.asynchrony,
                template: None,
                this_parameter,
                parameters,
                return_type: Some(return_type),
                is_generator: signature.is_generator,
            },
        )
    }

    /// Attempt one callable candidate without recording a decision.
    pub(in crate::check) fn attempt_callable(
        &mut self,
        origin: Origin,
        function_type: dir::GlobalTypeId,
        owner: Option<dir::GlobalSymbolId>,
        receiver: Option<dir::GlobalTypeId>,
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[CallableArgument],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<SignatureMatch>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // reduce the callable shape before selecting a signature
        let function_type = answer!(self.reduce_type_head(origin, function_type)?);
        let function = match self.ty(function_type)? {
            dir::Type::FunctionSignature(function) => {
                self.type_signature(function_type.module_id, function)?
            }
            dir::Type::Function(function) => {
                let function = function.signature;

                return self.attempt_callable(
                    origin,
                    function,
                    owner,
                    receiver,
                    carried,
                    type_arguments,
                    arguments,
                    expected_return,
                );
            }
            dir::Type::FunctionPointer(function) => {
                let function = function.signature;

                return self.attempt_callable(
                    origin,
                    function,
                    owner,
                    receiver,
                    carried,
                    type_arguments,
                    arguments,
                    expected_return,
                );
            }
            _ => {
                return Ok(Answer::Ready(SignatureMatch::Inapplicable(
                    SignatureRejection::Inapplicable,
                )));
            }
        };
        let return_type = function.return_type;

        let generic_parameters = self.signature_generic_parameters(&function)?;
        self.match_signature(
            origin,
            module,
            function_type.module_id,
            source,
            &generic_parameters,
            owner,
            carried,
            type_arguments,
            &function,
            return_type,
            receiver,
            arguments,
            expected_return,
        )
    }

    /// Match one function signature candidate.
    pub(in crate::check) fn match_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        signature_module: ModuleId,
        source: dir::LocalNodeIdAny,
        generic_parameters: &[dir::GlobalGenericParameterId],
        owner: Option<dir::GlobalSymbolId>,
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        receiver: Option<dir::GlobalTypeId>,
        arguments: &[CallableArgument],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<SignatureMatch>> {
        // reject arities the signature cannot accept
        let signature_parameters = self
            .signature_parameters(signature_module, function.parameters)?
            .to_vec();
        let required = signature_parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .count();
        let has_rest = signature_parameters
            .iter()
            .any(|parameter| parameter.is_rest);
        let parameter_count = signature_parameters.len();
        let supplied_count = arguments.len();
        if supplied_count < required || (!has_rest && supplied_count > parameter_count) {
            let rejection = SignatureRejection::Arity {
                required,
                total: parameter_count,
                has_rest,
                supplied: supplied_count,
            };

            return Ok(Answer::Ready(SignatureMatch::Inapplicable(rejection)));
        }

        // isolate variables allocated while matching this candidate
        let inference_variables = VariableDomain::after(self.check.solver.variable_count());

        // bind the receiver before evaluating generic defaults
        let substitution = receiver.map_or_else(TypeSubstitution::default, |receiver| {
            TypeSubstitution::default().with_receiver(receiver)
        });

        // instantiate the signature's generic parameters
        let substitution = match generic_parameters {
            [_, ..] => {
                let substitution = self.instantiate_parameter_arguments(
                    origin,
                    generic_parameters,
                    type_arguments,
                    substitution,
                )?;
                match substitution {
                    Some(substitution) => substitution,
                    None => {
                        return Ok(Answer::Ready(SignatureMatch::Inapplicable(
                            SignatureRejection::Inapplicable,
                        )));
                    }
                }
            }
            [] if type_arguments.is_empty() => substitution,
            [] => {
                return Ok(Answer::Ready(SignatureMatch::Inapplicable(
                    SignatureRejection::Inapplicable,
                )));
            }
        };

        // carry owner arguments before evaluating owner defaults
        let mut substitution = substitution.with_carried(carried);

        // bind owner parameters that member lookup did not bind
        let mut unbound = SmallVec::<[dir::GlobalGenericParameterId; 2]>::new();
        if let Some(owner) = owner
            && let Some(template) = self.symbol_template(owner)?
        {
            let mut parameters = self.generic_template_parameters(template);
            parameters.extend(self.owner_template_parameters(template)?);
            for parameter in parameters {
                let determined = carried.iter().any(|binding| binding.parameter == parameter);
                if !determined {
                    unbound.push(parameter);
                }
            }
        }
        if !unbound.is_empty() {
            let opened =
                self.instantiate_parameter_arguments(origin, &unbound, &[], substitution)?;
            let Some(opened) = opened else {
                return Ok(Answer::Ready(SignatureMatch::Inapplicable(
                    SignatureRejection::Inapplicable,
                )));
            };
            substitution = opened;
        }

        let mut rejection = None;
        let mut is_return_mismatch = false;
        let mut receiver_steps = None;
        let mut constraints = Vec::new();

        // judge the invocation in source order, stopping at its first failure
        'invocation: {
            // relate the implicit receiver before explicit arguments
            if let (Some(receiver), Some(this_parameter)) = (receiver, function.this_parameter) {
                let receiver_substitution = substitution.clone().with_receiver(receiver);
                let this_parameter =
                    self.substitute_type(origin.module(), this_parameter, &receiver_substitution)?;
                match self.constrain_receiver_argument(origin, module, receiver, this_parameter)? {
                    Answer::Ready(Some(steps)) => receiver_steps = Some(steps),
                    Answer::Ready(None) => {
                        rejection = Some(SignatureRejection::Receiver {
                            source: receiver,
                            target: this_parameter,
                        });

                        break 'invocation;
                    }
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                }
            }

            // substitute parameter types once for candidate inference
            let mut argument_parameters =
                SmallVec::<[(usize, CallableArgument, dir::GlobalTypeId); 4]>::new();
            for (index, argument) in arguments.iter().copied().enumerate() {
                let parameter = signature_parameters
                    .get(index)
                    .or_else(|| signature_parameters.last());
                let Some(parameter) = parameter else {
                    return Ok(Answer::Ready(SignatureMatch::Inapplicable(
                        SignatureRejection::Inapplicable,
                    )));
                };
                let parameter_type =
                    self.substitute_type(origin.module(), parameter.ty, &substitution)?;
                // receiver placement resolves relative member parameters
                let parameter_type =
                    answer!(self.receiver_relative_type(origin, receiver, parameter_type)?);
                let parameter_type = self.settled_root(parameter_type)?;
                argument_parameters.push((index, argument, parameter_type));
            }

            // materialize const literal arguments before other candidate judgments
            let mut plain_arguments =
                SmallVec::<[(usize, CallableArgument, dir::GlobalTypeId); 4]>::new();
            for (index, argument, parameter_type) in argument_parameters {
                let is_const = argument.ty.is_none()
                    && self.uses_const_argument_inference(parameter_type, &substitution);
                if !is_const {
                    plain_arguments.push((index, argument, parameter_type));

                    continue;
                }

                let judgment = answer!(self.match_signature_argument(
                    origin,
                    &substitution,
                    index,
                    argument,
                    parameter_type,
                )?);
                match judgment.into_constraint() {
                    Ok(constraint) => constraints.push(constraint),
                    Err(failure) => {
                        rejection = Some(failure);

                        break 'invocation;
                    }
                }
            }

            // collect the candidate obligations
            let (bounds, ret) = answer!(self.signature_bounds(
                origin,
                module,
                source,
                generic_parameters,
                type_arguments,
                function,
                function_return,
                expected_return,
                receiver,
                &substitution,
            )?);

            // prove obligations before matching arguments
            for bound in bounds {
                let source_node = source.into_global(module);
                let judgment = answer!(self.match_signature_bound(origin, source_node, bound)?);
                match judgment.into_constraint() {
                    Ok(constraint) => constraints.push(constraint),
                    Err(failure) => {
                        rejection = Some(failure);

                        break 'invocation;
                    }
                }
            }

            // let the surrounding contextual judgment own return mismatches
            if let Some(ret) = ret {
                let state =
                    match self.constrain_type(ret.cause, ret.relation, ret.source, ret.target)? {
                        Answer::Ready(true) => ConstraintState::Holds,
                        Answer::Ready(false) => ConstraintState::Fails,
                        Answer::Pending(_) => ConstraintState::Pending,
                    };
                if state == ConstraintState::Fails {
                    is_return_mismatch = true;

                    break 'invocation;
                }
                constraints.push((Constraint::Type(ret), state, None));
            }

            // match arguments against substituted parameter types
            let mut deferred_arguments =
                SmallVec::<[(usize, CallableArgument, dir::GlobalTypeId); 4]>::new();
            for (index, argument, parameter_type) in plain_arguments {
                if self.contains_inference_barrier(parameter_type)? {
                    deferred_arguments.push((index, argument, parameter_type));

                    continue;
                }

                let judgment = answer!(self.match_signature_argument(
                    origin,
                    &substitution,
                    index,
                    argument,
                    parameter_type,
                )?);
                match judgment.into_constraint() {
                    Ok(constraint) => constraints.push(constraint),
                    Err(failure) => {
                        rejection = Some(failure);

                        break 'invocation;
                    }
                }
            }

            // check NoInfer arguments after generic inference
            for (index, argument, parameter_type) in deferred_arguments {
                let parameter_type =
                    self.erase_inference_barriers(origin.module(), parameter_type)?;

                // barred parameters settle from the unbarred arguments alone
                let barred_variables = self.type_variables(parameter_type)?;
                self.check.solve_variables(inference_variables)?;
                self.check.default_variables(inference_variables)?;
                for variable in barred_variables {
                    if let Some(variable) = self.check.open_variable(variable)? {
                        return Ok(Answer::pending([Dependency::Variable(variable)]));
                    }
                }
                let parameter_type = answer!(self.reduce_type(origin, parameter_type)?);

                let judgment = answer!(self.match_signature_argument(
                    origin,
                    &substitution,
                    index,
                    argument,
                    parameter_type,
                )?);
                match judgment.into_constraint() {
                    Ok(constraint) => constraints.push(constraint),
                    Err(failure) => {
                        rejection = Some(failure);

                        break 'invocation;
                    }
                }
            }
        }

        let selection = answer!(self.signature_selection(
            origin,
            module,
            signature_module,
            function,
            function_return,
            &substitution,
            receiver,
            receiver_steps,
        )?);

        let matched = match rejection {
            Some(rejection) => SignatureMatch::Invalid {
                selection,
                rejection,
                variables: inference_variables,
            },
            None if is_return_mismatch => SignatureMatch::ReturnMismatch(selection),
            None => SignatureMatch::Selected(selection),
        };

        // retain judgments only when the invocation itself is valid
        if !matches!(matched, SignatureMatch::Invalid { .. }) {
            for (constraint, state, value_target) in constraints {
                match state {
                    ConstraintState::Pending => {
                        self.check.push_constraint(constraint);
                    }
                    ConstraintState::Holds => {
                        self.check
                            .record_constraint(constraint, state, value_target)?;
                    }
                    ConstraintState::Fails => {
                        return Err(CompilerError::Internal {
                            message: "matched signature retained a failed constraint".to_string(),
                        });
                    }
                }
            }
        }

        Ok(Answer::Ready(matched))
    }

    /// Collect the bound and predicate obligations plus the expected-return flow.
    fn signature_bounds(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        generic_parameters: &[dir::GlobalGenericParameterId],
        type_arguments: &[dir::GlobalTypeId],
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        expected_return: Option<dir::GlobalTypeId>,
        receiver: Option<dir::GlobalTypeId>,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Answer<(SmallVec<[TypeConstraint; 4]>, Option<TypeConstraint>)>> {
        let mut bounds = SmallVec::new();

        // written generic arguments verify against their declared bounds,
        //  while inferred arguments discharge theirs when their variable solves
        for (parameter, argument) in generic_parameters
            .iter()
            .copied()
            .zip(substitution.arguments.iter().copied())
            .take(type_arguments.len())
        {
            let bound = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint);
            let Some(bound) = bound else {
                continue;
            };
            let bound = self.substitute_type(origin.module(), bound, substitution)?;
            let source_node = source.into_global(module);
            let bound_origin = self.origin_at(origin, source_node)?;
            let cause = self
                .check
                .intern_cause(Cause::root(bound_origin, CauseKind::Bound { parameter }));
            bounds.push(TypeConstraint {
                relation: Relation::Satisfies,
                source: argument,
                target: bound,
                cause,
                invalidated_application: None,
            });
        }

        // template predicates verify with the completed substitution
        for predicate in self.template_predicates(function.template) {
            let left = self.substitute_type(origin.module(), predicate.left, substitution)?;
            let right = self.substitute_type(origin.module(), predicate.right, substitution)?;
            let cause = self
                .check
                .intern_cause(Cause::root(origin, CauseKind::Expression));
            bounds.push(TypeConstraint {
                relation: Relation::Satisfies,
                source: left,
                target: right,
                cause,
                invalidated_application: None,
            });
        }

        // the substituted return flows into the expected return
        let mut ret = None;
        if let (Some(return_type), Some(expected_return)) = (function_return, expected_return) {
            let return_type = self.substitute_type(origin.module(), return_type, substitution)?;
            // receiver placement resolves relative member returns
            let return_type =
                answer!(self.receiver_relative_type(origin, receiver, return_type)?);
            let cause = self
                .check
                .intern_cause(Cause::root(origin, CauseKind::Return { annotation: None }));
            ret = Some(TypeConstraint {
                relation: Relation::Assignable,
                source: return_type,
                target: expected_return,
                cause,
                invalidated_application: None,
            });
        }

        Ok(Answer::Ready((bounds, ret)))
    }

    /// Resolve one relative member type in the receiver's concrete place.
    fn receiver_relative_type(
        &mut self,
        origin: Origin,
        receiver: Option<dir::GlobalTypeId>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let Some(receiver) = receiver else {
            return Ok(Answer::Ready(ty));
        };
        let Some(place) = answer!(self.receiver_projected_place(origin, receiver)?) else {
            return Ok(Answer::Ready(ty));
        };

        Ok(Answer::Ready(self.place_relative_type(origin, place, ty)?))
    }

    /// Build the selected payload for one instantiated signature.
    fn signature_selection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        signature_module: ModuleId,
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        substitution: &TypeSubstitution,
        receiver: Option<dir::GlobalTypeId>,
        receiver_steps: Option<ReceiverSteps>,
    ) -> CompilerResult<Answer<SignatureSelection>> {
        // resolve the substituted return type
        let return_type = match function_return {
            Some(return_type) => {
                self.substitute_type(origin.module(), return_type, substitution)?
            }
            None => self.intern_type(module, dir::Type::Void)?,
        };
        let return_type = match self.receiver_relative_type(origin, receiver, return_type)? {
            Answer::Ready(return_type) => return_type,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let mut parameters = SmallVec::<[_; 4]>::new();
        for parameter in self
            .signature_parameters(signature_module, function.parameters)?
            .to_vec()
        {
            let ty = self.substitute_type(origin.module(), parameter.ty, substitution)?;
            let ty = match self.receiver_relative_type(origin, receiver, ty)? {
                Answer::Ready(ty) => ty,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            let ty = self.settled_root(ty)?;
            parameters.push(dir::FunctionParameterType {
                ty,
                is_optional: parameter.is_optional,
                is_rest: parameter.is_rest,
            });
        }
        let arguments =
            self.generic_argument_bindings(&substitution.parameters, &substitution.arguments)?;
        let function_type = self.instantiate_signature_type(
            signature_module,
            module,
            function,
            substitution,
            return_type,
        )?;

        Ok(Answer::Ready(SignatureSelection {
            callable: function_type,
            parameters,
            return_type,
            generic_arguments: arguments,
            receiver_steps,
        }))
    }

    /// Match one supplied argument against one substituted parameter type.
    fn match_signature_argument(
        &mut self,
        origin: Origin,
        substitution: &TypeSubstitution,
        index: usize,
        argument: CallableArgument,
        parameter_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<InvocationJudgment>> {
        let source = argument.source;
        let call = self.origin_source(origin)?;
        let origin = self.origin_at(origin, source)?;
        let relation = argument.relation;
        let cause = self.check.intern_cause(Cause::root(
            origin,
            CauseKind::Argument {
                call,
                index: index as u32,
            },
        ));
        let constraint = match argument.ty {
            None => Constraint::value(
                relation,
                ValueSource::Node(source),
                parameter_type,
                cause,
                ValueUse::Argument,
            ),
            Some(ty) => Constraint::value(
                relation,
                ValueSource::Type(ty),
                parameter_type,
                cause,
                ValueUse::Argument,
            ),
        };

        // preserve exact literal structure for const literal materialization
        if argument.ty.is_none() && self.uses_const_argument_inference(parameter_type, substitution)
        {
            let site = self.node_site(source)?;
            // const parameters type fresh literal arguments in const mode
            if self.committed_node_type(site.node).is_none() {
                answer!(self.infer_expression(site, PlaceUse::Read, InferMode::Const)?);
            }
            let ty = answer!(self.node_type_at(site)?);
            let ty = answer!(self.const_literal_expression_type(source, ty)?);

            let state = self.check_value_relation(cause, relation, ty, parameter_type)?;

            return self.signature_argument_judgment(
                index,
                argument,
                parameter_type,
                constraint,
                state,
            );
        }

        // check source expressions and relate explicit typed arguments
        let state = match argument.ty {
            None => {
                let site = self.node_site(source)?;
                let expectation = Expectation {
                    target: parameter_type,
                    relation,
                    cause,
                    use_: ValueUse::Argument,
                };
                self.check_node_target(site, expectation)?
            }
            Some(ty) => self.check_value_relation(cause, relation, ty, parameter_type)?,
        };

        self.signature_argument_judgment(index, argument, parameter_type, constraint, state)
    }

    /// Classify one argument constraint judgment.
    fn signature_argument_judgment(
        &mut self,
        index: usize,
        argument: CallableArgument,
        parameter_type: dir::GlobalTypeId,
        constraint: Constraint,
        state: Answer<ValueCheck>,
    ) -> CompilerResult<Answer<InvocationJudgment>> {
        let judgment = match state {
            Answer::Ready(check) if check.outcome == CheckOutcome::Holds => {
                InvocationJudgment::Holds {
                    constraint,
                    value_target: Some(check.target),
                }
            }
            Answer::Pending(_) => InvocationJudgment::Pending(constraint),
            Answer::Ready(_) => {
                let source = answer!(self.signature_argument_type(argument)?);
                let rejection = SignatureRejection::Argument {
                    index,
                    relation: argument.relation,
                    source,
                    target: parameter_type,
                };

                InvocationJudgment::Rejects(rejection)
            }
        };

        Ok(Answer::Ready(judgment))
    }

    /// Judge one substituted generic bound.
    fn match_signature_bound(
        &mut self,
        origin: Origin,
        source_node: dir::GlobalNodeIdAny,
        bound: TypeConstraint,
    ) -> CompilerResult<Answer<InvocationJudgment>> {
        let state = self.constrain_type(bound.cause, bound.relation, bound.source, bound.target)?;
        let judgment = match state {
            Answer::Ready(true) => InvocationJudgment::Holds {
                constraint: Constraint::Type(bound),
                value_target: None,
            },
            Answer::Pending(_) => InvocationJudgment::Pending(Constraint::Type(bound)),
            Answer::Ready(false) => {
                let rejection = self.signature_bound_rejection(
                    origin,
                    source_node,
                    bound.source,
                    bound.target,
                )?;

                InvocationJudgment::Rejects(rejection)
            }
        };

        Ok(Answer::Ready(judgment))
    }

    /// Return the current type of one signature argument.
    fn signature_argument_type(
        &mut self,
        argument: CallableArgument,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        match argument.ty {
            Some(ty) => Ok(Answer::Ready(ty)),
            None => self.node_type(argument.source),
        }
    }

    /// Return one signature rejection for a failed generic bound relation.
    fn signature_bound_rejection(
        &mut self,
        origin: Origin,
        source_node: dir::GlobalNodeIdAny,
        argument: dir::GlobalTypeId,
        bound: dir::GlobalTypeId,
    ) -> CompilerResult<SignatureRejection> {
        if let Some(signature) = self.first_writable_index_signature(origin, bound)? {
            return Ok(SignatureRejection::WritableIndex {
                source_node,
                source: argument,
                key: signature.key_type,
                value: signature.value_type,
            });
        }

        Ok(SignatureRejection::Bound {
            source_node,
            source: argument,
            target: bound,
        })
    }

    /// Return whether one substituted parameter uses const literal materialization.
    ///
    /// Only direct `value: T` positions materialize. TODO #Incomplete: tsc
    /// applies const inference wherever the literal flows into `T`, so
    /// `values: T[]` should freeze its elements without freezing the array.
    fn uses_const_argument_inference(
        &self,
        parameter_type: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
    ) -> bool {
        for (parameter, argument) in substitution
            .parameters
            .iter()
            .copied()
            .zip(substitution.arguments.iter().copied())
        {
            if argument != parameter_type {
                continue;
            }
            let Some(binding) = self.generic_parameter(parameter) else {
                continue;
            };
            if binding.is_const {
                return true;
            }
        }

        false
    }
}
