use serde::{Deserialize, Serialize};

use crate::TypeReference;

/// Shared call facts for one call-like instruction or terminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call<A> {
    /// The call arguments.
    pub arguments: A,
    /// The signature type for the callee.
    pub signature: TypeReference,
}

impl<A> Call<A> {
    /// Create one call payload with default per-call facts.
    pub fn new(arguments: A, signature: TypeReference) -> Self {
        Self {
            arguments,
            signature,
        }
    }
}
