use std::ops::{Deref, DerefMut};

use crate::CompilerResult;
use crate::check::{
    Answer, Cause, CauseKind, CheckState, Dependency, Expectation, FlowSite, PlaceUse, ValueUse,
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
    /// The function body source use.
    pub(in crate::check) site: FlowSite,
    /// The type the body completion must satisfy, except for constructors.
    pub(in crate::check) return_type: Option<dir::GlobalTypeId>,
    /// The yield targets when the body is a generator.
    pub(in crate::check) generator: Option<GeneratorTargets>,
    /// Whether the body constructs its own receiver.
    pub(in crate::check) is_constructor: bool,
}

/// Checking state for one function body.
pub(in crate::check) struct BodyState<'check, 'state> {
    /// The component check state.
    pub(in crate::check) check: &'check mut CheckState<'state>,
    /// The return target, when the body returns a value.
    pub(in crate::check) return_type: Option<dir::GlobalTypeId>,
    /// The yield targets, when the body is a generator.
    pub(in crate::check) generator: Option<GeneratorTargets>,
    /// Whether the body constructs its own receiver.
    pub(in crate::check) is_constructor: bool,
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
            is_constructor: false,
        }
    }
}

impl FunctionBody {
    /// Check this function body once.
    pub(in crate::check) fn check(
        self,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<Answer<bool>> {
        let mut state = BodyState {
            check,
            return_type: self.return_type,
            generator: self.generator,
            is_constructor: self.is_constructor,
        };
        let expectation = self.return_type.map(|return_type| {
            let cause = state.check.intern_cause(Cause::root(
                self.site.origin(),
                CauseKind::Return { annotation: None },
            ));

            Expectation::assignable(return_type, cause, ValueUse::Output)
        });
        let checked = state.attempt_node(self.site, PlaceUse::Read, expectation)?;

        // the body produces its return cell once its return evidence is in;
        //  parking on the cell itself means every return already bounded it
        if let Some(return_type) = self.return_type {
            let root = check.settled_root(return_type)?;
            if let Some(variable) = check.root_variable(root)? {
                let produced = match &checked {
                    Answer::Ready(_) => true,
                    Answer::Pending(blockers) => blockers
                        .iter()
                        .any(|blocker| *blocker == Dependency::Variable(variable)),
                };
                if produced {
                    check.settle_produced(variable)?;
                }
            }
        }

        Ok(match checked {
            Answer::Ready(checked) => Answer::Ready(checked.holds),
            Answer::Pending(blockers) => Answer::Pending(blockers),
        })
    }
}
