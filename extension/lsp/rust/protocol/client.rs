//! Client for sending notifications to the editor.

use std::sync::mpsc;

use serde_json::Value as JsonValue;

use crate::protocol::types;

use super::jsonrpc::JsonRpcMessage;

#[derive(Debug, Clone)]
pub struct Client {
    tx: mpsc::Sender<JsonRpcMessage>,
}

impl Client {
    pub(crate) fn new(tx: mpsc::Sender<JsonRpcMessage>) -> Self {
        Self { tx }
    }

    /// Send a window/logMessage notification.
    pub fn log_message(&self, typ: types::MessageType, message: impl Into<String>) {
        let params = types::LogMessageParams {
            typ,
            message: message.into(),
        };
        let notification = JsonRpcMessage::Notification {
            method: "window/logMessage".to_string(),
            params: serde_json::to_value(params).unwrap_or(JsonValue::Null),
        };
        let _ = self.tx.send(notification);
    }
}
