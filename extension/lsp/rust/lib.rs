//! Dyst Language Server library.
//!
//! Provides a synchronous stdio LSP server entrypoint and internal modules for
//! document storage and semantic token computation.

pub mod semantic;
pub mod server;
pub mod document;
pub mod workspace;
pub mod source;

pub use server::DestackLanguageServer;
pub use document::Document;
pub use workspace::Workspace;
pub use source::*;

use tower_lsp_server::{LspService, Server};

/// Run the language server over stdio.
pub async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    // create the server
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(DestackLanguageServer::new);
    
    // run the server
    Server::new(stdin, stdout, socket).serve(service).await;
    
    // done
    Ok(())
}
