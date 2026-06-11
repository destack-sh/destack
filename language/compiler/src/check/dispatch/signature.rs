use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CallArgument, CallFailure, CallableDispatch, CallableTarget, CheckState, Dependency,
    FunctionTerm, GenericArgument, GenericInstance, Origin, ShapeMember, TermId, TypeOperand,
    TypeTerm, VariableId,
};

/// Callable candidate considered by dispatch selection.
pub(in crate::check) struct CallableCandidate {
    /// The module whose type context owns the candidate.
    pub(in crate::check) module: ModuleId,
    /// The resolved declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The callable type operand to inspect.
    pub(in crate::check) ty: TypeOperand,
    /// The already resolved generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The final callable target if this candidate is selected.
    pub(in crate::check) target: CallableTarget,
}

/// Callable function signature extracted from a type term.
pub(in crate::check) enum CallableSignature {
    /// Signature extraction is waiting for dependency.
    Pending(SmallVec<[Dependency; 2]>),
    /// The term is not callable.
    Absent,
    /// The term has one function type.
    Present(TermId<FunctionTerm>),
}

/// Applicability result for one callable signature.
pub(in crate::check) enum CallableApplicability {
    /// Callable signature is waiting for dependency.
    Pending(SmallVec<[Dependency; 2]>),
    /// Callable signature rejected the arguments.
    Rejected(CallFailure),
    /// One callable signature accepts the arguments.
    Applicable {
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
        /// The instantiated function signature.
        function: FunctionTerm,
    },
}

impl CheckState<'_> {
    /// Select the first applicable callable candidate.
    pub(in crate::check) fn select_callable_candidate(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        candidates: &[CallableCandidate],
        generic_arguments: &[GenericArgument],
        arguments: &[CallArgument],
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let mut rejected_signature_count = 0;

        // run non-overloaded calls without speculative state
        if let [candidate] = candidates {
            return self.select_one_callable_candidate(
                origin,
                source,
                candidate,
                generic_arguments,
                arguments,
                expected,
            );
        }

        // choose the first compatible declaration order candidate
        for candidate in candidates {
            let probe = self.inference.begin_probe();
            let result = self.select_one_callable_candidate(
                origin,
                source,
                candidate,
                generic_arguments,
                arguments,
                expected,
            )?;

            match result {
                CallableDispatch::CallSelected { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
                CallableDispatch::Pending(blockers) => {
                    let has_external_dependency =
                        probe.has_external_dependency(&blockers, &self.inference);
                    self.inference.drop_probe(probe)?;

                    // block declaration order only on state outside this candidate
                    if has_external_dependency {
                        return Ok(CallableDispatch::Pending(blockers));
                    }
                }
                CallableDispatch::CallRejected(CallFailure::ArgumentType { .. }) => {
                    self.inference.drop_probe(probe)?;
                    rejected_signature_count += 1;
                }
                CallableDispatch::CallRejected(CallFailure::NoMatch) => {
                    self.inference.drop_probe(probe)?;
                    rejected_signature_count += 1;
                }
                CallableDispatch::CallRejected(CallFailure::NotCallable) => {
                    self.inference.drop_probe(probe)?;
                }
                CallableDispatch::ConstructRejected(_) => {
                    self.inference.drop_probe(probe)?;
                }
                CallableDispatch::ConstructSelected { .. } => {
                    self.inference.drop_probe(probe)?;
                }
                CallableDispatch::Invalid => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
            }
        }

        // distinguish rejected callable overloads from non callable values
        if rejected_signature_count > 0 {
            Ok(CallableDispatch::call_rejected(CallFailure::NoMatch))
        } else {
            Ok(CallableDispatch::call_rejected(CallFailure::NotCallable))
        }
    }

    /// Select one callable candidate inside the active inference probe.
    fn select_one_callable_candidate(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        candidate: &CallableCandidate,
        generic_arguments: &[GenericArgument],
        arguments: &[CallArgument],
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let reduction = self.reduce_type_operand(origin, candidate.ty)?;
        let Answer::Ready(term) = reduction else {
            return Ok(CallableDispatch::pending(candidate.ty.dependencies(self)));
        };
        let function = match self.callable_signature(origin, candidate.module, term)? {
            CallableSignature::Pending(blockers) => return Ok(CallableDispatch::Pending(blockers)),
            CallableSignature::Absent => {
                return Ok(CallableDispatch::call_rejected(CallFailure::NotCallable));
            }
            CallableSignature::Present(function) => function,
        };

        let target = candidate.target.clone();
        let signature = self.select_callable_signature(
            origin,
            candidate.module,
            source,
            Some(candidate.symbol),
            candidate.instance.clone(),
            function,
            generic_arguments,
            arguments,
            expected,
            target.receiver(),
        )?;

        Ok(target.into_callable_dispatch(signature))
    }

    /// Return the callable signature represented by one type operand.
    pub(in crate::check) fn callable_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        operand: TypeOperand,
    ) -> CompilerResult<CallableSignature> {
        let Answer::Ready(operand) = self.reduce_type_operand(origin, operand)? else {
            return Ok(CallableSignature::Pending(operand.dependencies(self)));
        };
        let operand = self.contextual_type_operand(origin, operand)?;
        let Some(term) = self.type_operand_term_id(operand)? else {
            return Ok(CallableSignature::Pending(operand.dependencies(self)));
        };
        let callable =
            match self.inference.term(term) {
                TypeTerm::Function(function) => CallableSignature::Present(*function),
                TypeTerm::Type(ty) => {
                    let term = self.import_type_term(Origin::Type(*ty), *ty)?;

                    match term {
                        TypeTerm::Function(function) => CallableSignature::Present(function),
                        _ => CallableSignature::Absent,
                    }
                }
                TypeTerm::Reference {
                    origin: _,
                    symbol: _,
                    arguments: _,
                } => CallableSignature::Absent,
                TypeTerm::Shape(shape) => {
                    let call_signature = self.inference.term(*shape).members.iter().find_map(
                        |member| match member {
                            ShapeMember::CallSignature { ty } => Some(*ty),
                            _ => None,
                        },
                    );

                    match call_signature {
                        Some(ty) => self.callable_signature(origin, module, ty)?,
                        None => CallableSignature::Absent,
                    }
                }
                _ => CallableSignature::Absent,
            };

        Ok(callable)
    }
}
