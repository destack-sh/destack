use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CallableSelection, CheckState, ConstructFailure, ConstructResolution, ConstructSelection,
    FunctionTerm, GenericArgument, GenericInstance, Origin, Progress, Reduction, TypeLiteralTerm,
    TypeOperand, TypeTerm, VariableId,
};

/// Runtime construct expression term.
///
/// ```ts
/// new User(name)
/// new Ctor()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ConstructTerm {
    /// The source construct expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The construct callee type.
    pub(in crate::check) callee: VariableId,
    /// The explicit construct generic arguments.
    pub(in crate::check) generic_arguments: SmallVec<[GenericArgument; 4]>,
    /// The argument expression types.
    pub(in crate::check) arguments: SmallVec<[TypeOperand; 4]>,
}

impl ConstructTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.push(self.callee);
        variables.extend(
            self.generic_arguments
                .iter()
                .flat_map(|argument| state.argument_variables(argument)),
        );
        variables.extend(
            self.arguments
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
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let result = self.select_construct_target(origin, module, construct, None)?;
        let progress = result.progress();
        let function = match &result {
            CallableSelection::Resolved {
                target, function, ..
            } => {
                let (symbol, instance) = target.as_constructor_target();

                self.select_construct_resolution(construct, symbol, instance, function)?;

                function
            }
            CallableSelection::Rejected(failure) => {
                let failure = failure.clone().into();

                self.select_construct_rejection(construct, failure)?;

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

    /// Expect resolved construct candidates to produce the expected result.
    pub(in crate::check) fn expect_construct_term(
        &mut self,
        origin: Origin,
        construct: &ConstructTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let module = result.module;
        let resolved = self.select_construct_target(origin, module, construct, Some(result))?;
        let progress = resolved.progress();
        let progress =
            match &resolved {
                CallableSelection::Resolved {
                    target, function, ..
                } => {
                    let (symbol, instance) = target.as_constructor_target();

                    self.select_construct_resolution(construct, symbol, instance, function)?;

                    match function.return_type {
                        Some(return_type) => progress.merge(
                            self.solve_contextual_type_assignability(origin, return_type, result)?,
                        ),
                        None => progress,
                    }
                }
                CallableSelection::Rejected(failure) => {
                    let failure = failure.clone().into();

                    self.select_construct_rejection(construct, failure)?;

                    progress
                }
                CallableSelection::Pending { .. } => progress,
            };

        Ok(progress)
    }

    /// Select one rejected construct expression for diagnostics.
    fn select_construct_rejection(
        &mut self,
        construct: &ConstructTerm,
        failure: ConstructFailure,
    ) -> CompilerResult<()> {
        let decision = ConstructSelection::Rejected(failure);

        self.select_construct(construct.source, decision);

        Ok(())
    }

    /// Select one resolved construct expression for commit.
    fn select_construct_resolution(
        &mut self,
        construct: &ConstructTerm,
        symbol: Option<dir::GlobalSymbolId>,
        instance: Option<&GenericInstance>,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        let resolution = ConstructResolution {
            source: construct.source,
            symbol,
            instance: instance.cloned(),
            function: function.clone(),
        };
        let decision = ConstructSelection::Resolved(resolution);

        self.select_construct(construct.source, decision);

        Ok(())
    }
}
