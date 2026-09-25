use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::Metadata;

/// One successful RPC response and its metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Response<T> {
    /// Response metadata.
    pub metadata: Metadata,
    /// Response value.
    pub value: T,
}

impl<T> Response<T> {
    /// Create one response without metadata.
    pub fn new(value: T) -> Self {
        Self {
            metadata: Metadata::new(),
            value,
        }
    }

    /// Consume this response and return its value.
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Return the response value.
    pub const fn get_ref(&self) -> &T {
        &self.value
    }

    /// Return the mutable response value.
    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T> From<T> for Response<T> {
    /// Wrap one value as a response without metadata.
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
