use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::Status;
use super::payload::{Payload, PayloadChunk};
use crate::{CallId, MethodId, Request, Response, ServiceId};

/// One encoded RPC message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) enum Message {
    /// Open one call.
    Start(CallStart),
    /// Transfer one stream item.
    Item(StreamItem),
    /// Close the caller-to-callee stream.
    Close(StreamClose),
    /// Increase one stream's send window.
    Window(WindowUpdate),
    /// Complete one call.
    Complete(Completion),
    /// Request cancellation of one call.
    Cancel(Cancel),
    /// Transfer contiguous bytes for one deferred payload.
    Chunk(PayloadChunk),
}

impl Message {
    /// Return this message's encoded value payload when it has one.
    pub(crate) fn payload_mut(&mut self) -> Option<&mut Payload> {
        match self {
            Self::Start(start) => Some(&mut start.request.value),
            Self::Item(item) => Some(&mut item.payload),
            Self::Complete(Completion::Response { response, .. }) => Some(&mut response.value),
            _ => None,
        }
    }
}

/// Message opening one RPC call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct CallStart {
    /// Caller-scoped call identifier.
    pub(crate) call: CallId,
    /// Called service.
    pub(crate) service: ServiceId,
    /// Called method.
    pub(crate) method: MethodId,
    /// Encoded request.
    pub(crate) request: Request<Payload>,
}

/// Message carrying one stream item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct StreamItem {
    /// Owning call.
    pub(crate) call: CallId,
    /// Encoded stream item.
    pub(crate) payload: Payload,
}

/// Message closing the caller-to-callee stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct StreamClose {
    /// Owning call.
    pub(crate) call: CallId,
}

/// Message increasing one stream's send window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct WindowUpdate {
    /// Owning call.
    pub(crate) call: CallId,
    /// Additional permitted items.
    pub(crate) items: u32,
}

/// Terminal completion of one RPC call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) enum Completion {
    /// Complete one call successfully.
    Response {
        /// Completed call.
        call: CallId,
        /// Successful response.
        response: Response<Payload>,
    },
    /// Complete one call unsuccessfully.
    Status {
        /// Completed call.
        call: CallId,
        /// Terminal failure.
        status: Status,
    },
}

impl Completion {
    /// Return the completed call identifier.
    pub(crate) const fn call(&self) -> CallId {
        match self {
            Self::Response { call, .. } | Self::Status { call, .. } => *call,
        }
    }

    /// Consume this completion as a typed result.
    pub(crate) fn into_result(self) -> Result<Response<Payload>, Status> {
        match self {
            Self::Response { response, .. } => Ok(response),
            Self::Status { status, .. } => Err(status),
        }
    }
}

/// Message requesting cancellation of one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct Cancel {
    /// Canceled call.
    pub(crate) call: CallId,
}
