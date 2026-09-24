use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    Answer, CallableArgument, Callee, CheckFailure, CheckOutcome, CheckState, Expectation,
    FlowSite, InferMode, Origin, OverloadRule, OverloadSelection, PlaceUse, Settle,
    SignatureFamily, SignatureMatch, SignatureSelection, Value, ValueCheck, ValueUse,
};
use crate::{CompilerError, CompilerResult};

/// The operation performed by one callable candidate.
#[derive(Debug)]
enum CallableTarget {
    /// Call a function-typed runtime value.
    Expression,
    /// Call one declaration-backed function.
    Symbol(dir::GlobalSymbolId),
    /// Invoke one erased interface signature.
    Signature {
        /// The selected call or construct signature.
        function: dir::DynamicFunction,
        /// The erased receiver value type.
        receiver: dir::GlobalTypeId,
        /// The interface constraint declaring the signature.
        constraint: dir::GlobalTypeId,
    },
    /// Construct one newtype.
    Newtype(dir::GlobalSymbolId),
}

/// One checked receiver and its durable member resolution.
struct CallableReceiver {
    /// The receiver stored in the selected call.
    resolution: dir::MemberReceiver,
    /// The checked receiver value used by signature selection.
    value: Value,
    /// The per-call receiver parameter a callable value is taken through.
    parameter: Option<dir::GlobalTypeId>,
    /// The base class application an inherited member is declared at.
    base: Option<dir::GlobalTypeId>,
}

/// One callable candidate collected from a callee node.
struct CallableCandidate {
    /// The operation performed after signature selection.
    target: CallableTarget,
    /// The generic scope whose arguments are carried into this call.
    generic_scope: Option<dir::GlobalSymbolId>,
    /// The receiver selected for member callees.
    receiver: Option<CallableReceiver>,
    /// The member space that selected this candidate, for member callees.
    member_space: Option<dir::MemberSpace>,
    /// The callable type.
    ty: dir::GlobalTypeId,
    /// The owner generic arguments already selected by member lookup.
    generic_arguments: Vec<dir::GenericArgumentBinding>,
}

impl CallableCandidate {
    /// Return the receiver after signature selection.
    fn selected_receiver(&self, signature: &SignatureSelection) -> Option<dir::MemberReceiver> {
        let mut receiver = self.receiver.as_ref()?.resolution.clone();
        if let Some(steps) = &signature.receiver_adjustments {
            receiver
                .adjusted_mut()
                .adjustments
                .extend(steps.iter().cloned());
        }

        Some(receiver)
    }

    /// Return the durable target for one declaration-backed function.
    fn function_target(
        &self,
        signature: &SignatureSelection,
        receiver: Option<dir::AdjustedReceiver>,
        key_receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::FunctionTarget> {
        let CallableTarget::Symbol(symbol) = &self.target else {
            return Err(CompilerError::Internal {
                message: format!("call target {:?} is not a symbol", self.target),
            });
        };

        Ok(dir::FunctionTarget {
            receiver,
            generic_scope: self.generic_scope,
            key: dir::InstanceKey::new(*symbol, signature.generic_arguments.clone())
                .with_receiver(key_receiver),
        })
    }
}

/// One runtime callee and its declaration alternatives.
struct CallableArm {
    /// The overloads declared for this runtime callee.
    overloads: SmallVec<[CallableCandidate; 2]>,
}

impl CheckState<'_> {
    /// Invoke a constructor value through its construct signatures.
    pub(in crate::sema) fn select_construct_call(
        &mut self,
        site: FlowSite,
        callee: FlowSite,
        ty: dir::GlobalTypeId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
        expectation: Option<Expectation>,
    ) -> CompilerResult<ValueCheck> {
        // collect construct overloads from every runtime alternative
        let origin = site.origin();
        let receiver = self.expression_value(callee, ty)?;
        let arms = self.callable_value_arms(origin, receiver, ty, SignatureFamily::Construct)?;
        if arms.iter().any(|arm| arm.overloads.is_empty()) {
            self.report_not_constructible(origin, ty, "'new'", "")?;

            return self.commit_rejected_call(site.node, expectation, None);
        }

        // read the generic arguments already checked by the construct expression
        let mut type_arguments = SmallVec::<[_; 4]>::new();
        for argument in generic_arguments {
            let argument = argument.into_global_any(site.node.module_id);
            type_arguments.push(self.require_node_type(argument)?);
        }

        let callee = callee.node.into_typed::<dir::Expression>().local_id;

        self.select_call_arms(site, callee, arguments, &arms, &type_arguments, expectation)
    }

