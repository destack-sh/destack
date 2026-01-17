use std::sync::Arc;

use super::error::HostResult;

/// Stream selector for host I/O.
#[derive(Debug, Clone, Copy)]
pub enum IoStream {
    /// Standard output stream.
    Stdout,
    /// Standard error stream.
    Stderr,
}

/// Line sink for host I/O.
#[derive(Clone)]
pub enum LineSink {
    /// Write to stdout.
    Stdout,
    /// Write to stderr.
    Stderr,
    /// Write to a custom sink.
    Custom(Arc<dyn Fn(&str) + Send + Sync>),
}

impl std::fmt::Debug for LineSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineSink::Stdout => f.write_str("LineSink::Stdout"),
            LineSink::Stderr => f.write_str("LineSink::Stderr"),
            LineSink::Custom(_) => f.write_str("LineSink::Custom(..)"),
        }
    }
}

impl LineSink {
    /// Write a line to this sink.
    pub fn write_line(&self, line: &str) -> HostResult<()> {
        match self {
            LineSink::Stdout => println!("{line}"),
            LineSink::Stderr => eprintln!("{line}"),
            LineSink::Custom(sink) => sink(line),
        }

        Ok(())
    }
}

/// Host-provided I/O streams.
#[derive(Debug, Clone)]
pub struct HostIo {
    /// Sink for stdout lines.
    pub stdout: LineSink,
    /// Sink for stderr lines.
    pub stderr: LineSink,
}

impl Default for HostIo {
    fn default() -> Self {
        Self {
            stdout: LineSink::Stdout,
            stderr: LineSink::Stderr,
        }
    }
}

impl HostIo {
    /// Route all output to stderr.
    pub fn stderr_only() -> Self {
        Self {
            stdout: LineSink::Stderr,
            stderr: LineSink::Stderr,
        }
    }

    /// Write a line to the selected stream.
    pub fn write_line(&self, stream: IoStream, line: &str) -> HostResult<()> {
        match stream {
            IoStream::Stdout => self.stdout.write_line(line),
            IoStream::Stderr => self.stderr.write_line(line),
        }
    }
}
