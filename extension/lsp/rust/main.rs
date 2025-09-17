#[cfg(debug_assertions)]
fn wait_for_debugger() {
    #[cfg(unix)]
    unsafe {
        libc::raise(libc::SIGSTOP);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(wait) = std::env::var("WAIT_FOR_DEBUGGER")
        && wait != "0"
    {
        wait_for_debugger();
    }
    destack_extension_lsp::run_stdio_server().await
}
