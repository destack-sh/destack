use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::infer::InferMode;
use crate::sema::{
    BodyState, CandidateOutcome, Cause, CauseId, CauseKind, CheckFailure, CheckOutcome,
    Expectation, Origin, PlaceUse, ReceiverSteps, Relation, RelationCheck, TypeArgumentInference,
    TypeSubstitution, Value, ValueUse, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// Callable signature accepted for an invocation.
#[derive(Debug, Clone)]
pub(in crate::sema) struct SignatureSelection {
    /// The callable type after substitution.
    pub(in crate::sema) callable: dir::GlobalTypeId,
    /// The selected parameters in declaration order.
    pub(in crate::sema) parameters: SmallVec<[ParameterSelection; 4]>,
    /// The return type after substitution.
    pub(in crate::sema) return_type: dir::GlobalTypeId,
    /// The solved generic argument bindings.
    pub(in crate::sema) generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The projection steps when the declared this adjusted the receiver.
    pub(in crate::sema) receiver_steps: Option<ReceiverSteps>,
    /// Runtime coercions selected for the supplied arguments.
    pub(in crate::sema) coercions: SmallVec<[(dir::GlobalNodeIdAny, dir::Coercion); 4]>,
}

/// One substituted parameter and its accepted argument type.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct ParameterSelection {
    /// The complete substituted parameter.
    pub(in crate::sema) parameter: dir::FunctionParameterType,
    /// The type accepted from each bound argument source.
    pub(in crate::sema) argument_type: dir::GlobalTypeId,
}

impl ParameterSelection {
    /// Bind one runtime source to this selected parameter.
    pub(in crate::sema) fn bind(&self, source: dir::ArgumentSource) -> dir::ArgumentBinding {
        dir::ArgumentBinding {
            parameter_type: self.parameter.ty,
            argument_type: self.argument_type,
            source,
        }
    }
}

/// One instantiated closed signature decision, replayed across call sites.
#[derive(Debug, Clone)]
pub(in crate::sema) struct SignatureInstance {
    /// The selected overload position within the callee's candidates.
    pub(in crate::sema) overload: usize,
    /// The instantiated selection without per-site coercions.
    pub(in crate::sema) selection: SignatureSelection,
}

/// Result of matching one callable signature.
pub(in crate::sema) enum SignatureMatch {
    /// The invocation satisfies the selected signature.
    Selected(SignatureSelection),
    /// The selected signature rejects one invocation constraint.
    Invalid {
        /// The selected signature.
        selection: SignatureSelection,
        /// The rejected invocation constraint.
        rejection: SignatureRejection,
    },
    /// The selected signature accepts the arguments and fails the expected result.
    ReturnMismatch(SignatureSelection),
    /// The invocation has no instantiable signature.
    Inapplicable(SignatureRejection),
}

impl SignatureMatch {
    /// Convert this match into candidate applicability.
    pub(in crate::sema) fn into_candidate(
        self,
    ) -> CandidateOutcome<SignatureSelection, SignatureRejection> {
        match self {
            Self::Selected(selection) | Self::ReturnMismatch(selection) => {
                CandidateOutcome::Accepted(selection)
            }
            Self::Invalid { rejection, .. } | Self::Inapplicable(rejection) => {
                CandidateOutcome::Rejected(rejection)
            }
        }
    }
}

