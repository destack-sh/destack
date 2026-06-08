use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_daemon::protocol::{
    CommandEnvVar, CommandInput, CommandMessagePayload, CommandOutputChunk, CommandPayload,
    CommandRequest, CommandResponse, CommandRunPayload, CommandTargetOverrides,
    CommonCommandOptions, DaemonMessageRecord, DaemonRequest, DaemonResponse, DiagnosticBatch,
    FileUpdateImage, ManifestOverride, OutputStream, ProtocolClient, QueryRequestBody,
    QueryResponseBody, RootHandleId, RootOpenOptions,
};
use destack_daemon::{
    CommandRevision, DaemonConnectOptions, DaemonConnection, DaemonEndpoint, DaemonLaunch,
    DaemonMessageKind, WatchBatch, connect_ipc_daemon, protocol,
};
use destack_repository::{Repository, Revision};
use destack_source::{DiagnosticCollection, File, FileId, FileType};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::common::program::{
    FormatterOptionsArgs, IndentStyleArg, LineEndingArg, LintPresetArg, LinterOptionsArgs,
};
use crate::common::{
    CommandError, CommandReport, DiagnosticFormat, FormatOptions, InputSource, LineWriter,
    ProgramArgs, ReportArgs, TargetArgs, collect_diagnostics_json, format_diagnostics_with_writer,
    parse_command_payload, parse_required_command_payload, print_report, report_error,
    report_from_message_payload, report_from_payload,
};
use crate::console;
use crate::error::{CliError, CliResult};
use crate::pipeline::watch::{WatchBatchSummary, WatchDaemon, WatchMessage, watch_roots};

/// Protocol backed daemon client for CLI flows.
#[derive(Debug)]
pub struct ProtocolDaemonClient {
    /// Protocol client for daemon requests.
    client: Arc<ProtocolClient>,
    /// Root handles keyed by root path.
    handles: Vec<RootHandle>,
    /// Connection state for the daemon.
    connection: Option<DaemonConnection>,
}

/// Root handle metadata for CLI usage.
#[derive(Debug, Clone)]
struct RootHandle {
    /// Workspace root for the handle.
    root: PathBuf,
    /// Daemon handle identifier.
    handle: RootHandleId,
}

/// Settings used to build a daemon launch.
#[derive(Debug, Clone)]
pub(crate) struct DaemonLaunchContext {
    /// Current working directory for the daemon.
    cwd: PathBuf,
    /// Destack home directory override.
    home: Option<PathBuf>,
    /// Package directory override.
    package_dir: Option<PathBuf>,
    /// Workspace cache directory override.
    cache_dir: Option<PathBuf>,
    /// Manifest path override.
    manifest_path: Option<PathBuf>,
}

impl DaemonLaunchContext {
    /// Build a launch context from program arguments.
    pub(crate) fn from_program(program: &ProgramArgs) -> Self {
        // resolve the cwd for the daemon
        let cwd = program.cwd.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|error| panic!("failed to resolve current directory: {error}"))
        });

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
            home: program.home.clone(),
            package_dir: program.package_dir.clone(),
            cache_dir,
            manifest_path: program.manifest.clone(),
        }
    }

    /// Build a daemon launch for an endpoint and initial root.
    pub(crate) fn build_launch(&self, endpoint: &DaemonEndpoint, root: PathBuf) -> DaemonLaunch {
        // build the base launch from the endpoint
        let mut launch = DaemonLaunch::for_endpoint(endpoint, root);

        // apply overrides from program settings
        launch.home = self.home.clone();
        launch.package_dir = self.package_dir.clone();
        launch.cache_dir = self.cache_dir.clone();
        launch.manifest_path = self.manifest_path.clone();
        launch.cwd = Some(self.cwd.clone());

        launch
    }
}

/// Connector for daemon clients.
#[derive(Debug)]
struct DaemonConnector {
    /// Connect options for the daemon.
    options: DaemonConnectOptions,
    /// Endpoint metadata for ipc daemons.
    endpoint: DaemonEndpoint,
    /// Launch values for daemon spawn.
    launch: DaemonLaunch,
    /// Injected daemon client.
    daemon_client: Option<Arc<ProtocolClient>>,
}

