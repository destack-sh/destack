use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use destack_compiler::{CompilerEventHandler, CompilerOptions};
use destack_daemon::protocol::{
    CommandEnvVar, CommandInput, CommandMessagePayload, CommandOutputChunk, CommandPayload,
    CommandRequest, CommandResponse, CommandStats, CommandTargetOverrides, CommonCommandOptions,
    ConfigOverride, DaemonMessageKind as ProtocolMessageKind, DaemonMessageRecord, DaemonQuery,
    DaemonQueryResponse, DaemonRequest, DaemonResponse, DiagnosticBatch, FileSnapshot,
    OpenWorkspaceRequest, OutputStream, ProtocolClient, WatchBatch as ProtocolWatchBatch,
    WatchBatchRequest, WatchEvent, WatchStatus, WorkspaceHandleId, WorkspaceOpenOptions,
};
use destack_daemon::{
    DaemonConnectOptions, DaemonConnection, DaemonInstance, DaemonLaunchConfig,
    connect_in_process_daemon, connect_ipc_daemon,
};
use destack_service::query::{QueryRequestEnvelope, QueryResponseEnvelope};
use destack_source::{
    DiagnosticCollection, DiagnosticOptions, File, FileRegistry, FileType, FileWatchStatus,
};
use destack_workspace::{DsConfigRuntimeOptionsJson, OptimizeLevel, Session};

use crate::common::report::CommandCacheStats;
use crate::common::{
    CommandStats as CliCommandStats, CommandTimingTagStats, InputSource, ProgramArgs, ReportArgs,
    TargetArgs, parse_command_payload, print_report, report_from_message_payload,
};
use crate::console;
use crate::error::{CliError, CliResult};
use crate::pipeline::watch::{
    WatchBatchSummary, WatchDaemon, WatchMessage, build_daemon_options, watch_roots,
};

/// Protocol backed daemon client for CLI flows.
#[derive(Debug)]
pub struct ProtocolDaemonClient {
    /// Protocol client for daemon requests.
    client: Arc<ProtocolClient>,
    /// Workspace handles keyed by root path.
    handles: Vec<WorkspaceHandle>,
    /// Connection state for the daemon.
    connection: Option<DaemonConnection>,
}

/// Workspace handle metadata for CLI usage.
#[derive(Debug, Clone)]
struct WorkspaceHandle {
    /// Workspace root for the handle.
    root: PathBuf,
    /// Daemon handle identifier.
    handle: WorkspaceHandleId,
}

/// Settings used to build a daemon launch config.
#[derive(Debug, Clone)]
pub(crate) struct DaemonLaunchContext {
    /// Current working directory for the daemon.
    cwd: PathBuf,
    /// Cache directory override.
    cache_dir: Option<PathBuf>,
    /// Config path override.
    config_path: Option<PathBuf>,
}

impl DaemonLaunchContext {
    /// Build a launch context from program arguments.
    pub(crate) fn from_program(program: &ProgramArgs) -> Self {
        // resolve the cwd for the daemon
        let cwd = program
            .cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // resolve cache dir relative to the cwd
        let cache_dir = program.cache_dir.as_ref().map(|cache_dir| {
            if cache_dir.is_absolute() {
                cache_dir.clone()
            } else {
                cwd.join(cache_dir)
            }
        });

        Self {
            cwd,
            cache_dir,
            config_path: program.config.clone(),
        }
    }

    /// Build a launch config for a daemon instance.
    pub(crate) fn build_launch_config(&self, instance: &DaemonInstance) -> DaemonLaunchConfig {
        // build the base launch config from the instance
        let mut launch = DaemonLaunchConfig::for_instance(instance);

        // apply overrides from program settings
        launch.cache_dir = self.cache_dir.clone();
        launch.config_path = self.config_path.clone();
        launch.cwd = Some(self.cwd.clone());

        launch
    }
}

