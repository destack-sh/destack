use std::borrow::Cow;

use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::infer::InferMode;
use crate::sema::{
    CandidateOutcome, Cause, CauseId, CauseKind, CheckFailure, CheckOutcome, CheckState,
    Expectation, Origin, REPORTED_REJECTIONS, Relation, RelationCheck, Settle, StoreTarget,
    TypeSubstitution, Value, ValueConversion, ValueUse, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// Callable signature accepted for an invocation.
#[derive(Debug, Clone)]
pub(in crate::sema) struct SignatureSelection {
    /// The callable type after substitution.
    pub(in crate::sema) callable: dir::GlobalTypeId,
    /// The selected parameters in declaration order.
    pub(in crate::sema) parameters: SmallVec<[ParameterSelection; 4]>,
    /// The prepared sources supplied to the selected signature.
    pub(in crate::sema) sources: Vec<dir::ArgumentSource>,
    /// The return type after substitution.
    pub(in crate::sema) return_type: dir::GlobalTypeId,
    /// The solved generic argument bindings.
    pub(in crate::sema) generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The solved region bindings, the callee's binders at this call.
    pub(in crate::sema) region_arguments: Vec<dir::GenericArgumentBinding>,
    /// The receiver adjustments the declared this selected.
    pub(in crate::sema) receiver_adjustments: Option<Vec<dir::ReceiverAdjustment>>,
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

/// One invocation constrained against a candidate signature.
struct CandidateTrial {
    /// The rejection the constraints produced, set when the candidate fails.
    rejection: Option<SignatureRejection>,
    /// Whether the expected result refused the substituted return.
    is_return_mismatch: bool,
    /// The receiver adjustments the declared `this` derived.
    receiver_adjustments: Option<Vec<dir::ReceiverAdjustment>>,
    /// The coercions selected for the supplied arguments.
    coercions: SmallVec<[(dir::GlobalNodeIdAny, dir::Coercion); 4]>,
}

/// The callable this selection settled on, with the outcome its site commits.
#[allow(clippy::large_enum_variant)]
pub(in crate::sema) enum OverloadSelection<'candidate, C> {
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

/// The value one callable argument supplies ahead of selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum ArgumentValue {
    /// One checked type, driving classification and conversion into its parameter.
    Typed(dir::GlobalTypeId),
    /// One contextually typed value, checked against each candidate's parameter.
    Contextual,
    /// One written value, checked only after selection; never rejects a candidate.
    Deferred,
}

/// Argument matched against one callable signature parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) struct CallableArgument {
    /// Source node used for origins and diagnostics.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The runtime source, including any selected spread iteration.
    pub(in crate::sema) argument: dir::ArgumentSource,
    /// The value this argument supplies.
    pub(in crate::sema) value: ArgumentValue,
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
    /// Bind the prepared arguments to the selected parameters.
    pub(in crate::sema) fn bind_arguments(
        &self,
        origin: Origin,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<Vec<dir::ArgumentBinding>> {
        let parameters = self
            .parameters
            .iter()
            .map(|selected| selected.parameter)
            .collect::<SmallVec<[_; 4]>>();

        check
            .bind_arguments(origin, &parameters, &self.sources)?
            .ok_or_else(|| CompilerError::Internal {
                message: "a selected signature has an invalid rest parameter".to_string(),
            })
    }

    /// Return a call resolution for one selected declaration-backed member.
    pub(in crate::sema) fn member_call(
        &self,
        mut receiver: dir::MemberReceiver,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
        key_receiver: Option<dir::GlobalTypeId>,
        arguments: Vec<dir::ArgumentBinding>,
    ) -> dir::Call {
        if let Some(adjustments) = &self.receiver_adjustments {
            receiver
                .adjusted_mut()
                .adjustments
                .extend(adjustments.iter().cloned());
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
            regions: self.region_arguments.clone(),
        }
    }
}

impl SignatureRejection {
    /// Return whether this rejection identifies a precise call failure.
    pub(in crate::sema) fn is_precise(&self) -> bool {
        !matches!(self, Self::Inapplicable)
    }
}

impl CheckState<'_> {
    /// Select one callable candidate and constrain the invocation against it.
    pub(in crate::sema) fn select_callable<'candidate, C>(
        &mut self,
        origin: Origin,
        candidates: &'candidate [C],
        rule: OverloadRule,
        candidate_type: impl Fn(&C) -> dir::GlobalTypeId,
        attempt: impl Fn(&mut Self, &C) -> CompilerResult<SignatureMatch>,
    ) -> CompilerResult<OverloadSelection<'candidate, C>> {
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
            SignatureMatch::Selected(signature) => OverloadSelection::Selected {
                position,
                candidate,
                signature,
                outcome: CheckOutcome::Holds,
            },
            // commit the invocation and fail it against the expected result
            SignatureMatch::ReturnMismatch(signature) => OverloadSelection::Selected {
                position,
                candidate,
                signature,
                outcome: CheckOutcome::Fails(CheckFailure::Relation),
            },
            // report a sole candidate's own invocation rejection and still commit
            SignatureMatch::Invalid { key, rejection } if is_single => {
                self.report_signature_rejection(origin, rejection)?;

                OverloadSelection::Selected {
                    position,
                    candidate,
                    signature: key,
                    outcome: CheckOutcome::Holds,
                }
            }
            // report a sole candidate's own precise refusal
            SignatureMatch::Inapplicable(rejection) if is_single && rejection.is_precise() => {
                self.report_signature_rejection(origin, rejection)?;

                OverloadSelection::Refused
            }
            // leave a refused candidate to the shared report
            SignatureMatch::Invalid { rejection, .. } | SignatureMatch::Inapplicable(rejection) => {
                let described =
                    self.format_signature_rejection(module, candidate_type(candidate), &rejection)?;
                rejections.push(described);
                rejections.truncate(REPORTED_REJECTIONS);

                OverloadSelection::Rejected(rejections)
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
    ) -> CompilerResult<Result<(usize, &'candidate C, Vec<String>), OverloadSelection<'candidate, C>>>
    {
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
                    return Ok(Err(OverloadSelection::Ambiguous));
                }
                (Verdict::Holds, OverloadRule::Exclusive) => selected = Some((position, candidate)),
                (Verdict::Ambiguous, OverloadRule::Ordered) => {
                    undecided.get_or_insert((position, candidate));
                }
                (Verdict::Ambiguous, OverloadRule::Exclusive) => {
                    return Ok(Err(OverloadSelection::Ambiguous));
                }
            }
        }

        // take the exclusive winner, else the first undecided ordered candidate
        Ok(match selected.or(undecided) {
            Some((position, candidate)) => Ok((position, candidate, rejections)),
            None => {
                rejections.truncate(REPORTED_REJECTIONS);

                Err(OverloadSelection::Rejected(rejections))
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

    /// Expand one substituted tuple rest into its positional parameters, the list itself otherwise.
    fn spread_tuple_rest_parameters<'p>(
        &mut self,
        origin: Origin,
        parameters: &'p [dir::FunctionParameterType],
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Cow<'p, [dir::FunctionParameterType]>> {
        // find the trailing rest parameter
        let Some(rest_index) = parameters.iter().position(|parameter| parameter.is_rest) else {
            return Ok(Cow::Borrowed(parameters));
        };

        // read the substituted rest as a closed tuple
        let rest = self.substitute_type(parameters[rest_index].ty, substitution)?;
        let rest = self.normalize(origin, rest)?;
        let dir::Type::Tuple(tuple) = self.ty(rest)? else {
            return Ok(Cow::Borrowed(parameters));
        };

        // rebuild positional parameters from the tuple elements
        let mut expanded = parameters[..rest_index].to_vec();
        for element in self.tuple_elements(rest.module_id, tuple.elements)? {
            expanded.push(dir::FunctionParameterType {
                name: None,
                ty: element.ty,
                is_optional: element.is_optional,
                is_rest: element.is_rest,
            });
        }
        expanded.extend(parameters[rest_index + 1..].iter().copied());

        Ok(Cow::Owned(expanded))
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
            parks: signature.parks,
            asynchrony: signature.asynchrony,
            template: None,
            arguments: dir::TypeListId::EMPTY,
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
    ) -> CompilerResult<Option<ParameterSelection>> {
        // substitute the declared type
        let parameter_type = self.substitute_type(parameter.ty, substitution)?;
        let parameter_type = self.shallow_resolve(parameter_type)?;

        // keep the collection type on a rest parameter and take its element per source
        let argument_type = if parameter.is_rest {
            let Some(element) = self.rest_element_type(origin, parameter_type)? else {
                return Ok(None);
            };

            element
        } else {
            parameter_type
        };

        // replace the declared type with the selected type
        let parameter = dir::FunctionParameterType {
            ty: parameter_type,
            ..parameter
        };

        // carry the selected parameter with its argument type
        Ok(Some(ParameterSelection {
            parameter,
            argument_type,
        }))
    }

    /// Select the pack constructor one rest parameter's collection requires.
    pub(in crate::sema) fn rest_pack_selection(
        &mut self,
        origin: Origin,
        rest: dir::GlobalTypeId,
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::InstanceKey>> {
        // slice parameters pack in place under any form
        let reduced = self.deeply_resolve(origin, rest)?;
        let collection = self.strip_form(origin, reduced)?;
        if matches!(self.ty(collection)?, dir::Type::Slice(_)) {
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
        let symbol = self.language_symbol(dir::LanguageItem::ArrayFromOwnedSlice)?;
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

        // read the element through the collection's form
        if let dir::Type::Form(form) = self.ty(reduced)? {
            let Some(element) = self.rest_element_type(origin, form.value)? else {
                return Ok(None);
            };

            return Ok(Some(element));
        }

        match self.ty(reduced)? {
            dir::Type::Slice(_) | dir::Type::Application(_) => self.sequence_element_type(reduced),
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
        receiver_parameter: Option<dir::GlobalTypeId>,
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
                    receiver_parameter,
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
                    receiver_parameter,
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
            receiver_parameter,
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
            None,
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
        receiver_parameter: Option<dir::GlobalTypeId>,
        arguments: &[CallableArgument],
        expectation: Option<Expectation>,
    ) -> CompilerResult<SignatureMatch> {
        // reject argument counts outside the accepted arity
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

        // bind this to the receiver's object before evaluating generic defaults
        let substitution = match receiver {
            Some(receiver) => {
                let object = self.strip_form(origin, receiver.ty)?;

                TypeSubstitution::default().with_receiver(object)
            }
            None => TypeSubstitution::default(),
        };

        // preserve generic bindings already selected by the callee
        let fixed = self.signature_arguments(signature_module, function.arguments)?;
        let substitution = substitution.with_carried(fixed)?.with_carried(carried)?;

        // read the parameters the signature declares
        let parameters = self.signature_generic_parameters(signature_module, function)?;

        // open the signature's own parameters
        let substitution =
            self.instantiate_parameters(origin, &parameters, type_arguments, substitution)?;
        let Some(mut substitution) = substitution else {
            return Ok(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            ));
        };

        // bind the owner parameters member lookup left open
        let mut owner = owner;
        if let Some(owner_symbol) = owner
            && let Some(template) = self.symbol_template(owner_symbol)?
        {
            // read the parameters the owner and its own template declare
            let mut parameters = self.generic_template_parameters(template)?;
            parameters.extend(self.owner_template_parameters(template)?);
            let is_nominal = !matches!(
                self.definition(owner_symbol)?.as_deref(),
                Some(dir::Definition::Extension(_) | dir::Definition::Interface(_))
            );

            // split a nominal owner's parameters by the ones this signature names
            if is_nominal {
                let named: SmallVec<[dir::GlobalTypeId; 8]> = signature_parameters
                    .iter()
                    .map(|parameter| parameter.ty)
                    .chain(function.return_type)
                    .chain(function.this_parameter)
                    .collect();
                let mut open = SmallVec::<[_; 4]>::new();
                let mut unnamed = SmallVec::<[_; 4]>::new();
                for parameter in parameters {
                    if substitution.argument(parameter).is_some() {
                        continue;
                    }
                    let parameter_type = self.generic_parameter_type(parameter)?;
                    let mut is_named = false;
                    for ty in &named {
                        if self.type_mentions(*ty, parameter_type)? {
                            is_named = true;
                            break;
                        }
                    }
                    match is_named {
                        true => open.push(parameter),
                        false => unnamed.push(parameter),
                    }
                }

                // drop an owner none of this signature's parameters name
                if open.is_empty() && !unnamed.is_empty() {
                    owner = None;
                }

                // bind each unnamed parameter to its default, else infer it at the call
                if !open.is_empty() {
                    for parameter in unnamed {
                        match self.require_generic_parameter(parameter)?.default {
                            Some(default) => {
                                let argument = self.substitute_type(default, &substitution)?;
                                substitution.bind(parameter, argument)?;
                            }
                            None => open.push(parameter),
                        }
                    }
                }

                parameters = open;
            }

            // open the owner parameters this signature still names
            if owner.is_some() {
                let opened = self.instantiate_parameters(origin, &parameters, &[], substitution)?;
                let Some(opened) = opened else {
                    return Ok(SignatureMatch::Inapplicable(
                        SignatureRejection::Inapplicable,
                    ));
                };

                substitution = opened;
            }
        }

        // point this at the applied extension target for bare type receivers
        if let Some(receiver_value) = receiver
            && matches!(
                self.resolved_ty(receiver_value.ty)?,
                dir::Type::Reference(_)
            )
            && let Some(owner) = owner
            && let Some(dir::Definition::Extension(extension)) = self.definition(owner)?.as_deref()
        {
            let target = extension.target.r#type();
            let applied = self.substitute_type(target, &substitution)?;
            substitution = substitution.with_receiver(applied);
        }

        // constrain every invocation relation before settling candidate inference
        let invocation = self.constrain_invocation(
            origin,
            function,
            signature_parameters,
            &parameters,
            &substitution,
            receiver,
            receiver_parameter,
            owner,
            function_return,
            expectation,
            arguments,
        )?;
        let Some(CandidateTrial {
            rejection,
            is_return_mismatch,
            receiver_adjustments,
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
            receiver_adjustments,
            arguments,
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
        receiver_parameter: Option<dir::GlobalTypeId>,
        owner: Option<dir::GlobalSymbolId>,
        function_return: Option<dir::GlobalTypeId>,
        expectation: Option<Expectation>,
        arguments: &[CallableArgument],
    ) -> CompilerResult<Option<CandidateTrial>> {
        // collect what the invocation decides about this candidate
        let mut is_return_mismatch = false;
        let mut receiver_adjustments = None;
        let mut coercions = SmallVec::new();
        // relate the receiver to the per-call parameter as written, else to the declared this
        if let (Some(receiver), Some(this_parameter)) =
            (receiver, receiver_parameter.or(function.this_parameter))
        {
            let this_parameter = if receiver_parameter.is_some() {
                this_parameter
            } else {
                self.substitute_type(this_parameter, substitution)?
            };
            match self.constrain_receiver(origin, receiver, this_parameter)? {
                Some(adjustments) => receiver_adjustments = Some(adjustments),
                None => {
                    let rejection = SignatureRejection::Receiver {
                        source: receiver.ty,
                        target: this_parameter,
                    };

                    return Ok(Some(CandidateTrial {
                        rejection: Some(rejection),
                        is_return_mismatch,
                        receiver_adjustments,
                        coercions,
                    }));
                }
            }
        }

        // spread a substituted tuple rest into positional parameters
        let signature_parameters =
            self.spread_tuple_rest_parameters(origin, signature_parameters, substitution)?;

        // reject argument tails past a fixed parameter count
        let has_rest = signature_parameters
            .iter()
            .any(|parameter| parameter.is_rest);
        if !has_rest && arguments.len() > signature_parameters.len() {
            return Ok(None);
        }

        // require every parameter to describe a valid argument type
        let mut selected = SmallVec::<[_; 4]>::new();
        for parameter in signature_parameters.iter() {
            let Some(parameter) = self.select_parameter(origin, *parameter, substitution)? else {
                return Ok(None);
            };
            selected.push(parameter);
        }

        // match arguments against the selected parameters
        let mut argument_parameters =
            SmallVec::<[(usize, &CallableArgument, dir::GlobalTypeId); 4]>::new();
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = selected.get(index).or_else(|| selected.last());
            let Some(parameter) = parameter else {
                return Ok(None);
            };

            // reject a spread outside a rest parameter
            if argument.is_spread && !parameter.parameter.is_rest {
                return Ok(None);
            }

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

                return Ok(Some(CandidateTrial {
                    rejection: Some(rejection),
                    is_return_mismatch,
                    receiver_adjustments,
                    coercions,
                }));
            }
        }

        // collect staged const parameters opened for this candidate
        let mut const_variables = SmallVec::<[dir::TypeVariableId; 2]>::new();
        for parameter in parameters {
            if self
                .generic_parameter(*parameter)?
                .is_some_and(|binding| binding.is_const && binding.memory_parameter().is_none())
                && let Some(instance) = substitution.argument(*parameter)
                && let Some(variable) = self.root_variable(instance)?
            {
                const_variables.push(variable);
            }
        }

        // convert the declared result to the contextual result type ahead of the arguments
        if let (Some(return_type), Some(expectation)) = (function_return, expectation)
            && expectation.contextual_target().is_some()
        {
            let return_type = self.substitute_type(return_type, substitution)?;
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

        // match every argument in order, refusing the candidate on the first rejection
        for entry in argument_parameters {
            if let Some(rejection) =
                self.constrain_argument(origin, entry, &const_variables, &mut coercions)?
            {
                return Ok(Some(CandidateTrial {
                    rejection: Some(rejection),
                    is_return_mismatch,
                    receiver_adjustments,
                    coercions,
                }));
            }
        }

        // answer with the accepted invocation and its adjustments
        Ok(Some(CandidateTrial {
            rejection: None,
            is_return_mismatch,
            receiver_adjustments,
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
        entry: (usize, &CallableArgument, dir::GlobalTypeId),
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

        // resolve the staged const parameters this argument binds
        let bound = self.type_variables(parameter_type)?;
        for variable in const_variables {
            if bound.contains(variable) && self.infer.variable(*variable)?.state.is_open() {
                self.settle_variables(&[*variable], Settle::All)?;
            }
        }

        Ok(None)
    }

    /// Build the selected payload for one instantiated signature.
    fn signature_selection(
        &mut self,
        origin: Origin,
        signature_module: ModuleId,
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        substitution: &TypeSubstitution,
        receiver_adjustments: Option<Vec<dir::ReceiverAdjustment>>,
        sources: &[CallableArgument],
    ) -> CompilerResult<SignatureSelection> {
        // substitute the declared result type
        let return_type = match function_return {
            Some(return_type) => self.substitute_type(return_type, substitution)?,
            None => self.intern_type(dir::Type::Void)?,
        };

        // select each parameter and erase the barriers inference left behind
        let declared = self.signature_parameters(signature_module, function.parameters)?;
        let declared = self.spread_tuple_rest_parameters(origin, declared, substitution)?;
        let mut parameters = SmallVec::<[_; 4]>::new();
        for &parameter in declared.iter() {
            let parameter = self
                .select_parameter(origin, parameter, substitution)?
                .ok_or_else(|| CompilerError::Internal {
                    message: "a selected signature has an invalid rest parameter".to_string(),
                })?;
            let parameter = ParameterSelection {
                parameter: dir::FunctionParameterType {
                    ty: self.erase_inference_barriers(parameter.parameter.ty)?,
                    ..parameter.parameter
                },
                argument_type: self.erase_inference_barriers(parameter.argument_type)?,
            };
            parameters.push(parameter);
        }

        // intern the instantiated signature alongside its solved arguments and regions
        let arguments = self.resolved_argument_bindings(&substitution.bindings)?;
        let regions = self.resolved_region_bindings(&substitution.bindings)?;
        let function_type =
            self.instantiate_signature_type(function, substitution, &parameters, return_type)?;

        // carry the instantiated signature with its slots
        Ok(SignatureSelection {
            callable: function_type,
            parameters,
            sources: sources
                .iter()
                .map(|argument| argument.argument.clone())
                .collect(),
            return_type,
            generic_arguments: arguments.to_vec(),
            region_arguments: regions,
            receiver_adjustments,
            coercions: SmallVec::new(),
        })
    }

    /// Match one supplied argument against one substituted parameter type.
    pub(in crate::sema) fn match_signature_argument(
        &mut self,
        origin: Origin,
        index: usize,
        argument: &CallableArgument,
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

        // defer a written value to the selected parameter
        if matches!(argument.value, ArgumentValue::Deferred) {
            return Ok(Ok(None));
        }

        // check a contextually typed value against the parameter, a spread as the array it builds
        let ArgumentValue::Typed(ty) = argument.value else {
            let parameter_type = if argument.is_spread {
                let array = self.array_type(parameter_type)?;

                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Owned,
                    value: array,
                }))?
            } else {
                parameter_type
            };
            let check = self.check_node(
                site,
                Expectation {
                    target: parameter_type,
                    relation,
                    cause,
                    use_: argument.use_,
                    mode,
                    store: StoreTarget::Exact,
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

        // constrain spread elements before their argument bindings select conversions
        let conversion = if argument.is_spread {
            let value = Value {
                ty,
                node: None,
                place: None,
                is_fresh: false,
            };
            let verdict = self.constrain_conversion(
                site,
                cause,
                relation,
                value,
                parameter_type,
                argument.use_,
            )?;
            let outcome = match verdict {
                Verdict::Holds => CheckOutcome::Holds,
                Verdict::Fails => CheckOutcome::Fails(CheckFailure::Relation),
                Verdict::Ambiguous => CheckOutcome::Pending,
            };

            ValueConversion {
                source: value.ty,
                target: parameter_type,
                outcome,
                coercion: None,
            }
        }
        // select the conversion of an individual argument
        else {
            let value = self.expression_value(site, ty)?;

            self.convert_value(
                site,
                cause,
                relation,
                value,
                parameter_type,
                argument.use_,
                mode,
            )?
        };

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
        self.sources.map_types(map)?;
        self.return_type.map_types(map)?;
        self.generic_arguments.map_types(map)?;
        self.region_arguments.map_types(map)?;
        self.receiver_adjustments.map_types(map)?;
        for (_, coercion) in &mut self.coercions {
            coercion.map_types(map)?;
        }

        Ok(())
    }
}
