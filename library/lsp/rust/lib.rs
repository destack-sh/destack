//! Destack Language Server library.
//!
//! Provides a synchronous stdio LSP server entrypoint and internal modules for
//! document storage and semantic token computation.

pub mod doc;
pub mod semantic;
pub mod server;
pub mod vendor;

pub use doc::DocumentStore;

use crate::vendor::{LspService, Server};

use crate::server::Backend;

/// Run the language server over stdio.
pub fn run_libraryio_server_stdio() -> Result<(), Box<dyn std::error::Error>> {
	let stdin = std::io::stdin();
	let stdout = std::io::stdout();
	let (service, socket) = LspService::new(|client| Backend {
		client,
		docs: DocumentStore::default(),
	});
	Server::new(stdin, stdout, socket).serve(service)?;
	Ok(())
}
