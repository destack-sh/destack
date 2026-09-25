use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::Metadata;

/// One RPC request and its call options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Request<T> {
    /// Request metadata.
    pub metadata: Metadata,
    /// Request value.
    pub value: T,
}

impl<T> Request<T> {
    /// Create one request from exact call options.
    pub(crate) fn from_parts(metadata: Metadata, value: T) -> Self {
        Self { metadata, value }
    }

    /// Create one request without metadata.
    pub fn new(value: T) -> Self {
        Self {
            metadata: Metadata::new(),
            value,
        }
    }

    /// Consume this request and return its value.
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Return the request value.
    pub const fn get_ref(&self) -> &T {
        &self.value
    }

    /// Return the mutable request value.
    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Map this request value while preserving its metadata.
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Request<U> {
        Request {
            metadata: self.metadata,
            value: map(self.value),
        }
    }
}

/// A value that can open one typed RPC request.
pub trait IntoRequest<T> {
    /// Convert this value into an RPC request.
    fn into_request(self) -> Request<T>;
}

impl<T> IntoRequest<T> for T {
    /// Wrap this value without metadata.
    fn into_request(self) -> Request<T> {
        Request::new(self)
    }
}

impl<T> IntoRequest<T> for Request<T> {
    /// Preserve this complete request.
    fn into_request(self) -> Request<T> {
        self
    }
}
