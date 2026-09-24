use serde::{Deserialize, Serialize};

/// Native execution exit details.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exit {
    /// Exit kind written by runtime operations that leave native execution.
    pub kind: ExitKind,
    /// Frame map associated with one non-completion exit.
    pub frame_map: u32,
}

/// Native entry exit kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitKind {
    /// Execution completed normally.
    Completed = 0,
    /// Execution completed through cancellation cleanup.
    Cancelled = 1,
    /// Execution awaited one asynchronous value.
    Awaited = 2,
    /// Execution yielded one generator value.
    Yielded = 3,
    /// Execution stopped with a language panic.
    Panicked = 4,
    /// Execution stopped for host inspection.
    Stopped = 5,
    /// Execution deoptimized into bytecode state.
    Deoptimized = 6,
}

#[allow(clippy::new_without_default)]
impl Exit {
    /// Create one completed native exit record.
    pub const fn new() -> Self {
        Self {
            kind: ExitKind::Completed,
            frame_map: 0,
        }
    }

    /// Set this record to one debugger stop exit.
    pub fn stop(&mut self, frame_map: u32) {
        self.kind = ExitKind::Stopped;
        self.frame_map = frame_map;
    }

    /// Set this record to one deoptimization exit.
    pub fn deoptimize(&mut self, frame_map: u32) {
        self.kind = ExitKind::Deoptimized;
        self.frame_map = frame_map;
    }
}
