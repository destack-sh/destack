//! Language server subcommand.

use crate::console::console;
use crate::console::parser::{App, CommandArgs};

/// Create the lsp command app.
pub fn app() -> App {
    App::new("lsp")
        .help("Run the Destack Language Server (stdio).")
        .command(
            "run",
            run,
            Some("Run the LSP server over stdio.".to_string()),
        )
}

/// Run the LSP server over stdio.
pub fn run(_ctx: CommandArgs) -> i32 {
    // create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new();
    let Ok(runtime) = rt else {
        console::error("Failed to create tokio runtime");
        return 1;
    };

    // run the lsp server and handle result
    console::print("Running destack_lsp (stdio)...");
    let exit_result = runtime.block_on(destack_lsp::run_stdio_server());
    match exit_result {
        Ok(_) => 0,
        Err(e) => {
            console::error(&format!("LSP exited with error: {e}"));
            1
        }
    }
}