/// Connector for daemon clients.
#[derive(Debug)]
struct DaemonConnector {
    /// Session for in process daemons.
    session: Arc<Session>,
    /// Connect options for the daemon.
    options: DaemonConnectOptions,
    /// Instance metadata for ipc daemons.
    instance: DaemonInstance,
    /// Launch configuration for ipc daemons.
    launch: DaemonLaunchConfig,
    /// Whether to prefer in process daemons.
    allow_in_process: bool,
}

impl DaemonConnector {
    /// Create a connector from program settings.
    fn new(
        session: Arc<Session>,
        compiler_options: CompilerOptions,
        program: &ProgramArgs,
    ) -> Self {
        // build connect options
        let options = DaemonConnectOptions {
            compiler: compiler_options,
            ..DaemonConnectOptions::default()
        };

        // resolve daemon instance metadata
        let instance = DaemonInstance::from_session(&session);
        let launch_context = DaemonLaunchContext::from_program(program);
        let launch = launch_context.build_launch_config(&instance);

        // decide whether to use in process connections
        let allow_in_process = program.fs_override.is_some();

        Self {
            session,
            options,
            instance,
            launch,
            allow_in_process,
        }
    }

    /// Connect to the daemon with the configured mode.
    fn connect(&self) -> CliResult<DaemonConnection> {
        // prefer in process connections when configured
        if self.allow_in_process {
            return self.connect_in_process();
        }

        // fall back to ipc connections
        self.connect_ipc()
    }

    /// Connect to an in process daemon.
    fn connect_in_process(&self) -> CliResult<DaemonConnection> {
        // connect via loopback transport
        connect_in_process_daemon(self.session.clone(), self.options.clone())
            .map_err(|error| CliError::message(format!("daemon connect failed: {error}")))
    }

    /// Connect to an ipc daemon, spawning when needed.
    fn connect_ipc(&self) -> CliResult<DaemonConnection> {
        // connect via ipc, spawning when needed
        connect_ipc_daemon(
            &self.instance,
            self.options.clone(),
            Some(self.launch.clone()),
        )
        .map_err(|error| CliError::message(format!("daemon connect failed: {error}")))
    }
}

/// Result of a daemon command execution.
#[derive(Debug)]
pub struct DaemonCommandResult {
    /// Raw protocol response.
    pub response: CommandResponse,
    /// Flattened diagnostics from the response.
    pub diagnostics: DiagnosticCollection,
    /// File registry reconstructed from snapshots.
    pub files: FileRegistry,
}

/// Builder for common daemon command options.
#[derive(Debug, Clone)]
pub struct CommandOptionsBuilder {
    options: CommonCommandOptions,
}

impl CommandOptionsBuilder {
    /// Create a builder seeded with program defaults.
    pub fn new(program: &ProgramArgs, diagnostic: Option<DiagnosticOptions>) -> Self {
        // build defaults from program settings
        let options = CommonCommandOptions {
            inputs: Vec::new(),
            allow_dsconfig_fallback: false,
            cache_dir: program.cache_dir.clone(),
            config_path: program.config.clone(),
            target: None,
            target_overrides: None,
            runtime_overrides: None,
            profile: None,
            diagnostic,
            env: Vec::new(),
            overrides: Vec::new(),
            watch: false,
            dry_run: false,
        };

        Self { options }
    }

    /// Set input sources for the command.
    pub fn inputs(mut self, inputs: Vec<CommandInput>) -> Self {
        self.options.inputs = inputs;
        self
    }

    /// Allow fallback to dsconfig discovery.
    pub fn allow_dsconfig_fallback(mut self, allow: bool) -> Self {
        self.options.allow_dsconfig_fallback = allow;
        self
    }

