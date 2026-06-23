use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{DiagnosticCollection, File, FileId, PhysicalFileWatcher};
use destack_workspace::{
    MessageKind, ReloadReason, WatchBatch, WatchPolicy, WatchStatus, Workspace,
};

use crate::common::format::{
    FormatOptions, FormatResult, LineWriter, format_diagnostics_with_writer,
};
use crate::common::program::ProgramArgs;
use crate::common::{
    ReportArgs, WatchCompileJson, WatchCompileReason, WatchReporter, collect_diagnostics_json,
    report_error,
};
use crate::console;
use crate::diagnostic::{ConsoleError, ConsoleResult};

/// Summary of watch updates produced by a workspace.
#[derive(Debug, Clone)]
pub(crate) struct WatchBatchSummary {
    /// Watch batch received from the workspace host.
    pub batch: WatchBatch,
    /// Whether updates were produced.
    pub updated: bool,
    /// Messages produced by the workspace host.
    pub messages: Vec<WatchMessage>,
}

/// Severity tagged message for watch output.
#[derive(Debug, Clone)]
pub(crate) struct WatchMessage {
    /// Message severity.
    pub kind: MessageKind,
    /// Message content.
    pub message: String,
}

/// One watch-triggered command cycle.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WatchCycle {
    /// Reason for the command run.
    pub(crate) reason: WatchCompileReason,
    /// Batch identifier for update-triggered runs.
    pub(crate) batch_id: Option<u64>,
    /// Whether source updates were applied.
    pub(crate) updated: bool,
    /// Whether sources should be rediscovered before running.
    pub(crate) requires_rescan: bool,
}

impl WatchCycle {
    /// Build the startup cycle.
    pub(crate) fn startup() -> Self {
        Self {
            reason: WatchCompileReason::Startup,
            batch_id: None,
            updated: false,
            requires_rescan: false,
        }
    }

    /// Build an update cycle.
    fn update(batch_id: u64, updated: bool, requires_rescan: bool) -> Self {
        let reason = match (updated, requires_rescan) {
            (true, true) => WatchCompileReason::UpdateRescan,
            (true, false) => WatchCompileReason::Update,
            (false, true) => WatchCompileReason::Rescan,
            (false, false) => WatchCompileReason::Update,
        };

        Self {
            reason,
            batch_id: Some(batch_id),
            updated,
            requires_rescan,
        }
    }
}

/// Workspace watch session used by CLI commands.
pub(crate) struct WorkspaceWatch {
    /// Active workspace.
    workspace: Arc<dyn Workspace>,
    /// Primary root used for workspace commands.
    root: PathBuf,
    /// Optional JSON reporter.
    reporter: Option<WatchReporter>,
    /// Latest emitted batch identifier.
    batch_id: u64,
}

impl WorkspaceWatch {
    /// Start workspace watching for a command.
    pub(crate) fn start(
        command_name: &str,
        program: &ProgramArgs,
        report: &ReportArgs,
        watch_policy: WatchPolicy,
    ) -> Result<Self, i32> {
        // open a local workspace with file watching enabled
        let file_watcher = Arc::new(PhysicalFileWatcher::new());
        let (workspace, roots) = match program.workspace(Some(file_watcher)) {
            Ok(workspace) => workspace,
            Err(error) => {
                let mut reporter = None;
                return Err(report_watch_start_error(
                    command_name,
                    report,
                    &mut reporter,
                    &error.to_string(),
                ));
            }
        };
        let Some(root) = roots.first().cloned() else {
            let mut reporter = None;
            return Err(report_watch_start_error(
                command_name,
                report,
                &mut reporter,
                "roots are empty",
            ));
        };

        // prepare reporter after roots are known
        let mut reporter = if report.is_json() {
            Some(WatchReporter::new(command_name))
        } else {
            None
        };
        if let Some(reporter) = reporter.as_mut() {
            reporter.emit_start(&roots);
        }

        // start workspace-owned watching
        if let Err(error) = workspace.watch(roots, watch_policy.clone()) {
            return Err(report_watch_start_error(
                command_name,
                report,
                &mut reporter,
                &error.to_string(),
            ));
        }

        Ok(Self {
            workspace,
            root,
            reporter,
            batch_id: 0,
        })
    }

