//! Language server subcommand.

use crate::console::console;
use crate::console::parse::{CommandApp, CommandArguments};

/// Create the lsp command app.
pub fn app() -> CommandApp {
    CommandApp::new("lsp")
        .help("Run the Destack Language Server (stdio).")
        .command(
            "run",
            run,
            Some("Run the LSP server over stdio.".to_string()),
        )
}

/// Run the LSP server over stdio.
fn run(_ctx: CommandArguments) -> i32 {
    // NOTE: don't print anything to stdout or stderr so we don't interfere with the LSP protocol

    // create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new();
    let Ok(runtime) = rt else {
        console::error("Failed to create tokio runtime");
        return 1;
    };

    // run the lsp server and handle result
    let exit_result = runtime.block_on(destack_std_lsp::run_stdio_server());
    match exit_result {
        Ok(_) => 0,
        Err(e) => {
            console::error(&format!("LSP exited with error: {e}"));
            1
        }
    }
}
