use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::check::{CheckState, FlowBranch, FunctionFrame, ReceiverBinding};

impl CheckState<'_> {
    /// Enter one function body's flow frame.
    pub(in crate::check) fn enter_function_frame(
        &mut self,
        symbol: dir::GlobalSymbolId,
        return_target: dir::GlobalTypeId,
        yield_target: Option<dir::GlobalTypeId>,
        asynchrony: dir::Asynchrony,
        receiver: Option<ReceiverBinding>,
    ) {
        // capture enclosing flow stack boundaries
        let flow = &self.flow;
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
            asynchrony,
            captured_symbols: FxIndexSet::default(),
            captured_receiver: None,
        };

        // expose function frame to nested flow checks
        self.flow.push_function(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn leave_function_frame(&mut self) -> FlowBranch {
        // collect captures and restore outer flow
        let (capture, flow) = self.flow.pop_function();

        // record the body's captures on its module
        self.module_mut(self.module_id)
            .pending_captures
            .push(capture);

        flow
    }

    /// Return whether the current function body is a generator.
    pub(in crate::check) fn is_in_generator(&self) -> bool {
        self.flow
            .current_function()
            .is_some_and(|function| function.yield_target.is_some())
    }

    /// Return the enclosing function return target.
    pub(in crate::check) fn current_return_target(&self) -> Option<dir::GlobalTypeId> {
        self.flow
            .current_function()
            .map(|function| function.return_target)
    }
}
