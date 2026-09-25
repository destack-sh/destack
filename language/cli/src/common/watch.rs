use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tspp_repository::{Commit, Revision};

use crate::common::format::DiagnosticOutputJson;
use crate::console;
use crate::diagnostic::{ConsoleError, ConsoleResult};

/// Schema identifier for semantic watch reports.
const WATCH_REPORT_SCHEMA: &str = "tspp.watch.v1";

/// Reason a watch compile was triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchCompileReason {
    /// Initial compile at the subscribed revision.
    Startup,
    /// A semantic workspace commit triggered the compile.
    Commit,
}

/// JSON payload for one watch compile.
#[derive(Debug, Serialize)]
pub struct WatchCompileJson {
    /// Reason for the compile.
    pub reason: WatchCompileReason,
    /// Exact revision compiled by the command.
    pub revision: Revision,
    /// Diagnostics emitted during the compile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticOutputJson>,
    /// Exit code derived from diagnostics.
    pub exit_code: i32,
}

/// JSON payload for semantic watch reports.
#[derive(Debug, Serialize)]
pub struct WatchReport {
    /// Watch schema identifier.
    pub schema: &'static str,
    /// Command associated with the report.
    pub command: String,
    /// Sequence identifier for report ordering.
    pub sequence: u64,
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Report event payload.
    #[serde(flatten)]
    pub event: WatchReportEvent,
}

impl WatchReport {
    /// Build one report using the current Unix time.
    fn now(command: String, sequence: u64, event: WatchReportEvent) -> ConsoleResult<Self> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                ConsoleError::message(format!("system clock is before unix epoch: {error}"))
            })?;

        Ok(Self {
            schema: WATCH_REPORT_SCHEMA,
            command,
            sequence,
            timestamp_ms: duration.as_millis() as u64,
            event,
        })
    }
}

/// Event payload for semantic watch reports.
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WatchReportEvent {
    /// Watch mode started at one exact revision.
    Start {
        /// Watched workspace root.
        root: String,
        /// Exact subscription revision.
        revision: Revision,
    },
    /// One semantic workspace commit was observed.
    Commit {
        /// Exact committed transition.
        commit: Commit,
    },
    /// One command compile completed.
    Compile {
        /// Compile result.
        compile: WatchCompileJson,
    },
    /// One warning was emitted.
    Warning {
        /// Warning message.
        message: String,
    },
    /// One error was emitted.
    Error {
        /// Error message.
        message: String,
    },
    /// Watch mode stopped.
    Stop,
}

/// JSON line emitter for semantic watch mode.
#[derive(Debug)]
pub struct WatchReporter {
    /// Command associated with emitted events.
    command: String,
    /// Sequence counter for emitted reports.
    sequence: u64,
}

impl WatchReporter {
    /// Create a reporter for one command.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            sequence: 0,
        }
    }

    /// Emit the exact semantic watch start.
    pub fn emit_start(&mut self, root: &Path, revision: Revision) {
        self.emit(WatchReportEvent::Start {
            root: root.display().to_string(),
            revision,
        });
    }

    /// Emit one exact semantic commit.
    pub fn emit_commit(&mut self, commit: &Commit) {
        self.emit(WatchReportEvent::Commit {
            commit: commit.clone(),
        });
    }

    /// Emit one command compile.
    pub fn emit_compile(&mut self, compile: WatchCompileJson) {
        self.emit(WatchReportEvent::Compile { compile });
    }

    /// Emit one watch warning.
    pub fn emit_warning(&mut self, message: &str) {
        self.emit(WatchReportEvent::Warning {
            message: message.to_string(),
        });
    }

    /// Emit one watch error.
    pub fn emit_error(&mut self, message: &str) {
        self.emit(WatchReportEvent::Error {
            message: message.to_string(),
        });
    }

    /// Emit watch termination.
    pub fn emit_stop(&mut self) {
        self.emit(WatchReportEvent::Stop);
    }

    /// Emit one serialized JSON report.
    fn emit(&mut self, event: WatchReportEvent) {
        self.sequence += 1;

        // build the complete timestamped report
        let report = match WatchReport::now(self.command.clone(), self.sequence, event) {
            Ok(report) => report,
            Err(error) => {
                console::error(&format!("watch: failed to emit event: {error}"));

                return;
            }
        };

        // serialize one complete report per line
        match serde_json::to_string(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => console::error(&format!("watch: failed to serialize event: {error}")),
        }
    }
}
