use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{
    Cause, CauseId, CauseKind, CheckState, Expectation, FlowSite, FlowSnapshot, InferMode,
    PlaceUse, ReceiverBinding, Relation, StoreTarget, ValueCheck, ValueUse,
};

/// Yield targets for one generator body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct GeneratorTargets {
    /// The generator protocol family the body implements.
    pub(in crate::sema) asynchrony: dir::Asynchrony,
    /// The type of values the body yields.
    pub(in crate::sema) yielded: dir::GlobalTypeId,
    /// The type yield expressions resume with.
    pub(in crate::sema) resumed: dir::GlobalTypeId,
}

/// One function body with its return and yield types.
#[derive(Debug, Clone)]
pub(in crate::sema) struct FunctionBody {
    /// The function declaration symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The function body source use.
    pub(in crate::sema) site: FlowSite,
    /// The type the body completion must satisfy, except for constructors.
    pub(in crate::sema) return_type: Option<dir::GlobalTypeId>,
    /// The yield targets when the body is a generator.
    pub(in crate::sema) generator: Option<GeneratorTargets>,
    /// The declaration whose fields this constructor initializes.
    pub(in crate::sema) initializes: Option<dir::GlobalSymbolId>,
    /// The body's asynchrony, entering its flow frame at check.
    pub(in crate::sema) asynchrony: dir::Asynchrony,
    /// The receiver the body binds itself.
    pub(in crate::sema) receiver: Option<ReceiverBinding>,
    /// The enclosing receiver a function value closes over.
    pub(in crate::sema) enclosing_receiver: Option<ReceiverBinding>,
    /// The parameter sources assigned on entry.
    pub(in crate::sema) entries: SmallVec<[dir::LocalNodeIdAny; 4]>,
    /// The enclosing flow at the function value, kept while its parameter types stay open.
    pub(in crate::sema) flow: Option<FlowSnapshot>,
}

impl FunctionBody {
    /// Check this function body once.
    pub(in crate::sema) fn check(
        self,
        check: &mut CheckState<'_>,
        output_mode: InferMode,
        parent: Option<CauseId>,
    ) -> CompilerResult<Option<ValueCheck>> {
        // expect the body's completion value at the contextual return
        let expectation = self.return_type.map(|return_type| {
            let origin = self.site.origin();
            let kind = CauseKind::Return { annotation: None };
            let cause = match parent {
                Some(parent) => Cause::child(origin, kind, parent),
                None => Cause::root(origin, kind),
            };
            let cause = check.intern_cause(cause);

            Expectation {
                target: return_type,
                relation: Relation::Storable,
                cause,
                use_: ValueUse::Output,
                mode: output_mode,
                store: StoreTarget::Exact,
            }
        });

        // enter the body's flow frame and mark its entry bindings
        check.enter_function_frame(
            self.symbol,
            self.return_type,
            self.generator,
            self.initializes,
            output_mode,
            self.asynchrony,
            self.receiver,
            self.enclosing_receiver,
        );
        for entry in &self.entries {
            check.assign_bindings(*entry);
        }

        // check the body under its generic template scope
        if let Some(template) = self.site.scope {
            check.flow.push_template_scope(template);
        }
        let checked = check.attempt_node(self.site, PlaceUse::Read, expectation);
        if self.site.scope.is_some() {
            check.flow.pop_template_scope();
        }
        let checked = checked?;
        let branch = check.leave_function_frame();

        // record the constructor's exit branch for class initialization
        if let Some(class) = self.initializes {
            check
                .constructor_branches
                .entry(class)
                .or_default()
                .push((self.symbol, branch));
        }

        Ok(checked)
    }
}
