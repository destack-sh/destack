use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::infer::InferMode;
use crate::check::{
    Answer, BodyState, CandidateOutcome, Cause, CauseId, CauseKind, CheckFailure, CheckOutcome,
    Constraint, Expectation, MemoryRank, Origin, ReceiverSteps, Relation, TypeArgumentInference,
    TypeSubstitution, Value, ValueUse, answer,
};

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
    /// Runtime coercions selected for the supplied arguments.
    pub(in crate::check) coercions: SmallVec<[(dir::GlobalNodeIdAny, dir::Coercion); 4]>,
    /// The greatest memory conversion required by one argument.
    pub(in crate::check) rank: MemoryRank,
}

/// Conversion selected for one callable argument.
struct ArgumentConversion {
    /// The required runtime coercion.
    coercion: Option<dir::Coercion>,
    /// The overload rank of the accepted memory relation.
    rank: MemoryRank,
}

/// Result of matching one callable signature.
pub(in crate::check) enum SignatureMatch {
    /// The invocation satisfies the selected signature.
    Selected(SignatureSelection),
    /// The selected signature rejects one invocation constraint.
    Invalid {
        /// The selected signature.
        selection: SignatureSelection,
        /// The rejected invocation constraint.
        rejection: SignatureRejection,
    },
    /// The selected signature accepts the arguments but not the expected result.
    ReturnMismatch(SignatureSelection),
    /// No coherent signature can be instantiated for the invocation.
    Inapplicable(SignatureRejection),
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
    /// Return a call resolution for one selected declaration-backed member.
    pub(in crate::check) fn member_call(
        &self,
        mut receiver: dir::MemberReceiver,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
        arguments: Vec<dir::ArgumentBinding>,
    ) -> dir::Call {
        if let Some(steps) = &self.receiver_steps {
            receiver
                .adjusted_mut()
                .adjustments
                .extend(steps.iter().cloned());
        }
        let generic_arguments = self.generic_arguments.clone();

        let target = match receiver {
            dir::MemberReceiver::Direct(receiver) => dir::CallTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: Some(receiver),
                    generic_scope: Some(owner),
                    symbol,
                    generic_arguments,
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            dir::MemberReceiver::Dynamic(dispatch) => dir::CallTarget::Dynamic {
                dispatch,
                function: dir::DynamicFunction::Symbol(symbol),
                generic_arguments,
            },
        };

        dir::Call {
            target,
            callable_type: self.callable,
            arguments,
            return_type: self.return_type,
        }
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
    /// The value role of this invocation argument.
    pub(in crate::check) use_: ValueUse,
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
    /// One invocation type relation failed.
    Mismatch {
        /// Why the rejected relation exists.
        cause: CauseId,
        /// The rejected relation.
        relation: Relation,
        /// The value role when this is an authored value conversion.
        use_: Option<ValueUse>,
        /// The supplied type.
        source: dir::GlobalTypeId,
        /// The required type.
        target: dir::GlobalTypeId,
        /// The precise reason the relation failed.
        failure: CheckFailure,
    },
    /// Receiver type not assignable to the declared this parameter.
    Receiver {
        /// Supplied receiver type.
        source: dir::GlobalTypeId,
        /// Declared this parameter type.
        target: dir::GlobalTypeId,
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
    /// Return the reduced signature type of one callable type.
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
        origin: Origin,
        source: ModuleId,
        target: ModuleId,
        signature: &dir::FunctionSignatureType,
        substitution: &TypeSubstitution,
        receiver: Option<dir::GlobalTypeId>,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let this_parameter = match signature.this_parameter {
            Some(this_parameter) => {
                let this_parameter = self.substitute_type(this_parameter, substitution)?;

                Some(this_parameter)
            }
            None => None,
        };
        let mut parameters = Vec::new();
        for parameter in self
            .signature_parameters(source, signature.parameters)?
            .to_vec()
        {
            let ty = answer!(self.instantiate_parameter_type(
                origin,
                target,
                parameter.ty,
                substitution,
                receiver,
            )?);
            parameters.push(dir::FunctionParameterType {
                ty,
                is_optional: parameter.is_optional,
                is_rest: parameter.is_rest,
            });
        }
        let parameters = self.intern_parameters(&parameters)?;

        let signature = self.intern_signature(dir::FunctionSignatureType {
            asynchrony: signature.asynchrony,
            template: None,
            this_parameter,
            parameters,
            return_type: Some(return_type),
            is_generator: signature.is_generator,
        })?;

        Ok(Answer::Ready(signature))
    }

    /// Instantiate one selected parameter type.
    fn instantiate_parameter_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        parameter: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let parameter = self.substitute_type(parameter, substitution)?;
        let parameter = self.erase_inference_barriers(module, parameter)?;
        let parameter = answer!(self.receiver_relative_type(origin, receiver, parameter)?);

        Ok(Answer::Ready(parameter))
    }