    /// Set the target name override.
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.options.target = Some(target.into());
        self
    }

    /// Set the target overrides.
    pub fn target_overrides(mut self, overrides: Option<CommandTargetOverrides>) -> Self {
        self.options.target_overrides = overrides;
        self
    }

    /// Set runtime overrides.
    pub fn runtime_overrides(mut self, overrides: Option<DsConfigRuntimeOptionsJson>) -> Self {
        self.options.runtime_overrides = overrides;
        self
    }

    /// Set the profile name override.
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        self.options.profile = Some(profile.into());
        self
    }

    /// Set environment overrides.
    pub fn env(mut self, env: Vec<CommandEnvVar>) -> Self {
        self.options.env = env;
        self
    }

    /// Set config overrides.
    pub fn overrides(mut self, overrides: Vec<ConfigOverride>) -> Self {
        self.options.overrides = overrides;
        self
    }

    /// Enable dry-run mode.
    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.options.dry_run = dry_run;
        self
    }

    /// Build the common command options.
    pub fn build(self) -> CommonCommandOptions {
        self.options
    }
}

impl ProtocolDaemonClient {
    /// Create a protocol daemon client for the provided roots.
    pub fn new(
        session: Arc<Session>,
        compiler_options: CompilerOptions,
        roots: Vec<PathBuf>,
        program: &ProgramArgs,
    ) -> CliResult<Self> {
        // connect to the daemon
        let connector = DaemonConnector::new(session.clone(), compiler_options, program);
        let connection = connector.connect()?;
        let client = connection.client.clone();

        // open each workspace root
        let mut handles = Vec::new();
        for root in roots {
            let request = OpenWorkspaceRequest {
                root: root.clone(),
                options: WorkspaceOpenOptions::default(),
            };
            let response = match client.send_request(DaemonRequest::OpenWorkspace(request)) {
                Ok(response) => response,
                Err(error) => {
                    return Err(CliError::message(format!("open workspace failed: {error}")));
                }
            };
            let handle = match response {
                DaemonResponse::WorkspaceOpened(response) => response.handle,
                DaemonResponse::Error(error) => {
                    return Err(CliError::message(format!("open workspace failed: {error}")));
                }
                other => {
                    return Err(CliError::message(format!("unexpected response: {other:?}")));
                }
            };
            handles.push(WorkspaceHandle { root, handle });
        }

        Ok(Self {
            client,
            handles,
            connection: Some(connection),
        })
    }

    /// Run a command for a workspace root.
    pub fn run_command(
        &self,
        root: &Path,
        common: CommonCommandOptions,
        payload: CommandPayload,
    ) -> CliResult<DaemonCommandResult> {
        // resolve the workspace handle
        let handle = self.handle_for_root(root).ok_or_else(|| {
            CliError::message(format!("workspace root not opened: {}", root.display()))
        })?;

        // send the request to the daemon
        let request = CommandRequest {
            handle: handle.handle,
            common,
            payload,
        };
        let response = self
            .client
            .send_request(DaemonRequest::Command(Box::new(request)))
            .map_err(|error| CliError::message(format!("command request failed: {error}")))?;

        // unwrap the protocol response
        let response = match response {
            DaemonResponse::CommandResult(response) => response,
            DaemonResponse::Error(error) => {
                return Err(CliError::message(format!("command failed: {error}")));
            }
            other => {
                return Err(CliError::message(format!("unexpected response: {other:?}")));
            }
        };

        Ok(command_result_from_response(response))
    }

    /// Run a workspace query for a root.
    pub fn run_query(
        &self,
        root: &Path,
        request: QueryRequestEnvelope,
    ) -> CliResult<QueryResponseEnvelope> {
        // resolve the workspace handle
        let handle = self.handle_for_root(root).ok_or_else(|| {
            CliError::message(format!("workspace root not opened: {}", root.display()))
        })?;

        // send the request to the daemon
        let response = self
            .client
            .send_request(DaemonRequest::Query(DaemonQuery::WorkspaceQuery {
                handle: handle.handle,
                request,
            }))
            .map_err(|error| CliError::message(format!("query request failed: {error}")))?;

        // unwrap the protocol response
        let response = match response {
            DaemonResponse::QueryResult(response) => response,
            DaemonResponse::Error(error) => {
                return Err(CliError::message(format!("query failed: {error}")));
            }
            other => {
                return Err(CliError::message(format!("unexpected response: {other:?}")));
            }
        };

        // unwrap the query response
        let response = match response {
            DaemonQueryResponse::WorkspaceQuery(response) => response,
            other => {
                return Err(CliError::message(format!(
                    "unexpected query response: {other:?}"
                )));
            }
        };

        Ok(response)
    }

