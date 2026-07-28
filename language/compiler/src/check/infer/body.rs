use std::ops::{Deref, DerefMut};

use crate::CompilerResult;
use crate::check::{
    Answer, Cause, CauseId, CauseKind, CheckState, Expectation, FlowSite, InferMode, PlaceUse,
    Relation, ValueCheck, ValueUse,
};
use destack_dir as dir;

/// Yield targets for one generator body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct GeneratorTargets {
    /// The type of values the body yields.
    pub(in crate::check) yielded: dir::GlobalTypeId,
    /// The type yield expressions resume with.
    pub(in crate::check) resumed: dir::GlobalTypeId,
}

/// One function body with its return and yield types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct FunctionBody {
    /// The function declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The function body source use.
    pub(in crate::check) site: FlowSite,
    /// The type the body completion must satisfy, except for constructors.
    pub(in crate::check) return_type: Option<dir::GlobalTypeId>,
    /// The yield targets when the body is a generator.
    pub(in crate::check) generator: Option<GeneratorTargets>,
    /// The declaration whose fields this constructor initializes.
    pub(in crate::check) initializes: Option<dir::GlobalSymbolId>,
}

/// Checking state for one function body.
pub(in crate::check) struct BodyState<'check, 'state> {
    /// The component check state.
    pub(in crate::check) check: &'check mut CheckState<'state>,
    /// The return target, when the body returns a value.
    pub(in crate::check) return_type: Option<dir::GlobalTypeId>,
    /// The yield targets, when the body is a generator.
    pub(in crate::check) generator: Option<GeneratorTargets>,
    /// The declaration whose fields this constructor initializes.
    pub(in crate::check) initializes: Option<dir::GlobalSymbolId>,
    /// Literal inference applied to inferred returns and yields.
    pub(in crate::check) output_mode: InferMode,
}

impl<'state> Deref for BodyState<'_, 'state> {
    type Target = CheckState<'state>;

    fn deref(&self) -> &Self::Target {
        self.check
    }
}

impl DerefMut for BodyState<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.check
    }
}

impl<'state> CheckState<'state> {
    /// Enter a body-less checking context.
    pub(in crate::check) fn body(&mut self) -> BodyState<'_, 'state> {
        BodyState {
            check: self,
            return_type: None,
            generator: None,
            initializes: None,
            output_mode: InferMode::Exact,
        }
    }
}

impl FunctionBody {
    /// Check this function body once.
    pub(in crate::check) fn check(
        self,
        check: &mut CheckState<'_>,
        output_mode: InferMode,
        parent: Option<CauseId>,
    ) -> CompilerResult<Answer<Option<ValueCheck>>> {
        let mut state = BodyState {
            check,
            return_type: self.return_type,
            generator: self.generator,
            initializes: self.initializes,
            output_mode,
        };
        let expectation = self.return_type.map(|return_type| {
            let origin = self.site.origin();
            let kind = CauseKind::Return { annotation: None };
            let cause = match parent {
                Some(parent) => Cause::child(origin, kind, parent),
                None => Cause::root(origin, kind),
            };
            let cause = state.check.intern_cause(cause);

            Expectation {
                target: return_type,
                relation: Relation::Assignable,
                cause,
                use_: ValueUse::Output,
                mode: output_mode,
            }
        });
        let checked = state.attempt_node(self.site, PlaceUse::Read, expectation)?;

        Ok(checked)
    }
}
