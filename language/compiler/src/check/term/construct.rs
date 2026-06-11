use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CallArgument, CheckState, Condition, ConstructDecision, ConstructDispatch,
    ConstructFailure, ConstructResolution, ConstructTargetResolution, FunctionTerm,
    GenericArgument, Origin, TermId, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm,
    VariableId,
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
    /// The runtime construct arguments.
    pub(in crate::check) arguments: SmallVec<[CallArgument; 4]>,
}

impl CheckState<'_> {
    /// Reduce one runtime construct expression to its return type.
    pub(in crate::check) fn reduce_construct_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        construct: TermId<ConstructTerm>,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let source = self.inference.term(construct).source;
        let result = self.select_construct_target(origin, module, construct, None)?;
        let function = match &result {
            ConstructDispatch::Selected { target, function } => {
                self.select_construct_resolution(source, target.clone(), function)?;

                function
            }
            ConstructDispatch::Rejected(failure) => {
                self.reject_construct(source, *failure)?;

                let ty = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error));

                return Ok(Answer::Ready(ty));
            }
            ConstructDispatch::Pending(blockers) => return Ok(Answer::Pending(blockers.clone())),
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

    /// Expect one runtime construct expression to produce one result type.
    pub(in crate::check) fn expect_construct_term(
        &mut self,
        origin: Origin,
        construct: TermId<ConstructTerm>,
        result: VariableId,
    ) -> CompilerResult<Answer<()>> {
        let source = self.inference.term(construct).source;
        let module = result.module;
        let resolved = self.select_construct_target(origin, module, construct, Some(result))?;
        match &resolved {
            ConstructDispatch::Selected { target, function } => {
                self.select_construct_resolution(source, target.clone(), function)?;

                if let Some(return_type) = function.return_type {
                    self.constrain_type(
                        origin,
                        TypeRelation::Assignable,
                        return_type,
                        result,
                        Condition::Always,
                    );
                }

                Ok(Answer::Ready(()))
            }
            ConstructDispatch::Rejected(failure) => {
                self.reject_construct(source, *failure)?;
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
            ConstructDispatch::Pending(blockers) => Ok(Answer::Pending(blockers.clone())),
        }
    }

    /// Reject one construct expression for diagnostics.
    fn reject_construct(
        &mut self,
        source: dir::GlobalNodeIdAny,
        failure: ConstructFailure,
    ) -> CompilerResult<()> {
        let decision = ConstructDecision::Rejected(failure);

        self.inference.select_construct(source, decision)?;

        Ok(())
    }

    /// Select one resolved construct expression for commit.
    fn select_construct_resolution(
        &mut self,
        source: dir::GlobalNodeIdAny,
        target: ConstructTargetResolution,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        let resolution = ConstructResolution {
            source,
            target,
            function: function.clone(),
        };
        let decision = ConstructDecision::Resolved(resolution);

        self.inference.select_construct(source, decision)?;

        Ok(())
    }
}