    /// Run a batch of workspace queries for a root.
    pub fn run_query_batch(
        &self,
        root: &Path,
        requests: Vec<QueryRequestEnvelope>,
    ) -> CliResult<Vec<QueryResponseEnvelope>> {
        // resolve the workspace handle
        let handle = self.handle_for_root(root).ok_or_else(|| {
            CliError::message(format!("workspace root not opened: {}", root.display()))
        })?;

        // send the request to the daemon
        let response = self
            .client
            .send_request(DaemonRequest::Query(DaemonQuery::WorkspaceQueryBatch {
                handle: handle.handle,
                requests,
            }))
            .map_err(|error| CliError::message(format!("query request failed: {error}")))?;

        // unwrap the protocol response
        let response = match response {
            DaemonResponse::QueryResult(response) => response,
            DaemonResponse::Error(error) => {
                return Err(CliError::message(format!("query failed: {error}")));
            }
            other => {
                return Err(CliError::message(format!("unexpected response: {other:?}")));
            }
        };

        // unwrap the query response
        let response = match response {
            DaemonQueryResponse::WorkspaceQueryBatch(response) => response,
            other => {
                return Err(CliError::message(format!(
                    "unexpected query response: {other:?}"
                )));
            }
        };

        Ok(response)
    }

    /// Release the daemon connection.
    pub fn shutdown(mut self) {
        let _ = self.connection.take();
    }

    /// Resolve the handle for a root, if known.
    fn handle_for_root(&self, root: &Path) -> Option<&WorkspaceHandle> {
        self.handles.iter().find(|handle| handle.root == root)
    }

    /// Build a protocol watch batch for a specific root.
    fn protocol_batch_for_root(
        &self,
        root: &Path,
        batch: &destack_daemon::WatchBatch,
    ) -> Option<ProtocolWatchBatch> {
        let events: Vec<WatchEvent> = batch
            .events
            .iter()
            .filter(|event| event.path.starts_with(root))
            .map(WatchEvent::from)
            .collect();
        let status: Vec<WatchStatus> = batch
            .status
            .iter()
            .filter_map(|status| filter_status_for_root(status, root))
            .map(|status| WatchStatus::from(&status))
            .collect();

        if events.is_empty() && status.is_empty() {
            return None;
        }

        let (started_at_ns, ended_at_ns) = watch_batch_timestamps(batch);
        Some(ProtocolWatchBatch {
            events,
            status,
            overflowed: batch.overflowed,
            started_at_ns,
            ended_at_ns,
        })
    }

    /// Apply a protocol watch batch for a handle.
    fn apply_batch_for_handle(
        &self,
        handle: &WorkspaceHandle,
        batch: ProtocolWatchBatch,
    ) -> CliResult<WatchBatchSummary> {
        let response = self
            .client
            .send_request(DaemonRequest::ApplyWatchBatch(WatchBatchRequest {
                handle: handle.handle,
                batch,
            }))
            .map_err(|error| CliError::message(format!("apply watch batch failed: {error}")))?;

        match response {
            DaemonResponse::WatchBatchApplied(response) => Ok(WatchBatchSummary {
                updated: !response.updates.is_empty(),
                messages: response
                    .messages
                    .iter()
                    .map(WatchMessage::from_record)
                    .collect(),
            }),
            DaemonResponse::Error(error) => Err(CliError::message(format!(
                "apply watch batch failed: {error}"
            ))),
            other => Err(CliError::message(format!("unexpected response: {other:?}"))),
        }
    }
}

impl WatchDaemon for ProtocolDaemonClient {
    fn apply_watch_batch(
        &self,
        batch: &destack_daemon::WatchBatch,
    ) -> CliResult<WatchBatchSummary> {
        let mut summary = WatchBatchSummary::default();
        for handle in &self.handles {
            let Some(protocol_batch) = self.protocol_batch_for_root(&handle.root, batch) else {
                continue;
            };
            let result = self.apply_batch_for_handle(handle, protocol_batch)?;
            summary.updated |= result.updated;
            summary.messages.extend(result.messages);
        }

        Ok(summary)
    }
}

