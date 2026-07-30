#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    destack_lsp::DestackLanguageServer::run_stdio().await
}
