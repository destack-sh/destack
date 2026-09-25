use tspp_core::FxIndexSet;
use tspp_dir as dir;

use crate::sema::{
    CheckState, FlowBranch, FunctionFrame, GeneratorTargets, InferMode, ReceiverBinding,
};

impl CheckState<'_> {
    /// Enter one function body's flow frame.
    pub(in crate::sema) fn enter_function_frame(
        &mut self,
        symbol: dir::GlobalSymbolId,
        return_target: Option<dir::GlobalTypeId>,
        generator: Option<GeneratorTargets>,
        initializes: Option<dir::GlobalSymbolId>,
        output_mode: InferMode,
        asynchrony: dir::Asynchrony,
        receiver: Option<ReceiverBinding>,
        enclosing_receiver: Option<ReceiverBinding>,
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
            enclosing_receiver,
            return_target,
            generator,
            initializes,
            output_mode,
            asynchrony,
            captured_symbols: FxIndexSet::default(),
            captured_receiver: None,
        };

        // expose function frame to nested flow checks
        self.flow.push_function(function);
    }

    /// Leave the current function body.
    pub(in crate::sema) fn leave_function_frame(&mut self) -> FlowBranch {
        // collect captures and restore outer flow
        let (capture, flow) = self.flow.pop_function();

        // record the body's captures on its module
        self.module_mut(self.module_id)
            .pending_captures
            .push(capture);

        flow
    }

    /// Return whether the current function body is a generator.
    pub(in crate::sema) fn is_in_generator(&self) -> bool {
        self.flow
            .current_function()
            .is_some_and(|function| function.generator.is_some())
    }

    /// Return the enclosing function's symbol.
    pub(in crate::sema) fn current_function_symbol(&self) -> Option<dir::GlobalSymbolId> {
        self.flow.current_function().map(|function| function.symbol)
    }

    /// Return the enclosing generator body's targets.
    pub(in crate::sema) fn current_generator(&self) -> Option<GeneratorTargets> {
        self.flow
            .current_function()
            .and_then(|function| function.generator)
    }

    /// Return the class the enclosing constructor body initializes.
    pub(in crate::sema) fn current_initializes(&self) -> Option<dir::GlobalSymbolId> {
        self.flow
            .current_function()
            .and_then(|function| function.initializes)
    }

    /// Return the class the nearest enclosing constructor body initializes, nested functions
    /// reading through.
    pub(in crate::sema) fn enclosing_initializes(&self) -> Option<dir::GlobalSymbolId> {
        self.flow.enclosing_initializes()
    }

    /// Return the literal inference mode of the enclosing body's outputs.
    pub(in crate::sema) fn current_output_mode(&self) -> InferMode {
        self.flow
            .current_function()
            .map_or(InferMode::Regular, |function| function.output_mode)
    }

    /// Return the enclosing function return target.
    pub(in crate::sema) fn current_return_target(&self) -> Option<dir::GlobalTypeId> {
        self.flow
            .current_function()
            .and_then(|function| function.return_target)
    }
}
