use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{FlowBranch, FunctionFrame, ReceiverBinding, VariableRole, WalkState, Widening};

impl WalkState<'_, '_> {
    /// Enter one function body while walking.
    pub(in crate::check) fn enter_function_frame(
        &mut self,
        symbol: dir::GlobalSymbolId,
        return_target: dir::GlobalTypeId,
        yield_target: Option<dir::GlobalTypeId>,
        resume_target: Option<dir::GlobalTypeId>,
        asynchrony: dir::Asynchrony,
        receiver: Option<ReceiverBinding>,
    ) {
        // capture enclosing flow stack boundaries
        let flow = self.flow();
        let checkpoint = flow.fork();
        let target_start = flow.targets.len();
        let try_start = flow.tries.len();
        let function = FunctionFrame {
            symbol,
            checkpoint,
            target_start,
            try_start,
            receiver,
            return_target,
            yield_target,
            resume_target,
            asynchrony,
            captured_symbols: FxIndexSet::default(),
            captured_receiver: None,
        };

        // expose function frame to nested flow checks
        self.flow_mut().push_function(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn leave_function_frame(&mut self) -> FlowBranch {
        // collect captures and restore outer flow
        let (capture, flow) = self.flow_mut().pop_function();

        // store capture result
        self.check.module_mut(self.module).captures.push(capture);

        flow
    }

    /// Record one delegated yield's protocol judgment and inner return output.
    pub(in crate::check) fn record_yield_delegate(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // read the generator targets, reported absent by the caller
        let Some(function) = self.flow().current_function() else {
            return Ok(());
        };
        let (Some(yield_target), Some(resume_target)) =
            (function.yield_target, function.resume_target)
        else {
            return Ok(());
        };
        let asynchrony = function.asynchrony;

        // the delegate return becomes the yield expression's own output
        let output =
            self.open_type_hole(source.into_any(), Widening::Never, VariableRole::Regular)?;

        // the delegate value must implement the generator protocol
        let item = match asynchrony {
            dir::Asynchrony::Sync => dir::LanguageItem::Iterable,
            dir::Asynchrony::Async => dir::LanguageItem::AsyncIterable,
        };
        let expected =
            self.language_type_reference(item, &[yield_target, output, resume_target])?;
        self.check
            .control_results
            .insert(value.into_global_any(self.module), expected);
        self.check
            .control_results
            .insert(source.into_global_any(self.module), output);

        Ok(())
    }

    /// Return whether the current function body is a generator.
    pub(in crate::check) fn is_in_generator(&self) -> bool {
        self.flow()
            .current_function()
            .is_some_and(|function| function.yield_target.is_some())
    }

    /// Return the enclosing function return target.
    pub(in crate::check) fn current_return_target(&self) -> Option<dir::GlobalTypeId> {
        self.flow()
            .current_function()
            .map(|function| function.return_target)
    }
}
