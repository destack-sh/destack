use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CallDecision, CallFailure, CallResolution, CallTargetResolution, CallableDispatch,
    CheckEvent, CheckState, Condition, ConstructDecision, ConstructFailure, ConstructResolution,
    ConstructTargetResolution, FunctionTerm, GenericArgument, MemberDecision, MemberReceiver,
    MemberResolution, MemberTargetResolution, Origin, SubstitutionSet, TermId,
    TraceCallableDispatch, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm, VariableId,
};

/// Runtime argument supplied to a call-like expression.
///
/// ```ds
/// value
/// ...values
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct CallArgument {
    /// The source node that introduced this argument.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The argument expression type.
    pub(in crate::check) ty: TypeOperand,
    /// Whether this argument spreads a sequence value.
    pub(in crate::check) is_spread: bool,
}

impl CallArgument {
    /// Substitute generic arguments through this call argument.
    pub(in crate::check) fn substitute(
        self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            source: self.source,
            ty: state.substitute_type_operand(module, substitution, self.ty)?,
            is_spread: self.is_spread,
        })
    }
}

/// Runtime call expression term.
///
/// ```ds
/// format(value)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallTerm {
    /// The source call expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The selected call callee.
    pub(in crate::check) callee: CallCallee,
    /// The explicit call generic arguments.
    pub(in crate::check) generic_arguments: SmallVec<[GenericArgument; 2]>,
    /// The runtime call arguments.
    pub(in crate::check) arguments: SmallVec<[CallArgument; 4]>,
}

/// Runtime expression selected as a call callee.
///
/// Examples:
/// ```ds
/// callback(value)
/// print(value)
/// receiver.method(value)
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum CallCallee {
    /// Arbitrary callable expression.
    ///
    /// Examples:
    /// ```ds
    /// callback(value)
    /// ```
    Expression(TypeOperand),
    /// Symbol-backed callable reference expression.
    ///
    /// Examples:
    /// ```ds
    /// print(value)
    /// ```
    Reference {
        /// The reference expression type.
        value: TypeOperand,
        /// The resolved referenced symbol.
        symbol: dir::GlobalSymbolId,
    },
    /// Member projection callable.
    ///
    /// Examples:
    /// ```ds
    /// receiver.method(value)
    /// ```
    Member(TermId<MemberCallTerm>),
}

/// Member projection used by a runtime call.
///
/// ```ds
/// value.toString()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberCallTerm {
    /// The origin that introduced this member projection.
    pub(in crate::check) origin: MemberProjectionOrigin,
    /// The member receiver.
    pub(in crate::check) receiver: MemberReceiver,
    /// The selected member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The applied static arguments.
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 2]>,
}

/// Origin of a member projection used as a call callee.
///
/// Examples:
/// ```ds
/// receiver.method(value)
/// receiver[index]()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MemberProjectionOrigin {
    /// Source member call expression.
    ///
    /// Examples:
    /// ```ds
    /// receiver.method(value)
    /// ```
    Expression {
        /// The source member expression.
        source: dir::GlobalNodeIdAny,
    },
    /// Compiler-lowered protocol call.
    ///
    /// Examples:
    /// ```ds
    /// receiver[index]()
    /// ```
    Protocol {
        /// The protocol that must own the resolved method.
        protocol: MemberProtocol,
    },
}

/// Protocol contract required for a member resolution.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberProtocol {
    /// The protocol language item.
    pub(in crate::check) item: dir::LanguageItem,
    /// The required protocol arguments.
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 2]>,
}

