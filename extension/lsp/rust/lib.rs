//! Dyst Language Server library.
//!
//! Provides a synchronous stdio LSP server entrypoint and internal modules for
//! document storage and semantic token computation.

pub mod analyzer;
pub mod protocol;
pub mod semantic;
pub mod server;
pub mod source;
pub mod workspace;

use crate::protocol::{LspService, Server};
pub use server::DestackLanguageServer;
pub use workspace::Workspace;

/// Run the language server over stdio.
pub fn run_libraryio_server_stdio() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let (service, socket) = LspService::new(|client| DestackLanguageServer {
        client,
        workspaces_by_uri: HashMap::default(),
    });
    Server::new(stdin, stdout, socket).serve(service)?;
    Ok(())
}
