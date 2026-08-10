use std::ops::{Deref, DerefMut};

use crate::CompilerResult;
use crate::check::{
    Cause, CauseId, CauseKind, CheckState, Expectation, FlowSite, InferMode, PlaceUse,
    ReceiverBinding, Relation, ValueCheck, ValueUse,
};
use destack_dir as dir;
use smallvec::SmallVec;

/// Yield targets for one generator body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct GeneratorTargets {
    /// The generator protocol family the body implements.
    pub(in crate::check) asynchrony: dir::Asynchrony,
    /// The type of values the body yields.
    pub(in crate::check) yielded: dir::GlobalTypeId,
    /// The type yield expressions resume with.
    pub(in crate::check) resumed: dir::GlobalTypeId,
}

/// One function body with its return and yield types.
#[derive(Debug, Clone)]
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
    /// The body's asynchrony, entering its flow frame at check.
    pub(in crate::check) asynchrony: dir::Asynchrony,
    /// The contextual receiver bound over the body.
    pub(in crate::check) receiver: Option<ReceiverBinding>,
    /// The parameter sources assigned on entry.
    pub(in crate::check) entries: SmallVec<[dir::LocalNodeIdAny; 4]>,
}

/// Checking state for one function body.
pub(in crate::check) struct BodyState<'check, 'state> {
    /// The module check state.
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
    ) -> CompilerResult<Option<ValueCheck>> {
        let mut state = BodyState {
            check,
            return_type: self.return_type,
            generator: self.generator,
            initializes: self.initializes,
            output_mode,
        };

        // expect the body's completion value at the declared return type
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

        // resolve the targets the body returns and yields to
        let (return_target, yield_target) = match self.generator {
            Some(targets) => (
                self.return_type.unwrap_or(targets.yielded),
                Some(targets.yielded),
            ),
            None => (
                self.return_type
                    .unwrap_or(state.check.intern_type(dir::Type::Void)?),
                None,
            ),
        };

        // enter the body's flow frame and mark its entry bindings
        state.check.enter_function_frame(
            self.symbol,
            return_target,
            yield_target,
            self.asynchrony,
            self.receiver,
        );
        for entry in &self.entries {
            state.check.mark_bindings_assigned(*entry);
        }

        // check the body under its generic template scope
        if let Some(template) = self.site.scope {
            state.check.flow.push_template_scope(template);
        }
        let checked = state.attempt_node(self.site, PlaceUse::Read, expectation);
        if self.site.scope.is_some() {
            state.check.flow.pop_template_scope();
        }
        let checked = checked?;
        let branch = state.check.leave_function_frame();

        // record the constructor's exit branch for class initialization
        if let Some(class) = self.initializes {
            state
                .check
                .constructor_branches
                .entry(class)
                .or_default()
                .push(branch);
        }

        Ok(checked)
    }
}
