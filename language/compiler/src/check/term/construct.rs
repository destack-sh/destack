use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CallableDispatch, CheckState, ConstructDecision, ConstructFailure, ConstructResolution,
    ConstructTargetResolution, FunctionTerm, GenericArgument, Origin, TypeLiteralTerm, TypeOperand,
    TypeTerm, VariableId,
};

/// Runtime construct expression term.
///
/// ```ds
/// new User(name)
/// new Ctor()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ConstructTerm {
    /// The source construct expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The construct callee type.
    pub(in crate::check) callee: TypeOperand,
    /// The explicit construct generic arguments.
    pub(in crate::check) generic_arguments: SmallVec<[GenericArgument; 2]>,
    /// The argument expression types.
    pub(in crate::check) argument_types: SmallVec<[TypeOperand; 4]>,
    /// The argument expression nodes in argument order.
    pub(in crate::check) arguments: SmallVec<[dir::GlobalNodeId<dir::Expression>; 4]>,
}

impl ConstructTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        variables.extend(self.callee.referenced_variables(state));
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
    /// Reduce one runtime construct expression to its return type.
    pub(in crate::check) fn reduce_construct_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        construct: &ConstructTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let result = self.select_construct_target(origin, module, construct, None)?;
        let function = match &result {
            CallableDispatch::ConstructSelected { target, function } => {
                self.select_construct_resolution(construct, target.clone(), function)?;

                function
            }
            CallableDispatch::ConstructRejected(failure) => {
                self.reject_construct(construct, *failure)?;

                return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error)));
            }
            CallableDispatch::CallSelected { .. } | CallableDispatch::CallRejected(_) => {
                unreachable!("construct term dispatch produced a call selection");
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

    /// Expect resolved construct candidates to produce the expected result.
    pub(in crate::check) fn expect_construct_term(
        &mut self,
        origin: Origin,
        construct: &ConstructTerm,
        result: VariableId,
    ) -> CompilerResult<()> {
        let module = result.module;
        let resolved = self.select_construct_target(origin, module, construct, Some(result))?;
        match &resolved {
            CallableDispatch::ConstructSelected { target, function } => {
                self.select_construct_resolution(construct, target.clone(), function)?;

                if let Some(return_type) = function.return_type {
                    self.reduce_contextual_type_assignability(origin, return_type, result)?;
                }
            }
            CallableDispatch::ConstructRejected(failure) => {
                self.reject_construct(construct, *failure)?;
                let error = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Error));

                self.reduce_contextual_type_assignability(origin, error, result)?;
            }
            CallableDispatch::CallSelected { .. } | CallableDispatch::CallRejected(_) => {
                unreachable!("construct term dispatch produced a call selection");
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

    /// Reject one construct expression for diagnostics.
    fn reject_construct(
        &mut self,
        construct: &ConstructTerm,
        failure: ConstructFailure,
    ) -> CompilerResult<()> {
        let decision = ConstructDecision::Rejected(failure);

        self.inference
            .select_construct(construct.source, decision)?;

        Ok(())
    }

    /// Select one resolved construct expression for commit.
    fn select_construct_resolution(
        &mut self,
        construct: &ConstructTerm,
        target: ConstructTargetResolution,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        let resolution = ConstructResolution {
            source: construct.source,
            target,
            function: function.clone(),
        };
        let decision = ConstructDecision::Resolved(resolution);

        self.inference
            .select_construct(construct.source, decision)?;

        Ok(())
    }
}
