use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::{ServerCall, ServiceError, ServiceSchema};

/// One RPC service implementation.
pub trait Service: std::fmt::Debug + Send + Sync + 'static {
    /// Return this service's canonical schema.
    fn schema(&self) -> &ServiceSchema;

    /// Handle one call.
    fn call(&self, call: ServerCall) -> ServiceFuture<'_>;
}

/// One resumable RPC service call.
pub type ServiceFuture<'a> = Pin<Box<dyn Future<Output = Result<(), ServiceError>> + Send + 'a>>;

/// Stable RPC service identifier.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct ServiceId(pub u64);

impl ServiceId {
    /// Create one service identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Derive one service identifier from its canonical name.
    pub fn for_name(name: &str) -> Self {
        Self(tspp_core::stable_hash_text(name))
    }
}

/// Stable canonical service schema fingerprint.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ServiceFingerprint(pub u128);

impl ServiceFingerprint {
    /// Create one service fingerprint.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }
}
