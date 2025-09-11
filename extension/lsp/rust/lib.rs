//! Dyst Language Server library.
//!
//! Provides a synchronous stdio LSP server entrypoint and internal modules for
//! document storage and semantic token computation.

pub mod doc;
pub mod protocol;
pub mod semantic;
pub mod server;

pub use doc::DocumentStore;

use crate::protocol::{LspService, Server};

use crate::server::DestackLanguageServer;

/// Run the language server over stdio.
pub fn run_libraryio_server_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let (service, socket) = LspService::new(|client| DestackLanguageServer {
        client,
        docs: DocumentStore::default(),
    });
    Server::new(stdin, stdout, socket).serve(service)?;
    Ok(())
}