    /// Run one workspace command with the active watch context.
    pub(crate) fn run_command<RunFn>(&mut self, run: RunFn) -> i32
    where
        RunFn: FnOnce(&dyn Workspace, &Path, &mut Option<WatchReporter>) -> i32,
    {
        run(self.workspace.as_ref(), &self.root, &mut self.reporter)
    }

    /// Receive the next watch cycle that should rerun the command.
    pub(crate) fn next_cycle(&mut self) -> Option<WatchCycle> {
        loop {
            // receive the next non-empty batch
            let result = match self.next_watch_batch() {
                Ok(Some(result)) => result,
                Ok(None) => return None,
                Err(error) => {
                    self.emit_warning(&watch_error(&error.to_string()));
                    continue;
                }
            };
            if result.batch.is_empty() {
                continue;
            }

            // emit batch and workspace messages
            self.batch_id = self.batch_id.saturating_add(1);
            if let Some(reporter) = self.reporter.as_mut() {
                reporter.emit_batch(self.batch_id, &result.batch);
            }
            for message in &result.messages {
                self.emit_message(message);
            }

            // return only cycles that require command work
            let rescan = batch_requires_rescan(&result.batch);
            if result.updated || rescan {
                return Some(WatchCycle::update(self.batch_id, result.updated, rescan));
            }
        }
    }

    /// Emit one watch warning.
    pub(crate) fn emit_warning(&mut self, message: &str) {
        emit_watch_warning(&mut self.reporter, message);
    }

    /// Stop watching and emit the final event.
    pub(crate) fn stop(mut self) {
        let _ = self.workspace.unwatch(&self.root);
        if let Some(reporter) = self.reporter.as_mut() {
            reporter.emit_stop();
        }
    }

    /// Receive and apply the next watch batch.
    fn next_watch_batch(&self) -> ConsoleResult<Option<WatchBatchSummary>> {
        let response = self
            .workspace
            .next_watch(&self.root)
            .map_err(|error| ConsoleError::message(format!("next watch batch failed: {error}")))?;
        let Some(response) = response else {
            return Ok(None);
        };

        Ok(Some(WatchBatchSummary {
            batch: response.batch,
            updated: !response.updates.updates.is_empty(),
            messages: response
                .updates
                .messages
                .into_iter()
                .map(|message| WatchMessage {
                    kind: message.kind,
                    message: message.message,
                })
                .collect(),
        }))
    }

    /// Emit one watch message.
    fn emit_message(&mut self, message: &WatchMessage) {
        emit_watch_message(&mut self.reporter, message);
    }
}

/// Prefix a message with the watch label.
pub(crate) fn watch_error(message: &str) -> String {
    format!("watch: {message}")
}

/// Context for emitting watch compile diagnostics.
pub(crate) struct WatchCompileContext<'a> {
    /// Files associated with the diagnostics.
    pub files: &'a BTreeMap<FileId, Arc<File>>,
    /// Diagnostics to render.
    pub diagnostics: &'a DiagnosticCollection,
    /// The human readable formatting options.
    pub format_options: &'a FormatOptions,
    /// The json formatting options.
    pub json_format_options: &'a FormatOptions,
    /// The active module count.
    pub module_count: usize,
    /// Optional line writer for progress output.
    pub line_writer: Option<&'a LineWriter>,
}

impl fmt::Debug for WatchCompileContext<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WatchCompileContext")
            .field("module_count", &self.module_count)
            .field("line_writer", &self.line_writer.is_some())
            .finish()
    }
}

