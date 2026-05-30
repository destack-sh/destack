#![allow(
    clippy::approx_constant,
    clippy::arc_with_non_send_sync,
    clippy::mut_from_ref,
    clippy::too_many_arguments
)]

pub mod diagnostic;
pub(crate) mod execute;
pub mod lower;
pub mod machine;
pub mod options;
pub mod program;
mod value;

pub use diagnostic::*;
pub use machine::*;
pub use options::*;
pub use program::*;
pub use value::*;

#[cfg(test)]
mod tests;
