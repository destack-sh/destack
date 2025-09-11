use destack_extension_lsp::doc::DocumentStore;
use destack_extension_lsp::server::DestackLanguageServer;

use destack_extension_lsp::protocol::{LspService, Server};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let (service, socket) = LspService::new(|client| DestackLanguageServer {
        client,
        docs: DocumentStore::default(),
    });
    Server::new(stdin, stdout, socket).serve(service)?;
    Ok(())
}
