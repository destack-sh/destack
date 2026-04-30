use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_daemon::{
    Daemon, DaemonMessage, DaemonMessageKind, WatchBatch, WatchCoordinator, WatchPolicy,
};
use destack_session::SessionEventHandler;
use destack_source::{
    DiagnosticCollection, DiagnosticOptions, File, FileId, FileType, FileWatchFilter,
    FileWatchOptions, FileWatchRescanReason, FileWatchStatus, FileWatcher, PhysicalFileWatcher,
};
use destack_workspace::Repository;

use crate::common::format::{
    FormatOptions, FormatResult, LineWriter, format_diagnostics_with_writer,
};
use crate::common::program::ProgramArgs;
use crate::common::{
    ReportArgs, WatchCompileJson, WatchCompileReason, WatchReporter, collect_diagnostics_json,
    report_error,
};
use crate::console;
use crate::error::CliResult;
use crate::pipeline::daemon::ProtocolDaemonClient;

/// Summary of watch updates produced by a daemon.
#[derive(Debug, Clone, Default)]
pub struct WatchBatchSummary {
    /// Whether updates were produced.
    pub updated: bool,
    /// Messages produced by the daemon.
    pub messages: Vec<WatchMessage>,
}

/// Severity tagged message for watch output.
#[derive(Debug, Clone)]
pub struct WatchMessage {
    /// Message severity.
    pub kind: DaemonMessageKind,
    /// Message content.
    pub message: String,
}

/// Daemon interface for watch updates.
pub trait WatchDaemon {
    /// Apply a watch batch and return a summary.
    fn apply_watch_batch(&self, batch: &WatchBatch) -> CliResult<WatchBatchSummary>;
}

impl WatchMessage {
    /// Build a watch message from a daemon message.
    pub fn from_daemon(message: &DaemonMessage) -> Self {
        Self {
            kind: message.kind(),
            message: message.render(),
        }
    }
}

impl WatchDaemon for Daemon {
    fn apply_watch_batch(&self, batch: &WatchBatch) -> CliResult<WatchBatchSummary> {
        let result = Daemon::apply_watch_batch(self, batch);
        Ok(WatchBatchSummary {
            updated: result.updated(),
            messages: result
                .messages
                .iter()
                .map(WatchMessage::from_daemon)
                .collect(),
        })
    }
}

/// Options for the shared watch loop.
#[derive(Clone)]
pub struct WatchLoopOptions {
    /// The watcher implementation to use.
    pub watcher: Arc<dyn FileWatcher>,
    /// Options for the watcher backend.
    pub options: FileWatchOptions,
    /// Policy for batching watch events.
    pub policy: WatchPolicy,
}

impl fmt::Debug for WatchLoopOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WatchLoopOptions")
            .field("watcher", &"<file_watcher>")
            .field("options", &self.options)
            .field("policy", &self.policy)
            .finish()
    }
}

/// Action returned by a watch compile hook.
#[derive(Debug, Default, Clone, Copy)]
pub struct WatchLoopAction {
    /// Optional exit code update.
    pub exit_code: Option<i32>,
    /// Whether the watch loop should stop.
    pub stop: bool,
}

impl WatchLoopAction {
    /// Continue watching while updating the exit code.
    pub fn continue_with(exit_code: Option<i32>) -> Self {
        Self {
            exit_code,
            stop: false,
        }
    }

    /// Stop the loop after applying the exit code.
    pub fn stop_with(exit_code: Option<i32>) -> Self {
        Self {
            exit_code,
            stop: true,
        }
    }
}

/// Determine watch roots for the workspace.
pub fn watch_roots(program: &ProgramArgs, repository: &Repository) -> Vec<PathBuf> {
    // prefer an explicit workspace override
    if let Some(root) = program.workspace.clone() {
        return vec![root];
    }

    // fall back to the repository working directory
    vec![repository.workspace_root().to_path_buf()]
}

/// Watch context shared by CLI watch commands.
#[derive(Debug)]
pub struct WatchContext {
    /// Workspace roots to watch.
    pub roots: Vec<PathBuf>,
    /// Primary root used for daemon commands.
    pub root: PathBuf,
    /// Optional reporter for JSON output.
    pub reporter: Option<WatchReporter>,
}

