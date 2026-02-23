mod common;
mod constraint;
mod declaration;
mod dependency;
mod expression;
pub(crate) mod member;
mod obligation;
mod operator;
mod process;
pub(crate) mod provisional;
mod r#type;

pub(crate) use expression::{argument, call};

/// Mode used when resolving call signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignatureResolutionMode {
    /// Resolve signatures to synthesize types while preserving inference variables.
    Synthesize,
    /// Resolve signatures to check compatibility and diagnostics.
    Check,
}

pub use common::*;

#[cfg(test)]
mod tests;
