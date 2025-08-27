//! Wiring for `LanguageServer` with client and socket.

use std::sync::{mpsc, Arc};

use crate::vendor::client::Client;
use crate::vendor::language_server::LanguageServer;

#[derive(Debug)]
pub struct LspService<S: LanguageServer> {
	server: Arc<S>,
	outgoing_tx: mpsc::Sender<super::server::OutgoingMessage>,
}

impl<S: LanguageServer> LspService<S> {
	pub fn new<F>(factory: F) -> (Self, Socket)
	where
		F: FnOnce(Client) -> S,
	{
		let (tx, rx) = mpsc::channel();
		let client = Client::new(tx.clone());
		let server = Arc::new(factory(client));
		(
			Self { server, outgoing_tx: tx },
			Socket { rx },
		)
	}

	pub(crate) fn sender(&self) -> mpsc::Sender<super::server::OutgoingMessage> { self.outgoing_tx.clone() }

	pub(crate) fn server(&self) -> Arc<S> { self.server.clone() }
}

#[derive(Debug)]
pub struct Socket {
	pub(crate) rx: mpsc::Receiver<super::server::OutgoingMessage>,
}


