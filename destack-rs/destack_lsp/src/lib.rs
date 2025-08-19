//! Destack Language Server library.
//!
//! Provides an async entrypoint to run the LSP over stdio and internal modules for
//! document storage and semantic token computation.

pub mod doc_store;
pub mod semantic;
pub mod server;

pub use doc_store::DocumentStore;

use tower_lsp_server::{LspService, Server};

use crate::server::Backend;

/// Run the language server over stdio.
pub async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        docs: DocumentStore::default(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