impl SignatureSelection {
    /// Return a call resolution for one selected declaration-backed member.
    pub(in crate::sema) fn member_call(
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

        // dispatch directly on a value receiver, dynamically on an erased one
        let generic_arguments = self.generic_arguments.clone();
        let target = match receiver {
            dir::MemberReceiver::Direct(receiver) => dir::CallableTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: Some(receiver),
                    generic_scope: Some(owner),
                    selection: dir::Selection::new(symbol, generic_arguments),
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            dir::MemberReceiver::Dynamic(dispatch) => dir::CallableTarget::Dynamic {
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
pub(in crate::sema) struct CallableArgument {
    /// Source node used for origins and diagnostics.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// Known argument type, or none when the source expression must be checked.
    pub(in crate::sema) ty: Option<dir::GlobalTypeId>,
    /// Relation selected from the authored argument expression.
    pub(in crate::sema) relation: Relation,
    /// The value role of this invocation argument.
    pub(in crate::sema) use_: ValueUse,
    /// Whether the authored argument spreads a sequence.
    pub(in crate::sema) is_spread: bool,
}

/// Reason one callable signature rejected an invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) enum SignatureRejection {
    /// The candidate rejects the invocation outright.
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
        /// The judged verdict: ambiguity retries once variables solve.
        verdict: Verdict,
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
    /// The receiver failed the declared this parameter.
    Receiver {
        /// Supplied receiver type.
        source: dir::GlobalTypeId,
        /// Declared this parameter type.
        target: dir::GlobalTypeId,
    },
}

impl SignatureRejection {
    /// Return whether this rejection identifies a precise call failure.
    pub(in crate::sema) fn is_precise(&self) -> bool {
        !matches!(self, Self::Inapplicable)
    }
}

#[allow(clippy::too_many_arguments)]
impl BodyState<'_, '_> {
    /// Return the signature type behind one callable type.
    pub(in crate::sema) fn callable_signature_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, dir::FunctionSignatureType)>> {
        // read the callable through its reduced head
        let ty = self.normalize(origin, ty)?;
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

        Ok(signature)
    }

    /// Expand one substituted tuple rest into its positional parameters.
    fn splat_tuple_rest_parameters(
        &mut self,
        origin: Origin,
        parameters: Vec<dir::FunctionParameterType>,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Vec<dir::FunctionParameterType>> {
        // find the trailing rest parameter
        let Some(rest_index) = parameters.iter().position(|parameter| parameter.is_rest) else {
            return Ok(parameters);
        };

        // read the substituted rest as a closed tuple
        let rest = self.substitute_type(parameters[rest_index].ty, substitution)?;
        let rest = self.check.normalize(origin, rest)?;
        let dir::Type::Tuple(tuple) = self.check.ty(rest)? else {
            return Ok(parameters);
        };

        // rebuild positional parameters from the tuple elements
        let mut expanded = parameters[..rest_index].to_vec();
        for element in self.check.tuple_elements(rest.module_id, tuple.elements)? {
            expanded.push(dir::FunctionParameterType {
                name: None,
                ty: element.ty,
                is_optional: element.is_optional,
                is_rest: false,
            });
        }
        expanded.extend(parameters[rest_index + 1..].iter().copied());

        Ok(expanded)
    }

    /// Create one substituted function signature type.
    fn instantiate_signature_type(
        &mut self,
        signature: &dir::FunctionSignatureType,
        substitution: &TypeSubstitution,
        parameters: &[ParameterSelection],
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // substitute the selected receiver parameter
        let this_parameter = match signature.this_parameter {
            Some(this_parameter) => {
                let this_parameter = self.substitute_type(this_parameter, substitution)?;

                Some(this_parameter)
            }
            None => None,
        };

        // collect selected parameters
        let parameters = parameters
            .iter()
            .map(|selected| selected.parameter)
            .collect::<Vec<_>>();
        let parameters = self.intern_parameters(&parameters)?;

        // intern the selected signature
        let signature = self.intern_signature(dir::FunctionSignatureType {
            asynchrony: signature.asynchrony,
            template: None,
            this_parameter,
            parameters,
            return_type: Some(return_type),
            is_generator: signature.is_generator,
            is_construct: signature.is_construct,
        })?;

        Ok(signature)
    }

    /// Select one parameter for an invocation.
    pub(in crate::sema) fn select_parameter(
        &mut self,
        origin: Origin,
        parameter: dir::FunctionParameterType,
        substitution: &TypeSubstitution,
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<ParameterSelection> {
        // substitute the complete declared parameter type
        let parameter_type = self.substitute_type(parameter.ty, substitution)?;

        // resolve relative member parameters at the receiver's place
        let parameter_type = self.receiver_relative_type(origin, receiver, parameter_type)?;
        let parameter_type = self.shallow_resolve(parameter_type)?;

        // rest parameters retain their collection type and accept its element per source
        let argument_type = match parameter.is_rest {
            true => self
                .rest_element_type(origin, parameter_type)?
                .unwrap_or(parameter_type),
            false => parameter_type,
        };

        // replace the declared type with the selected type
        let parameter = dir::FunctionParameterType {
            ty: parameter_type,
            ..parameter
        };

        Ok(ParameterSelection {
            parameter,
            argument_type,
        })
    }

    /// Bind authored arguments to selected parameters, projecting rest elements.
    pub(in crate::sema) fn selected_argument_bindings(
        &mut self,
        node: dir::GlobalNodeIdAny,
        module: ModuleId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: &SignatureSelection,
    ) -> CompilerResult<Vec<dir::ArgumentBinding>> {
        let parameters = signature
            .parameters
            .iter()
            .map(|selected| selected.parameter)
            .collect::<Vec<_>>();

        self.argument_bindings(
            Origin::Node(node, None),
            module,
            argument_nodes,
            &parameters,
        )
    }

    /// Bind recorded argument sources to selected parameters, projecting rest elements.
    pub(in crate::sema) fn bind_argument_sources(
        &mut self,
        origin: Origin,
        signature: &SignatureSelection,
        sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Vec<dir::ArgumentBinding>> {
        // bind each selected parameter to the source recorded at its position
        let mut bindings = Vec::with_capacity(signature.parameters.len());
        for (index, selected) in signature.parameters.iter().enumerate() {
            let parameter = selected.parameter;
            // project rest elements through the deep normal form for settled selections
            let argument_type = match parameter.is_rest {
                true => self
                    .rest_element_type(origin, parameter.ty)?
                    .unwrap_or(parameter.ty),
                false => parameter.ty,
            };
            let source = match sources.get(index) {
                Some(source) => source.clone(),
                None if parameter.is_rest => dir::ArgumentSource::Rest {
                    elements: Vec::new(),
                    pack: self.rest_pack_selection(origin, parameter.ty, argument_type)?,
                },
                None => dir::ArgumentSource::Omitted,
            };

            bindings.push(dir::ArgumentBinding {
                parameter_type: parameter.ty,
                argument_type,
                source,
            });
        }

        Ok(bindings)
    }

    /// Select the pack constructor one rest parameter's collection requires.
    pub(in crate::sema) fn rest_pack_selection(
        &mut self,
        origin: Origin,
        rest: dir::GlobalTypeId,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Selection>> {
        // slice parameters pack in place without a constructor
        let reduced = self.check.deeply_resolve(origin, rest)?;
        if matches!(self.ty(reduced)?, dir::Type::Slice(_)) {
            return Ok(None);
        }

        Ok(Some(self.array_pack_selection(element)?))
    }

    /// Select the array constructor over one element type.
    pub(in crate::sema) fn array_pack_selection(
        &mut self,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Selection> {
        let symbol = self
            .check
            .language_symbol(dir::LanguageItem::ArrayFromSlice)?;
        let Some(template) = self.check.symbol_template(symbol)? else {
            return Err(CompilerError::Internal {
                message: "the array pack constructor declares no template".to_string(),
            });
        };
        let parameters = self.check.generic_template_parameters(template)?;
        let Some(parameter) = parameters.first().copied() else {
            return Err(CompilerError::Internal {
                message: "the array pack constructor declares no element parameter".to_string(),
            });
        };
        let binding = dir::GenericArgumentBinding::new(parameter, element);

        Ok(dir::Selection::new(symbol, vec![binding]))
    }

    /// Return the element type one rest parameter accepts per tail argument.
    pub(in crate::sema) fn rest_element_type(
        &mut self,
        origin: Origin,
        rest: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let reduced = self.check.deeply_resolve(origin, rest)?;

        // placed collections accept their element in the collection's place
        if let dir::Type::Form(form) = self.check.ty(reduced)?
            && let dir::Form::Placed { place } = form.form
        {
            let Some(element) = self.rest_element_type(origin, form.value)? else {
                return Ok(None);
            };

            let element = self.check.resolve_relative_place(origin, element, place)?;

            return Ok(Some(element));
        }

        // project the element from the collection the annotation names
        match self.check.ty(reduced)? {
            dir::Type::Slice(slice) => Ok(Some(slice.element)),
            dir::Type::FixedArray(array) => Ok(Some(array.element)),
            dir::Type::Application(instance) => {
                let item = self.check.language_item(instance.symbol)?;
                let is_array = matches!(
                    item,
                    Some(dir::LanguageItem::Array | dir::LanguageItem::ReadonlyArray)
                );
                let arguments: SmallVec<[_; 8]> =
                    self.type_ids(reduced.module_id, instance.arguments)?.into();

                match (is_array, arguments.as_slice()) {
                    (true, [element, ..]) => Ok(Some(*element)),
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    /// Attempt one callable candidate without recording a decision.
    pub(in crate::sema) fn attempt_callable(
        &mut self,
        origin: Origin,
        function_type: dir::GlobalTypeId,
        owner: Option<dir::GlobalSymbolId>,
        receiver: Option<Value>,
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[CallableArgument],
        expectation: Option<Expectation>,
    ) -> CompilerResult<SignatureMatch> {
        // read the callable shape before selecting a signature
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
                return Ok(SignatureMatch::Inapplicable(
                    SignatureRejection::Inapplicable,
                ));
            }
        };

        let return_type = function.return_type;

        self.constrain_signature(
            origin,
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
    pub(in crate::sema) fn match_signature(
        &mut self,
        origin: Origin,
        signature_module: ModuleId,
        owner: Option<dir::GlobalSymbolId>,
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        receiver: Option<dir::GlobalTypeId>,
        arguments: &[CallableArgument],
        expectation: Option<Expectation>,
    ) -> CompilerResult<SignatureMatch> {
        let receiver = receiver.map(|ty| Value {
            ty,
            node: None,
            place: None,
        });

        self.constrain_signature(
            origin,
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
        signature_module: ModuleId,
        owner: Option<dir::GlobalSymbolId>,
        carried: &[dir::GenericArgumentBinding],
        type_arguments: &[dir::GlobalTypeId],
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        receiver: Option<Value>,
        arguments: &[CallableArgument],
        expectation: Option<Expectation>,
    ) -> CompilerResult<SignatureMatch> {
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

            return Ok(SignatureMatch::Inapplicable(rejection));
        }

        // bind the receiver before evaluating generic defaults
        let substitution = receiver.map_or_else(TypeSubstitution::default, |receiver| {
            TypeSubstitution::default().with_receiver(receiver.ty)
        });

        // preserve generic bindings already selected by the callee
        let substitution = substitution.with_carried(carried)?;

        // const arguments pin their parameters to exact literals
        let parameters = self.signature_generic_parameters(function)?;
        let inference = match arguments
            .iter()
            .all(|argument| argument.use_ == ValueUse::Const)
        {
            true => TypeArgumentInference::Exact,
            false => TypeArgumentInference::Callable {
                parameters: &signature_parameters,
                return_type: function_return,
            },
        };

        // open the signature's own parameters
        let substitution = self.instantiate_parameters(
            origin,
            &parameters,
            type_arguments,
            substitution,
            inference,
        )?;
        let Some(mut substitution) = substitution else {
            return Ok(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            ));
        };

        // bind owner parameters that member lookup did not bind
        if let Some(owner) = owner
            && let Some(template) = self.symbol_template(owner)?
        {
            let mut parameters = self.generic_template_parameters(template)?;
            parameters.extend(self.owner_template_parameters(template)?);
            let opened =
                self.instantiate_parameters(origin, &parameters, &[], substitution, inference)?;
            let Some(opened) = opened else {
                return Ok(SignatureMatch::Inapplicable(
                    SignatureRejection::Inapplicable,
                ));
            };

            substitution = opened;
        }

        // point this at the applied extension target for bare type receivers
        if let Some(receiver_value) = receiver
            && matches!(self.ty(receiver_value.ty)?, dir::Type::Reference(_))
            && let Some(owner) = owner
            && let Some(dir::Definition::Extension(extension)) = self.definition(owner)?
        {
            let target = extension.target.r#type();
            let applied = self.substitute_type(target, &substitution)?;
            substitution = substitution.with_receiver(applied);
        }

        // collect what the invocation decides about this candidate
        let mut rejection = None;
        let mut is_return_mismatch = false;
        let mut receiver_steps = None;
        let mut coercions = SmallVec::new();

        // constrain every invocation relation before settling candidate inference
        'invocation: {
            // relate the implicit receiver before explicit arguments
            if let (Some(receiver), Some(this_parameter)) = (receiver, function.this_parameter) {
                let receiver_substitution = substitution.clone().with_receiver(receiver.ty);
                let this_parameter =
                    self.substitute_type(this_parameter, &receiver_substitution)?;
                match self.constrain_receiver_argument(origin, receiver, this_parameter)? {
                    Some(steps) => receiver_steps = Some(steps),
                    None => {
                        rejection = Some(SignatureRejection::Receiver {
                            source: receiver.ty,
                            target: this_parameter,
                        });

                        break 'invocation;
                    }
                }
            }

            // apply the contextual result type before contextualizing arguments
            if let (Some(return_type), Some(expectation)) = (function_return, expectation) {
                let return_type = self.substitute_type(return_type, &substitution)?;
                let return_type = self.receiver_relative_type(
                    origin,
                    receiver.map(|receiver| receiver.ty),
                    return_type,
                )?;
                let source = self.origin_source(origin)?;
                let site = self.visit_site(source)?;
                let converted = self.convert_value(
                    site,
                    expectation.cause,
                    expectation.relation,
                    Value {
                        ty: return_type,
                        node: None,
                        place: None,
                    },
                    expectation.target,
                    expectation.use_,
                    expectation.mode,
                )?;
                // leave a pending expectation to the queue after commitment
                if matches!(converted.outcome, CheckOutcome::Fails(_)) {
                    is_return_mismatch = true;
                }
            }

            // splat a substituted tuple rest into positional parameters
            let signature_parameters = self.splat_tuple_rest_parameters(
                origin,
                signature_parameters.clone(),
                &substitution,
            )?;

            // reject argument tails a splatted signature cannot accept
            let has_rest = signature_parameters
                .iter()
                .any(|parameter| parameter.is_rest);
            if !has_rest && arguments.len() > signature_parameters.len() {
                return Ok(SignatureMatch::Inapplicable(
                    SignatureRejection::Inapplicable,
                ));
            }

            // substitute parameter types once for candidate inference
            let mut argument_parameters =
                SmallVec::<[(usize, CallableArgument, dir::GlobalTypeId); 4]>::new();
            for (index, argument) in arguments.iter().copied().enumerate() {
                let parameter = signature_parameters
                    .get(index)
                    .or_else(|| signature_parameters.last());
                let Some(parameter) = parameter else {
                    return Ok(SignatureMatch::Inapplicable(
                        SignatureRejection::Inapplicable,
                    ));
                };

                // a spread supplies elements only through a rest window
                if argument.is_spread && !parameter.is_rest {
                    return Ok(SignatureMatch::Inapplicable(
                        SignatureRejection::Inapplicable,
                    ));
                }

                let parameter = self.select_parameter(
                    origin,
                    *parameter,
                    &substitution,
                    receiver.map(|receiver| receiver.ty),
                )?;
                argument_parameters.push((index, argument, parameter.argument_type));
            }

            // collect the callable and owner template constraints
            let mut bounds = SmallVec::<[RelationCheck; 4]>::new();
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

            // queue obligations, rejecting on decided failures only
            for bound in bounds {
                let id = self.check.push_relation(bound)?;
                let Some(outcome) = self.check.fulfill.checks.result(id)?.copied() else {
                    continue;
                };

                if let CheckOutcome::Fails(failure) = outcome {
                    rejection = Some(SignatureRejection::Mismatch {
                        verdict: self.check.verdict(
                            false,
                            bound.origin,
                            bound.relation,
                            bound.source,
                            bound.target,
                        )?,
                        cause: bound.cause,
                        relation: bound.relation,
                        use_: None,
                        source: bound.source,
                        target: bound.target,
                        failure,
                    });

                    break 'invocation;
                }
            }

            // collect staged const parameters opened for this candidate
            let mut const_variables = SmallVec::<[dir::TypeVariableId; 2]>::new();
            for parameter in &parameters {
                if self
                    .check
                    .generic_parameter(*parameter)
                    .is_some_and(|binding| binding.is_const && binding.memory_parameter().is_none())
                    && let Some(instance) = substitution.argument(*parameter)
                    && let Some(variable) = self.check.root_variable(instance)?
                {
                    const_variables.push(variable);
                }
            }

            // match plain arguments first, then contextual closures against the fixed shapes
            let mut plain = Vec::new();
            let mut contextual = Vec::new();
            for entry in argument_parameters {
                let (_, argument, _) = &entry;
                match self.check.lambdas.contains_key(&argument.source) {
                    true => contextual.push(entry),
                    false => plain.push(entry),
                }
            }

            // settle the shapes the plain arguments fix before the closures read them
            let fixed = plain
                .iter()
                .map(|(_, _, parameter_type)| *parameter_type)
                .collect::<Vec<_>>();
            for (round, entries) in [plain, contextual].into_iter().enumerate() {
                if round == 1 && !entries.is_empty() {
                    let roots = self.check.open_type_variables(fixed.iter().copied())?;
                    if !roots.is_empty() {
                        self.check.fix_scope_variables(&roots)?;
                    }
                }

                for (index, argument, parameter_type) in entries {
                    let conversion =
                        self.match_signature_argument(origin, index, argument, parameter_type)?;
                    match conversion {
                        Ok(conversion) => {
                            if let Some(coercion) = conversion {
                                coercions.push((argument.source, coercion));
                            }
                        }
                        Err(failure) => {
                            rejection = Some(failure);

                            break 'invocation;
                        }
                    }

                    for variable in &const_variables {
                        if self.check.infer.variable(*variable)?.state.is_open() {
                            self.check.resolve_variables(&[*variable])?;
                        }
                    }
                }
            }
        }

        // build the selection from whatever the invocation settled
        let mut selection = self.signature_selection(
            origin,
            signature_module,
            function,
            function_return,
            &substitution,
            receiver.map(|receiver| receiver.ty),
            receiver_steps,
        )?;
        selection.coercions = coercions;

        // classify the candidate by what rejected it, if anything
        let matched = match rejection {
            Some(rejection) => SignatureMatch::Invalid {
                selection,
                rejection,
            },
            None if is_return_mismatch => SignatureMatch::ReturnMismatch(selection),
            None => SignatureMatch::Selected(selection),
        };

        Ok(matched)
    }

    /// Resolve one relative member type in the receiver's concrete place.
    fn receiver_relative_type(
        &mut self,
        origin: Origin,
        receiver: Option<dir::GlobalTypeId>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(receiver) = receiver else {
            return Ok(ty);
        };
        let Some(place) = self.receiver_projected_place(receiver)? else {
            return Ok(ty);
        };

        self.place_relative_type(origin, place, ty)
    }

    /// Build the selected payload for one instantiated signature.
    fn signature_selection(
        &mut self,
        origin: Origin,
        signature_module: ModuleId,
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        substitution: &TypeSubstitution,
        receiver: Option<dir::GlobalTypeId>,
        receiver_steps: Option<ReceiverSteps>,
    ) -> CompilerResult<SignatureSelection> {
        // resolve the substituted return type
        let return_type = match function_return {
            Some(return_type) => self.substitute_type(return_type, substitution)?,
            None => self.intern_type(dir::Type::Void)?,
        };
        let return_type = self.receiver_relative_type(origin, receiver, return_type)?;

        // select each parameter and erase the barriers inference left behind
        let declared = self
            .signature_parameters(signature_module, function.parameters)?
            .to_vec();
        let declared = self.splat_tuple_rest_parameters(origin, declared, substitution)?;
        let mut parameters = SmallVec::<[_; 4]>::new();
        for parameter in declared {
            let parameter = self.select_parameter(origin, parameter, substitution, receiver)?;
            let parameter = ParameterSelection {
                parameter: dir::FunctionParameterType {
                    ty: self.erase_inference_barriers(origin.module(), parameter.parameter.ty)?,
                    ..parameter.parameter
                },
                argument_type: self
                    .erase_inference_barriers(origin.module(), parameter.argument_type)?,
            };
            parameters.push(parameter);
        }

        // intern the instantiated signature alongside its solved arguments
        let arguments = self.settled_argument_bindings(&substitution.bindings)?;
        let function_type =
            self.instantiate_signature_type(function, substitution, &parameters, return_type)?;

        Ok(SignatureSelection {
            callable: function_type,
            parameters,
            return_type,
            generic_arguments: arguments.to_vec(),
            receiver_steps,
            coercions: SmallVec::new(),
        })
    }

    /// Match one supplied argument against one substituted parameter type.
    pub(in crate::sema) fn match_signature_argument(
        &mut self,
        origin: Origin,
        index: usize,
        argument: CallableArgument,
        parameter_type: dir::GlobalTypeId,
    ) -> CompilerResult<Result<Option<dir::Coercion>, SignatureRejection>> {
        // root the argument's cause at its position in the call
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
        // a spread argument supplies its element sequence to the rest slot
        if argument.is_spread {
            let site = self.visit_site(source)?;
            let ty = self.infer_node_type(site, PlaceUse::Read)?;
            let element = self.spread_element_type(ty)?;
            if !self
                .check
                .evaluate_relation(origin, relation, element, parameter_type)?
                .holds()
            {
                let rejection = SignatureRejection::Mismatch {
                    verdict: self.check.verdict(
                        false,
                        origin,
                        relation,
                        element,
                        parameter_type,
                    )?,
                    cause,
                    relation,
                    use_: Some(argument.use_),
                    source: element,
                    target: parameter_type,
                    failure: CheckFailure::Relation,
                };

                return Ok(Err(rejection));
            }

            return Ok(Ok(None));
        }

        let mode = self.contextual_literal_mode(origin, parameter_type, InferMode::Widen)?;

        // apply target-directed syntax before converting the resulting value
        let ty = match argument.ty {
            Some(ty) => ty,
            None => {
                let site = self.visit_site(source)?;
                let expectation = Expectation {
                    target: parameter_type,
                    relation,
                    cause,
                    use_: argument.use_,
                    mode,
                };
                let check = self.check_node_target(site, expectation)?;
                let ty = check.source;

                if let CheckOutcome::Fails(failure) = check.outcome {
                    let rejection = SignatureRejection::Mismatch {
                        verdict: self
                            .check
                            .verdict(false, origin, relation, ty, check.target)?,
                        cause,
                        relation,
                        use_: Some(argument.use_),
                        source: ty,
                        target: check.target,
                        failure,
                    };

                    return Ok(Err(rejection));
                }

                ty
            }
        };

        // select the complete runtime conversion for this candidate
        let site = self.visit_site(source)?;
        let value = self.expression_value(site, ty)?;
        let conversion = self.convert_value(
            site,
            cause,
            relation,
            value,
            parameter_type,
            argument.use_,
            mode,
        )?;

        if let CheckOutcome::Fails(failure) = conversion.outcome {
            let rejection = SignatureRejection::Mismatch {
                verdict: self
                    .check
                    .verdict(false, origin, relation, ty, conversion.target)?,
                cause,
                relation,
                use_: Some(argument.use_),
                source: ty,
                target: conversion.target,
                failure,
            };

            return Ok(Err(rejection));
        }

        let coercion = conversion.coercion.map(|coercion| *coercion);

        Ok(Ok(coercion))
    }
}