impl WatchMessage {
    /// Build a watch message from a protocol record.
    pub fn from_record(record: &DaemonMessageRecord) -> Self {
        let kind = match record.kind {
            ProtocolMessageKind::Info => destack_daemon::DaemonMessageKind::Info,
            ProtocolMessageKind::Warning => destack_daemon::DaemonMessageKind::Warning,
            ProtocolMessageKind::Error => destack_daemon::DaemonMessageKind::Error,
        };
        Self {
            kind,
            message: record.message.clone(),
        }
    }
}

/// Convert input sources into daemon command inputs.
pub fn command_inputs_from_sources(
    sources: &[InputSource],
    default_file_type: FileType,
) -> CliResult<Vec<CommandInput>> {
    // convert each input source into a command input
    let mut inputs = Vec::new();
    for source in sources {
        match source {
            InputSource::File(path) => inputs.push(CommandInput::File { path: path.clone() }),
            InputSource::Inline { code, name } => {
                let file_type = FileType::from_path(Path::new(name)).unwrap_or(default_file_type);
                inputs.push(CommandInput::Inline {
                    name: name.clone(),
                    content: code.clone(),
                    file_type,
                });
            }
            InputSource::Stdin { name } => {
                let mut content = String::new();
                std::io::stdin()
                    .read_to_string(&mut content)
                    .map_err(|error| CliError::message(format!("failed to read stdin: {error}")))?;
                let file_type = FileType::from_path(Path::new(name)).unwrap_or(default_file_type);
                inputs.push(CommandInput::Stdin {
                    name: name.clone(),
                    content,
                    file_type,
                });
            }
        }
    }

    Ok(inputs)
}

/// Convert CLI target arguments into daemon target overrides.
pub fn target_overrides_from_args(args: &TargetArgs) -> Option<CommandTargetOverrides> {
    // return early when no overrides are provided
    if !args.has_adhoc_options() {
        return None;
    }

    Some(CommandTargetOverrides {
        output: args.output.map(Into::into),
        runtime: args.runtime.map(Into::into),
        platform: args.platform.map(Into::into),
        target_triple: args.target_triple.clone(),
        cpu: args.cpu.clone(),
        cpu_features: args.cpu_features.clone(),
        link_mode: args.link_mode.map(Into::into),
        lto: args.lto.map(Into::into),
        linker: args.linker.clone(),
        link_args: args.link_args.clone(),
        sysroot: args.sysroot.clone(),
        out_dir: args.out_dir.clone(),
        out_file: args.out_file.clone(),
        declaration: args.declaration,
        source_map: args.source_map,
        emit: args.emit.iter().copied().map(Into::into).collect(),
        optimize: args.optimize,
        opt_level: args.opt_level.map(OptimizeLevel::from),
        debug: args.debug,
        release: args.release,
        debug_info: args.debug_info.map(Into::into),
        strip: args.strip.map(Into::into),
    })
}

/// Convert a daemon command stats payload into CLI stats.
pub fn command_stats_from_protocol(stats: &CommandStats, include_timings: bool) -> CliCommandStats {
    // map cache stats when present
    let cache = stats.cache.as_ref().map(|cache| CommandCacheStats {
        hits_memory: saturating_usize(cache.hits_memory),
        hits_disk: saturating_usize(cache.hits_disk),
        misses: saturating_usize(cache.misses),
        writes_memory: saturating_usize(cache.writes_memory),
        writes_disk: saturating_usize(cache.writes_disk),
        errors: saturating_usize(cache.errors),
        hit_rate: cache.hit_rate,
    });
    let timings = stats.timings.as_ref().and_then(|entries| {
        if !include_timings {
            return None;
        }
        let mapped = entries
            .iter()
            .map(|entry| CommandTimingTagStats {
                name: entry.name.clone(),
                duration_ms: entry.duration_ms,
                sample_count: entry.sample_count,
            })
            .collect::<Vec<_>>();
        Some(mapped)
    });

    CliCommandStats {
        elapsed_ms: stats.elapsed_ms,
        tasks_completed: saturating_usize(stats.tasks_completed),
        tasks_failed: saturating_usize(stats.tasks_failed),
        tasks_skipped: saturating_usize(stats.tasks_skipped),
        modules_processed: saturating_usize(stats.modules_processed),
        lines_processed: saturating_usize(stats.lines_processed),
        slow_tasks: saturating_usize(stats.slow_tasks),
        cache,
        timings,
    }
}

