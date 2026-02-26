use std::backtrace::Backtrace;
use tracing_subscriber::EnvFilter;

/// Initialize tracing for the language server process.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_ansi(false)
        .init();
}

/// Install a panic hook that reports crash details through tracing.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = Backtrace::force_capture();
        tracing::error!(panic = %panic_info, %backtrace, "destack-lsp panic");
    }));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    install_panic_hook();
    destack_lsp::run_stdio_server().await
}
