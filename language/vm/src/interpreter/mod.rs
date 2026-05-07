mod continuation;
mod frame;
mod interpreter;
mod machine;
mod stack;

pub use continuation::{Continuation, ContinuationFrame, ContinuationImage};
pub(crate) use frame::{
    ExceptionalCall, visit_frame_slot_root_slots, visit_frame_slot_roots, visit_materialized_slots,
};
pub use frame::{Frame, FrameImage};
pub use interpreter::{Interpreter, InterpreterImage, Outcome};
pub(crate) use machine::Machine;
pub(crate) use stack::Stack;
