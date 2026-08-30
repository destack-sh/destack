use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::infer::InferMode;
use crate::sema::{
    CandidateOutcome, Cause, CauseId, CauseKind, CheckFailure, CheckOutcome, CheckState,
    Expectation, Origin, REPORTED_REJECTIONS, ReceiverSteps, Relation, RelationCheck, Settle,
    TypeArgumentInference, TypeSubstitution, Value, ValueUse, Verdict,
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

/// One invocation constrained against a candidate signature.
struct Invocation {
    /// The rejection the constraints produced, set when the candidate fails.
    rejection: Option<SignatureRejection>,
    /// Whether the expected result refused the substituted return.
    is_return_mismatch: bool,
    /// The projection steps the declared `this` derived for the receiver.
    receiver_steps: Option<ReceiverSteps>,
    /// The coercions selected for the supplied arguments.
    coercions: SmallVec<[(dir::GlobalNodeIdAny, dir::Coercion); 4]>,
}

/// The callable one selection settled on, with the outcome its site commits.
/// FUGU #Architecture: "Selection", "Invocation", .. selecT/signature,.rs is full of terrible nouns
#[allow(clippy::large_enum_variant)]
pub(in crate::sema) enum Selection<'candidate, C> {
    /// One candidate accepts the invocation, or a sole one rejects it with a reported reason.
    Selected {
        /// The candidate position.
        position: usize,
        /// The selected candidate.
        candidate: &'candidate C,
        /// The matched signature.
        signature: SignatureSelection,
        /// The outcome the site commits against its expectation.
        outcome: CheckOutcome,
    },
    /// Every candidate rejects the invocation, described for the report.
    Rejected(Vec<String>),
    /// A sole candidate refuses the invocation with a reason already reported.
    Refused,
    /// Several candidates accept an exclusive invocation.
    Ambiguous,
}

/// How several candidates resolve one invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum OverloadRule {
    /// Select the first viable candidate in declaration order.
    Ordered,
    /// Require exactly one viable candidate.
    Exclusive,
}

/// Result of matching one callable signature.
pub(in crate::sema) enum SignatureMatch {
    /// The invocation satisfies the selected signature.
    Selected(SignatureSelection),
    /// The selected signature rejects one invocation constraint.
    Invalid {
        /// The selected signature.
        key: SignatureSelection,
        /// The rejected invocation constraint.
        rejection: SignatureRejection,
    },
    /// The selected signature accepts the arguments and fails the expected result.
    ReturnMismatch(SignatureSelection),
    /// The signature refuses instantiation for this invocation.
    Inapplicable(SignatureRejection),
}

impl SignatureMatch {
    /// Convert this match into candidate applicability.
    pub(in crate::sema) fn into_candidate(
        self,
    ) -> CandidateOutcome<SignatureSelection, SignatureRejection> {
        match self {
            Self::Selected(key) | Self::ReturnMismatch(key) => CandidateOutcome::Accepted(key),
            Self::Invalid { rejection, .. } | Self::Inapplicable(rejection) => {
                CandidateOutcome::Rejected(rejection)
            }
        }
    }
}

/// Argument matched against one callable signature parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct CallableArgument {
    /// Source node used for origins and diagnostics.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The argument type, typed once ahead of every candidate.
    /// A composite has none until the selected parameter checks it in context.
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
        /// The decided verdict: ambiguity retries once variables solve.
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

impl SignatureSelection {
    /// Return a call resolution for one selected declaration-backed member.
    pub(in crate::sema) fn member_call(
        &self,
        mut receiver: dir::MemberReceiver,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
        key_receiver: Option<dir::GlobalTypeId>,
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
                    key: dir::InstanceKey::new(symbol, generic_arguments)
                        .with_receiver(key_receiver),
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            dir::MemberReceiver::Dynamic(dispatch) => dir::CallableTarget::Dynamic {
                dispatch,
                function: dir::DynamicFunction::Symbol(symbol),
                generic_arguments,
            },
        };

        // build the call over the selected target
        dir::Call {
            target,
            callable_type: self.callable,
            arguments,
            return_type: self.return_type,
        }
    }
}

impl SignatureRejection {
    /// Return whether this rejection identifies a precise call failure.
    pub(in crate::sema) fn is_precise(&self) -> bool {
        !matches!(self, Self::Inapplicable)
    }
}

