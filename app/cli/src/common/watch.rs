use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use destack_daemon::WatchBatch;
use destack_source::{FileWatchEvent, FileWatchEventKind, FileWatchRescanReason, FileWatchStatus};
use serde::Serialize;

use crate::common::format::DiagnosticOutputJson;

/// Schema identifier for watch reports.
const WATCH_REPORT_SCHEMA: &str = "destack.watch.v1";

/// Reason a watch compile was triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchCompileReason {
    /// Initial compile when watch mode starts.
    Startup,
    /// A file update triggered a recompile.
    Update,
    /// A rescan triggered a recompile.
    Rescan,
    /// Both updates and rescans triggered a recompile.
    UpdateRescan,
}

/// JSON payload for a watch compile event.
#[derive(Debug, Serialize)]
pub struct WatchCompileJson {
    /// Reason for the compile.
    pub reason: WatchCompileReason,
    /// Whether at least one file update was applied.
    pub updated: bool,
    /// Whether a rescan was required.
    pub rescan: bool,
    /// The batch identifier associated with this compile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<u64>,
    /// Diagnostics emitted during the compile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticOutputJson>,
    /// Exit code derived from diagnostics.
    pub exit_code: i32,
}

/// JSON payload for a file watch event.
#[derive(Debug, Clone, Serialize)]
pub struct WatchFileEventJson {
    /// The event kind.
    pub kind: WatchFileEventKindJson,
    /// The path associated with the event.
    pub path: String,
    /// The previous path for rename events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_path: Option<String>,
}

/// JSON payload for a file watch status update.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WatchStatusJson {
    /// Watcher started and is ready.
    Ready { roots: Vec<String> },
    /// A rescan was requested.
    RescanRequested {
        roots: Vec<String>,
        reason: WatchRescanReasonJson,
    },
    /// A watcher error occurred.
    Error { message: String },
    /// Watcher has stopped.
    Stopped,
}

/// JSON payload for a watch batch.
#[derive(Debug, Clone, Serialize)]
pub struct WatchBatchJson {
    /// The batch identifier.
    pub id: u64,
    /// File events in this batch.
    pub events: Vec<WatchFileEventJson>,
    /// Status updates in this batch.
    pub status: Vec<WatchStatusJson>,
    /// Whether overflow events were observed.
    pub overflowed: bool,
    /// Batch duration in milliseconds.
    pub duration_ms: u64,
}

/// JSON payload for watch reports.
#[derive(Debug, Serialize)]
pub struct WatchReport {
    /// Watch schema identifier.
    pub schema: &'static str,
    /// Command name associated with the report.
    pub command: String,
    /// Sequence id for ordering events.
    pub sequence: u64,
    /// Timestamp in milliseconds since epoch.
    pub timestamp_ms: u64,
    /// The report event payload.
    #[serde(flatten)]
    pub event: WatchReportEvent,
}

/// Event payload for watch reports.
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WatchReportEvent {
    /// Watch mode started.
    Start { roots: Vec<String> },
    /// Batch of watch events.
    Batch { batch: WatchBatchJson },
    /// Compile result for a watch cycle.
    Compile { compile: WatchCompileJson },
    /// Warning message emitted during watch.
    Warning { message: String },
    /// Watch mode stopped.
    Stop,
}

/// JSON event kind for file watch events.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchFileEventKindJson {
    /// The path was created.
    Created,
    /// The path content changed.
    Modified,
    /// The path was removed.
    Deleted,
    /// The path was renamed or moved.
    Renamed,
    /// Events were dropped.
    Overflow,
}

/// JSON payload for rescan reasons.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchRescanReasonJson {
    /// Requested when the watcher first starts.
    Startup,
    /// Requested because events were dropped.
    Overflow,
    /// Requested by the caller.
    Manual,
    /// Requested after watch roots or options changed.
    Update,
}

/// JSON line emitter for watch mode.
#[derive(Debug)]
pub struct WatchReporter {
    /// Command name associated with events.
    command: String,
    /// Sequence counter for events.
    sequence: u64,
}

