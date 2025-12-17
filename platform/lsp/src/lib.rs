#![feature(default_field_values)]
#![feature(if_let_guard)]

pub mod query;
pub mod server;

pub use query::*;
pub use server::DestackLanguageServer;

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
