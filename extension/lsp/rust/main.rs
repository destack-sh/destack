use destack_extension_lsp::DestackLanguageServer;

use destack_extension_lsp::protocol::{LspService, Server};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