impl WatchReporter {
    /// Create a new watch reporter for a command.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            sequence: 0,
        }
    }

    /// Emit a watch start event.
    pub fn emit_start(&mut self, roots: &[PathBuf]) {
        let roots = roots
            .iter()
            .map(|root| root.display().to_string())
            .collect();
        self.emit(WatchReportEvent::Start { roots });
    }

    /// Emit a watch batch event.
    pub fn emit_batch(&mut self, id: u64, batch: &WatchBatch) {
        let payload = watch_batch_json(id, batch);
        self.emit(WatchReportEvent::Batch { batch: payload });
    }

    /// Emit a watch compile event.
    pub fn emit_compile(&mut self, compile: WatchCompileJson) {
        self.emit(WatchReportEvent::Compile { compile });
    }

    /// Emit a watch warning event.
    pub fn emit_warning(&mut self, message: &str) {
        self.emit(WatchReportEvent::Warning {
            message: message.to_string(),
        });
    }

    /// Emit a watch stop event.
    pub fn emit_stop(&mut self) {
        self.emit(WatchReportEvent::Stop);
    }

    /// Emit a watch event as json.
    fn emit(&mut self, event: WatchReportEvent) {
        // increment sequence
        self.sequence = self.sequence.saturating_add(1);

        // build the report payload
        let report = WatchReport {
            schema: WATCH_REPORT_SCHEMA,
            command: self.command.clone(),
            sequence: self.sequence,
            timestamp_ms: now_ms(),
            event,
        };

        // serialize to json
        match serde_json::to_string(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => eprintln!("watch: failed to serialize watch event: {error}"),
        }
    }
}

/// Build a json payload for a watch batch.
fn watch_batch_json(id: u64, batch: &WatchBatch) -> WatchBatchJson {
    let events = batch.events.iter().map(watch_event_json).collect();
    let status = batch.status.iter().map(watch_status_json).collect();
    WatchBatchJson {
        id,
        events,
        status,
        overflowed: batch.overflowed,
        duration_ms: batch.duration().as_millis() as u64,
    }
}

/// Build a json payload for a watch event.
fn watch_event_json(event: &FileWatchEvent) -> WatchFileEventJson {
    WatchFileEventJson {
        kind: watch_event_kind(&event.kind),
        path: event.path.display().to_string(),
        previous_path: event
            .previous_path
            .as_ref()
            .map(|path| path.display().to_string()),
    }
}

/// Build a json payload for a watch status update.
fn watch_status_json(status: &FileWatchStatus) -> WatchStatusJson {
    match status {
        FileWatchStatus::Ready { roots } => WatchStatusJson::Ready {
            roots: roots
                .iter()
                .map(|root| root.display().to_string())
                .collect(),
        },
        FileWatchStatus::RescanRequested { roots, reason } => WatchStatusJson::RescanRequested {
            roots: roots
                .iter()
                .map(|root| root.display().to_string())
                .collect(),
            reason: watch_rescan_reason(reason),
        },
        FileWatchStatus::Error { message } => WatchStatusJson::Error {
            message: message.clone(),
        },
        FileWatchStatus::Stopped => WatchStatusJson::Stopped,
    }
}

/// Convert a watch event kind to json.
fn watch_event_kind(kind: &FileWatchEventKind) -> WatchFileEventKindJson {
    match kind {
        FileWatchEventKind::Created => WatchFileEventKindJson::Created,
        FileWatchEventKind::Modified => WatchFileEventKindJson::Modified,
        FileWatchEventKind::Deleted => WatchFileEventKindJson::Deleted,
        FileWatchEventKind::Renamed => WatchFileEventKindJson::Renamed,
        FileWatchEventKind::Overflow => WatchFileEventKindJson::Overflow,
    }
}

/// Convert a rescan reason to json.
fn watch_rescan_reason(reason: &FileWatchRescanReason) -> WatchRescanReasonJson {
    match reason {
        FileWatchRescanReason::Startup => WatchRescanReasonJson::Startup,
        FileWatchRescanReason::Overflow => WatchRescanReasonJson::Overflow,
        FileWatchRescanReason::Manual => WatchRescanReasonJson::Manual,
        FileWatchRescanReason::Update => WatchRescanReasonJson::Update,
    }
}

