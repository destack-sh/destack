use std::path::Path;
use std::time::{Duration, Instant};

use destack_compiler::StatsSnapshot;
use destack_source::{Diagnostic, DiagnosticCollection};

use crate::Daemon;
use crate::command::context::CommandContext;
use crate::command::{CommandPayload, CommonCommandOptions, DaemonCommandError};

/// Result of executing a daemon command.
#[derive(Debug, Clone)]
pub struct DaemonCommandResult {
    /// Whether the command succeeded.
    pub success: bool,
    /// Exit code for the command.
    pub exit_code: i32,
    /// Diagnostics produced by the command.
    pub diagnostics: Vec<Diagnostic>,
    /// Output collected during execution.
    pub output: Vec<DaemonCommandOutputChunk>,
    /// Command-specific payload.
    pub data: Option<serde_json::Value>,
    /// Count of modules involved.
    pub module_count: usize,
    /// Count of profiles involved.
    pub profile_count: usize,
    /// Count of targets involved.
    pub target_count: usize,
    /// Optional stats payload.
    pub stats: Option<DaemonCommandStats>,
}

/// Buffered output for command execution.
#[derive(Debug, Default)]
pub(super) struct CommandOutputBuffer {
    /// Output chunks emitted by the command.
    pub(super) chunks: Vec<DaemonCommandOutputChunk>,
}

impl CommandOutputBuffer {
    /// Push stdout bytes into the buffer.
    pub(super) fn push_stdout(&mut self, bytes: Vec<u8>) {
        self.push(DaemonOutputStream::Stdout, bytes);
    }

    /// Push stderr bytes into the buffer.
    pub(super) fn push_stderr(&mut self, bytes: Vec<u8>) {
        self.push(DaemonOutputStream::Stderr, bytes);
    }

    /// Push bytes into the buffer for the provided stream.
    pub(super) fn push(&mut self, stream: DaemonOutputStream, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }

        self.chunks.push(DaemonCommandOutputChunk { stream, bytes });
    }
}

/// Command output chunk from daemon command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonCommandOutputChunk {
    /// Output stream kind.
    pub stream: DaemonOutputStream,
    /// Output bytes.
    pub bytes: Vec<u8>,
}

/// Command output stream kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonOutputStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// Command cache statistics payload.
#[derive(Debug, Clone, PartialEq)]
pub struct DaemonCommandCacheStats {
    /// Cache hits from memory.
    pub hits_memory: u64,
    /// Cache hits from disk.
    pub hits_disk: u64,
    /// Cache misses.
    pub misses: u64,
    /// Cache writes to memory.
    pub writes_memory: u64,
    /// Cache writes to disk.
    pub writes_disk: u64,
    /// Cache errors.
    pub errors: u64,
    /// Cache hit rate across all cache kinds.
    pub hit_rate: f32,
}

/// Timing tag statistics for command payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonCommandTimingTagStats {
    /// Timing tag name.
    pub name: String,
    /// Total time spent in this tag (milliseconds).
    pub duration_ms: u64,
    /// Number of samples recorded.
    pub sample_count: u64,
}

/// Command statistics payload.
#[derive(Debug, Clone, PartialEq)]
pub struct DaemonCommandStats {
    /// Elapsed time in milliseconds.
    pub elapsed_ms: u64,
    /// Number of tasks completed.
    pub tasks_completed: u64,
    /// Number of tasks failed.
    pub tasks_failed: u64,
    /// Number of tasks skipped.
    pub tasks_skipped: u64,
    /// Number of modules processed.
    pub modules_processed: u64,
    /// Number of lines processed.
    pub lines_processed: u64,
    /// Number of slow tasks detected.
    pub slow_tasks: u64,
    /// Cache statistics when available.
    pub cache: Option<DaemonCommandCacheStats>,
    /// Timing tag statistics when available.
    pub timings: Option<Vec<DaemonCommandTimingTagStats>>,
}

/// Command outcome used for response assembly.
#[derive(Debug)]
pub(super) struct CommandOutcome {
    /// Diagnostics captured during execution.
    pub(super) diagnostics: DiagnosticCollection,
    /// Exit code for the command.
    pub(super) exit_code: i32,
    /// Optional command payload data.
    pub(super) data: Option<serde_json::Value>,
    /// Number of modules included in the command.
    pub(super) module_count: usize,
    /// Number of profiles included in the command.
    pub(super) profile_count: usize,
    /// Number of targets included in the command.
    pub(super) target_count: usize,
    /// Stats snapshot from compiler execution.
    pub(super) stats: Option<StatsSnapshot>,
}