/// Prepare watch roots and reporter for a CLI command.
pub fn build_watch_context(
    command_name: &str,
    program: &ProgramArgs,
    report: &ReportArgs,
    repository: &Repository,
) -> WatchContext {
    // resolve workspace roots
    let roots = watch_roots(program, repository);

    // select the primary root
    let root = roots
        .first()
        .cloned()
        .expect("watch roots should always include a primary root");

    // initialize the reporter when json output is requested
    let mut reporter = if report.is_json() {
        Some(WatchReporter::new(command_name))
    } else {
        None
    };
    if let Some(reporter) = reporter.as_mut() {
        reporter.emit_start(&roots);
    }

    WatchContext {
        roots,
        root,
        reporter,
    }
}

/// Build watch options for CLI watch mode.
pub fn build_watch_options() -> FileWatchOptions {
    // filter to relevant file types
    let filter: FileWatchFilter = Arc::new(|path: &Path| is_watchable_path(path));

    // build the watcher options
    FileWatchOptions {
        filter: Some(filter),
        ..Default::default()
    }
}

/// Build watch loop options for CLI watch mode.
pub fn build_watch_loop_options() -> WatchLoopOptions {
    // use the physical watcher with default batching policy
    WatchLoopOptions {
        watcher: Arc::new(PhysicalFileWatcher::new()),
        options: build_watch_options(),
        policy: WatchPolicy::default(),
    }
}

/// Check if a path should be handled by watch mode.
pub fn is_watchable_path(path: &Path) -> bool {
    // skip unknown or non file paths
    let Some(file_type) = FileType::from_path(path) else {
        return false;
    };

    // allow code, data, and text files
    file_type.is_code() || file_type.is_data() || file_type.is_text()
}

/// Prefix a message with the watch label.
pub fn watch_error(message: &str) -> String {
    format!("watch: {message}")
}

