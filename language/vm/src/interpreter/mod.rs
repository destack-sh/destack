mod continuation;
mod frame;
mod interpreter;
mod machine;
mod root;
mod stack;

pub use continuation::{Continuation, ContinuationFrame, ContinuationImage};
pub use frame::{Frame, FrameImage};
pub use interpreter::{Interpreter, InterpreterImage, Outcome};
pub(crate) use machine::Machine;
pub(crate) use root::{visit_frame_slot_root_slots, visit_materialized_slots};
pub(crate) use stack::Stack;
pub use stack::StackImage;
