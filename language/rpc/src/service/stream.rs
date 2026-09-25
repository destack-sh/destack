use std::marker::PhantomData;
use std::sync::Arc;

use futures::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;
use futures::future::poll_fn;
use tspp_serde::Codec;

use super::ServiceError;
use super::call::{CallState, ServerEvent};

/// Caller-to-service items for one active RPC call.
#[derive(Debug)]
pub struct RequestStream<T> {
    /// Shared call state.
    state: Arc<CallState>,
    /// Routed caller stream events.
    receiver: UnboundedReceiver<ServerEvent>,
    /// Statically typed stream item.
    marker: PhantomData<fn() -> T>,
}

impl<T> RequestStream<T> {
    /// Create one typed request stream.
    pub(super) fn new(state: Arc<CallState>, receiver: UnboundedReceiver<ServerEvent>) -> Self {
        Self {
            state,
            receiver,
            marker: PhantomData,
        }
    }

    /// Receive one caller-to-service item.
    pub async fn receive(&mut self) -> Result<Option<T>, ServiceError>
    where
        T: Codec,
    {
        match self.receiver.next().await {
            Some(ServerEvent::Input(payload)) => {
                let input = payload.decode().map_err(ServiceError::from_payload)?;
                self.state.release_input()?;

                Ok(Some(input))
            }
            Some(ServerEvent::Close) => Ok(None),
            Some(ServerEvent::Cancel) => Err(ServiceError::Canceled),
            None if self.state.is_canceled() => Err(ServiceError::Canceled),
            None => Err(ServiceError::Complete),
        }
    }
}

/// Service-to-caller sender for one active RPC call.
#[derive(Debug)]
pub struct ResponseSender<T> {
    /// Shared call state.
    state: Arc<CallState>,
    /// Statically typed stream item.
    marker: PhantomData<fn(T)>,
}

impl<T> ResponseSender<T> {
    /// Create one typed response sender.
    pub(crate) fn new(state: Arc<CallState>) -> Self {
        Self {
            state,
            marker: PhantomData,
        }
    }

    /// Send one service-to-caller item.
    pub async fn send(&mut self, response: &T) -> Result<(), ServiceError>
    where
        T: Codec,
    {
        self.state.send(response).await
    }

    /// Send one item only when stream window is immediately available.
    pub fn try_send(&mut self, response: &T) -> Result<(), ServiceError>
    where
        T: Codec,
    {
        self.state.try_send(response)
    }

    /// Return whether the caller requested cancellation.
    pub fn is_canceled(&self) -> bool {
        self.state.is_canceled()
    }

    /// Wait until the caller cancels this request.
    pub async fn canceled(&self) {
        poll_fn(|context| self.state.poll_canceled(context)).await
    }
}