/// Emit watch diagnostics and return the exit code.
pub(crate) fn emit_watch_compile_report(
    reporter: &mut Option<WatchReporter>,
    context: WatchCompileContext<'_>,
    cycle: WatchCycle,
) -> i32 {
    // emit json diagnostics for watch reporters
    if let Some(reporter) = reporter.as_mut() {
        let file_for_id = |file_id| context.files.get(&file_id).cloned();
        let (output, format_result) = collect_diagnostics_json(
            &file_for_id,
            context.diagnostics,
            context.json_format_options,
        );
        let diagnostics_payload = if context.json_format_options.suppress_diagnostics {
            None
        } else {
            Some(output)
        };
        reporter.emit_compile(WatchCompileJson {
            reason: cycle.reason,
            updated: cycle.updated,
            rescan: cycle.requires_rescan,
            batch_id: cycle.batch_id,
            diagnostics: diagnostics_payload,
            exit_code: format_result.exit_code(),
        });
        return format_result.exit_code();
    }

    // emit text diagnostics when json output is not requested
    print_watch_diagnostics(
        context.files,
        context.diagnostics,
        context.format_options,
        context.module_count,
        context.line_writer,
    )
    .exit_code()
}

/// Print diagnostics for watch mode without consuming program state.
pub(crate) fn print_watch_diagnostics(
    files: &BTreeMap<FileId, Arc<File>>,
    diagnostics: &DiagnosticCollection,
    format_options: &FormatOptions,
    module_count: usize,
    line_writer: Option<&LineWriter>,
) -> FormatResult {
    // emit diagnostics using the shared formatter
    let file_for_id = |file_id| files.get(&file_id).cloned();
    format_diagnostics_with_writer(
        &file_for_id,
        diagnostics,
        format_options,
        module_count,
        line_writer,
    )
}

/// Emit a watch message through the reporter or console.
fn emit_watch_message(reporter: &mut Option<WatchReporter>, message: &WatchMessage) {
    // report to json watchers when available
    if let Some(reporter) = reporter.as_mut() {
        reporter.emit_warning(&message.message);
        return;
    }

    let rendered = message.message.as_str();
    match message.kind {
        MessageKind::Info => console::info(rendered),
        MessageKind::Warning => console::warn(rendered),
        MessageKind::Error => console::error(rendered),
    }
}

/// Emit a watch warning through the reporter or console.
fn emit_watch_warning(reporter: &mut Option<WatchReporter>, message: &str) {
    // report to json watchers when available
    if let Some(reporter) = reporter.as_mut() {
        reporter.emit_warning(message);
        return;
    }

    console::warn(message);
}

/// Report a watch start error.
fn report_watch_start_error(
    command_name: &str,
    report: &ReportArgs,
    reporter: &mut Option<WatchReporter>,
    error: &str,
) -> i32 {
    let message = watch_error(error);
    if let Some(reporter) = reporter.as_mut() {
        reporter.emit_warning(&message);
        reporter.emit_stop();
        1
    } else {
        report_error(command_name, report, &message)
    }
}

/// Return true when a watch batch should refresh caller source inputs.
fn batch_requires_rescan(batch: &WatchBatch) -> bool {
    // treat overflow batches as requiring a refresh
    if batch.overflowed {
        return true;
    }

    // treat explicit watcher rescan requests as requiring a refresh
    if batch.status.iter().any(|status| {
        matches!(
            status,
            WatchStatus::ReloadRequested {
                reason: ReloadReason::Overflow | ReloadReason::Manual | ReloadReason::Watch,
                ..
            }
        )
    }) {
        return true;
    }

    // treat config path updates as requiring a refresh
    batch.events.iter().any(|event| {
        is_config_path(&event.path)
            || event
                .previous_path
                .as_ref()
                .is_some_and(|path| is_config_path(path))
    })
}

/// Return true when a path is a workspace config file.
fn is_config_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    file_name == "destack.json"
}
