mod call;
mod decode;
mod dispatch;
mod frame;
mod global;
mod instruction;
mod interpreter;
mod intrinsic;
mod operator;
mod options;
mod statistics;
mod threaded;

use crate::memory::Value;

pub use frame::Frame;
pub use global::GlobalStorage;
pub use interpreter::{
    Continuation, ExecutionOutcome, ExecutionOutput, ExecutionYield, ExternalFn, Interpreter,
};
pub use options::{
    BorrowMode, CheckPolicy, ExecutionMode, ExternalCallPolicy, MachineOptions, RuntimePolicy,
};
pub use statistics::Statistics;
pub use threaded::{
    ArgumentRange, ControlFlow, CopyPair, CopyRange, SwitchCase, SwitchRange, ThreadedBlock,
    ThreadedFunction, ThreadedHandler, ThreadedInstruction, ThreadedInstructionData, ThreadedState,
};

/// Resize a stack and clear the active range.
#[allow(clippy::uninit_vec)]
fn resize_and_clear_stack(stack: &mut Vec<Value>, base: usize, end: usize) {
    // validate bounds
    debug_assert!(base <= end, "stack range out of bounds: {base}..{end}");

    // resize without redundant initialization
    if end > stack.len() {
        let additional = end - stack.len();
        stack.reserve(additional);

        // safety: fill the new range immediately
        unsafe {
            stack.set_len(end);
        }
    } else {
        stack.truncate(end);
    }

    // clear active stack slots
    stack[base..end].fill(Value::VOID);
}
