use clap::Args;

/// Arguments for the lsp command.
#[derive(Args, Debug)]
pub struct LspArgs {}

/// Run the language server over stdio.
pub fn run(_args: &LspArgs) -> i32 {
    // build a runtime for the LSP (it's async)
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("lsp error: {error}");
            return 1;
        }
    };
    runtime.block_on(async {
        if let Err(e) = destack_lsp::run_stdio_server().await {
            eprintln!("lsp error: {e}");
            return 1;
        }
        0
    })
}
