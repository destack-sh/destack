use destack_extension_lsp::Analyzer;
use destack_extension_lsp::server::DestackLanguageServer;
use destack_extension_lsp::source::SourceStore;

use destack_extension_lsp::protocol::{LspService, Server};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let (service, socket) = LspService::new(|client| DestackLanguageServer {
        client,
        sources: SourceStore::default(),
        analyzer: Analyzer::default(),
    });
    Server::new(stdin, stdout, socket).serve(service)?;
    Ok(())
}
