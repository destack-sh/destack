use destack_library_lsp::doc::DocumentStore;
use destack_library_lsp::server::Backend;

use destack_library_lsp::vendor::{LspService, Server};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        docs: DocumentStore::default(),
    });
    Server::new(stdin, stdout, socket).serve(service)?;
    Ok(())
}
