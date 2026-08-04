#![allow(
    clippy::approx_constant,
    clippy::arc_with_non_send_sync,
    clippy::too_many_arguments
)]

pub mod diagnostic;
pub(crate) mod execute;
pub mod machine;
pub mod options;

pub use diagnostic::*;
pub use execute::TensorExecutor;
pub use machine::*;
pub use options::*;

#[cfg(test)]
mod tests;
