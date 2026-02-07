mod argument;
mod assign;
mod call;
mod constraint;
mod context;
mod declaration;
mod r#enum;
mod expected;
mod expression;
mod flow;
mod instance;
mod key;
mod known;
mod member;
mod merge;
mod operator;
mod parameter;
mod pattern;
mod process;
mod resolution;
mod session;
mod solve;
mod template;
mod r#type;

use key::*;

/// Mode used when resolving call signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SignatureResolutionMode {
    /// Resolve signatures for inference (preserve inference variables).
    Inference,
    /// Resolve signatures for assignability and diagnostics.
    Checking,
}

pub use assign::*;
pub use context::*;
pub use session::*;
pub use solve::*;

#[cfg(test)]
mod tests;