/// Run a daemon command with a one-shot client.
pub fn run_daemon_command(
    program: &ProgramArgs,
    diagnostic: Option<DiagnosticOptions>,
    common: CommonCommandOptions,
    payload: CommandPayload,
    event_handler: Option<CompilerEventHandler>,
) -> CliResult<DaemonCommandResult> {
    // prepare the session for a one-shot run
    let session = program.setup();

    let diagnostic = diagnostic.unwrap_or_default();

    run_daemon_command_with_session(session, program, diagnostic, common, payload, event_handler)
}

/// Run a daemon command using an existing session.
pub fn run_daemon_command_with_session(
    session: Arc<Session>,
    program: &ProgramArgs,
    diagnostic: DiagnosticOptions,
    common: CommonCommandOptions,
    payload: CommandPayload,
    event_handler: Option<CompilerEventHandler>,
) -> CliResult<DaemonCommandResult> {
    // resolve workspace roots for the daemon session
    let roots = watch_roots(program, &session);
    let root = roots
        .first()
        .cloned()
        .unwrap_or_else(|| session.cwd.clone());

    // build compiler options for the daemon
    let daemon_options = build_daemon_options(program, diagnostic, event_handler);
    let daemon = ProtocolDaemonClient::new(session.clone(), daemon_options, roots, program)?;

    // execute the command and shutdown
    let result = daemon.run_command(&root, common, payload)?;
    daemon.shutdown();
    Ok(result)
}

/// Emit output for a daemon command that returns a message payload.
pub fn finish_daemon_message_command(
    command: &str,
    report_args: &ReportArgs,
    result: &DaemonCommandResult,
) -> i32 {
    // emit text output for non json modes
    if !report_args.is_json() {
        emit_daemon_text_output(
            report_args,
            &result.response.messages,
            &result.response.output,
        );
        return result.response.exit_code;
    }

    // decode the message payload for structured output
    let payload = match parse_command_payload::<CommandMessagePayload>(
        command,
        report_args,
        result.response.data.as_ref(),
        command,
        false,
    ) {
        Ok(payload) => payload,
        Err(code) => return code,
    };

    // emit json report output
    let exit_code = result.response.exit_code;
    let report = report_from_message_payload(command, exit_code, payload);
    print_report(&report, report_args.format());
    exit_code
}

/// Emit daemon messages and output for text mode.
pub fn emit_daemon_text_output(
    report_args: &ReportArgs,
    messages: &[DaemonMessageRecord],
    output: &[CommandOutputChunk],
) {
    // skip emission when json output is enabled
    if report_args.is_json() {
        return;
    }

    // emit message records first
    emit_daemon_messages(messages);

    // emit command output when available
    if let Err(error) = emit_command_output(output) {
        console::error(&error.to_string());
    }
}

/// Emit command output chunks to stdout and stderr.
pub fn emit_command_output(output: &[CommandOutputChunk]) -> CliResult<()> {
    // write buffered chunks to stdout/stderr
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    for chunk in output {
        let result = match chunk.stream {
            OutputStream::Stdout => stdout.write_all(&chunk.bytes),
            OutputStream::Stderr => stderr.write_all(&chunk.bytes),
        };
        if let Err(error) = result {
            return Err(CliError::message(format!(
                "failed to write command output: {error}"
            )));
        }
    }

    // flush outputs to ensure they are visible
    if let Err(error) = stdout.flush() {
        return Err(CliError::message(format!(
            "failed to flush stdout: {error}"
        )));
    }
    if let Err(error) = stderr.flush() {
        return Err(CliError::message(format!(
            "failed to flush stderr: {error}"
        )));
    }

    Ok(())
}