impl CheckState<'_> {
    /// Reduce one runtime call to its return type.
    pub(in crate::check) fn reduce_call_term(
        &mut self,
        origin: Origin,
        _module: ModuleId,
        call: TermId<CallTerm>,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let source = self.inference.term(call).source;
        let result = self.select_call_target(origin, call, None)?;
        self.record_event(CheckEvent::CallableDispatch {
            origin,
            source,
            result: TraceCallableDispatch::from(&result),
        });

        let function = match &result {
            CallableDispatch::CallSelected { target, function } => {
                self.select_call_resolution(call, target.clone(), function)?;

                function
            }
            CallableDispatch::ConstructSelected { target, function } => {
                self.select_construct_resolution_from_call(call, target.clone(), function)?;

                function
            }
            CallableDispatch::CallRejected(failure) => {
                self.reject_call_from_callable(source, failure.clone())?;

                let ty = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error));

                return Ok(Answer::Ready(ty));
            }
            CallableDispatch::ConstructRejected(failure) => {
                self.reject_construct_from_callable(source, *failure)?;

                let ty = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error));

                return Ok(Answer::Ready(ty));
            }
            CallableDispatch::Invalid => {
                let ty = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error));

                return Ok(Answer::Ready(ty));
            }
            CallableDispatch::Pending(blockers) => return Ok(Answer::Pending(blockers.clone())),
        };

        let ty = match function.return_type {
            Some(return_type) => {
                let Answer::Ready(return_type) = self.reduce_type_operand(origin, return_type)?
                else {
                    return Ok(Answer::pending(return_type.dependencies(self)));
                };
                return_type
            }
            None => self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Void)),
        };

        Ok(Answer::Ready(ty))
    }

    /// Expect one runtime call expression to produce one result type.
    pub(in crate::check) fn expect_call_term(
        &mut self,
        origin: Origin,
        call: TermId<CallTerm>,
        result: VariableId,
    ) -> CompilerResult<Answer<()>> {
        let source = self.inference.term(call).source;
        let resolved = self.select_call_target(origin, call, Some(result))?;
        self.record_event(CheckEvent::CallableDispatch {
            origin,
            source,
            result: TraceCallableDispatch::from(&resolved),
        });

        match &resolved {
            CallableDispatch::CallSelected { target, function } => {
                self.select_call_resolution(call, target.clone(), function)?;
                self.constrain_call_return(origin, function, Some(result))?;

                Ok(Answer::Ready(()))
            }
            CallableDispatch::ConstructSelected { target, function } => {
                self.select_construct_resolution_from_call(call, target.clone(), function)?;
                self.constrain_call_return(origin, function, Some(result))?;

                Ok(Answer::Ready(()))
            }
            CallableDispatch::CallRejected(failure) => {
                self.reject_call_from_callable(source, failure.clone())?;
                let error = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Error));

                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    error,
                    result,
                    Condition::Always,
                );

                Ok(Answer::Ready(()))
            }
            CallableDispatch::ConstructRejected(failure) => {
                self.reject_construct_from_callable(source, *failure)?;
                let error = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Error));

                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    error,
                    result,
                    Condition::Always,
                );

                Ok(Answer::Ready(()))
            }
            CallableDispatch::Invalid => {
                let error = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Error));

                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    error,
                    result,
                    Condition::Always,
                );

                Ok(Answer::Ready(()))
            }
            CallableDispatch::Pending(blockers) => Ok(Answer::Pending(blockers.clone())),
        }
    }

    /// Reject one call for diagnostics.
    pub(in crate::check) fn reject_call_from_callable(
        &mut self,
        source: dir::GlobalNodeIdAny,
        failure: CallFailure,
    ) -> CompilerResult<()> {
        let decision = CallDecision::Rejected(failure);

        self.inference.select_call(source, decision)?;

        Ok(())
    }

    /// Select one resolved call for commit.
    pub(in crate::check) fn select_call_resolution(
        &mut self,
        call: TermId<CallTerm>,
        target: CallTargetResolution,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        self.select_member_call_resolution(call, &target)?;

        let source = self.inference.term(call).source;
        let resolution = CallResolution {
            source,
            target,
            function: function.clone(),
        };

        let decision = CallDecision::Resolved(resolution);

        self.inference.select_call(source, decision)?;

        Ok(())
    }

    /// Select one construct resolved through a call expression.
    fn select_construct_resolution_from_call(
        &mut self,
        call: TermId<CallTerm>,
        target: ConstructTargetResolution,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        let source = self.inference.term(call).source;
        let resolution = ConstructResolution {
            source,
            target,
            function: function.clone(),
        };
        let decision = ConstructDecision::Resolved(resolution);

        self.inference.select_construct(source, decision)?;

        Ok(())
    }

    /// Reject one construct resolved through a call expression.
    fn reject_construct_from_callable(
        &mut self,
        source: dir::GlobalNodeIdAny,
        failure: ConstructFailure,
    ) -> CompilerResult<()> {
        let decision = ConstructDecision::Rejected(failure);

        self.inference.select_construct(source, decision)?;

        Ok(())
    }

    /// Select the member resolution implied by one resolved member call.
    fn select_member_call_resolution(
        &mut self,
        call: TermId<CallTerm>,
        target: &CallTargetResolution,
    ) -> CompilerResult<()> {
        let call = self.inference.term(call);
        let CallCallee::Member(member) = call.callee else {
            return Ok(());
        };
        let member = self.inference.term(member);
        let MemberProjectionOrigin::Expression { source } = member.origin else {
            return Ok(());
        };
        let receiver = member.receiver.clone();
        let receiver = self.member_receiver_type_operand(&receiver);
        let target = match target {
            CallTargetResolution::Symbol {
                symbol,
                instance,
                receiver: Some(_),
            } => MemberTargetResolution::Symbol {
                symbol: *symbol,
                instance: instance.clone(),
            },
            CallTargetResolution::Union {
                candidates,
                receiver: Some(_),
            } => MemberTargetResolution::Union(candidates.clone()),
            CallTargetResolution::Expression { .. }
            | CallTargetResolution::Symbol {
                symbol: _,
                instance: _,
                receiver: None,
            }
            | CallTargetResolution::Union {
                candidates: _,
                receiver: None,
            } => return Ok(()),
        };
        let resolution = MemberResolution {
            source,
            receiver,
            target,
        };

        self.inference
            .select_member(source, MemberDecision::Resolved(resolution))?;

        Ok(())
    }
}
