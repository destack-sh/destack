mod activation;
mod continuation;
mod execute;
mod frame;
mod global;
mod heap;
mod machine;
mod root;
mod stack;

pub(crate) use activation::Activation;
pub use destack_program::{Continuation, FrameImage, StackImage};
pub use frame::Frame;
pub use machine::{Machine, MachineImage, Outcome};
pub(crate) use root::{visit_frame_slot_root_slots, visit_materialized_slots};
pub(crate) use stack::Stack;