    /// Collect callable candidates in declaration order from one callee node.
    fn callable_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        callee_site: FlowSite,
        is_optional: bool,
        expected: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<SmallVec<[CallableArm; 2]>>> {
        let callee_node = callee_site.node;
        let callee = callee_node.into_typed::<dir::Expression>().local_id;

        // construct an omitted head's newtype from the expected result
        let is_hole = matches!(
            self.module(module).view().get(callee),
            dir::Expression::Infer {
                form: dir::InferForm::Hole,
                name: None,
            }
        );
        if is_hole {
            return self.hole_callable_candidates(origin, callee_node, expected);
        }

        // use the recorded callee decision for reference and member callees
        let is_decided_callee = {
            let view = self.module(module).view();
            let expression = view.get(callee);

            expression.is_reference() || matches!(expression, dir::Expression::Member { .. })
        };

        // call every other callee through its function-typed value
        if !is_decided_callee {
            return self.value_callable_candidates(origin, callee_site, is_optional);
        }

        // resolve declaration callees through their recorded name decision
        if let Some(resolution) = self.name_decision(callee_node).cloned() {
            // classify a type qualifier through its checked operand
            if resolution.denoted_type().is_some() {
                return self.value_callable_candidates(origin, callee_site, is_optional);
            }

            let symbols = resolution
                .symbols()
                .iter()
                .copied()
                .collect::<SmallVec<[_; 2]>>();

            // call a value binding through its type, a declaration through its overloads
            let mut is_value_binding = true;
            for symbol in &symbols {
                is_value_binding &= self.symbol_kind(*symbol)?.is_binding();
            }
            if is_value_binding {
                return self.value_callable_candidates(origin, callee_site, is_optional);
            }

            // build one overload candidate per declared symbol
            let mut candidates = SmallVec::new();
            for symbol in symbols {
                // skip declarations removed by statically false gates
                if self.is_absent_symbol(symbol) {
                    continue;
                }

                // a newtype head constructs, every other declaration calls
                let ty = self.symbol_type(symbol)?;
                let is_newtype = matches!(self.symbol_kind(symbol)?, dir::SymbolKind::Newtype);
                let target = if is_newtype {
                    CallableTarget::Newtype(symbol)
                } else {
                    CallableTarget::Symbol(symbol)
                };

                // keep a declaration only while its type is invocable
                let ty = match &target {
                    CallableTarget::Newtype(_) => ty,
                    CallableTarget::Expression
                    | CallableTarget::Symbol(_)
                    | CallableTarget::Signature { .. } => {
                        let Some(ty) = self.callable_type(ty, SignatureFamily::Call)? else {
                            continue;
                        };

                        ty
                    }
                };

                candidates.push(CallableCandidate {
                    target,
                    generic_scope: None,
                    receiver: None,
                    member_space: None,
                    ty,
                    generic_arguments: Vec::new(),
                });
            }

            // declaration callees name exactly one runtime callee
            let mut arms = SmallVec::new();
            arms.push(CallableArm {
                overloads: candidates,
            });

            return Ok(Some(arms));
        }

        // fall back to the callee's own decision
        match self.decision(callee_node).cloned() {
            // build candidates from the recorded member decision
            Some(dir::Decision::Member(_)) => {
                let callee_expression = self.module(module).view().get(callee).clone();
                let dir::Expression::Member { left, .. } = callee_expression else {
                    return Err(CompilerError::Internal {
                        message: format!("member call callee {callee_node:?} is not a member"),
                    });
                };
                let receiver_site = self.visit_site(left.into_global_any(module))?;
                let (_, receiver_type) = self.infer_receiver(receiver_site)?;
                let receiver = self.expression_value(receiver_site, receiver_type)?;
                let Some(resolution) = self
                    .decisions(callee_node.module_id)
                    .member_decision(callee_node)
                    .cloned()
                else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "call callee {callee_node:?} has a member decision without a resolution"
                        ),
                    });
                };
                let candidates = match &resolution {
                    dir::OperationResolution::One(access) => self.member_access_candidates(
                        origin,
                        callee_site,
                        receiver,
                        access,
                        is_optional,
                    )?,
                    dir::OperationResolution::Union { arms, .. } => {
                        let mut runtime_arms = SmallVec::with_capacity(arms.len());
                        for access in arms {
                            let candidates = self.member_access_candidates(
                                origin,
                                callee_site,
                                receiver,
                                access,
                                is_optional,
                            )?;
                            runtime_arms.extend(candidates);
                        }

                        runtime_arms
                    }
                };

                Ok(Some(candidates))
            }
            // skip a callee that already reported
            Some(dir::Decision::Rejected | dir::Decision::Poisoned) => Ok(None),
            // fail loudly on a callee decided as anything else
            Some(other) => Err(CompilerError::Internal {
                message: format!("call callee {callee_node:?} decided as {other:?}"),
            }),
            // resolve or check an undecided callee in place, then re-dispatch
            None => {
                // decide any lexical or qualified reference before reading its value
                if self.decide_reference(callee_node)?.is_some() {
                    return self.callable_candidates(
                        origin,
                        module,
                        callee_site,
                        is_optional,
                        expected,
                    );
                }

                // decide a member callee by checking it in place, else call its value
                self.infer_node(callee_site, PlaceUse::Read, InferMode::Regular)?;
                match self.decision(callee_node) {
                    Some(_) => {
                        self.callable_candidates(origin, module, callee_site, is_optional, expected)
                    }
                    None => self.value_callable_candidates(origin, callee_site, is_optional),
                }
            }
        }
    }

    /// Build callable candidates from one singular member access.
    fn member_access_candidates(
        &mut self,
        origin: Origin,
        callee: FlowSite,
        receiver: Value,
        access: &dir::MemberAccess,
        is_optional: bool,
    ) -> CompilerResult<SmallVec<[CallableArm; 2]>> {
        let ty = self.select_chain_operand(origin, access.ty, is_optional)?;
        if let Some(arm) = self.member_target_candidates(origin, receiver, &access.target)? {
            let mut arms = SmallVec::new();
            arms.push(arm);

            return Ok(arms);
        }

        // stored and computed members call through their selected value
        let member = self.expression_value(callee, access.ty)?;
        let arms = self.callable_value_arms(origin, member, ty, SignatureFamily::Call)?;

        Ok(arms)
    }

    /// Build callable candidates from one singular member target.
    fn member_target_candidates(
        &mut self,
        origin: Origin,
        receiver: Value,
        target: &dir::MemberTarget,
    ) -> CompilerResult<Option<CallableArm>> {
        // collect the callable arms the target exposes
        match target {
            dir::MemberTarget::Symbol(candidate) => {
                let mut candidates = SmallVec::new();
                if let Some(candidate) = self.member_candidate(origin, receiver, candidate)? {
                    candidates.push(candidate);
                }

                Ok(Some(CallableArm {
                    overloads: candidates,
                }))
            }
            dir::MemberTarget::OverloadSet(targets) => {
                let mut candidates = SmallVec::new();
                for target in targets {
                    let dir::MemberTarget::Symbol(candidate) = target else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "member overload set contains a non-symbol target: {targets:?}"
                            ),
                        });
                    };
                    if let Some(candidate) = self.member_candidate(origin, receiver, candidate)? {
                        candidates.push(candidate);
                    }
                }

                Ok(Some(CallableArm {
                    overloads: candidates,
                }))
            }
            dir::MemberTarget::Intersection(targets) => {
                let mut candidates = SmallVec::new();
                for target in targets {
                    let Some(selected) = self.member_target_candidates(origin, receiver, target)?
                    else {
                        continue;
                    };
                    candidates.extend(selected.overloads);
                }

                Ok(Some(CallableArm {
                    overloads: candidates,
                }))
            }
            dir::MemberTarget::Projection { .. }
            | dir::MemberTarget::Field(_)
            | dir::MemberTarget::Call(_)
            | dir::MemberTarget::Index(_) => Ok(None),
        }
    }

    /// Build one callable candidate from a declaration-backed member target.
    fn member_candidate(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &dir::MemberCandidate,
    ) -> CompilerResult<Option<CallableCandidate>> {
        let Some(ty) = candidate.callable_type else {
            return Ok(None);
        };
        let target = self.member_operation(candidate)?;

        let symbol = match target {
            CallableTarget::Symbol(symbol) => Some(symbol),
            _ => None,
        };
        let generic_arguments = self.member_call_arguments(
            origin,
            symbol,
            &candidate.key.arguments,
            &candidate.regions,
            candidate.receiver.ty(),
        )?;

        // read the receiver each callable target carries
        let receiver = match &target {
            CallableTarget::Expression => None,
            _ => Some(CallableReceiver {
                resolution: candidate.receiver.clone(),
                value: Value {
                    ty: candidate.receiver.ty(),
                    ..receiver
                },
                parameter: None,
                base: self.inherited_base(origin, candidate)?,
            }),
        };
        let candidate = CallableCandidate {
            target,
            generic_scope: Some(candidate.owner),
            receiver,
            member_space: Some(candidate.space),
            ty,
            generic_arguments,
        };

        Ok(Some(candidate))
    }

    /// Return the base class application one inherited member is declared at.
    fn inherited_base(
        &mut self,
        origin: Origin,
        candidate: &dir::MemberCandidate,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // require a class owner other than the receiver's own class
        if !matches!(
            self.definition(candidate.owner)?.as_deref(),
            Some(dir::Definition::Class(_))
        ) {
            return Ok(None);
        }
        let object = self.strip_form(origin, candidate.receiver.ty())?;
        let symbol = match self.ty(object)? {
            dir::Type::Application(instance) => instance.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(None),
        };
        if symbol == candidate.owner {
            return Ok(None);
        }

        // apply the owner at the arguments the receiver matched through it
        let mut arguments = candidate.key.arguments.clone();
        arguments.extend(candidate.regions.iter().cloned());
        let key = dir::InstanceKey::new(candidate.owner, arguments);

        Ok(Some(self.instance_application(&key)?))
    }

    /// Return the arguments of one member call.
    pub(in crate::sema) fn member_call_arguments(
        &mut self,
        origin: Origin,
        symbol: Option<dir::GlobalSymbolId>,
        generic_arguments: &[dir::GenericArgumentBinding],
        region_arguments: &[dir::GenericArgumentBinding],
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let mut carried = generic_arguments.to_vec();
        carried.extend(region_arguments.iter().copied());
        if let Some(symbol) = symbol
            && let Some(binding) = self.receiver_region_binding(origin, symbol, receiver)?
        {
            carried.push(binding);
        }

        Ok(carried)
    }

    /// Return the binding of one member's receiver region at the receiver's region.
    fn receiver_region_binding(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GenericArgumentBinding>> {
        let declared = self.symbol_type(symbol)?;
        let Some(this) = self
            .signature_head(declared)?
            .and_then(|head| head.this_parameter)
        else {
            return Ok(None);
        };
        let Some(region) = self.form_chain(Origin::Symbol(symbol), this)?.region() else {
            return Ok(None);
        };
        let Some(held) = self.form_chain(origin, receiver)?.region() else {
            return Ok(None);
        };

        // bind a region parameter to the held region whole, a lifetime parameter to its extent
        let region = self.shallow_resolve(region)?;
        if let dir::Type::Parameter(parameter) = self.ty(region)? {
            return Ok(Some(dir::GenericArgumentBinding::new(parameter, held)));
        }
        let extent = self.region_extent(region)?;
        let dir::Type::Parameter(parameter) = self.ty(extent)? else {
            return Ok(None);
        };
        let held = self.region_extent(held)?;

        Ok(Some(dir::GenericArgumentBinding::new(parameter, held)))
    }

    /// Return the operation selected by one declaration-backed member.
    fn member_operation(
        &mut self,
        candidate: &dir::MemberCandidate,
    ) -> CompilerResult<CallableTarget> {
        // read the operation each member kind names
        match self.symbol_kind(candidate.key.symbol)? {
            dir::SymbolKind::AssociatedConst => Ok(CallableTarget::Expression),
            dir::SymbolKind::Function => Ok(CallableTarget::Symbol(candidate.key.symbol)),
            kind => Err(CompilerError::Internal {
                message: format!(
                    "callable member {:?} has non-callable symbol kind {kind:?}",
                    candidate.key.symbol
                ),
            }),
        }
    }

    /// Return the invocable form of one type.
    fn callable_type(
        &self,
        ty: dir::GlobalTypeId,
        family: SignatureFamily,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // select only the signature family the invocation requests
        let Some(signature) = self.signature_head(ty)? else {
            return Ok(None);
        };
        let is_construct = family == SignatureFamily::Construct;
        let callable = (signature.is_construct == is_construct).then_some(ty);

        Ok(callable)
    }

    /// Collect the callable candidate behind one function-typed callee value.
    fn value_callable_candidates(
        &mut self,
        origin: Origin,
        callee: FlowSite,
        is_optional: bool,
    ) -> CompilerResult<Option<SmallVec<[CallableArm; 2]>>> {
        let ty = self.infer_node_type(callee, PlaceUse::Read)?;

        // settle an open callee before dispatching through it
        if let Some(root) = self.root_variable(ty)? {
            self.settle_variables(&[root], Settle::All)?;
        }
        let ty = self.shallow_resolve(ty)?;
        let receiver = self.expression_value(callee, ty)?;
        let ty = self.strip_form(origin, ty)?;
        let ty = self.select_chain_operand(origin, ty, is_optional)?;
        let arms = self.callable_value_arms(origin, receiver, ty, SignatureFamily::Call)?;

        Ok(Some(arms))
    }

    /// Collect runtime callable arms from one value type.
    fn callable_value_arms(
        &mut self,
        origin: Origin,
        receiver: Value,
        ty: dir::GlobalTypeId,
        family: SignatureFamily,
    ) -> CompilerResult<SmallVec<[CallableArm; 2]>> {
        // distribute runtime union alternatives into independent arms
        let ty = self.shallow_resolve(ty)?;
        if let dir::Type::Union(union) = self.ty(ty)? {
            let elements: SmallVec<[_; 8]> = self.type_ids(ty.module_id, union.elements)?.into();
            let mut arms = SmallVec::with_capacity(elements.len());
            for element in elements {
                let nested = self.callable_value_arms(origin, receiver, element, family)?;
                arms.extend(nested);
            }

            return Ok(arms);
        }

        // intersections contribute overload alternatives to one runtime value
        let overloads = self.callable_value_overloads(origin, receiver, ty, family)?;
        let mut arms = SmallVec::new();
        arms.push(CallableArm { overloads });

        Ok(arms)
    }

    /// Collect overload alternatives from one runtime callable value.
    fn callable_value_overloads(
        &mut self,
        origin: Origin,
        receiver: Value,
        ty: dir::GlobalTypeId,
        family: SignatureFamily,
    ) -> CompilerResult<SmallVec<[CallableCandidate; 2]>> {
        // read the arm's callable object beneath its forms
        let ty = self.normalize(origin, ty)?;
        let ty = match self.ty(ty)? {
            dir::Type::Form(_) => {
                let chain = self.form_chain(origin, ty)?;

                self.normalize(origin, chain.base())?
            }
            _ => ty,
        };

        // flatten intersection signatures into one declaration alternative set
        if let dir::Type::Intersection(intersection) = self.ty(ty)? {
            let elements: SmallVec<[_; 8]> =
                self.type_ids(ty.module_id, intersection.elements)?.into();
            let mut overloads = SmallVec::with_capacity(elements.len());
            for element in elements {
                let nested = self.callable_value_overloads(origin, receiver, element, family)?;
                overloads.extend(nested);
            }

            return Ok(overloads);
        }

        // call an erased value through its constraint signatures
        if let Some(constraint) = self.erased_constraint(ty)? {
            let signatures = self.apparent_signatures(constraint, family)?;
            let generic_arguments = self.application_generic_argument_bindings(constraint)?;
            let mut overloads = SmallVec::with_capacity(signatures.len());
            for signature in signatures {
                let function = match family {
                    SignatureFamily::Call => dir::DynamicFunction::CallSignature(signature.source),
                    SignatureFamily::Construct => {
                        dir::DynamicFunction::ConstructSignature(signature.source)
                    }
                };
                overloads.push(CallableCandidate {
                    target: CallableTarget::Signature {
                        function,
                        receiver: ty,
                        constraint,
                    },
                    generic_scope: None,
                    receiver: None,
                    member_space: None,
                    ty: signature.ty,
                    generic_arguments: generic_arguments.clone(),
                });
            }

            return Ok(overloads);
        }

        // take a fat callable as its own receiver in one invocable candidate
        let mut overloads = SmallVec::new();
        if let Some(ty) = self.callable_type(ty, family)? {
            let receiver = match self.ty(ty)? {
                dir::Type::Function(function) => {
                    let mode = self.receiver_mode(function.receiver)?;
                    let callee = self.shallow_strip_forms(receiver.ty)?;
                    let parameter = self.call_receiver_parameter(origin, callee, mode)?;

                    Some(CallableReceiver {
                        resolution: dir::MemberReceiver::Direct(dir::AdjustedReceiver::direct(
                            receiver.ty,
                        )),
                        value: receiver,
                        parameter: Some(parameter),
                        base: None,
                    })
                }
                _ => None,
            };
            overloads.push(CallableCandidate {
                target: CallableTarget::Expression,
                generic_scope: None,
                receiver,
                member_space: None,
                ty,
                generic_arguments: Vec::new(),
            });
        }

        Ok(overloads)
    }

    /// Match one callable candidate against collected arguments.
    fn match_call(
        &mut self,
        origin: Origin,
        candidate: &CallableCandidate,
        arguments: &[CallableArgument],
        type_arguments: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<SignatureMatch> {
        let matched = self.match_callable(
            origin,
            candidate.ty,
            candidate.generic_scope,
            candidate.receiver.as_ref().map(|receiver| receiver.value),
            candidate
                .receiver
                .as_ref()
                .and_then(|receiver| receiver.parameter),
            &candidate.generic_arguments,
            type_arguments,
            arguments,
            expectation,
        )?;

        Ok(matched)
    }

    /// Build one kept call attempt from an unmatched candidate.
    fn attempted_call(
        &mut self,
        origin: Origin,
        candidate: &CallableCandidate,
        arguments: &[CallableArgument],
    ) -> CompilerResult<Option<dir::Call>> {
        // name the callable the candidate reached for
        let target = match &candidate.target {
            CallableTarget::Symbol(symbol) => dir::CallableTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: None,
                    generic_scope: candidate.generic_scope,
                    key: dir::InstanceKey::new(*symbol, candidate.generic_arguments.clone()),
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            CallableTarget::Expression => dir::CallableTarget::Expression {
                generic_arguments: Vec::new(),
            },
            _ => return Ok(None),
        };

        let sources = arguments
            .iter()
            .map(|argument| argument.argument.clone())
            .collect::<Vec<_>>();

        // bind the authored arguments against the declared parameters the candidate reached for
        let signature = self.callable_signature_type(origin, candidate.ty)?;
        let (arguments, return_type) = match signature {
            // bind against the parameters the signature declares
            Some((signature_type, signature)) => {
                let parameters =
                    self.signature_parameters(signature_type.module_id, signature.parameters)?;
                let Some(arguments) = self.bind_arguments(origin, parameters, &sources)? else {
                    return Ok(None);
                };
                let return_type = match signature.return_type {
                    Some(return_type) => return_type,
                    None => self.intern_type(dir::Type::Error)?,
                };

                (arguments, return_type)
            }
            // leave a callable without a signature unbound
            None => (Vec::new(), self.intern_type(dir::Type::Error)?),
        };

        Ok(Some(dir::Call {
            regions: Vec::new(),
            target,
            callable_type: candidate.ty,
            arguments,
            return_type,
        }))
    }

    /// Return the attempt a sole candidate leaves behind, an overload set leaving none.
    fn sole_attempted_call(
        &mut self,
        origin: Origin,
        asked: &[CallableCandidate],
        arguments: &[CallableArgument],
    ) -> CompilerResult<Option<dir::Call>> {
        match asked {
            [single] => self.attempted_call(origin, single, arguments),
            _ => Ok(None),
        }
    }

    /// Commit one rejected call node, keeping its best attempt when any.
    pub(in crate::sema) fn commit_rejected_call(
        &mut self,
        node: dir::GlobalNodeIdAny,
        expectation: Option<Expectation>,
        attempt: Option<dir::Call>,
    ) -> CompilerResult<ValueCheck> {
        // keep the call the node reached for, or commit a bare rejection
        match attempt {
            Some(attempt) => {
                self.commit_decision(node, dir::Decision::Attempted(Box::new(attempt)))?
            }
            None => self.commit_decision(node, dir::Decision::Rejected)?,
        }

        // fail the node against its expectation, or against itself
        let source = self.commit_error_node(node)?;
        let target = expectation.map_or(source, |expectation| expectation.target);

        Ok(ValueCheck {
            source,
            outcome: CheckOutcome::Fails(CheckFailure::Relation),
            target,
        })
    }

    /// Poison one call whose operand already reported an error.
    pub(in crate::sema) fn poison_call(
        &mut self,
        node: dir::GlobalNodeIdAny,
        expectation: Option<Expectation>,
    ) -> CompilerResult<ValueCheck> {
        let source = self.poison_node(node)?;
        let target = expectation.map_or(source, |expectation| expectation.target);

        Ok(ValueCheck {
            source,
            outcome: CheckOutcome::Fails(CheckFailure::Relation),
            target,
        })
    }

    /// Select the callable meaning of one call node.
    pub(in crate::sema) fn select_call(
        &mut self,
        site: FlowSite,
        callee: dir::LocalNodeId<dir::Expression>,
        generic_argument_nodes: &[dir::LocalNodeId<dir::GenericArgument>],
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        is_optional: bool,
        expectation: Option<Expectation>,
    ) -> CompilerResult<ValueCheck> {
        let node = site.node;
        let module = node.module_id;
        let origin = site.origin();

        // walk the written generic arguments of the call node
        self.walk_body_generic_arguments(module, generic_argument_nodes)?;

        // walk the argument decorators, keeping the statically present arguments
        let argument_nodes = self.walk_body_arguments(module, argument_nodes)?;
        let argument_nodes = argument_nodes.as_slice();

        // decide identifier and receiver arguments ahead of the candidates
        for argument in argument_nodes {
            if let Some(value) = self.module(module).view().get(*argument).value() {
                let value_node = value.into_global_any(module);
                self.decide_reference(value_node)?;
                if matches!(self.module(module).view().get(value), dir::Expression::This)
                    && self.decision(value_node).is_none()
                {
                    self.commit_active_receiver_decision(value_node, dir::ReceiverKind::This)?;
                }
            }
        }

        // collect the written generic argument types
        let mut type_arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in generic_argument_nodes {
            let argument = argument.into_global_any(module);
            let ty = self.require_node_type(argument)?;
            type_arguments.push(ty);
        }

        // select the base class constructor for a super callee
        if matches!(
            self.module(module).view().get(callee),
            dir::Expression::Super
        ) {
            return self.select_super_construct(site, callee, argument_nodes);
        }

        // visit the callee at its own site
        let callee_site = self.visit_site(callee.into_global_any(module))?;

        // collect callable candidates from the callee
        let expected = expectation.and_then(Expectation::contextual_target);
        let Some(callees) =
            self.callable_candidates(origin, module, callee_site, is_optional, expected)?
        else {
            // rejected callees already reported their own diagnostic; the arguments still check
            self.infer_argument_types(site, argument_nodes)?;

            return self.commit_rejected_call(node, expectation, None);
        };

        // decide the callee reference before selecting its call
        self.decide_reference(callee.into_global_any(module))?;

        // select the nominal constructor for a newtype target
        if let [arm] = callees.as_slice()
            && let [
                CallableCandidate {
                    target: CallableTarget::Newtype(symbol),
                    ..
                },
            ] = arm.overloads.as_slice()
        {
            // read a newtype head as its declaration reference
            let symbol = *symbol;
            let callee_node = callee.into_global_any(module);
            let reference =
                self.intern_type(dir::Type::Reference(dir::TypeReference::new(symbol)))?;
            self.commit_node_type(callee_node, reference)?;

            return self.select_newtype_construct(
                site,
                node,
                origin,
                symbol,
                argument_nodes,
                &type_arguments,
                expectation,
            );
        }

        // poison or reject a callee with a runtime arm that exposes no callable
        if callees.iter().any(|arm| arm.overloads.is_empty()) {
            let callee_type = self.require_node_type(callee_site.node)?;
            let callee_type = self.flow_type_at(callee_site, callee_type)?;
            let callee_type = self.resolve_structurally(site, callee_type)?;
            if self.has_error_operand(&[callee_type])? {
                return self.poison_call(node, expectation);
            }
            self.report_not_callable(origin, callee_type)?;

            return self.commit_rejected_call(node, expectation, None);
        }

        // select one call per runtime arm
        self.select_call_arms(
            site,
            callee,
            argument_nodes,
            &callees,
            &type_arguments,
            expectation,
        )
    }

    /// Name the newtype an omitted call head constructs from the expected result.
    fn hole_callable_candidates(
        &mut self,
        origin: Origin,
        callee: dir::GlobalNodeIdAny,
        expected: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<SmallVec<[CallableArm; 2]>>> {
        // require an expected result naming one newtype
        let Some(target) = expected else {
            self.report_cannot_infer_node(callee)?;
            self.commit_error_node(callee)?;

            return Ok(None);
        };
        // name the newtype the target constructs, an object beneath its handle
        let object = self.normalize(origin, target)?;
        let symbol = match self.ty(object)? {
            dir::Type::Application(instance)
                if matches!(self.symbol_kind(instance.symbol)?, dir::SymbolKind::Newtype) =>
            {
                instance.symbol
            }
            _ => {
                self.report_invalid_inferred_construct_target(origin, target)?;
                self.commit_error_node(callee)?;

                return Ok(None);
            }
        };

        // construct that newtype through the head
        let ty = self.symbol_type(symbol)?;
        let mut arms = SmallVec::<[CallableArm; 2]>::new();
        arms.push(CallableArm {
            overloads: SmallVec::new(),
        });
        arms[0].overloads.push(CallableCandidate {
            target: CallableTarget::Newtype(symbol),
            generic_scope: None,
            receiver: None,
            member_space: None,
            ty,
            generic_arguments: Vec::new(),
        });

        Ok(Some(arms))
    }

    /// Select one call through every runtime callee arm, joining the arms as one decision.
    fn select_call_arms(
        &mut self,
        site: FlowSite,
        callee: dir::LocalNodeId<dir::Expression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        arms: &[CallableArm],
        type_arguments: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<ValueCheck> {
        let node = site.node;
        let module = node.module_id;
        let origin = site.origin();

        // read the call's argument values once, ahead of every candidate
        let arguments = self.callable_arguments(module, argument_nodes, ValueUse::Argument)?;

        // pose the selection goal of a sole declared callee over the committed argument types
        let expected = expectation.and_then(Expectation::contextual_target);
        let is_pending = arguments
            .iter()
            .any(|argument| self.lambdas.contains_key(&argument.source));
        // key a single closed arm by its canonical goal
        let goal = match arms {
            [arm] if !is_pending => match arm.overloads.first() {
                Some(
                    first @ CallableCandidate {
                        target: CallableTarget::Symbol(symbol),
                        ..
                    },
                ) => {
                    let mut operands = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                    operands.push(first.ty);
                    operands.extend(dir::GenericArgumentBinding::values(
                        &first.generic_arguments,
                    ));
                    operands.extend(type_arguments.iter().copied());
                    let committed = arguments
                        .iter()
                        .map(|argument| self.committed_node_type(argument.source))
                        .collect::<Option<SmallVec<[dir::GlobalTypeId; 8]>>>();
                    match committed {
                        Some(committed) => {
                            operands.extend(committed);
                            self.selection_goal(
                                origin,
                                Callee::Symbol(*symbol),
                                expected,
                                &operands,
                            )?
                        }
                        None => None,
                    }
                }
                _ => None,
            },
            _ => None,
        };
        let remembered = match &goal {
            Some(goal) => match self.answers.get(goal) {
                Some(Answer::Selection(position)) if *position < arms[0].overloads.len() => {
                    Some(*position)
                }
                _ => None,
            },
            None => None,
        };

        // select one overload per runtime arm, preferring a remembered choice
        let mut selected = SmallVec::<[(&CallableCandidate, SignatureSelection); 2]>::new();
        let mut outcome = CheckOutcome::Holds;
        for arm in arms {
            let asked = match remembered {
                Some(position) => &arm.overloads[position..=position],
                None => arm.overloads.as_slice(),
            };
            let selection = self.select_callable(
                origin,
                asked,
                OverloadRule::Ordered,
                |candidate| candidate.ty,
                |state, candidate| {
                    state.match_call(origin, candidate, &arguments, type_arguments, expectation)
                },
            )?;
            match selection {
                // keep the overload this arm accepted
                OverloadSelection::Selected {
                    position,
                    candidate,
                    signature,
                    outcome: arm_outcome,
                } => {
                    // remember the winning overload for equal asks
                    if arm_outcome == CheckOutcome::Holds
                        && let Some(goal) = &goal
                    {
                        let position = remembered.unwrap_or(position);
                        self.answers
                            .entry(goal.clone())
                            .or_insert(Answer::Selection(position));
                    }

                    // pass a failing arm's outcome to the joined call
                    if arm_outcome != CheckOutcome::Holds {
                        outcome = arm_outcome;
                    }

                    selected.push((candidate, signature));
                }
                // keep the refused candidate so downstream passes keep a target
                OverloadSelection::Refused => {
                    let attempt = self.sole_attempted_call(origin, asked, &arguments)?;

                    return self.commit_rejected_call(node, expectation, attempt);
                }
                // report that no candidate matched the arguments
                OverloadSelection::Rejected(rejections) => {
                    let types = self.infer_argument_types(site, argument_nodes)?;
                    self.report_no_matching_call(origin, &types, &rejections)?;
                    let attempt = self.sole_attempted_call(origin, asked, &arguments)?;

                    return self.commit_rejected_call(node, expectation, attempt);
                }
                // fail on an ambiguity an ordered rule settles
                OverloadSelection::Ambiguous => {
                    return Err(CompilerError::Internal {
                        message: "ordered call selection reported an ambiguous overload"
                            .to_string(),
                    });
                }
            }
        }

        // type the arguments a refused candidate left unchecked
        self.infer_argument_types(site, argument_nodes)?;

        // require one argument conversion across every runtime arm, then commit them
        let mut coercions = SmallVec::<[(dir::GlobalNodeIdAny, dir::Coercion); 4]>::new();
        for (_, signature) in &selected {
            for (source, coercion) in &signature.coercions {
                match coercions.iter().find(|(known, _)| known == source) {
                    Some((_, known)) if known != coercion => {
                        let types = self.infer_argument_types(site, argument_nodes)?;
                        let rejection =
                            "runtime call arms require incompatible argument conversions";
                        self.report_no_matching_call(origin, &types, &[rejection.to_string()])?;

                        return self.commit_rejected_call(node, expectation, None);
                    }
                    Some(_) => {}
                    None => coercions.push((*source, coercion.clone())),
                }
            }
        }
        for (source, coercion) in coercions {
            self.commit_coercion(source, coercion)?;
        }

        // type a bare declaration head as its selected overload
        if let [(candidate, signature)] = selected.as_slice()
            && matches!(candidate.target, CallableTarget::Symbol(_))
            && candidate.member_space.is_none()
        {
            let callee = callee.into_global_any(module);
            self.commit_node_type(callee, signature.callable)?;
        }

        // build one call decision per runtime arm and join the arms
        let call_origin = Origin::Node(node, None);
        let mut calls = Vec::with_capacity(selected.len());
        let mut returns = SmallVec::<[_; 4]>::new();
        for (candidate, signature) in &selected {
            let call = self.call_decision(call_origin, candidate, signature)?;
            returns.push(call.return_type);
            calls.push(call);
        }
        // join the selected calls into one resolution, arms agreeing on one call as that call
        let resolution = match calls.as_slice() {
            [first, rest @ ..] if rest.iter().all(|call| call == first) => {
                dir::OperationResolution::One(calls.remove(0))
            }
            _ => dir::OperationResolution::Union {
                ty: self.normalized_union_type(returns)?,
                arms: calls,
            },
        };
        let source = self.commit_call_selection(node, resolution)?;
        let target = expectation.map_or(source, |expectation| expectation.target);

        Ok(ValueCheck {
            source,
            outcome,
            target,
        })
    }

    /// Build one expression or symbol call resolution.
    fn call_decision(
        &mut self,
        origin: Origin,
        candidate: &CallableCandidate,
        signature: &SignatureSelection,
    ) -> CompilerResult<dir::Call> {
        let arguments = signature.bind_arguments(origin, self)?;
        let return_type = signature.return_type;
        // name the target each candidate calls through
        let target = match &candidate.target {
            // call a runtime value through its own callable type
            CallableTarget::Expression => dir::CallableTarget::Expression {
                generic_arguments: signature.generic_arguments.clone(),
            },
            // dispatch erased signature calls through the callee's own table
            CallableTarget::Signature {
                function,
                receiver,
                constraint,
            } => dir::CallableTarget::Dynamic {
                dispatch: dir::DynamicDispatch {
                    receiver: dir::AdjustedReceiver::direct(*receiver),
                    constraint: *constraint,
                },
                function: *function,
                // record the signature's own bindings, dropping the constraint's parameters
                generic_arguments: signature
                    .generic_arguments
                    .iter()
                    .filter(|binding| {
                        !candidate
                            .generic_arguments
                            .iter()
                            .any(|carried| carried.parameter == binding.parameter)
                    })
                    .cloned()
                    .collect(),
            },
            // drop the receiver a static member selection went through
            CallableTarget::Symbol(symbol) => {
                let key_receiver = match candidate.generic_scope {
                    Some(owner) => {
                        let called_on = candidate
                            .receiver
                            .as_ref()
                            .filter(|_| candidate.member_space == Some(dir::MemberSpace::Static))
                            .map(|receiver| receiver.value.ty);

                        self.interface_member_receiver(owner, signature.callable, called_on)?
                    }
                    None => None,
                };

                match candidate
                    .selected_receiver(signature)
                    .filter(|_| candidate.member_space != Some(dir::MemberSpace::Static))
                {
                    Some(dir::MemberReceiver::Direct(mut receiver)) => {
                        // reinterpret the receiver at the base class an inherited member names
                        if let Some(base) = candidate
                            .receiver
                            .as_ref()
                            .and_then(|receiver| receiver.base)
                        {
                            let ty = self.replace_form_value(origin, receiver.ty(), base)?;
                            receiver
                                .adjustments
                                .push(dir::ReceiverAdjustment::Upcast { ty });
                        }

                        dir::CallableTarget::Symbol {
                            function: candidate.function_target(
                                signature,
                                Some(receiver),
                                key_receiver,
                            )?,
                            dispatch: dir::FunctionDispatch::Direct,
                        }
                    }
                    Some(dir::MemberReceiver::Dynamic(dispatch)) => dir::CallableTarget::Dynamic {
                        dispatch,
                        function: dir::DynamicFunction::Symbol(*symbol),
                        generic_arguments: signature.generic_arguments.clone(),
                    },
                    None => dir::CallableTarget::Symbol {
                        function: candidate.function_target(signature, None, key_receiver)?,
                        dispatch: dir::FunctionDispatch::Direct,
                    },
                }
            }
            // fail loudly on a constructor that reached ordinary call resolution
            CallableTarget::Newtype(_) => {
                return Err(CompilerError::Internal {
                    message: "constructor target reached call resolution".to_string(),
                });
            }
        };
        let resolution = dir::Call {
            regions: signature.region_arguments.clone(),
            target,
            callable_type: signature.callable,
            arguments,
            return_type,
        };

        Ok(resolution)
    }

    /// Commit one accepted call selection.
    fn commit_call_selection(
        &mut self,
        node: dir::GlobalNodeIdAny,
        resolution: dir::CallDecision,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // a written call to a parking callable needs a body that parks itself
        let mut parks = false;
        for call in resolution.arms() {
            parks |= self.signature_parks(call.callable_type)?;
        }
        if parks && !self.current_function_parks()? {
            self.report_park_outside_protocol(node.module_id, node.local_id);
        }

        let return_type = resolution.return_type();
        self.commit_decision(node, dir::Decision::Call(resolution))?;
        self.commit_node_type(node, return_type)?;

        Ok(return_type)
    }
}
