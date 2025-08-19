#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    destack_lsp::run_stdio_server().await
}
