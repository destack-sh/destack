use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CallFailure, CallResolution, CallResolutionTarget, CallSelection, CallableSelection,
    CallableSignature, CheckState, FunctionTerm, GenericArgument, GenericInstance,
    GenericSubstitution, MemberResolution, MemberResolutionTarget, MemberSelection, Origin,
    Progress, Reduction, TermId, TypeLiteralTerm, TypeOperand, TypeTerm, VariableId,
};

/// Runtime call expression term.
///
/// ```ts
/// format(value)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallTerm {
    /// The source call expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The selected call callee.
    pub(in crate::check) callee: CallCallee,
    /// The explicit call generic arguments.
    pub(in crate::check) generic_arguments: SmallVec<[GenericArgument; 4]>,
    /// The argument expression types.
    pub(in crate::check) arguments: SmallVec<[TypeOperand; 4]>,
}

/// Runtime value selected as a call callee.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum CallCallee {
    /// Arbitrary callable value expression.
    Value(VariableId),
    /// Member projection callable.
    Member(TermId<MemberCallTerm>),
}

/// Member projection used as a runtime call callee.
///
/// ```ts
/// value.toString()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberCallTerm {
    /// How the member call was introduced.
    pub(in crate::check) callee: MemberCallCallee,
    /// The receiver type.
    pub(in crate::check) receiver: VariableId,
    /// The selected member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The applied static arguments.
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 4]>,
}

/// How a member call was introduced.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MemberCallCallee {
    /// Source member call syntax.
    Source {
        /// The source member expression.
        source: dir::GlobalNodeIdAny,
    },
    /// Compiler-lowered protocol call.
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
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 4]>,
}

impl CallTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self.callee {
            CallCallee::Value(callee) => variables.push(callee),
            CallCallee::Member(member) => {
                let member = state.terms.get(member);

                variables.push(member.receiver);
                variables.extend(
                    member
                        .arguments
                        .iter()
                        .flat_map(|argument| state.argument_variables(argument)),
                );
            }
        }
        variables.extend(
            self.generic_arguments
                .iter()
                .flat_map(|argument| state.argument_variables(argument)),
        );
        variables.extend(state.call_instantiation_variables(self.source));
        variables.extend(
            self.arguments
                .iter()
                .flat_map(|argument| argument.referenced_variables(state)),
        );

        variables
    }
}

/// Stable key for one call generic instantiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct CallInstantiationKey {
    /// The call expression being instantiated.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The selected generic owner.
    pub(in crate::check) owner: Option<dir::GlobalSymbolId>,
    /// The selected dispatch target.
    pub(in crate::check) target: Option<dir::GlobalSymbolId>,
}

/// Function signature after call generic substitution.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallInstantiation {
    /// The instantiated function signature.
    pub(in crate::check) function: FunctionTerm,
    /// The resolved generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The substitution used for this call.
    pub(in crate::check) substitution: GenericSubstitution,
    /// The original generic parameters.
    pub(in crate::check) generic_parameters: Vec<VariableId>,
}