impl DaemonConnector {
    /// Create a connector from program settings.
    fn new(repository: Arc<Repository>, program: &ProgramArgs) -> Self {
        // build connect options
        let options = DaemonConnectOptions::default();

        // resolve daemon endpoint metadata
        let endpoint = DaemonEndpoint::new(repository.layout().home.clone());
        let launch_context = DaemonLaunchContext::from_program(program);
        let launch = launch_context.build_launch(&endpoint, repository.path().to_path_buf());

        Self {
            options,
            endpoint,
            launch,
            daemon_client: program.daemon_client.clone(),
        }
    }

    /// Connect to the daemon endpoint.
    fn connect(&self) -> CliResult<DaemonConnection> {
        // use an injected daemon client
        if let Some(client) = self.daemon_client.clone() {
            return Ok(DaemonConnection { client });
        }

        // connect via ipc, spawning when needed
        connect_ipc_daemon(
            &self.endpoint,
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
    /// Files reconstructed from response images.
    pub files: BTreeMap<FileId, Arc<File>>,
}

/// Builder for common daemon command options.
#[derive(Debug, Clone)]
pub struct CommandOptionsBuilder {
    options: CommonCommandOptions,
}

impl CommandOptionsBuilder {
    /// Create a builder seeded with program defaults.
    pub fn new(program: &ProgramArgs) -> Self {
        // build defaults from program settings
        let options = CommonCommandOptions {
            inputs: Vec::new(),
            use_destack_config_inputs: false,
            cwd: Some(program.effective_cwd()),
            manifest_path: program.manifest.clone(),
            target: None,
            target_overrides: None,
            profile: None,
            env: Vec::new(),
            manifest_overrides: manifest_overrides_from_program(program),
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

    /// Use destack.json sources when explicit inputs are empty.
    pub fn use_destack_config_inputs(mut self, allow: bool) -> Self {
        self.options.use_destack_config_inputs = allow;
        self
    }

    /// Set the target name override.
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.options.target = Some(target.into());
        self
    }

    /// Set target overrides.
    pub fn target_overrides(mut self, overrides: Option<CommandTargetOverrides>) -> Self {
        self.options.target_overrides = overrides;
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

    /// Set manifest overrides.
    pub fn manifest_overrides(mut self, overrides: Vec<ManifestOverride>) -> Self {
        self.options.manifest_overrides = overrides;
        self
    }

    /// Add one optional manifest override.
    pub fn manifest_override(mut self, override_: Option<ManifestOverride>) -> Self {
        if let Some(override_) = override_ {
            self.options.manifest_overrides.push(override_);
        }

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

/// Build manifest overrides from explicit CLI formatter and linter options.
pub fn manifest_overrides_from_program(program: &ProgramArgs) -> Vec<ManifestOverride> {
    let mut overrides = Vec::new();

    // formatter
    if let Some(value) = formatter_override_value(&program.formatter) {
        overrides.push(ManifestOverride {
            path: "formatter".to_string(),
            value,
        });
    }

    // linter
    if let Some(value) = linter_override_value(&program.linter) {
        overrides.push(ManifestOverride {
            path: "linter".to_string(),
            value,
        });
    }

    overrides
}

/// Build one formatter override object from explicit CLI flags.
fn formatter_override_value(args: &FormatterOptionsArgs) -> Option<Value> {
    let mut object: Map<String, Value> = Map::new();

    // layout
    if let Some(indent_style) = args.indent_style {
        object.insert(
            "indentStyle".to_string(),
            json!(indent_style_override_value(indent_style)),
        );
    }
    if let Some(indent_width) = args.indent_width {
        object.insert("indentWidth".to_string(), json!(indent_width));
    }
    if let Some(line_ending) = args.line_ending {
        object.insert(
            "lineEnding".to_string(),
            json!(line_ending_override_value(line_ending)),
        );
    }
    if let Some(line_width) = args.line_width {
        object.insert("lineWidth".to_string(), json!(line_width));
    }

    if object.is_empty() {
        return None;
    }

    Some(Value::Object(object))
}

/// Build one linter override object from explicit CLI flags.
fn linter_override_value(args: &LinterOptionsArgs) -> Option<Value> {
    let mut object: Map<String, Value> = Map::new();
    let mut rules: Map<String, Value> = Map::new();
    let mut complexity: Map<String, Value> = Map::new();

    // rule selection
    if let Some(preset) = args.preset {
        rules.insert(
            "preset".to_string(),
            json!(lint_preset_override_value(preset)),
        );
    }
    for rule in &args.allow {
        rules.insert(rule.clone(), json!("off"));
    }
    for rule in &args.warn {
        rules.insert(rule.clone(), json!("warn"));
    }
    for rule in &args.deny {
        rules.insert(rule.clone(), json!("error"));
    }

    // complexity
    if let Some(max_complexity) = args.max_complexity {
        complexity.insert("maxCyclomaticComplexity".to_string(), json!(max_complexity));
    }
    if let Some(max_params) = args.max_params {
        complexity.insert("maxParams".to_string(), json!(max_params));
    }
    if let Some(max_depth) = args.max_depth {
        complexity.insert("maxDepth".to_string(), json!(max_depth));
    }
    if let Some(max_lines) = args.max_lines {
        complexity.insert("maxLines".to_string(), json!(max_lines));
    }

    if !rules.is_empty() {
        object.insert("rules".to_string(), Value::Object(rules));
    }
    if !complexity.is_empty() {
        object.insert("complexity".to_string(), Value::Object(complexity));
    }

    if object.is_empty() {
        return None;
    }

    Some(Value::Object(object))
}

/// Convert one indent style argument to one config value.
fn indent_style_override_value(value: IndentStyleArg) -> &'static str {
    match value {
        IndentStyleArg::Tab => "tab",
        IndentStyleArg::Space => "space",
    }
}

/// Convert one line ending argument to one config value.
fn line_ending_override_value(value: LineEndingArg) -> &'static str {
    match value {
        LineEndingArg::Lf => "lf",
        LineEndingArg::Crlf => "crlf",
        LineEndingArg::Cr => "cr",
    }
}

/// Convert one lint preset argument to one config value.
fn lint_preset_override_value(value: LintPresetArg) -> &'static str {
    match value {
        LintPresetArg::None => "none",
        LintPresetArg::Recommended => "recommended",
        LintPresetArg::All => "all",
    }
}

impl ProtocolDaemonClient {
    /// Create a protocol daemon client for the provided roots.
    pub fn new(
        repository: Arc<Repository>,
        roots: Vec<PathBuf>,
        program: &ProgramArgs,
    ) -> CliResult<Self> {
        // connect to the daemon
        let connector = DaemonConnector::new(repository.clone(), program);
        let connection = connector.connect()?;
        let client = connection.client.clone();
        let workspace_root = repository.path().to_path_buf();

        // open each root
        let mut handles = Vec::new();
        for root in roots {
            let response = client
                .open_root(
                    workspace_root.clone(),
                    root.clone(),
                    RootOpenOptions::default(),
                )
                .map_err(|error| CliError::message(format!("open root failed: {error}")))?;
            let handle = response.handle;
            handles.push(RootHandle { root, handle });
        }

        Ok(Self {
            client,
            handles,
            connection: Some(connection),
        })
    }

    /// Run a root command for a root.
    pub fn run_root_command(
        &self,
        root: &Path,
        common: CommonCommandOptions,
        payload: CommandPayload,
    ) -> CliResult<DaemonCommandResult> {
        // resolve the root handle
        let handle = self
            .root_handle(root)
            .ok_or_else(|| CliError::message(format!("root not opened: {}", root.display())))?;

        // send the request to the daemon
        let request = CommandRequest {
            handle: handle.handle,
            revision: CommandRevision::Current,
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

    /// Run a query for a root.
    pub fn run_query(
        &self,
        root: &Path,
        request: QueryRequestBody,
    ) -> CliResult<QueryResponseBody> {
        // resolve the root handle
        let handle = self
            .root_handle(root)
            .ok_or_else(|| CliError::message(format!("root not opened: {}", root.display())))?;

        // send and decode the query
        self.client
            .execute_query(handle.handle, request)
            .map_err(|error| CliError::message(format!("query failed: {error}")))
    }

    /// Resolve the current semantic revision for a root.
    pub fn run_current_revision(&self, root: &Path) -> CliResult<Revision> {
        // resolve the root handle
        let handle = self
            .root_handle(root)
            .ok_or_else(|| CliError::message(format!("root not opened: {}", root.display())))?;

        // send and decode the revision query
        self.client
            .current_revision(handle.handle)
            .map_err(|error| CliError::message(format!("current revision failed: {error}")))
    }

    /// Run a batch of queries for a root.
    pub fn run_query_batch(
        &self,
        root: &Path,
        requests: Vec<QueryRequestBody>,
    ) -> CliResult<Vec<QueryResponseBody>> {
        // resolve the root handle
        let handle = self
            .root_handle(root)
            .ok_or_else(|| CliError::message(format!("root not opened: {}", root.display())))?;

        // send and decode the query batch
        self.client
            .execute_query_batch(handle.handle, requests)
            .map_err(|error| CliError::message(format!("query batch failed: {error}")))
    }

    /// Release the daemon connection.
    pub fn shutdown(mut self) {
        let _ = self.connection.take();
    }

    /// Resolve the root handle for a root, if known.
    fn root_handle(&self, root: &Path) -> Option<&RootHandle> {
        self.handles.iter().find(|handle| handle.root == root)
    }

    /// Return the primary watch handle.
    fn watch_handle(&self) -> CliResult<&RootHandle> {
        self.handles
            .first()
            .ok_or_else(|| CliError::message("no root handle opened for watch"))
    }
}

impl WatchDaemon for ProtocolDaemonClient {
    fn start_watch(&self, policy: &destack_daemon::WatchPolicy) -> CliResult<()> {
        let handle = self.watch_handle()?;
        let roots = self
            .handles
            .iter()
            .map(|handle| handle.root.clone())
            .collect();
        let options = protocol::WatchStartOptions::from(policy);

        self.client
            .start_watch(handle.handle, roots, options)
            .map_err(|error| CliError::message(format!("start watch failed: {error}")))?;

        Ok(())
    }

    fn next_watch_batch(&self) -> CliResult<Option<WatchBatchSummary>> {
        let handle = self.watch_handle()?;
        let response = self
            .client
            .next_watch_batch(handle.handle)
            .map_err(|error| CliError::message(format!("next watch batch failed: {error}")))?;
        let Some(batch) = response.batch else {
            return Ok(None);
        };
        let batch = WatchBatch::from(&batch);

        Ok(Some(WatchBatchSummary {
            batch,
            updated: !response.updates.is_empty(),
            messages: response
                .messages
                .iter()
                .map(WatchMessage::from_record)
                .collect(),
        }))
    }

    fn stop_watch(&self) {
        if let Ok(handle) = self.watch_handle() {
            let _ = self.client.stop_watch(handle.handle);
        }
    }
}

impl WatchMessage {
    /// Build a watch message from a protocol record.
    pub fn from_record(record: &DaemonMessageRecord) -> Self {
        let kind = match record.kind {
            protocol::DaemonMessageKind::Info => DaemonMessageKind::Info,
            protocol::DaemonMessageKind::Warning => DaemonMessageKind::Warning,
            protocol::DaemonMessageKind::Error => DaemonMessageKind::Error,
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
    if !args.has_output_options() {
        return None;
    }

    Some(CommandTargetOverrides {
        out_dir: args.out_dir.clone(),
        out_file: args.out_file.clone(),
    })
}

/// Run a daemon command with a one-shot client.
pub fn run_root_command_once(
    program: &ProgramArgs,
    common: CommonCommandOptions,
    payload: CommandPayload,
) -> CliResult<DaemonCommandResult> {
    // prepare the repository for a one-shot run
    let repository = program.setup();

    run_root_command_with_repository(repository, program, common, payload)
}

/// Run a daemon command using an existing repository.
pub fn run_root_command_with_repository(
    repository: Arc<Repository>,
    program: &ProgramArgs,
    common: CommonCommandOptions,
    payload: CommandPayload,
) -> CliResult<DaemonCommandResult> {
    // resolve roots for the daemon repository
    let roots = watch_roots(program, &repository);
    let Some(root) = roots.first().cloned() else {
        return Err(CliError::message("roots are empty"));
    };

    let daemon = ProtocolDaemonClient::new(repository.clone(), roots, program)?;

    // execute the command and shutdown
    let result = daemon.run_root_command(&root, common, payload)?;
    daemon.shutdown();
    Ok(result)
}

/// Execute a root command or emit a CLI error report.
pub fn run_root_command_or_report(
    command: &str,
    report_args: &ReportArgs,
    program: &ProgramArgs,
    common: CommonCommandOptions,
    payload: CommandPayload,
) -> Result<DaemonCommandResult, i32> {
    run_root_command_once(program, common, payload)
        .map_err(|error| report_error(command, report_args, &error.to_string()))
}

/// Execute a root command and decode a required payload or emit a CLI error report.
pub fn run_root_command_with_required_payload_or_report<T: DeserializeOwned>(
    command: &str,
    report_args: &ReportArgs,
    program: &ProgramArgs,
    common: CommonCommandOptions,
    payload: CommandPayload,
    payload_label: &str,
) -> Result<(DaemonCommandResult, T, Value), i32> {
    let result = run_root_command_or_report(command, report_args, program, common, payload)?;
    let (payload, value) = parse_required_command_payload::<T>(
        command,
        report_args,
        result.response.data.as_ref(),
        payload_label,
    )?;

    Ok((result, payload, value))
}

/// Execute a root command with a prepared repository or emit a CLI error report.
pub fn run_root_command_with_repository_or_report(
    command: &str,
    report_args: &ReportArgs,
    repository: Arc<Repository>,
    program: &ProgramArgs,
    common: CommonCommandOptions,
    payload: CommandPayload,
) -> Result<DaemonCommandResult, i32> {
    run_root_command_with_repository(repository, program, common, payload)
        .map_err(|error| report_error(command, report_args, &error.to_string()))
}

/// Shared summary metadata for diagnostic commands.
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticCommandSummary<'a> {
    /// Verb used for the final summary.
    pub verb: &'a str,
    /// Number of modules processed.
    pub modules: usize,
    /// Number of profiles processed.
    pub profiles: usize,
    /// Number of targets processed.
    pub targets: usize,
}

/// Print a compact diagnostic command summary.
fn print_diagnostic_command_summary(
    summary: &DiagnosticCommandSummary<'_>,
    errors: usize,
    warnings: usize,
    line_writer: Option<&LineWriter>,
) {
    let mut parts = vec![
        format!("{} {}", summary.verb, pluralize(summary.modules, "module")),
        pluralize(summary.profiles, "profile"),
    ];

    if summary.targets > 0 {
        parts.push(pluralize(summary.targets, "target"));
    }

    parts.push(pluralize(errors, "error"));
    parts.push(pluralize(warnings, "warning"));

    let line = parts.join(", ");
    if let Some(line_writer) = line_writer {
        line_writer(&line);
    } else {
        eprintln!("{line}");
    }
}

/// Pluralize a word for a display count.
fn pluralize(count: usize, word: &str) -> String {
    if count == 1 {
        return format!("{count} {word}");
    }

    format!("{count} {word}s")
}

/// Run a daemon command that returns a required typed payload.
pub fn run_root_payload_command_or_report<T, JsonFn, TextFn>(
    command: &str,
    report_args: &ReportArgs,
    program: &ProgramArgs,
    common: CommonCommandOptions,
    payload: CommandPayload,
    payload_label: &str,
    json_report: JsonFn,
    text_report: TextFn,
) -> i32
where
    T: DeserializeOwned,
    JsonFn: FnOnce(i32, T, Value) -> CommandReport,
    TextFn: FnOnce(i32, T),
{
    // execute the command and decode the payload
    let (result, payload, payload_value) = match run_root_command_with_required_payload_or_report::<T>(
        command,
        report_args,
        program,
        common,
        payload,
        payload_label,
    ) {
        Ok(result) => result,
        Err(code) => return code,
    };

    // emit daemon output before command specific rendering
    emit_daemon_text_output(
        report_args,
        &result.response.messages,
        &result.response.output,
    );

    let exit_code = result.response.exit_code;

    // emit the structured report when requested
    if report_args.is_json() {
        let report = json_report(exit_code, payload, payload_value);
        print_report(&report, report_args.format());
        return exit_code;
    }

    // otherwise render the payload in text mode
    text_report(exit_code, payload);
    exit_code
}

/// Finish a daemon command that primarily reports diagnostics.
pub fn finish_diagnostic_command(
    command: &str,
    report_args: &ReportArgs,
    result: &DaemonCommandResult,
    json_format_options: &FormatOptions,
    text_format_options: &FormatOptions,
    line_writer: Option<&LineWriter>,
    summary: Option<DiagnosticCommandSummary<'_>>,
    data: Option<Value>,
) -> i32 {
    // emit daemon output only for text mode
    if !report_args.is_json() {
        emit_daemon_text_output(
            report_args,
            &result.response.messages,
            &result.response.output,
        );
    }

    // build a structured diagnostics report when requested
    if report_args.is_json() {
        let (output, format_result) = collect_diagnostics_json(
            &|file_id| result.files.get(&file_id).cloned(),
            &result.diagnostics,
            json_format_options,
        );
        let mut report = report_from_payload(command, format_result.exit_code(), data, None, None);
        report.diagnostics = Some(output);
        print_report(&report, report_args.format());
        return format_result.exit_code();
    }

    // render diagnostics for text oriented output
    let format_result = format_diagnostics_with_writer(
        &|file_id| result.files.get(&file_id).cloned(),
        &result.diagnostics,
        text_format_options,
        result.response.module_count,
        line_writer,
    );

    // emit a compact command summary after text diagnostics
    if matches!(text_format_options.format, DiagnosticFormat::Text)
        && let Some(summary) = summary
    {
        print_diagnostic_command_summary(
            &summary,
            format_result.error_count,
            format_result.warning_count,
            line_writer,
        );
    }

    // keep warning threshold failures loud in text mode
    if format_result.max_warnings_exceeded {
        console::warn(&format!(
            "warning count ({}) exceeds --max-warnings ({})",
            format_result.warning_count,
            text_format_options.max_warnings.unwrap_or(0)
        ));
        return 1;
    }

    format_result.exit_code()
}

/// Finish a run command with diagnostic and payload rendering.
pub fn finish_run_command(
    command: &str,
    report_args: &ReportArgs,
    result: &DaemonCommandResult,
) -> i32 {
    // render diagnostics before payload output
    if report_args.is_json() {
        let json_options = FormatOptions {
            format: DiagnosticFormat::Json,
            ..FormatOptions::default()
        };
        let (output, format_result) = collect_diagnostics_json(
            &|file_id| result.files.get(&file_id).cloned(),
            &result.diagnostics,
            &json_options,
        );

        if format_result.exit_code() != 0 {
            let mut report = CommandReport::failure(command, format_result.exit_code());
            report.diagnostics = Some(output);
            print_report(&report, report_args.format());
            return format_result.exit_code();
        }
    } else {
        let text_options = FormatOptions::default();
        let format_result = format_diagnostics_with_writer(
            &|file_id| result.files.get(&file_id).cloned(),
            &result.diagnostics,
            &text_options,
            result.response.module_count,
            None,
        );
        if format_result.exit_code() != 0 {
            return format_result.exit_code();
        }
    }

    // emit daemon text output before the final run status
    emit_daemon_text_output(
        report_args,
        &result.response.messages,
        &result.response.output,
    );

    let exit_code = result.response.exit_code;

    // print the structured payload for json output
    if report_args.is_json() {
        let payload = match parse_command_payload::<CommandRunPayload>(
            command,
            report_args,
            result.response.data.as_ref(),
            "run",
            false,
        ) {
            Ok(payload) => payload,
            Err(code) => return code,
        };

        let (summary, error, data) = match payload {
            Some((CommandRunPayload::RuntimeError { message }, value)) => {
                let error = Some(CommandError::new("runtime_error", "run", message.clone()));
                (Some(message), error, Some(value))
            }
            Some((CommandRunPayload::Value { .. }, value)) => (None, None, Some(value)),
            None => (None, None, None),
        };
        let report = report_from_payload(command, exit_code, data, summary, error);
        print_report(&report, report_args.format());
    } else if exit_code != 0 {
        console::warn(&format!("process exited with code {exit_code}"));
    }

    exit_code
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
            protocol::DaemonMessageKind::Info => console::info(&rendered),
            protocol::DaemonMessageKind::Warning => console::warn(&rendered),
            protocol::DaemonMessageKind::Error => console::error(&rendered),
        }
    }
}

/// Convert a command response into CLI friendly diagnostics and files.
fn command_result_from_response(response: CommandResponse) -> DaemonCommandResult {
    // collect diagnostics from protocol batches
    let diagnostics = diagnostics_from_batches(&response.diagnostics);

    // rebuild file registry from images
    let files = files_from_update_images(&response.files);

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

/// Convert file images into a file registry.
fn files_from_update_images(images: &[FileUpdateImage]) -> BTreeMap<FileId, Arc<File>> {
    // rebuild explicit files from image metadata
    let mut files = BTreeMap::new();
    for image in images {
        let Some(content) = image.content.as_ref() else {
            continue;
        };
        let file = File::from_text(
            image.id,
            image.name.clone(),
            image.uri.clone(),
            image.path.clone(),
            image.file_type,
            content.clone(),
        );
        files.insert(image.id, Arc::new(file));
    }
    files
}
