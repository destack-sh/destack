//! Language server subcommand.

use crate::console::console;
use crate::console::parse::{CommandApp, CommandArguments};

/// Create the lsp command app.
pub fn app() -> CommandApp {
    CommandApp::new("lsp")
        .help("Run the Destack Language Server (stdio).")
        .default_command("run")
        .command(
            "run",
            run,
            Some("Run the LSP server over stdio.".to_string()),
        )
}

/// Run the LSP server over stdio.
fn run(_ctx: CommandArguments) -> i32 {
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(destack_lsp::run_stdio_server()) {
        Ok(_) => 0,
        Err(e) => {
            console::error(&format!("LSP exited with error: {e}"));
            1
        }
    }
}