impl CommandOutcome {
    /// Build a command outcome payload.
    pub(super) fn new(
        diagnostics: DiagnosticCollection,
        exit_code: i32,
        module_count: usize,
        profile_count: usize,
        target_count: usize,
        stats: Option<StatsSnapshot>,
    ) -> Self {
        Self {
            diagnostics,
            exit_code,
            data: None,
            module_count,
            profile_count,
            target_count,
            stats,
        }
    }

    /// Attach a command-specific payload.
    pub(super) fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

impl Daemon {
    /// Execute a command request for the given workspace root.
    pub fn run_workspace_command(
        &self,
        root: &Path,
        common: &CommonCommandOptions,
        payload: &CommandPayload,
    ) -> super::CommandResult<DaemonCommandResult> {
        // resolve workspace program and compiler handles before command execution
        let outcome = self
            .workspace_service
            .with_workspace_handles_for_path(root, |program, compiler| {
                // gather shared context
                let start_time = Instant::now();
                let mut output = CommandOutputBuffer::default();
                let mut context = CommandContext::new(
                    self,
                    program.clone(),
                    compiler.clone(),
                    common,
                    &mut output,
                );

                // execute the requested command
                let result = match payload {
                    CommandPayload::Check(options) => context.run_check_command(options)?,
                    CommandPayload::Lint(options) => context.run_lint_command(options)?,
                    CommandPayload::Build(options) => context.run_build_command(options)?,
                    CommandPayload::Run(options) => context.execute_run_command(options)?,
                    CommandPayload::Test(options) => context.run_test_command(options)?,
                    CommandPayload::Format(options) => context.run_format_command(root, options)?,
                    CommandPayload::Doc(options) => context.run_doc_command(options)?,
                    CommandPayload::Bench(options) => context.run_bench_command(options)?,
                    CommandPayload::Info(options) => context.run_info_command(options)?,
                    CommandPayload::Config(options) => context.run_config_command(options)?,
                    CommandPayload::Targets(options) => context.run_targets_command(options)?,
                    CommandPayload::Cache(options) => context.run_cache_command(options)?,
                    CommandPayload::Doctor(options) => context.run_doctor_command(options)?,
                    CommandPayload::Task(options) => context.run_task_command(options)?,
                    CommandPayload::Repl(options) => context.run_repl_command(options)?,
                    CommandPayload::Clean(options) => context.run_clean_command(root, options)?,
                };

                // finalize stats and output
                let stats_payload = result
                    .stats
                    .as_ref()
                    .map(|stats| command_stats_from_snapshot(stats, start_time.elapsed()));
                let data = result.data.clone();
                let exit_code = result.exit_code;
                let success = exit_code == 0;

                Ok(DaemonCommandResult {
                    success,
                    exit_code,
                    diagnostics: result.diagnostics.iter(),
                    output: output.chunks,
                    data,
                    module_count: result.module_count,
                    profile_count: result.profile_count,
                    target_count: result.target_count,
                    stats: stats_payload,
                })
            })
            .map_err(|error| {
                DaemonCommandError::internal(format!("workspace program routing failed: {error}"))
            })?;

        outcome
    }
}

/// Map stats snapshots into daemon command payloads.
fn command_stats_from_snapshot(snapshot: &StatsSnapshot, elapsed: Duration) -> DaemonCommandStats {
    // collect cache totals
    let elapsed_ms = elapsed.as_millis() as u64;
    let cache_totals = snapshot.cache_totals();
    let cache = DaemonCommandCacheStats {
        hits_memory: cache_totals.hits_memory as u64,
        hits_disk: cache_totals.hits_disk as u64,
        misses: cache_totals.misses as u64,
        writes_memory: cache_totals.writes_memory as u64,
        writes_disk: cache_totals.writes_disk as u64,
        errors: cache_totals.errors as u64,
        hit_rate: snapshot.cache_hit_rate(),
    };
    let timings = if snapshot.timings.is_empty() {
        None
    } else {
        Some(
            snapshot
                .timings
                .iter()
                .map(|entry| DaemonCommandTimingTagStats {
                    name: entry.name.clone(),
                    duration_ms: entry.duration.as_millis() as u64,
                    sample_count: entry.sample_count as u64,
                })
                .collect(),
        )
    };

    // build stats payload
    DaemonCommandStats {
        elapsed_ms,
        tasks_completed: snapshot.tasks.completed as u64,
        tasks_failed: snapshot.tasks.failed as u64,
        tasks_skipped: snapshot.tasks.skipped as u64,
        modules_processed: snapshot.modules_processed() as u64,
        lines_processed: snapshot.modules.lines_processed as u64,
        slow_tasks: snapshot.slow_tasks as u64,
        cache: Some(cache),
        timings,
    }
}
