//! MIR interpreter for the Destack machine.

mod frame;
mod interpreter;

pub use frame::Frame;
pub use interpreter::{ExternalFn, Interpreter};
