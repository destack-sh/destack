use std::convert::Infallible;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::ServiceId;

/// One statically typed RPC method.
#[derive(Debug)]
pub struct Method<Request, Response, Input = Infallible, Output = Infallible> {
    /// Owning service.
    service: ServiceId,
    /// Stable method identifier.
    id: MethodId,
    /// Exact method contract.
    fingerprint: MethodFingerprint,
    /// Method streaming behavior.
    kind: MethodKind,
    /// Compile-time method value types.
    marker: PhantomData<fn(Request, Input) -> (Output, Response)>,
}

impl<Request, Response, Input, Output> Method<Request, Response, Input, Output> {
    /// Create one statically typed method with its canonical streaming kind.
    const fn new(
        service: ServiceId,
        id: MethodId,
        fingerprint: MethodFingerprint,
        kind: MethodKind,
    ) -> Self {
        Self {
            service,
            id,
            fingerprint,
            kind,
            marker: PhantomData,
        }
    }

    /// Return this method's service identifier.
    pub const fn service(&self) -> ServiceId {
        self.service
    }

    /// Return this method's identifier.
    pub const fn id(&self) -> MethodId {
        self.id
    }

    /// Return this method's exact contract fingerprint.
    pub const fn fingerprint(&self) -> MethodFingerprint {
        self.fingerprint
    }

    /// Return this method's streaming behavior.
    pub const fn kind(&self) -> MethodKind {
        self.kind
    }
}

impl<Request, Response> Method<Request, Response> {
    /// Create one unary method.
    pub const fn unary(service: ServiceId, id: MethodId, fingerprint: MethodFingerprint) -> Self {
        Self::new(service, id, fingerprint, MethodKind::Unary)
    }
}

impl<Request, Response, Output> Method<Request, Response, Infallible, Output> {
    /// Create one server-streaming method.
    pub const fn server_streaming(
        service: ServiceId,
        id: MethodId,
        fingerprint: MethodFingerprint,
    ) -> Self {
        Self::new(service, id, fingerprint, MethodKind::ServerStreaming)
    }
}

impl<Request, Response, Input> Method<Request, Response, Input> {
    /// Create one client-streaming method.
    pub const fn client_streaming(
        service: ServiceId,
        id: MethodId,
        fingerprint: MethodFingerprint,
    ) -> Self {
        Self::new(service, id, fingerprint, MethodKind::ClientStreaming)
    }
}

impl<Request, Response, Input, Output> Method<Request, Response, Input, Output> {
    /// Create one bidirectional-streaming method.
    pub const fn bidirectional_streaming(
        service: ServiceId,
        id: MethodId,
        fingerprint: MethodFingerprint,
    ) -> Self {
        Self::new(service, id, fingerprint, MethodKind::BidirectionalStreaming)
    }
}

impl<Request, Response, Input, Output> Clone for Method<Request, Response, Input, Output> {
    /// Copy this statically typed method.
    fn clone(&self) -> Self {
        *self
    }
}

impl<Request, Response, Input, Output> Copy for Method<Request, Response, Input, Output> {}

/// Stable method identifier within one RPC service.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct MethodId(pub u64);

impl MethodId {
    /// Create one method identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Derive one method identifier from its service and method names.
    pub fn for_name(service: &str, method: &str) -> Self {
        Self(tspp_core::stable_hash_key_value(
            service.as_bytes(),
            method.as_bytes(),
        ))
    }
}

/// Stable canonical RPC method schema fingerprint.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MethodFingerprint(pub u128);

impl MethodFingerprint {
    /// Create one method fingerprint.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }
}

/// RPC method streaming behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum MethodKind {
    /// One request followed by one response.
    Unary,
    /// One request followed by output items and one response.
    ServerStreaming,
    /// One request and input items followed by one response.
    ClientStreaming,
    /// One request with input and output items followed by one response.
    BidirectionalStreaming,
}

impl MethodKind {
    /// Return whether callers may send stream items.
    pub const fn has_input(self) -> bool {
        matches!(self, Self::ClientStreaming | Self::BidirectionalStreaming)
    }

    /// Return whether services may send stream items.
    pub const fn has_output(self) -> bool {
        matches!(self, Self::ServerStreaming | Self::BidirectionalStreaming)
    }
}

/// Whether repeated calls may change observable application state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Idempotency {
    /// The service makes no guarantee about repeated calls.
    Unknown,
    /// Repeating the call has the same intended effect as calling it once.
    Idempotent,
    /// The call does not change observable application state.
    NoSideEffects,
}
