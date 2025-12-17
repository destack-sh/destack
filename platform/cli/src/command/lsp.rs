use clap::Args;

#[derive(Args, Debug)]
pub struct LspArgs {}

/// Run the language server over stdio.
pub fn run(_args: &LspArgs) -> i32 {
    // build a runtime for the LSP (it's async)
    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    runtime.block_on(async {
        if let Err(e) = destack_lsp::run_stdio_server().await {
            eprintln!("lsp error: {e}");
            return 1;
        }
        0
    })
}
