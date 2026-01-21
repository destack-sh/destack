use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::{CompilerEventHandler, CompilerOptions, StatsSnapshot};
use destack_daemon::{Daemon, DaemonMessage, DaemonMessageKind, WatchCoordinator, WatchPolicy};
use destack_source::{
    DiagnosticOptions, FileType, FileWatchFilter, FileWatchOptions, FileWatcher,
    PhysicalFileWatcher,
};
use destack_workspace::{Program, Session};

use crate::common::format::{FormatOptions, LineWriter, format_diagnostics_with_writer};
use crate::common::program::ProgramArgs;
use crate::common::{
    CommandStats, WatchCompileJson, WatchCompileReason, WatchReporter, collect_diagnostics_json,
};
use crate::console;

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

/// Build compiler options for daemon updates.
pub fn build_daemon_options(
    program: &ProgramArgs,
    diagnostic_options: DiagnosticOptions,
    event_handler: Option<CompilerEventHandler>,
) -> CompilerOptions {
    // build compiler options aligned with the CLI run
    CompilerOptions {
        diagnostic: diagnostic_options,
        workers: program.workers,
        load_libs: !program.no_libs,
        inject_prelude: !program.no_prelude,
        follow_imports: !program.no_follow_imports,
        event_handler,
        ..Default::default()
    }
}

/// Determine watch roots for the workspace.
pub fn watch_roots(program: &ProgramArgs, session: &Session) -> Vec<PathBuf> {
    // prefer an explicit workspace override
    if let Some(root) = program.workspace.clone() {
        return vec![root];
    }

    // fall back to the session working directory
    vec![session.cwd.clone()]
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
    /// The program used to format diagnostics.
    pub program: &'a Program,
    /// The diagnostic options for formatting.
    pub diagnostic_options: &'a DiagnosticOptions,
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
    stats_snapshot: Option<StatsSnapshot>,
    reason: WatchCompileReason,
    updated: bool,
    rescan: bool,
    batch_id: Option<u64>,
) -> i32 {
    // emit json diagnostics for watch reporters
    if let Some(reporter) = reporter.as_mut() {
        let diagnostics = context
            .program
            .diagnostics
            .collect()
            .map(context.diagnostic_options);
        let (output, format_result) = collect_diagnostics_json(
            &context.program.files,
            &diagnostics,
            context.json_format_options,
        );
        let diagnostics_payload = if context.json_format_options.suppress_diagnostics {
            None
        } else {
            Some(output)
        };
        let stats = stats_snapshot.as_ref().map(CommandStats::from_snapshot);
        reporter.emit_compile(WatchCompileJson {
            reason,
            updated,
            rescan,
            batch_id,
            diagnostics: diagnostics_payload,
            exit_code: format_result.exit_code(),
            stats,
        });
        return format_result.exit_code();
    }

    // emit text diagnostics when json output is not requested
    print_watch_diagnostics(
        context.program,
        context.diagnostic_options,
        context.format_options,
        context.module_count,
        context.line_writer,
    )
    .exit_code()
}

/// Run the shared watch loop and dispatch updates to the provided hooks.
#[allow(clippy::too_many_arguments)]
pub fn run_watch_loop<State, StartFn, RescanFn, CompileFn>(
    daemon: &Daemon,
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
    RescanFn: FnMut(&mut State) -> Result<(), String>,
    CompileFn: FnMut(
        &mut State,
        &mut Option<WatchReporter>,
        WatchCompileReason,
        u64,
        bool,
        bool,
    ) -> WatchLoopAction,
{
    // retain roots for rescans
    let watch_roots = roots.clone();

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
        let result = daemon.apply_watch_batch(&batch);
        let requires_rescan = result.rescan;
        for message in &result.messages {
            emit_watch_message(reporter, message);
        }

        let mut updated = result.updated();

        // refresh sources when a rescan is requested
        if requires_rescan {
            let rescan_result = daemon.rescan_roots(&watch_roots);
            updated = updated || rescan_result.updated();
            for message in &rescan_result.messages {
                emit_watch_message(reporter, message);
            }

            if let Err(message) = on_rescan(state) {
                emit_watch_warning(reporter, &message);
                continue;
            }
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

/// Print diagnostics for watch mode without consuming program state.
pub fn print_watch_diagnostics(
    program: &Program,
    diagnostic_options: &DiagnosticOptions,
    format_options: &FormatOptions,
    module_count: usize,
    line_writer: Option<&LineWriter>,
) -> crate::common::format::FormatResult {
    // collect diagnostics from the program
    let diagnostics = program.diagnostics.collect().map(diagnostic_options);

    // emit diagnostics using the shared formatter
    format_diagnostics_with_writer(
        &program.files,
        &diagnostics,
        format_options,
        module_count,
        line_writer,
    )
}

/// Emit a watch message through the reporter or console.
fn emit_watch_message(reporter: &mut Option<WatchReporter>, message: &DaemonMessage) {
    // report to json watchers when available
    if let Some(reporter) = reporter.as_mut() {
        reporter.emit_warning(&message.render());
        return;
    }

    let rendered = message.render();
    match message.kind() {
        DaemonMessageKind::Info => console::info(&rendered),
        DaemonMessageKind::Warning => console::warn(&rendered),
        DaemonMessageKind::Error => console::error(&rendered),
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
