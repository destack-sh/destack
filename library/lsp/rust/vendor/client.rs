//! Client for sending notifications to the editor.

use std::sync::mpsc;

use serde_json::Value as JsonValue;

use crate::vendor::lsp_types;

use super::server::OutgoingMessage;

#[derive(Debug, Clone)]
pub struct Client {
	tx: mpsc::Sender<OutgoingMessage>,
}

impl Client {
	pub(crate) fn new(tx: mpsc::Sender<OutgoingMessage>) -> Self { Self { tx } }

	/// Send a window/logMessage notification.
	pub fn log_message(&self, typ: lsp_types::MessageType, message: impl Into<String>) {
		let params = lsp_types::LogMessageParams { typ, message: message.into() };
		let notification = OutgoingMessage::Notification {
			method: "window/logMessage".to_string(),
			params: serde_json::to_value(params).unwrap_or(JsonValue::Null),
		};
		let _ = self.tx.send(notification);
	}
}