/// Emit daemon message records to the console.
pub fn emit_daemon_messages(messages: &[DaemonMessageRecord]) {
    // print each message based on severity
    for message in messages {
        let rendered = if let Some(path) = message.path.as_ref() {
            format!("{}: {}", path.display(), message.message)
        } else {
            message.message.clone()
        };

        match message.kind {
            ProtocolMessageKind::Info => console::info(&rendered),
            ProtocolMessageKind::Warning => console::warn(&rendered),
            ProtocolMessageKind::Error => console::error(&rendered),
        }
    }
}

/// Convert a command response into CLI friendly diagnostics and files.
fn command_result_from_response(response: CommandResponse) -> DaemonCommandResult {
    // collect diagnostics from protocol batches
    let diagnostics = diagnostics_from_batches(&response.diagnostics);

    // rebuild file registry from snapshots
    let files = files_from_snapshots(&response.files);

    DaemonCommandResult {
        response,
        diagnostics,
        files,
    }
}

/// Convert diagnostic batches into a collection.
fn diagnostics_from_batches(batches: &[DiagnosticBatch]) -> DiagnosticCollection {
    // flatten diagnostics into a collection
    let mut collection = DiagnosticCollection::new();
    for batch in batches {
        for diagnostic in &batch.diagnostics {
            collection.insert(diagnostic.clone());
        }
    }
    collection
}

/// Convert file snapshots into a file registry.
fn files_from_snapshots(snapshots: &[FileSnapshot]) -> FileRegistry {
    // rebuild a registry from snapshot metadata
    let registry = FileRegistry::new();
    for snapshot in snapshots {
        let file = match snapshot.content.as_ref() {
            Some(content) => File::from_text(
                snapshot.id,
                snapshot.name.clone(),
                snapshot.uri.clone(),
                snapshot.path.clone(),
                snapshot.file_type,
                content.clone(),
            ),
            None => File::unloaded(
                snapshot.id,
                snapshot.name.clone(),
                snapshot.uri.clone(),
                snapshot.path.clone(),
                snapshot.file_type,
            ),
        };
        registry.insert(file);
    }
    registry
}

/// Convert a u64 count into usize without panicking.
fn saturating_usize(value: u64) -> usize {
    // clamp counts that exceed usize
    usize::try_from(value).unwrap_or(usize::MAX)
}

fn filter_status_for_root(status: &FileWatchStatus, root: &Path) -> Option<FileWatchStatus> {
    match status {
        FileWatchStatus::Ready { roots } => {
            let roots: Vec<PathBuf> = roots.iter().filter(|path| path == &root).cloned().collect();
            if roots.is_empty() {
                return None;
            }
            Some(FileWatchStatus::Ready { roots })
        }
        FileWatchStatus::RescanRequested { roots, reason } => {
            let roots: Vec<PathBuf> = roots.iter().filter(|path| path == &root).cloned().collect();
            if roots.is_empty() {
                return None;
            }
            Some(FileWatchStatus::RescanRequested {
                roots,
                reason: reason.clone(),
            })
        }
        FileWatchStatus::Error { message } => Some(FileWatchStatus::Error {
            message: message.clone(),
        }),
        FileWatchStatus::Stopped => Some(FileWatchStatus::Stopped),
    }
}

fn watch_batch_timestamps(batch: &destack_daemon::WatchBatch) -> (u64, u64) {
    let duration = batch.ended_at.saturating_duration_since(batch.started_at);
    let end = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let start = end
        .checked_sub(duration)
        .unwrap_or_else(|| Duration::from_secs(0));
    (duration_to_ns(start), duration_to_ns(end))
}

fn duration_to_ns(duration: Duration) -> u64 {
    let nanos = duration.as_nanos();
    if nanos > u64::MAX as u128 {
        u64::MAX
    } else {
        nanos as u64
    }
}
