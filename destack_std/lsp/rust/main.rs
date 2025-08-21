use destack_std_lsp::doc::DocumentStore;
use destack_std_lsp::server::Backend;

use tower_lsp_server::{LspService, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    async move {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(|client| Backend {
            client,
            docs: DocumentStore::default(),
        });
        Server::new(stdin, stdout, socket).serve(service).await;
        Ok(())
    }
    .await
}
