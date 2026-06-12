use std::path::Path;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use destack_compiler::Compiler;
use destack_repository::Revision;
use destack_session::{SessionEvent, SessionEventHandler};
use destack_source::{Diagnostic, DiagnosticCollection};
use parking_lot::Mutex;

use crate::protocol::{CommandOutputChunk, OutputStream, ProgressEvent};
use crate::{Daemon, Workspace};

use super::context::CommandContext;
use super::{
    CommandMessagePayload, CommandPayload, CommandResult, CommandRevision, CommonCommandOptions,
    DaemonCommandError,
};

/// Result of executing a daemon command.
#[derive(Debug, Clone)]
pub struct DaemonCommandResult {
    /// The revision used for this command result.
    pub revision: Revision,
    /// Whether the command succeeded.
    pub success: bool,
    /// Exit code for the command.
    pub exit_code: i32,
    /// Diagnostics produced by the command.
    pub diagnostics: Vec<Diagnostic>,
    /// Output collected during execution.
    pub output: Vec<CommandOutputChunk>,
    /// Command-specific payload.
    pub data: Option<serde_json::Value>,
    /// Count of modules involved.
    pub module_count: usize,
    /// Count of profiles involved.
    pub profile_count: usize,
    /// Count of targets involved.
    pub target_count: usize,
}

/// Buffered output for command execution.
#[derive(Debug, Default)]
pub(super) struct CommandOutputBuffer {
    /// Output chunks emitted by the command.
    pub(super) chunks: Vec<CommandOutputChunk>,
}

impl CommandOutputBuffer {
    /// Push stdout bytes into the buffer.
    pub(super) fn push_stdout(&mut self, bytes: Vec<u8>) {
        self.push(OutputStream::Stdout, bytes);
    }

    /// Push stderr bytes into the buffer.
    pub(super) fn push_stderr(&mut self, bytes: Vec<u8>) {
        self.push(OutputStream::Stderr, bytes);
    }

    /// Push bytes into the buffer for the provided stream.
    pub(super) fn push(&mut self, stream: OutputStream, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }

        self.chunks.push(CommandOutputChunk { stream, bytes });
    }
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
}

/// Progress sink for one command execution.
pub type CommandProgress<'a> = Option<&'a (dyn Fn(ProgressEvent) + Sync)>;

/// Build one session event handler forwarding throttled progress.
/// Counting stays exact; emission throttles to one event per interval
/// so slow transports never stall the workers.
fn session_progress_handler(sender: mpsc::Sender<ProgressEvent>) -> SessionEventHandler {
    let throttle = Mutex::new((None::<Instant>, 0usize));

    Arc::new(move |event| match event {
        SessionEvent::TaskFinished { artifact_key, .. }
        | SessionEvent::TaskFailed { artifact_key, .. } => {
            let mut throttle = throttle.lock();
            throttle.1 += 1;
            let due = throttle
                .0
                .is_none_or(|last| last.elapsed() >= Duration::from_millis(80));
            if due {
                throttle.0 = Some(Instant::now());
                let _ = sender.send(ProgressEvent {
                    task: artifact_key.stage().name().to_string(),
                    message: Some(format!("{} artifacts", throttle.1)),
                    percent: None,
                    done: false,
                });
            }
        }
        SessionEvent::RunFinished { .. } => {
            let _ = sender.send(ProgressEvent {
                task: String::new(),
                message: None,
                percent: None,
                done: true,
            });
        }
        _ => {}
    })
}

impl CommandOutcome {
    /// Build a command outcome payload.
    pub(super) fn new(
        diagnostics: DiagnosticCollection,
        exit_code: i32,
        module_count: usize,
        profile_count: usize,
        target_count: usize,
    ) -> Self {
        Self {
            diagnostics,
            exit_code,
            data: None,
            module_count,
            profile_count,
            target_count,
        }
    }

    /// Attach a command-specific payload.
    pub(super) fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Build a standard unimplemented command outcome.
    pub(super) fn unimplemented(message: &str) -> CommandResult<Self> {
        let payload = CommandMessagePayload {
            message: message.to_string(),
            implemented: false,
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| DaemonCommandError::payload(format!("invalid payload: {error}")))?;

        Ok(Self::new(DiagnosticCollection::default(), 1, 0, 0, 0).with_data(data))
    }
}

impl Daemon {
    /// Execute a command request for the given root.
    pub fn run_root_command(
        &self,
        workspace: &Workspace,
        root: &Path,
        common: &CommonCommandOptions,
        payload: &CommandPayload,
        revision: CommandRevision,
        progress: CommandProgress<'_>,
    ) -> CommandResult<DaemonCommandResult> {
        std::thread::scope(|scope| {
            // forward throttled session progress to the connection
            let event_handler = progress.map(|notify| {
                let (sender, receiver) = mpsc::channel::<ProgressEvent>();
                scope.spawn(move || {
                    while let Ok(event) = receiver.recv() {
                        notify(event);
                    }
                });

                session_progress_handler(sender)
            });

            self.run_root_command_inner(workspace, root, common, payload, revision, event_handler)
        })
    }

    /// Execute a command request against one forked command session.
    fn run_root_command_inner(
        &self,
        workspace: &Workspace,
        root: &Path,
        common: &CommonCommandOptions,
        payload: &CommandPayload,
        revision: CommandRevision,
        event_handler: Option<SessionEventHandler>,
    ) -> CommandResult<DaemonCommandResult> {
        let repository = Arc::clone(&workspace.repository);
        let compiler = Arc::new(Compiler::new(Arc::clone(&repository)));

        // gather shared context
        let mut output = CommandOutputBuffer::default();
        let mut context = CommandContext::new(
            self,
            root.to_path_buf(),
            repository,
            compiler,
            common,
            revision,
            &mut output,
            event_handler,
        )?;

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
            CommandPayload::Inspect(options) => context.run_inspect_command(options)?,
            CommandPayload::Manifest(options) => context.run_manifest_command(options)?,
            CommandPayload::Targets(options) => context.run_targets_command(options)?,
            CommandPayload::Cache(options) => context.run_cache_command(options)?,
            CommandPayload::Settings(options) => context.run_settings_command(options)?,
            CommandPayload::Doctor(options) => context.run_doctor_command(options)?,
            CommandPayload::Task(options) => context.run_task_command(options)?,
            CommandPayload::Clean(options) => context.run_clean_command(root, options)?,
        };

        // finalize command output
        let CommandOutcome {
            diagnostics,
            exit_code,
            data,
            module_count,
            profile_count,
            target_count,
        } = result;
        let success = exit_code == 0;
        let revision = context.revision()?;

        Ok(DaemonCommandResult {
            revision,
            success,
            exit_code,
            diagnostics: diagnostics.iter().cloned().collect(),
            output: output.chunks,
            data,
            module_count,
            profile_count,
            target_count,
        })
    }
}