    /// Attempt one callable candidate without recording a decision.
    pub(in crate::check) fn attempt_callable(
        &mut self,
        origin: Origin,
        function_type: dir::GlobalTypeId,
        owner: Option<dir::GlobalSymbolId>,
        receiver: Option<Value>,
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[CallableArgument],
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<SignatureMatch>> {
        let module = origin.module();

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
                    expectation,
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
                    expectation,
                );
            }
            _ => {
                return Ok(Answer::Ready(SignatureMatch::Inapplicable(
                    SignatureRejection::Inapplicable,
                )));
            }
        };
        let return_type = function.return_type;

        self.constrain_signature(
            origin,
            module,
            function_type.module_id,
            owner,
            carried,
            type_arguments,
            &function,
            return_type,
            receiver,
            arguments,
            expectation,
        )
    }

    /// Match one function signature candidate.
    pub(in crate::check) fn match_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        signature_module: ModuleId,
        owner: Option<dir::GlobalSymbolId>,
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        receiver: Option<dir::GlobalTypeId>,
        arguments: &[CallableArgument],
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<SignatureMatch>> {
        let receiver = receiver.map(|ty| Value { ty, place: None });

        self.constrain_signature(
            origin,
            module,
            signature_module,
            owner,
            carried,
            type_arguments,
            function,
            function_return,
            receiver,
            arguments,
            expectation,
        )
    }

    /// Constrain one function signature candidate without fulfilling residual constraints.
    fn constrain_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        signature_module: ModuleId,
        owner: Option<dir::GlobalSymbolId>,
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        receiver: Option<Value>,
        arguments: &[CallableArgument],
        expectation: Option<Expectation>,
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

        // bind the receiver before evaluating generic defaults
        let substitution = receiver.map_or_else(TypeSubstitution::default, |receiver| {
            TypeSubstitution::default().with_receiver(receiver.ty)
        });

        // preserve generic bindings already selected by the callee
        let substitution = substitution.with_carried(carried)?;
        let parameters = self.signature_generic_parameters(function)?;
        let inference = match arguments
            .iter()
            .all(|argument| argument.use_ == ValueUse::Comptime)
        {
            true => TypeArgumentInference::Exact,
            false => TypeArgumentInference::Callable {
                parameters: &signature_parameters,
                return_type: function_return,
            },
        };
        let substitution = answer!(self.instantiate_parameters(
            origin,
            &parameters,
            type_arguments,
            substitution,
            inference,
        )?);
        let Some(mut substitution) = substitution else {
            return Ok(Answer::Ready(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            )));
        };

        // bind owner parameters that member lookup did not bind
        if let Some(owner) = owner
            && let Some(template) = self.symbol_template(owner)?
        {
            let mut parameters = self.generic_template_parameters(template)?;
            parameters.extend(self.owner_template_parameters(template)?);
            let opened = answer!(self.instantiate_parameters(
                origin,
                &parameters,
                &[],
                substitution,
                inference,
            )?);
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
        let mut coercions = SmallVec::new();
        let mut rank = MemoryRank::Exact;
        let mut blockers = SmallVec::new();

        // constrain every invocation relation before settling candidate inference
        'invocation: {
            // relate the implicit receiver before explicit arguments
            if let (Some(receiver), Some(this_parameter)) = (receiver, function.this_parameter) {
                let receiver_substitution = substitution.clone().with_receiver(receiver.ty);
                let this_parameter =
                    self.substitute_type(this_parameter, &receiver_substitution)?;
                match self.constrain_receiver_argument(origin, receiver, this_parameter)? {
                    Answer::Ready(Some(steps)) => receiver_steps = Some(steps),
                    Answer::Ready(None) => {
                        rejection = Some(SignatureRejection::Receiver {
                            source: receiver.ty,
                            target: this_parameter,
                        });

                        break 'invocation;
                    }
                    Answer::Pending(pending) => blockers.extend(pending),
                }
            }

            // apply the contextual result type before contextualizing arguments
            if let (Some(return_type), Some(expectation)) = (function_return, expectation) {
                let return_type = self.substitute_type(return_type, &substitution)?;
                let return_type = answer!(self.receiver_relative_type(
                    origin,
                    receiver.map(|receiver| receiver.ty),
                    return_type,
                )?);
                let site = self.node_site(self.origin_source(origin)?)?;
                let converted = self.convert_value(
                    site,
                    expectation.cause,
                    expectation.relation,
                    Value {
                        ty: return_type,
                        place: None,
                    },
                    expectation.target,
                    expectation.use_,
                    expectation.mode,
                )?;
                match converted {
                    Answer::Ready(converted) if !converted.outcome.is_holds() => {
                        is_return_mismatch = true;

                        break 'invocation;
                    }
                    Answer::Ready(_) => {}
                    Answer::Pending(pending) => blockers.extend(pending),
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
                let parameter_type = self.substitute_type(parameter.ty, &substitution)?;
                // receiver placement resolves relative member parameters
                let parameter_type = answer!(self.receiver_relative_type(
                    origin,
                    receiver.map(|receiver| receiver.ty),
                    parameter_type,
                )?);
                let parameter_type = self.settled_root(parameter_type)?;
                argument_parameters.push((index, argument, parameter_type));
            }

            // collect the callable and owner template constraints
            let mut bounds = SmallVec::<[Constraint; 4]>::new();
            if let Some(template) = function.template {
                bounds.extend(self.substitute_application_constraints(
                    origin,
                    template,
                    &substitution,
                )?);
            }
            if let Some(owner) = owner
                && let Some(template) = self.symbol_template(owner)?
            {
                bounds.extend(self.substitute_application_constraints(
                    origin,
                    template,
                    &substitution,
                )?);
            }

            // prove obligations before matching arguments
            for bound in bounds {
                let outcome = self.check_type_constraint(
                    bound.origin,
                    bound.cause,
                    bound.relation,
                    bound.source,
                    bound.target,
                )?;
                match outcome {
                    Answer::Ready(CheckOutcome::Fails(failure)) => {
                        rejection = Some(SignatureRejection::Mismatch {
                            cause: bound.cause,
                            relation: bound.relation,
                            use_: None,
                            source: bound.source,
                            target: bound.target,
                            failure,
                        });

                        break 'invocation;
                    }
                    Answer::Ready(CheckOutcome::Holds) => {}
                    Answer::Pending(pending) => blockers.extend(pending),
                }
            }

            // match arguments against substituted parameter types
            for (index, argument, parameter_type) in argument_parameters {
                let conversion =
                    self.match_signature_argument(origin, index, argument, parameter_type)?;
                match conversion {
                    Answer::Ready(Ok(conversion)) => {
                        rank = rank.max(conversion.rank);
                        if let Some(coercion) = conversion.coercion {
                            coercions.push((argument.source, coercion));
                        }
                    }
                    Answer::Ready(Err(failure)) => {
                        rejection = Some(failure);

                        break 'invocation;
                    }
                    Answer::Pending(pending) => blockers.extend(pending),
                }
            }
        }

        // settle the complete candidate relation graph before selecting its result
        if rejection.is_none() && !is_return_mismatch && !blockers.is_empty() {
            return Ok(Answer::Pending(blockers));
        }

        let mut selection = answer!(self.signature_selection(
            origin,
            module,
            signature_module,
            function,
            function_return,
            &substitution,
            receiver.map(|receiver| receiver.ty),
            receiver_steps,
        )?);
        selection.coercions = coercions;
        selection.rank = rank;

        let matched = match rejection {
            Some(rejection) => SignatureMatch::Invalid {
                selection,
                rejection,
            },
            None if is_return_mismatch => SignatureMatch::ReturnMismatch(selection),
            None => SignatureMatch::Selected(selection),
        };

        Ok(Answer::Ready(matched))
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

        self.place_relative_type(origin, place, ty)
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
            Some(return_type) => self.substitute_type(return_type, substitution)?,
            None => self.intern_type(dir::Type::Void)?,
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
            let ty = answer!(self.instantiate_parameter_type(
                origin,
                origin.module(),
                parameter.ty,
                substitution,
                receiver,
            )?);
            parameters.push(dir::FunctionParameterType {
                ty,
                is_optional: parameter.is_optional,
                is_rest: parameter.is_rest,
            });
        }
        let arguments = self.settled_argument_bindings(&substitution.bindings)?;
        let function_type = answer!(self.instantiate_signature_type(
            origin,
            signature_module,
            module,
            function,
            substitution,
            receiver,
            return_type,
        )?);

        Ok(Answer::Ready(SignatureSelection {
            callable: function_type,
            parameters,
            return_type,
            generic_arguments: arguments.to_vec(),
            receiver_steps,
            coercions: SmallVec::new(),
            rank: MemoryRank::Exact,
        }))
    }

    /// Match one supplied argument against one substituted parameter type.
    fn match_signature_argument(
        &mut self,
        origin: Origin,
        index: usize,
        argument: CallableArgument,
        parameter_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Result<ArgumentConversion, SignatureRejection>>> {
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
        let mode = self.contextual_literal_mode(parameter_type, InferMode::Widen)?;
        // apply target-directed syntax before converting the resulting value
        let ty = match argument.ty {
            Some(ty) => ty,
            None => {
                let site = self.node_site(source)?;
                let expectation = Expectation {
                    target: parameter_type,
                    relation,
                    cause,
                    use_: argument.use_,
                    mode,
                };
                let check = answer!(self.check_node_target(site, expectation)?);
                let ty = check.source;

                if let CheckOutcome::Fails(failure) = check.outcome {
                    let rejection = SignatureRejection::Mismatch {
                        cause,
                        relation,
                        use_: Some(argument.use_),
                        source: ty,
                        target: check.target,
                        failure,
                    };

                    return Ok(Answer::Ready(Err(rejection)));
                }

                ty
            }
        };

        // select the complete runtime conversion for this candidate
        let site = self.node_site(source)?;
        let value = answer!(self.expression_value(site, ty)?);
        let conversion = answer!(self.convert_value(
            site,
            cause,
            relation,
            value,
            parameter_type,
            argument.use_,
            mode,
        )?);
        if let CheckOutcome::Fails(failure) = conversion.outcome {
            let rejection = SignatureRejection::Mismatch {
                cause,
                relation,
                use_: Some(argument.use_),
                source: ty,
                target: conversion.target,
                failure,
            };

            return Ok(Answer::Ready(Err(rejection)));
        }

        let rank = answer!(self.memory_rank(origin, ty, conversion.target)?);
        let coercion = conversion.coercion.map(|coercion| *coercion);

        Ok(Answer::Ready(Ok(ArgumentConversion { coercion, rank })))
    }
}