#[allow(clippy::too_many_arguments)]
impl CheckState<'_> {
    /// Select one callable candidate and constrain the invocation against it.
    pub(in crate::sema) fn select_callable<'candidate, C>(
        &mut self,
        origin: Origin,
        candidates: &'candidate [C],
        rule: OverloadRule,
        candidate_type: impl Fn(&C) -> dir::GlobalTypeId,
        attempt: impl Fn(&mut Self, &C) -> CompilerResult<SignatureMatch>,
    ) -> CompilerResult<Selection<'candidate, C>> {
        // decide the candidate the rule selects
        let module = origin.module();
        let selected =
            self.select_signature(module, candidates, rule, &candidate_type, &attempt)?;
        let (position, candidate, mut rejections) = match selected {
            Ok((position, candidate, rejections)) => (position, candidate, rejections),
            Err(rejected) => return Ok(rejected),
        };

        // constrain the invocation against the selected candidate
        let is_single = candidates.len() == 1;
        let selection = match attempt(self, candidate)? {
            // commit the accepted invocation
            SignatureMatch::Selected(signature) => Selection::Selected {
                position,
                candidate,
                signature,
                outcome: CheckOutcome::Holds,
            },
            // commit the invocation and fail it against the expected result
            SignatureMatch::ReturnMismatch(signature) => Selection::Selected {
                position,
                candidate,
                signature,
                outcome: CheckOutcome::Fails(CheckFailure::Relation),
            },
            // report a sole candidate's own invocation rejection and still commit
            SignatureMatch::Invalid { key, rejection } if is_single => {
                self.report_signature_rejection(origin, rejection)?;

                Selection::Selected {
                    position,
                    candidate,
                    signature: key,
                    outcome: CheckOutcome::Holds,
                }
            }
            // report a sole candidate's own precise refusal
            SignatureMatch::Inapplicable(rejection) if is_single && rejection.is_precise() => {
                self.report_signature_rejection(origin, rejection)?;

                Selection::Refused
            }
            // leave a refused candidate to the shared report
            SignatureMatch::Invalid { rejection, .. } | SignatureMatch::Inapplicable(rejection) => {
                let described =
                    self.format_signature_rejection(module, candidate_type(candidate), &rejection)?;
                rejections.push(described);
                rejections.truncate(REPORTED_REJECTIONS);

                Selection::Rejected(rejections)
            }
        };

        Ok(selection)
    }

    /// Select one signature candidate under the overload rule, describing the rejected ones.
    pub(in crate::sema) fn select_signature<'candidate, C>(
        &mut self,
        module: ModuleId,
        candidates: &'candidate [C],
        rule: OverloadRule,
        candidate_type: impl Fn(&C) -> dir::GlobalTypeId,
        attempt: impl Fn(&mut Self, &C) -> CompilerResult<SignatureMatch>,
    ) -> CompilerResult<Result<(usize, &'candidate C, Vec<String>), Selection<'candidate, C>>> {
        if let [single] = candidates {
            return Ok(Ok((0, single, Vec::new())));
        }

        // decide each candidate in declaration order
        let mut selected = None;
        let mut undecided = None;
        let mut rejections = Vec::new();
        for (position, candidate) in candidates.iter().enumerate() {
            let (verdict, rejection) =
                self.decide_signature(module, candidate_type(candidate), |state| {
                    attempt(state, candidate)
                })?;
            match (verdict, rule) {
                (Verdict::Fails, _) => rejections.extend(rejection),
                (Verdict::Holds, OverloadRule::Ordered) => {
                    return Ok(Ok((position, candidate, rejections)));
                }
                (Verdict::Holds, OverloadRule::Exclusive) if selected.is_some() => {
                    return Ok(Err(Selection::Ambiguous));
                }
                (Verdict::Holds, OverloadRule::Exclusive) => selected = Some((position, candidate)),
                (Verdict::Ambiguous, OverloadRule::Ordered) => {
                    undecided.get_or_insert((position, candidate));
                }
                (Verdict::Ambiguous, OverloadRule::Exclusive) => {
                    return Ok(Err(Selection::Ambiguous));
                }
            }
        }

        // take the exclusive winner, else the first undecided ordered candidate
        Ok(match selected.or(undecided) {
            Some((position, candidate)) => Ok((position, candidate, rejections)),
            None => {
                rejections.truncate(REPORTED_REJECTIONS);

                Err(Selection::Rejected(rejections))
            }
        })
    }

    /// Decide one signature candidate, describing its rejection.
    pub(in crate::sema) fn decide_signature(
        &mut self,
        module: ModuleId,
        candidate: dir::GlobalTypeId,
        attempt: impl FnOnce(&mut Self) -> CompilerResult<SignatureMatch>,
    ) -> CompilerResult<(Verdict, Option<String>)> {
        let (outcome, verdict) = self.decide(|state| {
            let outcome = attempt(state)?.into_candidate();
            let note = match &outcome {
                CandidateOutcome::Rejected(rejection) => {
                    Some(state.format_signature_rejection(module, candidate, rejection)?)
                }
                CandidateOutcome::Accepted(_) => None,
            };

            Ok((outcome, note))
        })?;
        Ok(match outcome {
            (CandidateOutcome::Accepted(_), note) => (verdict, note),
            (CandidateOutcome::Rejected(_), note) => (Verdict::Fails, note),
        })
    }

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
            // function values and pointers read the signature they wrap
            dir::Type::Function(function) => {
                return self.callable_signature_type(origin, function.signature);
            }
            dir::Type::FunctionPointer(function) => {
                return self.callable_signature_type(origin, function.signature);
            }
            // treat every other type as uncallable
            _ => None,
        };

        Ok(signature)
    }

    /// Expand one substituted tuple rest into its positional parameters.
    fn spread_tuple_rest_parameters(
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
        let rest = self.normalize(origin, rest)?;
        let dir::Type::Tuple(tuple) = self.ty(rest)? else {
            return Ok(parameters);
        };

        // rebuild positional parameters from the tuple elements
        let mut expanded = parameters[..rest_index].to_vec();
        for element in self.tuple_elements(rest.module_id, tuple.elements)? {
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

        // keep the collection type on a rest parameter and take its element per source
        let argument_type = if parameter.is_rest {
            self.rest_element_type(origin, parameter_type)?
                .unwrap_or(parameter_type)
        } else {
            parameter_type
        };

        // replace the declared type with the selected type
        let parameter = dir::FunctionParameterType {
            ty: parameter_type,
            ..parameter
        };

        // carry the selected parameter with its argument type
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

        // bind each written argument to its selected parameter
        self.argument_bindings(
            Origin::Node(node, None),
            module,
            argument_nodes,
            &parameters,
        )
    }

    /// Bind written argument sources to selected parameters, projecting rest elements.
    pub(in crate::sema) fn bind_argument_sources(
        &mut self,
        origin: Origin,
        signature: &SignatureSelection,
        sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Vec<dir::ArgumentBinding>> {
        // bind each selected parameter to the source written at its position
        let mut bindings = Vec::with_capacity(signature.parameters.len());
        for (index, selected) in signature.parameters.iter().enumerate() {
            let parameter = selected.parameter;

            // project rest elements through the deep normal form for resolved selections
            let argument_type = if parameter.is_rest {
                self.rest_element_type(origin, parameter.ty)?
                    .unwrap_or(parameter.ty)
            } else {
                parameter.ty
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
    ) -> CompilerResult<Option<dir::InstanceKey>> {
        // slice parameters pack in place
        let reduced = self.deeply_resolve(origin, rest)?;
        if matches!(self.ty(reduced)?, dir::Type::Slice(_)) {
            return Ok(None);
        }

        Ok(Some(self.array_pack_selection(element)?))
    }

    /// Select the array constructor over one element type.
    pub(in crate::sema) fn array_pack_selection(
        &mut self,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<dir::InstanceKey> {
        // read the element parameter of the array pack constructor
        let symbol = self.language_symbol(dir::LanguageItem::ArrayFromSlice)?;
        let Some(template) = self.symbol_template(symbol)? else {
            return Err(CompilerError::Internal {
                message: "the array pack constructor declares no template".to_string(),
            });
        };
        let parameters = self.generic_template_parameters(template)?;
        let Some(parameter) = parameters.first().copied() else {
            return Err(CompilerError::Internal {
                message: "the array pack constructor declares no element parameter".to_string(),
            });
        };
        let binding = dir::GenericArgumentBinding::new(parameter, element);

        Ok(dir::InstanceKey::new(symbol, vec![binding]))
    }

    /// Return the element type one rest parameter accepts per tail argument.
    pub(in crate::sema) fn rest_element_type(
        &mut self,
        origin: Origin,
        rest: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let reduced = self.deeply_resolve(origin, rest)?;

        // resolve the element of a placed collection in the collection's place
        if let dir::Type::Form(form) = self.ty(reduced)?
            && let dir::Form::Managed { place } = form.form
        {
            let Some(element) = self.rest_element_type(origin, form.value)? else {
                return Ok(None);
            };

            let element = self.resolve_relative_place(origin, element, place)?;

            return Ok(Some(element));
        }

        // project the element from the collection the annotation names
        match self.ty(reduced)? {
            dir::Type::Slice(slice) => Ok(Some(slice.element)),
            dir::Type::FixedArray(array) => Ok(Some(array.element)),
            dir::Type::Application(instance) => {
                let item = self.language_item(instance.symbol)?;
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

    /// Match one callable candidate against the supplied arguments.
    pub(in crate::sema) fn match_callable(
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
            // function values call through the signature they wrap
            dir::Type::Function(function) => {
                let function = function.signature;

                return self.match_callable(
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
            // function pointers call through the signature they point at
            dir::Type::FunctionPointer(function) => {
                let function = function.signature;

                return self.match_callable(
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
            // every other callee shape refuses the invocation
            _ => {
                return Ok(SignatureMatch::Inapplicable(
                    SignatureRejection::Inapplicable,
                ));
            }
        };

        // constrain the invocation against the declared signature
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
            is_fresh: false,
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

    /// Constrain one function signature candidate, leaving residual constraints to fulfillment.
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
        // reject argument counts outside the accepted arity
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

        // read the parameters the signature declares
        let parameters = self.signature_generic_parameters(function)?;

        // fix the parameters to exact literals for a const call
        let is_const_call = arguments
            .iter()
            .all(|argument| argument.use_ == ValueUse::Const);
        let inference = if is_const_call {
            TypeArgumentInference::Exact
        } else {
            TypeArgumentInference::Callable {
                parameters: &signature_parameters,
                return_type: function_return,
            }
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

        // bind the owner parameters member lookup left open
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
            && matches!(
                self.ty(self.shallow_resolve(receiver_value.ty)?)?,
                dir::Type::Reference(_)
            )
            && let Some(owner) = owner
            && let Some(dir::Definition::Extension(extension)) = self.definition(owner)?
        {
            let target = extension.target.r#type();
            let applied = self.substitute_type(target, &substitution)?;
            substitution = substitution.with_receiver(applied);
        }

        // constrain every invocation relation before settling candidate inference
        let invocation = self.constrain_invocation(
            origin,
            function,
            &signature_parameters,
            &parameters,
            &substitution,
            receiver,
            owner,
            function_return,
            expectation,
            arguments,
        )?;
        let Some(Invocation {
            rejection,
            is_return_mismatch,
            receiver_steps,
            coercions,
        }) = invocation
        else {
            return Ok(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            ));
        };

        // build the selection from whatever the invocation resolved
        let mut key = self.signature_selection(
            origin,
            signature_module,
            function,
            function_return,
            &substitution,
            receiver.map(|receiver| receiver.ty),
            receiver_steps,
        )?;
        key.coercions = coercions;

        // classify the candidate by what rejected it
        let matched = match rejection {
            Some(rejection) => SignatureMatch::Invalid { key, rejection },
            None if is_return_mismatch => SignatureMatch::ReturnMismatch(key),
            None => SignatureMatch::Selected(key),
        };

        Ok(matched)
    }

    /// Constrain one invocation's receiver, expectation, and arguments.
    fn constrain_invocation(
        &mut self,
        origin: Origin,
        function: &dir::FunctionSignatureType,
        signature_parameters: &[dir::FunctionParameterType],
        parameters: &[dir::GlobalGenericParameterId],
        substitution: &TypeSubstitution,
        receiver: Option<Value>,
        owner: Option<dir::GlobalSymbolId>,
        function_return: Option<dir::GlobalTypeId>,
        expectation: Option<Expectation>,
        arguments: &[CallableArgument],
    ) -> CompilerResult<Option<Invocation>> {
        // collect what the invocation decides about this candidate
        let mut is_return_mismatch = false;
        let mut receiver_steps = None;
        let mut coercions = SmallVec::new();
        // relate the implicit receiver before explicit arguments
        if let (Some(receiver), Some(this_parameter)) = (receiver, function.this_parameter) {
            let receiver_substitution = substitution.clone().with_receiver(receiver.ty);
            let this_parameter = self.substitute_type(this_parameter, &receiver_substitution)?;
            match self.constrain_receiver(origin, receiver, this_parameter)? {
                Some(steps) => receiver_steps = Some(steps),
                None => {
                    let rejection = SignatureRejection::Receiver {
                        source: receiver.ty,
                        target: this_parameter,
                    };

                    return Ok(Some(Invocation {
                        rejection: Some(rejection),
                        is_return_mismatch,
                        receiver_steps,
                        coercions,
                    }));
                }
            }
        }

        // apply the contextual result type before contextualizing arguments
        if let (Some(return_type), Some(expectation)) = (function_return, expectation) {
            let return_type = self.substitute_type(return_type, substitution)?;
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
                    is_fresh: false,
                },
                expectation.target,
                expectation.use_,
                expectation.mode,
            )?;

            // record a failed expectation as a return mismatch
            if matches!(converted.outcome, CheckOutcome::Fails(_)) {
                is_return_mismatch = true;
            }
        }

        // spread a substituted tuple rest into positional parameters
        let signature_parameters =
            self.spread_tuple_rest_parameters(origin, signature_parameters.to_vec(), substitution)?;

        // reject argument tails past a fixed parameter count
        let has_rest = signature_parameters
            .iter()
            .any(|parameter| parameter.is_rest);
        if !has_rest && arguments.len() > signature_parameters.len() {
            return Ok(None);
        }

        // substitute parameter types once for candidate inference
        let mut argument_parameters =
            SmallVec::<[(usize, CallableArgument, dir::GlobalTypeId); 4]>::new();
        for (index, argument) in arguments.iter().copied().enumerate() {
            let parameter = signature_parameters
                .get(index)
                .or_else(|| signature_parameters.last());
            let Some(parameter) = parameter else {
                return Ok(None);
            };

            // reject a spread outside a rest parameter
            if argument.is_spread && !parameter.is_rest {
                return Ok(None);
            }

            let parameter = self.select_parameter(
                origin,
                *parameter,
                substitution,
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
                substitution,
            )?);
        }

        // add the constraints the owner template declares
        if let Some(owner) = owner
            && let Some(template) = self.symbol_template(owner)?
        {
            bounds.extend(self.substitute_application_constraints(
                origin,
                template,
                substitution,
            )?);
        }

        // queue obligations, rejecting on decided failures only
        for bound in bounds {
            let id = self.push_relation(bound)?;
            let Some(outcome) = self.fulfill.checks.result(id)?.copied() else {
                continue;
            };

            if let CheckOutcome::Fails(failure) = outcome {
                let rejection = self.mismatch_rejection(
                    bound.cause,
                    bound.relation,
                    None,
                    bound.source,
                    bound.target,
                    failure,
                )?;

                return Ok(Some(Invocation {
                    rejection: Some(rejection),
                    is_return_mismatch,
                    receiver_steps,
                    coercions,
                }));
            }
        }

        // collect staged const parameters opened for this candidate
        let mut const_variables = SmallVec::<[dir::TypeVariableId; 2]>::new();
        for parameter in parameters {
            if self
                .generic_parameter(*parameter)
                .is_some_and(|binding| binding.is_const && binding.memory_parameter().is_none())
                && let Some(instance) = substitution.argument(*parameter)
                && let Some(variable) = self.root_variable(instance)?
            {
                const_variables.push(variable);
            }
        }

        // match every argument in order, refusing the candidate on the first rejection
        for entry in argument_parameters {
            if let Some(rejection) =
                self.constrain_argument(origin, entry, &const_variables, &mut coercions)?
            {
                return Ok(Some(Invocation {
                    rejection: Some(rejection),
                    is_return_mismatch,
                    receiver_steps,
                    coercions,
                }));
            }
        }

        // carry the accepted invocation with its steps
        Ok(Some(Invocation {
            rejection: None,
            is_return_mismatch,
            receiver_steps,
            coercions,
        }))
    }

    /// Build one mismatch rejection over a failed relation.
    fn mismatch_rejection(
        &mut self,
        cause: CauseId,
        relation: Relation,
        use_: Option<ValueUse>,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        failure: CheckFailure,
    ) -> CompilerResult<SignatureRejection> {
        Ok(SignatureRejection::Mismatch {
            verdict: self.decide_outcome(false, source, target)?,
            cause,
            relation,
            use_,
            source,
            target,
            failure,
        })
    }

    /// Constrain one supplied argument, resolving staged const parameters.
    fn constrain_argument(
        &mut self,
        origin: Origin,
        entry: (usize, CallableArgument, dir::GlobalTypeId),
        const_variables: &[dir::TypeVariableId],
        coercions: &mut SmallVec<[(dir::GlobalNodeIdAny, dir::Coercion); 4]>,
    ) -> CompilerResult<Option<SignatureRejection>> {
        // convert the argument against its substituted parameter
        let (index, argument, parameter_type) = entry;
        let conversion = self.match_signature_argument(origin, index, argument, parameter_type)?;
        match conversion {
            Ok(Some(coercion)) => coercions.push((argument.source, coercion)),
            Ok(None) => {}
            Err(failure) => return Ok(Some(failure)),
        }

        // resolve staged const parameters as their arguments settle
        for variable in const_variables {
            if self.infer.variable(*variable)?.state.is_open() {
                self.settle_variables(&[*variable], Settle::All)?;
            }
        }

        Ok(None)
    }

    /// Settle one relative member type in the receiver's concrete place.
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
        let declared = self.spread_tuple_rest_parameters(origin, declared, substitution)?;
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
        let arguments = self.resolved_argument_bindings(&substitution.bindings)?;
        let function_type =
            self.instantiate_signature_type(function, substitution, &parameters, return_type)?;

        // carry the instantiated signature with its slots
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
        let cause = self.intern_cause(Cause::root(
            origin,
            CauseKind::Argument {
                call,
                index: index as u32,
            },
        ));

        // read a const parameter's argument as const, widening every other argument mutably
        let mut mode = InferMode::Regular;
        for variable in self.type_variables(parameter_type)? {
            if let Some(parameter) = self.infer.variable(variable)?.parameter
                && self.require_generic_parameter(parameter)?.is_const
            {
                mode = InferMode::Const;
            }
        }
        let site = self.visit_site(source)?;

        // check a composite's values against the parameter, recording its own conversions
        let Some(ty) = argument.ty else {
            let check = self.check_node(
                site,
                Expectation {
                    target: parameter_type,
                    relation,
                    cause,
                    use_: argument.use_,
                    mode,
                },
            )?;
            if let CheckOutcome::Fails(failure) = check.outcome {
                let rejection = self.mismatch_rejection(
                    cause,
                    relation,
                    Some(argument.use_),
                    check.source,
                    parameter_type,
                    failure,
                )?;

                return Ok(Err(rejection));
            }

            return Ok(Ok(None));
        };

        // constrain a spread element against the rest parameter
        if argument.is_spread {
            let element = self.spread_element_type(ty)?;
            if self.constrain_type(origin, cause, relation, element, parameter_type)?
                == Verdict::Fails
            {
                let rejection = self.mismatch_rejection(
                    cause,
                    relation,
                    Some(argument.use_),
                    element,
                    parameter_type,
                    CheckFailure::Relation,
                )?;

                return Ok(Err(rejection));
            }

            return Ok(Ok(None));
        }

        // select the complete runtime conversion for this candidate
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

        // record the rejection a failed conversion reports
        if let CheckOutcome::Fails(failure) = conversion.outcome {
            let rejection = self.mismatch_rejection(
                cause,
                relation,
                Some(argument.use_),
                ty,
                conversion.target,
                failure,
            )?;

            return Ok(Err(rejection));
        }

        let coercion = conversion.coercion.map(|coercion| *coercion);

        Ok(Ok(coercion))
    }
}

impl dir::TypeFold for ParameterSelection {
    /// Map every type this parameter carries.
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.parameter.ty.map_types(map)?;
        self.argument_type.map_types(map)
    }
}

impl dir::TypeFold for SignatureSelection {
    /// Map every type this selection carries.
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.callable.map_types(map)?;
        self.parameters.map_types(map)?;
        self.return_type.map_types(map)?;
        self.generic_arguments.map_types(map)?;
        self.receiver_steps.map_types(map)?;
        for (_, coercion) in &mut self.coercions {
            coercion.map_types(map)?;
        }

        Ok(())
    }
}
