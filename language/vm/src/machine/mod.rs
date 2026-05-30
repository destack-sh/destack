mod activation;
mod continuation;
mod engine;
mod execute;
mod frame;
mod machine;
mod root;
mod stack;

pub(crate) use activation::Activation;
pub use continuation::{Continuation, ContinuationFrame, ContinuationImage};
pub use frame::{Frame, FrameImage};
pub use machine::{Machine, MachineImage, Outcome};
pub(crate) use root::{visit_frame_slot_root_slots, visit_materialized_slots};
pub(crate) use stack::Stack;
pub use stack::StackImage;