/// Get the current time in milliseconds since epoch.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use destack_source::{
        Diagnostic, DiagnosticCollection, DiagnosticSeverity, File, FileType, LabeledSpan, Span,
        Uri,
    };
    use serde_json::Value;

    use crate::common::format::{FormatOptions, collect_diagnostics_json};

    use super::{
        WATCH_REPORT_SCHEMA, WatchCompileJson, WatchCompileReason, WatchReport, WatchReportEvent,
    };

    #[test]
    fn test_watch_report_serializes_warning() {
        let report = WatchReport {
            schema: WATCH_REPORT_SCHEMA,
            command: "check".to_string(),
            sequence: 1,
            timestamp_ms: 42,
            event: WatchReportEvent::Warning {
                message: "boom".to_string(),
            },
        };

        let value = serde_json::to_value(report).expect("report should serialize");
        let object = value.as_object().expect("report should be a json object");

        let schema = object.get("schema").expect("schema should exist");
        assert_eq!(schema, &Value::String(WATCH_REPORT_SCHEMA.to_string()));

        let event = object.get("event").expect("event should exist");
        assert_eq!(event, &Value::String("warning".to_string()));

        let message = object.get("message").expect("message should exist");
        assert_eq!(message, &Value::String("boom".to_string()));
    }

    #[test]
    fn test_watch_report_serializes_compile_with_diagnostics() {
        let mut files = BTreeMap::new();
        let uri = Uri::from_string("memory://test.ds");
        let file_id = destack_source::FileId::from_logical_str(uri.as_ref());
        let file = File::from_text(
            file_id,
            "test.ds".to_string(),
            uri,
            None,
            FileType::Destack,
            "export const value = ;".to_string(),
        );
        files.insert(file_id, Arc::new(file));

        let span = Span::at(file_id, 0, 1);
        let diagnostic = Diagnostic {
            code: "E000".to_string(),
            original_code: None,
            severity: DiagnosticSeverity::Error,
            original_severity: None,
            message: "syntax error".to_string(),
            file_id,
            primary_span: LabeledSpan::new(span, "here"),
            primary_highlight_spans: None,
            secondary_spans: None,
            suggestions: None,
        };
        let diagnostics = DiagnosticCollection::from_diagnostics(vec![diagnostic]);
        let format_options = FormatOptions::default();
        let (output, format_result) = collect_diagnostics_json(
            &|current_file_id| files.get(&current_file_id).cloned(),
            &diagnostics,
            &format_options,
        );

        let report = WatchReport {
            schema: WATCH_REPORT_SCHEMA,
            command: "check".to_string(),
            sequence: 1,
            timestamp_ms: 42,
            event: WatchReportEvent::Compile {
                compile: WatchCompileJson {
                    reason: WatchCompileReason::Update,
                    updated: true,
                    rescan: false,
                    batch_id: Some(7),
                    diagnostics: Some(output),
                    exit_code: format_result.exit_code(),
                },
            },
        };

        let value = serde_json::to_value(report).expect("report should serialize");
        let object = value.as_object().expect("report should be a json object");

        let event = object.get("event").expect("event should exist");
        assert_eq!(event, &Value::String("compile".to_string()));

        let compile = object.get("compile").expect("compile should exist");
        let compile = compile
            .as_object()
            .expect("compile should be a json object");
        let exit_code = compile.get("exit_code").expect("exit_code should exist");
        assert_eq!(exit_code.as_i64(), Some(1));

        let diagnostics = compile
            .get("diagnostics")
            .expect("diagnostics should exist");
        let diagnostics = diagnostics
            .get("diagnostics")
            .expect("diagnostics list should exist");
        assert_eq!(
            diagnostics
                .as_array()
                .expect("diagnostics should be an array")
                .len(),
            1
        );
    }
}
