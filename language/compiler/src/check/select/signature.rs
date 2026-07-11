use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::infer::InferMode;
use crate::check::{
    Answer, BodyState, BoundMode, CandidateOutcome, Constraint, Origin, PlaceUse, ReceiverSteps,
    Relation, TypeSubstitution, ValueUse, answer,
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
}

/// Argument matched against one callable signature parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CallableArgument {
    /// Source expression that can be checked against the parameter.
    Expression(dir::GlobalNodeIdAny),
    /// Source occurrence whose value type is already known.
    Typed {
        /// Source node used for diagnostics.
        source: dir::GlobalNodeIdAny,
        /// Argument type.
        ty: dir::GlobalTypeId,
    },
}

impl CallableArgument {
    /// Return the source node used for origins and diagnostics.
    fn source(self) -> dir::GlobalNodeIdAny {
        match self {
            Self::Expression(source) => source,
            Self::Typed { source, .. } => source,
        }
    }

    /// Return the argument type when it is already known.
    fn ty(self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Expression(_) => None,
            Self::Typed { ty, .. } => Some(ty),
        }
    }
}

/// How one signature candidate is matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CandidatePass {
    /// Decide applicability eagerly for candidate winnowing.
    Winnow,
    /// Commit the winner, queueing undecided work for the solver.
    Confirm,
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
        pass: CandidatePass,
        origin: Origin,
        function_type: dir::GlobalTypeId,
        owner: Option<dir::GlobalSymbolId>,
        receiver: Option<dir::GlobalTypeId>,
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[CallableArgument],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<CandidateOutcome<SignatureSelection, SignatureRejection>>> {
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
                    pass,
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
                    pass,
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
                return Ok(Answer::Ready(CandidateOutcome::Rejected(
                    SignatureRejection::Inapplicable,
                )));
            }
        };
        let return_type = function.return_type;

        let generic_parameters = self.signature_generic_parameters(&function)?;
        self.match_signature(
            pass,
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
        pass: CandidatePass,
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
    ) -> CompilerResult<Answer<CandidateOutcome<SignatureSelection, SignatureRejection>>> {
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

            return Ok(Answer::Ready(CandidateOutcome::Rejected(rejection)));
        }

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
                        return Ok(Answer::Ready(CandidateOutcome::Rejected(
                            SignatureRejection::Inapplicable,
                        )));
                    }
                }
            }
            [] if type_arguments.is_empty() => substitution,
            [] => {
                return Ok(Answer::Ready(CandidateOutcome::Rejected(
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
                return Ok(Answer::Ready(CandidateOutcome::Rejected(
                    SignatureRejection::Inapplicable,
                )));
            };
            substitution = opened;
        }

        // relate the implicit receiver before explicit arguments
        let mut receiver_steps = None;
        if let (Some(receiver), Some(this_parameter)) = (receiver, function.this_parameter) {
            let receiver_substitution = substitution.clone().with_receiver(receiver);
            let this_parameter =
                self.substitute_type(origin.module(), this_parameter, &receiver_substitution)?;
            let steps =
                match self.constrain_receiver_argument(origin, module, receiver, this_parameter)? {
                    Answer::Ready(Some(steps)) => steps,
                    Answer::Ready(None) => {
                        let rejection = SignatureRejection::Receiver {
                            source: receiver,
                            target: this_parameter,
                        };

                        return Ok(Answer::Ready(CandidateOutcome::Rejected(rejection)));
                    }
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };
            receiver_steps = Some(steps);
        }

        // substitute parameter types once for candidate inference
        let mut argument_parameters =
            SmallVec::<[(usize, CallableArgument, dir::GlobalTypeId); 4]>::new();
        for (index, argument) in arguments.iter().copied().enumerate() {
            let parameter = signature_parameters
                .get(index)
                .or_else(|| signature_parameters.last());
            let Some(parameter) = parameter else {
                return Ok(Answer::Ready(CandidateOutcome::Rejected(
                    SignatureRejection::Inapplicable,
                )));
            };
            let parameter_type =
                self.substitute_type(origin.module(), parameter.ty, &substitution)?;
            argument_parameters.push((index, argument, parameter_type));
        }

        // use settled expectations when checking nested argument expressions
        for (_, _, parameter_type) in &mut argument_parameters {
            *parameter_type = self.settled_root(*parameter_type)?;
        }

        // materialize const literal arguments before any queued work, so
        //  confirmation rejects only before its first side effect
        let mut plain_arguments =
            SmallVec::<[(usize, CallableArgument, dir::GlobalTypeId); 4]>::new();
        for (index, argument, parameter_type) in argument_parameters {
            let is_const = matches!(argument, CallableArgument::Expression(_))
                && self.uses_const_argument_inference(parameter_type, &substitution);
            if !is_const {
                plain_arguments.push((index, argument, parameter_type));

                continue;
            }

            let holds =
                self.match_signature_argument(origin, &substitution, argument, parameter_type)?;
            if !answer!(holds) {
                let source = answer!(self.signature_argument_type(argument)?);
                let rejection = SignatureRejection::Argument {
                    index,
                    source,
                    target: parameter_type,
                };

                return Ok(Answer::Ready(CandidateOutcome::Rejected(rejection)));
            }
        }

        // confirmed candidates queue undecided work and accept; expression
        //  arguments are checked by the caller's commit path
        if pass == CandidatePass::Confirm {
            self.queue_signature_bounds(
                origin,
                module,
                source,
                generic_parameters,
                type_arguments,
                function,
                function_return,
                expected_return,
                &substitution,
            )?;

            for (_, argument, parameter_type) in plain_arguments {
                let CallableArgument::Typed { source, ty } = argument else {
                    continue;
                };
                // barred parameters settle from the unbarred arguments alone
                if self.contains_inference_barrier(parameter_type)? {
                    for variable in self.type_variables(parameter_type)? {
                        answer!(self.check.solve_variable(variable, BoundMode::Strong)?);
                    }
                }
                let parameter_type =
                    self.erase_inference_barriers(origin.module(), parameter_type)?;
                let origin = self.origin_at(origin, source)?;
                let origin = self.intern_origin(origin);
                self.push_constraint(Constraint::r#type(
                    Relation::Assignable,
                    ty,
                    parameter_type,
                    origin,
                ));
            }

            return self.accept_signature(
                origin,
                module,
                signature_module,
                function,
                function_return,
                &substitution,
                receiver_steps,
            );
        }

        // check written generic arguments against their declared bounds
        if !generic_parameters.is_empty() {
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
                let bound = self.substitute_type(origin.module(), bound, &substitution)?;

                let source_node = source.into_global(module);
                let bound_origin = self.origin_at(origin, source_node)?;
                if !answer!(self.constrain_type(
                    bound_origin,
                    Relation::Satisfies,
                    argument,
                    bound
                )?) {
                    let rejection =
                        self.signature_bound_rejection(origin, source_node, argument, bound)?;

                    return Ok(Answer::Ready(CandidateOutcome::Rejected(rejection)));
                }
            }
        }

        // prove template predicates with the completed substitution
        for predicate in self.template_predicates(function.template) {
            let left = self.substitute_type(origin.module(), predicate.left, &substitution)?;
            let right = self.substitute_type(origin.module(), predicate.right, &substitution)?;

            if !answer!(self.constrain_type(origin, Relation::Satisfies, left, right)?) {
                let source_node = source.into_global(module);
                let rejection = self.signature_bound_rejection(origin, source_node, left, right)?;

                return Ok(Answer::Ready(CandidateOutcome::Rejected(rejection)));
            }
        }

        // relate expected returns in the same candidate context
        if let (Some(return_type), Some(expected_return)) = (function_return, expected_return) {
            let return_type = self.substitute_type(origin.module(), return_type, &substitution)?;
            if !answer!(self.constrain_type(
                origin,
                Relation::Assignable,
                return_type,
                expected_return
            )?) {
                return Ok(Answer::Ready(CandidateOutcome::Rejected(
                    SignatureRejection::Inapplicable,
                )));
            }
        }

        // match arguments against substituted parameter types
        let mut deferred_arguments =
            SmallVec::<[(usize, CallableArgument, dir::GlobalTypeId); 4]>::new();
        for (index, argument, parameter_type) in plain_arguments {
            if self.contains_inference_barrier(parameter_type)? {
                deferred_arguments.push((index, argument, parameter_type));

                continue;
            }

            let holds =
                self.match_signature_argument(origin, &substitution, argument, parameter_type)?;
            if !answer!(holds) {
                let source = answer!(self.signature_argument_type(argument)?);
                let rejection = SignatureRejection::Argument {
                    index,
                    source,
                    target: parameter_type,
                };

                return Ok(Answer::Ready(CandidateOutcome::Rejected(rejection)));
            }
        }

        // check NoInfer arguments after generic inference
        for (index, argument, parameter_type) in deferred_arguments {
            let parameter_type = self.erase_inference_barriers(origin.module(), parameter_type)?;
            // barred parameters settle from the unbarred arguments alone
            for variable in self.type_variables(parameter_type)? {
                answer!(self.check.solve_variable(variable, BoundMode::Strong)?);
            }
            let parameter_type = answer!(self.reduce_type(origin, parameter_type)?);

            let holds =
                self.match_signature_argument(origin, &substitution, argument, parameter_type)?;
            if !answer!(holds) {
                let source = answer!(self.signature_argument_type(argument)?);
                let rejection = SignatureRejection::Argument {
                    index,
                    source,
                    target: parameter_type,
                };

                return Ok(Answer::Ready(CandidateOutcome::Rejected(rejection)));
            }
        }

        self.accept_signature(
            origin,
            module,
            signature_module,
            function,
            function_return,
            &substitution,
            receiver_steps,
        )
    }

    /// Queue one confirmed candidate's bound and return relations.
    fn queue_signature_bounds(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        generic_parameters: &[dir::GlobalGenericParameterId],
        type_arguments: &[dir::GlobalTypeId],
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        expected_return: Option<dir::GlobalTypeId>,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<()> {
        // written generic arguments verify against their declared bounds
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
            let bound_origin = self.intern_origin(bound_origin);
            self.push_constraint(Constraint::r#type(
                Relation::Satisfies,
                argument,
                bound,
                bound_origin,
            ));
        }

        // template predicates verify with the completed substitution
        for predicate in self.template_predicates(function.template) {
            let left = self.substitute_type(origin.module(), predicate.left, substitution)?;
            let right = self.substitute_type(origin.module(), predicate.right, substitution)?;
            let origin = self.intern_origin(origin);
            self.push_constraint(Constraint::r#type(Relation::Satisfies, left, right, origin));
        }

        // the substituted return flows into the expected return
        if let (Some(return_type), Some(expected_return)) = (function_return, expected_return) {
            let return_type = self.substitute_type(origin.module(), return_type, substitution)?;
            let origin = self.intern_origin(origin);
            self.push_constraint(Constraint::r#type(
                Relation::Assignable,
                return_type,
                expected_return,
                origin,
            ));
        }

        Ok(())
    }

    /// Build the accepted payload for one matched signature.
    fn accept_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        signature_module: ModuleId,
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        substitution: &TypeSubstitution,
        receiver_steps: Option<ReceiverSteps>,
    ) -> CompilerResult<Answer<CandidateOutcome<SignatureSelection, SignatureRejection>>> {
        // resolve the substituted return type
        let return_type = match function_return {
            Some(return_type) => {
                self.substitute_type(origin.module(), return_type, substitution)?
            }
            None => self.intern_type(module, dir::Type::Void)?,
        };
        let parameters = self
            .signature_parameters(signature_module, function.parameters)?
            .to_vec()
            .iter()
            .map(|parameter| {
                let ty = self.substitute_type(origin.module(), parameter.ty, substitution)?;
                let ty = self.settled_root(ty)?;

                Ok(dir::FunctionParameterType {
                    ty,
                    is_optional: parameter.is_optional,
                    is_rest: parameter.is_rest,
                })
            })
            .collect::<CompilerResult<SmallVec<[_; 4]>>>()?;
        let arguments =
            self.generic_argument_bindings(&substitution.parameters, &substitution.arguments)?;
        let function_type = self.instantiate_signature_type(
            signature_module,
            module,
            function,
            substitution,
            return_type,
        )?;

        Ok(Answer::Ready(CandidateOutcome::Accepted(
            SignatureSelection {
                callable: function_type,
                parameters,
                return_type,
                generic_arguments: arguments,
                receiver_steps,
            },
        )))
    }

    /// Match one supplied argument against one substituted parameter type.
    fn match_signature_argument(
        &mut self,
        origin: Origin,
        substitution: &TypeSubstitution,
        argument: CallableArgument,
        parameter_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = argument.source();
        let origin = self.origin_at(origin, source)?;

        // preserve exact literal structure for const literal materialization
        if matches!(argument, CallableArgument::Expression(_))
            && self.uses_const_argument_inference(parameter_type, substitution)
        {
            let site = self.node_site(source)?;
            // const parameters type fresh literal arguments in const mode
            if self.node_type_maybe(site.node).is_none() {
                answer!(self.infer_expression(site, PlaceUse::Read, InferMode::Const)?);
            }
            let ty = answer!(self.node_type_at(site)?);
            let ty = answer!(self.const_literal_expression_type(source, ty)?);

            return self.constrain_type(origin, Relation::Assignable, ty, parameter_type);
        }

        // check source expressions with the parameter as their expected type
        match argument {
            CallableArgument::Expression(_) => self.check_expression_relation(
                origin,
                Relation::Assignable,
                parameter_type,
                Some(ValueUse::Argument),
            ),
            CallableArgument::Typed { ty, .. } => {
                self.constrain_type(origin, Relation::Assignable, ty, parameter_type)
            }
        }
    }

    /// Return the current type of one signature argument.
    fn signature_argument_type(
        &mut self,
        argument: CallableArgument,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        match argument.ty() {
            Some(ty) => Ok(Answer::Ready(ty)),
            None => self.node_type(argument.source()),
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
