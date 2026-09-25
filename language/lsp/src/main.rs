#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tspp_lsp::TsppLanguageServer::run_stdio().await
}
