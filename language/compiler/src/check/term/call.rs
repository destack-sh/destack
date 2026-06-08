use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CallDecision, CallFailure, CallResolution, CallTargetResolution, CallableDispatch, CheckState,
    ConstructDecision, ConstructFailure, ConstructResolution, ConstructTargetResolution,
    FunctionTerm, GenericArgument, MemberDecision, MemberResolution, MemberTargetResolution,
    Origin, TermId, TypeLiteralTerm, TypeOperand, TypeTerm, VariableId,
};

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
    /// The argument expression types.
    pub(in crate::check) argument_types: SmallVec<[TypeOperand; 4]>,
    /// The argument expression nodes in argument order.
    pub(in crate::check) arguments: SmallVec<[dir::GlobalNodeId<dir::Expression>; 4]>,
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
    /// The receiver type.
    pub(in crate::check) receiver: TypeOperand,
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

impl CallTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        match self.callee {
            CallCallee::Expression(callee) => variables.extend(callee.referenced_variables(state)),
            CallCallee::Reference { value, symbol: _ } => {
                variables.extend(value.referenced_variables(state));
            }
            CallCallee::Member(member) => {
                let member = state.inference.term(member);

                variables.extend(member.receiver.referenced_variables(state));
                variables.extend(
                    member
                        .arguments
                        .iter()
                        .flat_map(|argument| argument.referenced_variables(state)),
                );
            }
        }
        variables.extend(
            self.generic_arguments
                .iter()
                .flat_map(|argument| argument.referenced_variables(state)),
        );
        variables.extend(
            self.argument_types
                .iter()
                .flat_map(|argument| argument.referenced_variables(state)),
        );

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one runtime call to its return type.
    pub(in crate::check) fn reduce_call_term(
        &mut self,
        origin: Origin,
        _module: ModuleId,
        call: &CallTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let result = self.select_call_target(origin, call, None)?;
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
                self.reject_call_from_callable(call, failure.clone())?;

                return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error)));
            }
            CallableDispatch::ConstructRejected(failure) => {
                self.reject_construct_from_callable(call, *failure)?;

                return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error)));
            }
            CallableDispatch::Invalid => {
                return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error)));
            }
            CallableDispatch::Pending => return Ok(None),
        };

        let term = match function.return_type {
            Some(return_type) => {
                let Some(term) = self.type_operand_term(return_type)? else {
                    return Ok(None);
                };

                term
            }
            None => TypeTerm::Literal(TypeLiteralTerm::Void),
        };
        Ok(Some(term))
    }

    /// Expect resolved call candidates to produce the expected result.
    pub(in crate::check) fn expect_call_term(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        result: VariableId,
    ) -> CompilerResult<()> {
        let resolved = self.select_call_target(origin, call, Some(result))?;
        match &resolved {
            CallableDispatch::CallSelected { target, function } => {
                self.select_call_resolution(call, target.clone(), function)?;

                if let Some(return_type) = function.return_type {
                    self.reduce_contextual_type_assignability(origin, return_type, result)?;
                }
            }
            CallableDispatch::ConstructSelected { target, function } => {
                self.select_construct_resolution_from_call(call, target.clone(), function)?;

                if let Some(return_type) = function.return_type {
                    self.reduce_contextual_type_assignability(origin, return_type, result)?;
                }
            }
            CallableDispatch::CallRejected(failure) => {
                self.reject_call_from_callable(call, failure.clone())?;
                let error = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Error));

                self.reduce_contextual_type_assignability(origin, error, result)?;
            }
            CallableDispatch::ConstructRejected(failure) => {
                self.reject_construct_from_callable(call, *failure)?;
                let error = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Error));

                self.reduce_contextual_type_assignability(origin, error, result)?;
            }
            CallableDispatch::Invalid => {
                let error = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Error));

                self.reduce_contextual_type_assignability(origin, error, result)?;
            }
            CallableDispatch::Pending => {}
        }

        Ok(())
    }

    /// Reject one call for diagnostics.
    pub(in crate::check) fn reject_call_from_callable(
        &mut self,
        call: &CallTerm,
        failure: CallFailure,
    ) -> CompilerResult<()> {
        let decision = CallDecision::Rejected(failure);

        self.inference.select_call(call.source, decision)?;

        Ok(())
    }

    /// Select one resolved call for commit.
    pub(in crate::check) fn select_call_resolution(
        &mut self,
        call: &CallTerm,
        target: CallTargetResolution,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        self.select_member_call_resolution(call, &target)?;

        let resolution = CallResolution {
            source: call.source,
            target,
            function: function.clone(),
        };

        let decision = CallDecision::Resolved(resolution);

        self.inference.select_call(call.source, decision)?;

        Ok(())
    }

    /// Select one construct resolved through call syntax.
    fn select_construct_resolution_from_call(
        &mut self,
        call: &CallTerm,
        target: ConstructTargetResolution,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        let resolution = ConstructResolution {
            source: call.source,
            target,
            function: function.clone(),
        };
        let decision = ConstructDecision::Resolved(resolution);

        self.inference.select_construct(call.source, decision)?;

        Ok(())
    }

    /// Reject one construct resolved through call syntax.
    fn reject_construct_from_callable(
        &mut self,
        call: &CallTerm,
        failure: ConstructFailure,
    ) -> CompilerResult<()> {
        let decision = ConstructDecision::Rejected(failure);

        self.inference.select_construct(call.source, decision)?;

        Ok(())
    }

    /// Select the member resolution implied by one resolved member call.
    fn select_member_call_resolution(
        &mut self,
        call: &CallTerm,
        target: &CallTargetResolution,
    ) -> CompilerResult<()> {
        let CallCallee::Member(member) = call.callee else {
            return Ok(());
        };
        let member = self.inference.term(member);
        let MemberProjectionOrigin::Expression { source } = member.origin else {
            return Ok(());
        };
        let receiver = member.receiver;
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
