use crate::platform::host::HostResult;

/// Console output stream selector.
#[derive(Debug, Clone, Copy)]
pub enum ConsoleStream {
    /// Standard output stream.
    Stdout,
    /// Standard error stream.
    Stderr,
}

/// Emit a formatted console line.
pub fn emit_console_line(line: &str, stream: ConsoleStream) -> HostResult<()> {
    // emit the formatted line
    match stream {
        ConsoleStream::Stdout => println!("{line}"),
        ConsoleStream::Stderr => eprintln!("{line}"),
    }

    Ok(())
}
