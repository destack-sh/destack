use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, GenericPosition, Origin, Relation, TypeSubstitution, answer,
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
        source: String,
        /// Parameter type.
        target: String,
    },
    /// Generic argument does not satisfy its parameter bound.
    Bound {
        /// Source occurrence that induced or supplied the argument.
        source_node: dir::GlobalNodeIdAny,
        /// Applied argument type.
        source: String,
        /// Required parameter bound.
        target: String,
    },
    /// Generic argument lacks writable index support required by its bound.
    WritableIndex {
        /// Source occurrence that induced or supplied the argument.
        source_node: dir::GlobalNodeIdAny,
        /// Applied argument type.
        source: String,
        /// Required index key type.
        key: String,
        /// Required index value type.
        value: String,
    },
}

impl SignatureRejection {
    /// Return whether this rejection identifies a precise call failure.
    pub(in crate::check) fn is_precise(&self) -> bool {
        !matches!(self, Self::Inapplicable)
    }
}

impl CheckState<'_> {
    /// Return the reduced signature type carried by one callable type.
    pub(in crate::check) fn callable_signature_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<(dir::GlobalTypeId, dir::FunctionSignatureType)>>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);
        let signature = match self.ty(ty)? {
            dir::Type::FunctionSignature(signature) => Some((ty, signature)),
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
    /// The source module owns the signature's payload lists; the target
    /// module owns the rebuilt type and must be writable.
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
                let this_parameter = self.resolve_type_variables(target, this_parameter)?;

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
                let ty = self.resolve_type_variables(target, ty)?;
                let ty = self.erase_inference_barriers(target, ty)?;

                Ok(dir::FunctionParameterType {
                    ty,
                    static_parameter: None,
                    is_optional: parameter.is_optional,
                    is_rest: parameter.is_rest,
                })
            })
            .collect::<CompilerResult<Vec<_>>>()?;
        let parameters = self.intern_parameters(target, &parameters)?;

        self.intern_type(
            target,
            dir::Type::FunctionSignature(dir::FunctionSignatureType {
                asynchrony: signature.asynchrony,
                template: None,
                this_parameter,
                parameters,
                return_type: Some(return_type),
                is_generator: signature.is_generator,
            }),
        )
    }

    /// Attempt one callable candidate without recording a decision.
    pub(in crate::check) fn attempt_callable(
        &mut self,
        origin: Origin,
        function_type: dir::GlobalTypeId,
        receiver: Option<dir::GlobalTypeId>,
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[dir::GlobalTypeId],
        argument_sources: &[dir::GlobalNodeIdAny],
    ) -> CompilerResult<Answer<Result<SignatureSelection, SignatureRejection>>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // reduce the callable shape before selecting a signature
        let function_type = answer!(self.reduce_type_head(origin, function_type)?);
        let function = match self.ty(function_type)? {
            dir::Type::FunctionSignature(function) => function,
            dir::Type::Function(function) => {
                let function = function.signature;

                return self.attempt_callable(
                    origin,
                    function,
                    receiver,
                    carried,
                    type_arguments,
                    arguments,
                    argument_sources,
                );
            }
            dir::Type::FunctionPointer(function) => {
                let function = function.signature;

                return self.attempt_callable(
                    origin,
                    function,
                    receiver,
                    carried,
                    type_arguments,
                    arguments,
                    argument_sources,
                );
            }
            _ => {
                return Ok(Answer::Ready(Err(SignatureRejection::Inapplicable)));
            }
        };
        let return_type = function.return_type;

        let generic_parameters = self.signature_generic_parameters(&function)?;
        self.attempt_signature(
            origin,
            module,
            function_type.module_id,
            source,
            &generic_parameters,
            carried,
            type_arguments,
            &function,
            return_type,
            receiver,
            arguments,
            argument_sources,
        )
    }

    /// Attempt one signature candidate.
    pub(in crate::check) fn attempt_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        signature_module: ModuleId,
        source: dir::LocalNodeIdAny,
        generic_parameters: &[dir::GlobalGenericParameterId],
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        receiver: Option<dir::GlobalTypeId>,
        arguments: &[dir::GlobalTypeId],
        argument_sources: &[dir::GlobalNodeIdAny],
    ) -> CompilerResult<Answer<Result<SignatureSelection, SignatureRejection>>> {
        let probe = self.begin_probe();
        let attempt = self.match_signature(
            origin,
            module,
            signature_module,
            source,
            generic_parameters,
            carried,
            type_arguments,
            function,
            function_return,
            receiver,
            arguments,
            argument_sources,
        )?;

        match attempt {
            Answer::Ready(Ok(signature)) => {
                self.commit_probe(probe);

                Ok(Answer::Ready(Ok(signature)))
            }
            Answer::Ready(Err(rejection)) => {
                self.reject_probe(probe);

                Ok(Answer::Ready(Err(rejection)))
            }
            Answer::Pending(blockers) => {
                self.reject_probe(probe);

                // discard blockers owned by the rejected probe
                let blockers = self.live_blockers(blockers);
                Ok(Answer::ready_unless_blocked(
                    Err(SignatureRejection::Inapplicable),
                    blockers,
                ))
            }
        }
    }

    /// Match one function signature inside an active candidate attempt.
    /// The signature module owns the signature payload's interned lists;
    /// the origin module owns the call's source nodes.
    fn match_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        signature_module: ModuleId,
        source: dir::LocalNodeIdAny,
        generic_parameters: &[dir::GlobalGenericParameterId],
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        receiver: Option<dir::GlobalTypeId>,
        arguments: &[dir::GlobalTypeId],
        argument_sources: &[dir::GlobalNodeIdAny],
    ) -> CompilerResult<Answer<Result<SignatureSelection, SignatureRejection>>> {
        // reject arities the signature cannot accept
        let signature_parameters =
            self.signature_parameters(signature_module, function.parameters)?;
        let required = signature_parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .count();
        let has_rest = signature_parameters
            .iter()
            .any(|parameter| parameter.is_rest);
        let parameter_count = signature_parameters.len();
        if arguments.len() < required || (!has_rest && arguments.len() > parameter_count) {
            let rejection = SignatureRejection::Arity {
                required,
                total: parameter_count,
                has_rest,
                supplied: arguments.len(),
            };

            return Ok(Answer::Ready(Err(rejection)));
        }

        // instantiate the signature's generic parameters
        let substitution = match generic_parameters {
            [_, ..] => {
                let substitution = self.instantiate_generic_parameters(
                    origin,
                    generic_parameters,
                    type_arguments,
                    GenericPosition::Inference,
                )?;
                match substitution {
                    Some(substitution) => substitution,
                    None => {
                        return Ok(Answer::Ready(Err(SignatureRejection::Inapplicable)));
                    }
                }
            }
            [] if type_arguments.is_empty() => Default::default(),
            [] => {
                return Ok(Answer::Ready(Err(SignatureRejection::Inapplicable)));
            }
        };

        // check written arguments against their declared bounds
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
                if !answer!(self.constrain_generic_bound(origin, source_node, argument, bound,)?) {
                    let source_node = source.into_global(module);
                    let rejection =
                        self.signature_bound_rejection(origin, source_node, argument, bound)?;

                    return Ok(Answer::Ready(Err(rejection)));
                }
            }
        }

        // prove template predicates under the composed call substitution,
        // with the receiver bound so self predicates prove against it
        let predicates = self.template_predicates(function.template);
        if !predicates.is_empty() {
            let mut composed = substitution.with_carried(carried);
            if let Some(receiver) = receiver {
                composed = composed.with_receiver(receiver);
            }
            for predicate in predicates {
                let left = self.substitute_type(origin.module(), predicate.left, &composed)?;
                let right = self.substitute_type(origin.module(), predicate.right, &composed)?;

                if !answer!(self.constrain(origin, Relation::Satisfies, left, right)?) {
                    let source_node = source.into_global(module);
                    let rejection =
                        self.signature_bound_rejection(origin, source_node, left, right)?;

                    return Ok(Answer::Ready(Err(rejection)));
                }
            }
        }

        // relate the implicit receiver before explicit arguments
        if let (Some(receiver), Some(this_parameter)) = (receiver, function.this_parameter) {
            let this_parameter =
                self.substitute_type(origin.module(), this_parameter, &substitution)?;
            if !answer!(self.constrain_receiver_argument(
                origin,
                module,
                source,
                receiver,
                this_parameter,
            )?) {
                return Ok(Answer::Ready(Err(SignatureRejection::Inapplicable)));
            }
        }

        // relate inference-bearing arguments first
        let mut deferred_arguments = SmallVec::<
            [(
                usize,
                dir::GlobalNodeIdAny,
                dir::GlobalTypeId,
                dir::GlobalTypeId,
            ); 4],
        >::new();
        let signature_parameters = self
            .signature_parameters(signature_module, function.parameters)?
            .to_vec();
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = signature_parameters
                .get(index)
                .or_else(|| signature_parameters.last());
            let Some(parameter) = parameter else {
                return Ok(Answer::Ready(Err(SignatureRejection::Inapplicable)));
            };
            let argument_source = argument_sources
                .get(index)
                .copied()
                .unwrap_or_else(|| source.into_global(module));
            let parameter_type =
                self.substitute_type(origin.module(), parameter.ty, &substitution)?;
            if self.contains_inference_barrier(parameter_type)? {
                deferred_arguments.push((index, argument_source, *argument, parameter_type));

                continue;
            }

            let argument_origin = self.origin_at(origin, argument_source);
            if !answer!(self.constrain(
                argument_origin,
                Relation::Assignable,
                *argument,
                parameter_type,
            )?) {
                let rejection = SignatureRejection::Argument {
                    index,
                    source: self.format_type(*argument),
                    target: self.format_type(parameter_type),
                };

                return Ok(Answer::Ready(Err(rejection)));
            }
        }

        // reject candidates whose inferred arguments violate their bounds
        let variables = substitution.variables(self)?;
        match self.solve_probe_variables(variables)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) => {
                let rejection = answer!(self.signature_generic_bound_rejection(
                    origin,
                    module,
                    source,
                    generic_parameters,
                    &substitution,
                )?);
                let rejection = rejection.unwrap_or(SignatureRejection::Inapplicable);

                return Ok(Answer::Ready(Err(rejection)));
            }
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }

        // check NoInfer arguments after generic inference
        for (index, argument_source, argument, parameter_type) in deferred_arguments {
            let parameter_type = self.resolve_type_variables(origin.module(), parameter_type)?;
            let parameter_type = self.erase_inference_barriers(origin.module(), parameter_type)?;
            let parameter_type = answer!(self.reduce_type(origin, parameter_type)?);

            let argument_origin = self.origin_at(origin, argument_source);
            if !answer!(self.constrain(
                argument_origin,
                Relation::Assignable,
                argument,
                parameter_type,
            )?) {
                let rejection = SignatureRejection::Argument {
                    index,
                    source: self.format_type(argument),
                    target: self.format_type(parameter_type),
                };

                return Ok(Answer::Ready(Err(rejection)));
            }
        }

        // resolve the substituted return type
        let return_type = match function_return {
            Some(return_type) => {
                let return_type =
                    self.substitute_type(origin.module(), return_type, &substitution)?;

                self.resolve_type_variables(origin.module(), return_type)?
            }
            None => self.intern_type(module, dir::Type::Void)?,
        };
        let parameters = self
            .signature_parameters(signature_module, function.parameters)?
            .to_vec()
            .iter()
            .map(|parameter| {
                let ty = self.substitute_type(origin.module(), parameter.ty, &substitution)?;
                let ty = self.resolve_type_variables(origin.module(), ty)?;
                let ty = self.erase_inference_barriers(origin.module(), ty)?;

                Ok(dir::FunctionParameterType {
                    ty,
                    static_parameter: None,
                    is_optional: parameter.is_optional,
                    is_rest: parameter.is_rest,
                })
            })
            .collect::<CompilerResult<SmallVec<[_; 4]>>>()?;
        let raw_arguments = substitution
            .arguments
            .iter()
            .copied()
            .map(|argument| self.resolve_type_variables(origin.module(), argument))
            .collect::<CompilerResult<SmallVec<[_; 4]>>>()?;
        let arguments = self.generic_argument_bindings(generic_parameters, &raw_arguments)?;
        let function_type = self.instantiate_signature_type(
            signature_module,
            module,
            function,
            &substitution,
            return_type,
        )?;

        Ok(Answer::Ready(Ok(SignatureSelection {
            callable: function_type,
            parameters,
            return_type,
            generic_arguments: arguments,
        })))
    }

    /// Return the first inferred generic argument bound rejection.
    fn signature_generic_bound_rejection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        generic_parameters: &[dir::GlobalGenericParameterId],
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Answer<Option<SignatureRejection>>> {
        let source_node = source.into_global(module);

        for (parameter, argument) in generic_parameters
            .iter()
            .copied()
            .zip(substitution.arguments.iter().copied())
        {
            let Some(bound) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                continue;
            };
            let argument = self.resolve_type_variables(origin.module(), argument)?;
            let bound = self.substitute_type(origin.module(), bound, substitution)?;
            let bound = self.resolve_type_variables(origin.module(), bound)?;

            // replay the bound relation with solved arguments
            if !answer!(self.constrain_generic_bound(origin, source_node, argument, bound,)?) {
                let source_node = self.generic_argument_rejection_source(source_node, argument)?;
                let rejection =
                    self.signature_bound_rejection(origin, source_node, argument, bound)?;

                return Ok(Answer::Ready(Some(rejection)));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Return one signature rejection for a failed generic bound relation.
    fn signature_bound_rejection(
        &mut self,
        origin: Origin,
        source_node: dir::GlobalNodeIdAny,
        argument: dir::GlobalTypeId,
        bound: dir::GlobalTypeId,
    ) -> CompilerResult<SignatureRejection> {
        if let Some((source, key, value)) =
            self.writable_index_signature_rejection(origin, argument, bound)?
        {
            return Ok(SignatureRejection::WritableIndex {
                source_node,
                source,
                key,
                value,
            });
        }

        Ok(SignatureRejection::Bound {
            source_node,
            source: self.format_type(argument),
            target: self.format_type(bound),
        })
    }

    /// Return the source occurrence that produced one generic argument.
    fn generic_argument_rejection_source(
        &self,
        source_node: dir::GlobalNodeIdAny,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        let Some(variable) = self.root_variable(argument)? else {
            return Ok(source_node);
        };
        let variable = self.solver.representative(variable)?;
        let source = self
            .solver
            .variable(variable)?
            .lower
            .first()
            .map(|bound| bound.source)
            .unwrap_or(source_node);

        Ok(source)
    }
}
