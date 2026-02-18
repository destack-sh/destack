mod commit;
mod common;
mod constraint;
mod declaration;
mod expression;
pub(crate) mod member;
mod operator;
mod process;
mod r#type;

pub(crate) use expression::{argument, call};

/// Mode used when resolving call signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignatureResolutionMode {
    /// Resolve signatures for inference (preserve inference variables).
    Inference,
    /// Resolve signatures for assignability and diagnostics.
    Checking,
}

pub use common::*;
pub use constraint::*;

#[cfg(test)]
mod tests;
