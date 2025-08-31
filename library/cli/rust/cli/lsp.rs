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
    // NOTE: don't print anything to stdout or stderr so we don't interfere with the LSP protocol

    match destack_extension_lsp::run_libraryio_server_stdio() {
        Ok(_) => 0,
        Err(e) => {
            console::error(&format!("LSP exited with error: {e}"));
            1
        }
    }
}