impl CheckState<'_> {
    /// Reduce one generic function value reference to its specialized function type.
    pub(in crate::check) fn reduce_function_reference_term(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[GenericArgument],
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let value = self.symbol_type_variable(module, symbol);
        let Some(term) = self.solved_type_term(value)? else {
            return Ok(Reduction::pending());
        };
        let function = match self.call_signature(module, &term)? {
            CallableSignature::Pending => return Ok(Reduction::pending()),
            CallableSignature::Absent => {
                return Ok(Reduction::value(TypeTerm::Reference {
                    origin: Origin::Node(source),
                    symbol,
                    arguments: generic_arguments.to_vec().into(),
                }));
            }
            CallableSignature::Present(function) => function,
        };
        let instantiation = self.instantiate_call_signature(
            module,
            source,
            Some(symbol),
            Some(symbol),
            None,
            function,
            generic_arguments,
        )?;

        let function = self.terms.push(instantiation.function);

        Ok(Reduction::value(TypeTerm::Function(function)))
    }

    /// Reduce one runtime call to its return type.
    pub(in crate::check) fn reduce_call_term(
        &mut self,
        origin: Origin,
        _module: ModuleId,
        call: &CallTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let result = self.select_call_target(origin, call, None)?;
        let progress = result.progress();
        let function = match &result {
            CallableSelection::Resolved {
                target, function, ..
            } => {
                self.select_call_resolution(call, target.clone(), function)?;

                function
            }
            CallableSelection::Rejected(failure) => {
                self.select_call_rejection(call, failure.clone())?;

                return Ok(Reduction::progress(progress));
            }
            CallableSelection::Pending { .. } => return Ok(Reduction::progress(progress)),
        };

        let term = match function.return_type {
            Some(TypeOperand::Variable(return_type)) => TypeTerm::Variable(return_type),
            Some(TypeOperand::Term(return_type)) => self.terms.get(return_type).clone(),
            None => TypeTerm::Literal(TypeLiteralTerm::Void),
        };
        Ok(Reduction {
            value: Some(term),
            progress,
        })
    }

    /// Expect resolved call candidates to produce the expected result.
    pub(in crate::check) fn expect_call_term(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let resolved = self.select_call_target(origin, call, Some(result))?;
        let progress = resolved.progress();
        let progress =
            match &resolved {
                CallableSelection::Resolved {
                    target, function, ..
                } => {
                    self.select_call_resolution(call, target.clone(), function)?;

                    match function.return_type {
                        Some(return_type) => progress.merge(
                            self.solve_contextual_type_assignability(origin, return_type, result)?,
                        ),
                        None => progress,
                    }
                }
                CallableSelection::Rejected(failure) => {
                    self.select_call_rejection(call, failure.clone())?;

                    progress
                }
                CallableSelection::Pending { .. } => progress,
            };

        Ok(progress)
    }

    /// Select one rejected call for diagnostics.
    pub(in crate::check) fn select_call_rejection(
        &mut self,
        call: &CallTerm,
        failure: CallFailure,
    ) -> CompilerResult<()> {
        let decision = CallSelection::Rejected(failure);

        self.select_call(call.source, decision);

        Ok(())
    }

    /// Select one resolved call for commit.
    pub(in crate::check) fn select_call_resolution(
        &mut self,
        call: &CallTerm,
        target: CallResolutionTarget,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        self.select_member_call_resolution(call, &target);

        let resolution = CallResolution {
            source: call.source,
            target,
            function: function.clone(),
        };

        let decision = CallSelection::Resolved(resolution);

        self.select_call(call.source, decision);

        Ok(())
    }

    /// Select the member resolution implied by one resolved member call.
    fn select_member_call_resolution(&mut self, call: &CallTerm, target: &CallResolutionTarget) {
        let CallCallee::Member(member) = call.callee else {
            return;
        };
        let member = self.terms.get(member);
        let MemberCallCallee::Source { source } = member.callee else {
            return;
        };
        let receiver = member.receiver;
        let target = match target {
            CallResolutionTarget::Symbol {
                symbol,
                instance,
                receiver: Some(_),
            } => MemberResolutionTarget::Symbol {
                symbol: *symbol,
                instance: instance.clone(),
            },
            CallResolutionTarget::Select {
                candidates,
                receiver: Some(_),
            } => MemberResolutionTarget::Select(candidates.clone()),
            CallResolutionTarget::Value
            | CallResolutionTarget::Constructor { .. }
            | CallResolutionTarget::Symbol {
                symbol: _,
                instance: _,
                receiver: None,
            }
            | CallResolutionTarget::Select {
                candidates: _,
                receiver: None,
            } => return,
        };
        let resolution = MemberResolution {
            source,
            receiver,
            target,
        };

        self.select_member(source, MemberSelection::Resolved(resolution));
    }
}
