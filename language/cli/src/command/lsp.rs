use clap::Args;

use crate::console;

/// Arguments for the lsp command.
#[derive(Args, Debug, Clone)]
pub struct LspArgs {}

/// Run the language server over stdio.
pub fn run(_args: &LspArgs) -> i32 {
    // build a runtime for the LSP (it's async)
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            console::error(&format!("lsp error: {error}"));
            return 1;
        }
    };

    runtime.block_on(async {
        if let Err(error) = tspp_lsp::TsppLanguageServer::run_stdio().await {
            console::error(&format!("lsp error: {error}"));
            return 1;
        }
        0
    })
}