/// Context for emitting watch compile diagnostics.
pub struct WatchCompileContext<'a> {
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
pub fn emit_watch_compile_report(
    reporter: &mut Option<WatchReporter>,
    context: WatchCompileContext<'_>,
    reason: WatchCompileReason,
    updated: bool,
    rescan: bool,
    batch_id: Option<u64>,
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
            reason,
            updated,
            rescan,
            batch_id,
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

/// Run the shared watch loop and dispatch updates to the provided hooks.
#[allow(clippy::too_many_arguments)]
pub fn run_watch_loop<State, StartFn, RescanFn, CompileFn>(
    daemon: &dyn WatchDaemon,
    roots: Vec<PathBuf>,
    reporter: &mut Option<WatchReporter>,
    loop_options: WatchLoopOptions,
    state: &mut State,
    on_start: StartFn,
    mut on_rescan: RescanFn,
    mut on_compile: CompileFn,
    mut exit_code: i32,
) -> i32
where
    StartFn: FnOnce(&mut State),
    RescanFn: FnMut(&mut State) -> CliResult<()>,
    CompileFn: FnMut(
        &mut State,
        &mut Option<WatchReporter>,
        WatchCompileReason,
        u64,
        bool,
        bool,
    ) -> WatchLoopAction,
{
    // start the file watcher
    let coordinator = WatchCoordinator::new(
        loop_options.watcher,
        roots,
        loop_options.options,
        loop_options.policy,
    );

    // notify the caller after the watcher is ready
    on_start(state);

    // process watch batches until the watcher stops
    let mut batch_id = 0_u64;
    loop {
        let Some(batch) = coordinator.next_batch() else {
            break;
        };

        // skip empty batches
        if batch.is_empty() {
            continue;
        }

        batch_id = batch_id.saturating_add(1);
        if let Some(reporter) = reporter.as_mut() {
            reporter.emit_batch(batch_id, &batch);
        }

        // apply daemon watch updates
        let result = match daemon.apply_watch_batch(&batch) {
            Ok(result) => result,
            Err(error) => {
                emit_watch_warning(reporter, &watch_error(&error.to_string()));
                continue;
            }
        };
        let requires_rescan = batch_requires_rescan(&batch);
        for message in &result.messages {
            emit_watch_message(reporter, message);
        }

        let updated = result.updated;

        // refresh caller state when a rescan is requested
        if requires_rescan && let Err(error) = on_rescan(state) {
            emit_watch_warning(reporter, &error.to_string());
            continue;
        }

        // compile on updates or rescans
        if updated || requires_rescan {
            let reason = match (updated, requires_rescan) {
                (true, true) => WatchCompileReason::UpdateRescan,
                (true, false) => WatchCompileReason::Update,
                (false, true) => WatchCompileReason::Rescan,
                (false, false) => WatchCompileReason::Update,
            };

            let action = on_compile(state, reporter, reason, batch_id, updated, requires_rescan);
            if let Some(next_exit_code) = action.exit_code {
                exit_code = next_exit_code;
            }
            if action.stop {
                break;
            }
        }
    }

    exit_code
}

/// Run a CLI watch command backed by a protocol daemon client.
#[allow(clippy::too_many_arguments)]
pub fn run_daemon_watch_command<State, StartFn, RescanFn, CompileFn, ObserveFn>(
    command_name: &str,
    repository: Arc<Repository>,
    program: &ProgramArgs,
    report: &ReportArgs,
    _diagnostic_options: DiagnosticOptions,
    event_handler: Option<SessionEventHandler>,
    watch_loop_options: WatchLoopOptions,
    state: &mut State,
    on_start: StartFn,
    mut on_rescan: RescanFn,
    mut compile: CompileFn,
    mut on_compile: ObserveFn,
    is_one_shot: bool,
) -> i32
where
    StartFn: FnOnce(&mut State),
    RescanFn: FnMut(&mut State, &Repository) -> CliResult<()>,
    CompileFn: FnMut(
        &ProtocolDaemonClient,
        &Path,
        &mut Option<WatchReporter>,
        &mut State,
        WatchCompileReason,
        Option<u64>,
        bool,
        bool,
    ) -> i32,
    ObserveFn: FnMut(WatchCompileReason, bool, bool),
{
    // prepare watch mode output
    let WatchContext {
        roots,
        root,
        mut reporter,
    } = build_watch_context(command_name, program, report, &repository);

    // configure the daemon client for incremental updates
    let daemon = match ProtocolDaemonClient::new(
        repository.clone(),
        program.workers as usize,
        event_handler,
        roots.clone(),
        program,
    ) {
        Ok(daemon) => daemon,
        Err(error) => {
            let message = watch_error(&error.to_string());
            if let Some(reporter) = reporter.as_mut() {
                reporter.emit_warning(&message);
                reporter.emit_stop();
                return 1;
            }
            return report_error(command_name, report, &message);
        }
    };

    // run the initial compile
    let mut exit_code = compile(
        &daemon,
        &root,
        &mut reporter,
        state,
        WatchCompileReason::Startup,
        None,
        false,
        false,
    );

    // run the shared watch loop
    exit_code = run_watch_loop(
        &daemon,
        roots,
        &mut reporter,
        watch_loop_options,
        state,
        on_start,
        |state| on_rescan(state, &repository),
        |state, reporter, reason, batch_id, updated, requires_rescan| {
            let next_exit_code = compile(
                &daemon,
                &root,
                reporter,
                state,
                reason,
                Some(batch_id),
                updated,
                requires_rescan,
            );

            on_compile(reason, updated, requires_rescan);

            if is_one_shot {
                return WatchLoopAction::stop_with(Some(next_exit_code));
            }

            WatchLoopAction::continue_with(Some(next_exit_code))
        },
        exit_code,
    );

    if let Some(reporter) = reporter.as_mut() {
        reporter.emit_stop();
    }

    daemon.shutdown();

    exit_code
}

/// Print diagnostics for watch mode without consuming program state.
pub fn print_watch_diagnostics(
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
        DaemonMessageKind::Info => console::info(rendered),
        DaemonMessageKind::Warning => console::warn(rendered),
        DaemonMessageKind::Error => console::error(rendered),
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
            FileWatchStatus::RescanRequested {
                reason: FileWatchRescanReason::Overflow
                    | FileWatchRescanReason::Manual
                    | FileWatchRescanReason::Update,
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

    if file_name == "destack.json" || file_name == "jsconfig.json" {
        return true;
    }

    file_name.starts_with("tsconfig") && file_name.ends_with(".json")
}
