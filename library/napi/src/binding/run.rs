use std::path::{Path, PathBuf};
use std::sync::Arc;

use napi::Error;
use napi_derive::napi;
use {destack_daemon as daemon, destack_source as source, destack_workspace as workspace};

use super::{
    CompilerOptions, Diagnostic, FileType, diagnostics_have_errors, diagnostics_have_warnings,
};

/// Input kind for run operations.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunInputKind {
    /// Execute a physical file path.
    File,
    /// Execute inline content.
    Inline,
    /// Execute stdin content.
    Stdin,
}

/// Input payload for run operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RunInput {
    /// The input kind.
    pub kind: RunInputKind,
    /// File path for file inputs.
    pub path: Option<String>,
    /// Virtual file name for inline or stdin inputs.
    pub name: Option<String>,
    /// Source content for inline or stdin inputs.
    pub content: Option<String>,
    /// Optional file type override for inline or stdin inputs.
    pub file_type: Option<FileType>,
}

/// Execution mode for run operations.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// Execute entry module mode.
    Program,
    /// Evaluate inline or stdin mode without printing.
    Eval,
    /// Evaluate inline or stdin mode and print the result.
    EvalPrint,
}

impl From<RunMode> for daemon::CommandRunMode {
    fn from(mode: RunMode) -> Self {
        match mode {
            RunMode::Program => daemon::CommandRunMode::Program,
            RunMode::Eval => daemon::CommandRunMode::Eval { print: false },
            RunMode::EvalPrint => daemon::CommandRunMode::Eval { print: true },
        }
    }
}

/// Environment variable override for run operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RunEnvironmentVariable {
    /// Environment variable name.
    pub key: String,
    /// Environment variable value.
    pub value: String,
}

/// Config override for run operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RunConfigOverride {
    /// Override path (for example compilerOptions.strict).
    pub path: String,
    /// Override payload as JSON.
    pub value_json: String,
}

/// Options for run operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// The current working directory.
    pub cwd: String,
    /// Workspace roots to open.
    pub roots: Vec<String>,
    /// Compiler options for execution.
    pub compiler: CompilerOptions,
    /// Run mode.
    pub mode: RunMode,
    /// Entry function name.
    pub entry: String,
    /// Command line arguments passed to the program.
    pub args: Vec<String>,
    /// Optional target name override.
    pub target: Option<String>,
    /// Optional target overrides as JSON.
    pub target_overrides_json: Option<String>,
    /// Optional runtime overrides as JSON.
    pub runtime_overrides_json: Option<String>,
    /// Optional cache directory override.
    pub cache_dir: Option<String>,
    /// Optional dsconfig path override.
    pub config_path: Option<String>,
    /// Optional profile name override.
    pub profile: Option<String>,
    /// Environment variable overrides.
    pub env: Vec<RunEnvironmentVariable>,
    /// Config overrides.
    pub overrides: Vec<RunConfigOverride>,
    /// Whether to allow dsconfig input fallback when no inputs are provided.
    pub allow_dsconfig_fallback: bool,
    /// Whether to skip writes.
    pub dry_run: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            roots: Vec::new(),
            compiler: CompilerOptions::default(),
            mode: RunMode::Program,
            entry: "main".to_string(),
            args: Vec::new(),
            target: None,
            target_overrides_json: None,
            runtime_overrides_json: None,
            cache_dir: None,
            config_path: None,
            profile: None,
            env: Vec::new(),
            overrides: Vec::new(),
            allow_dsconfig_fallback: false,
            dry_run: false,
        }
    }
}

/// Output stream for run operations.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutputStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

impl From<daemon::DaemonOutputStream> for RunOutputStream {
    fn from(stream: daemon::DaemonOutputStream) -> Self {
        match stream {
            daemon::DaemonOutputStream::Stdout => RunOutputStream::Stdout,
            daemon::DaemonOutputStream::Stderr => RunOutputStream::Stderr,
        }
    }
}

/// Output chunk payload for run operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RunOutputChunk {
    /// Output stream kind.
    pub stream: RunOutputStream,
    /// Output bytes.
    pub bytes: Vec<u8>,
}

/// Cache stats payload for run operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RunCacheStats {
    /// Cache hits from memory.
    pub hits_memory: u32,
    /// Cache hits from disk.
    pub hits_disk: u32,
    /// Cache misses.
    pub misses: u32,
    /// Cache writes to memory.
    pub writes_memory: u32,
    /// Cache writes to disk.
    pub writes_disk: u32,
    /// Cache errors.
    pub errors: u32,
    /// Cache hit rate.
    pub hit_rate: f64,
}

/// Timing stats payload for run operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RunTimingTagStats {
    /// Timing tag name.
    pub name: String,
    /// Total time spent in this tag in milliseconds.
    pub duration_ms: u32,
    /// Number of samples recorded.
    pub sample_count: u32,
}

/// Command stats payload for run operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RunStats {
    /// Elapsed time in milliseconds.
    pub elapsed_ms: u32,
    /// Number of tasks completed.
    pub tasks_completed: u32,
    /// Number of tasks failed.
    pub tasks_failed: u32,
    /// Number of tasks skipped.
    pub tasks_skipped: u32,
    /// Number of modules processed.
    pub modules_processed: u32,
    /// Number of lines processed.
    pub lines_processed: u32,
    /// Number of slow tasks detected.
    pub slow_tasks: u32,
    /// Cache statistics when available.
    pub cache: Option<RunCacheStats>,
    /// Timing tag statistics when available.
    pub timings: Option<Vec<RunTimingTagStats>>,
}

/// Run payload kind.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPayloadKind {
    /// Run completed with a value payload.
    Value,
    /// Run failed with a runtime error payload.
    RuntimeError,
    /// Run payload kind is unknown.
    Unknown,
}

/// Structured payload for run results.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RunPayload {
    /// Run payload kind.
    pub kind: RunPayloadKind,
    /// JSON value payload for successful runs.
    pub value_json: Option<String>,
    /// Runtime error message payload for failed runs.
    pub message: Option<String>,
}

/// Result payload for run operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Whether command execution succeeded.
    pub success: bool,
    /// Command exit code.
    pub exit_code: i32,
    /// Diagnostics emitted by this run.
    pub diagnostics: Vec<Diagnostic>,
    /// Number of diagnostics.
    pub diagnostic_count: u32,
    /// Whether diagnostics include at least one error.
    pub has_errors: bool,
    /// Whether diagnostics include at least one warning.
    pub has_warnings: bool,
    /// Output chunks captured during execution.
    pub output: Vec<RunOutputChunk>,
    /// Structured run payload data when available.
    pub payload: Option<RunPayload>,
    /// Number of modules involved.
    pub module_count: u32,
    /// Number of profiles involved.
    pub profile_count: u32,
    /// Number of targets involved.
    pub target_count: u32,
    /// Command stats when available.
    pub stats: Option<RunStats>,
}

/// Get the default run options.
#[napi(js_name = "defaultRunOptions")]
pub fn default_run_options() -> RunOptions {
    RunOptions::default()
}

/// Run a program synchronously.
#[napi(js_name = "runSync")]
pub fn run_sync(input: RunInput, options: Option<RunOptions>) -> napi::Result<RunResult> {
    let options = options.unwrap_or_default();
    let cwd = PathBuf::from(&options.cwd);
    let roots = run_roots_from_options(&options, &cwd);
    let daemon_root = roots.first().cloned().unwrap_or_else(|| cwd.clone());
    let compiler_options = options.compiler.clone();
    let command_input = command_input_from_run_input(input, &cwd)?;
    let command_options = common_command_options_from_run_options(&options, command_input)?;
    let command_payload = daemon::CommandPayload::Run(daemon::CommandRunOptions {
        entry: Some(options.entry),
        args: options.args,
        run_mode: options.mode.into(),
    });

    let session = Arc::new(workspace::Session::new(daemon_root.clone()));
    let daemon = daemon::Daemon::with_options(session, compiler_options.into());
    let response = daemon
        .run_command(&daemon_root, &command_options, &command_payload)
        .map_err(napi_error_from_run)?;

    run_result_from_command_response(response)
}

fn run_roots_from_options(options: &RunOptions, cwd: &Path) -> Vec<PathBuf> {
    if options.roots.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        options.roots.iter().map(PathBuf::from).collect()
    }
}

fn command_input_from_run_input(input: RunInput, cwd: &Path) -> napi::Result<daemon::CommandInput> {
    match input.kind {
        RunInputKind::File => {
            let path = input
                .path
                .ok_or_else(|| Error::from_reason("file input requires path".to_string()))?;
            let path = PathBuf::from(path);
            let path = if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            };
            Ok(daemon::CommandInput::File { path })
        }
        RunInputKind::Inline => {
            let name = input.name.unwrap_or_else(|| "<eval>.ds".to_string());
            let content = input
                .content
                .ok_or_else(|| Error::from_reason("inline input requires content".to_string()))?;
            let file_type = run_input_file_type_from_name(&name, input.file_type);
            Ok(daemon::CommandInput::Inline {
                name,
                content,
                file_type,
            })
        }
        RunInputKind::Stdin => {
            let name = input.name.unwrap_or_else(|| "<stdin>.ds".to_string());
            let content = input
                .content
                .ok_or_else(|| Error::from_reason("stdin input requires content".to_string()))?;
            let file_type = run_input_file_type_from_name(&name, input.file_type);
            Ok(daemon::CommandInput::Stdin {
                name,
                content,
                file_type,
            })
        }
    }
}

fn run_input_file_type_from_name(name: &str, file_type: Option<FileType>) -> source::FileType {
    if let Some(file_type) = file_type {
        return file_type.into();
    }

    source::FileType::from_path(Path::new(name)).unwrap_or(source::FileType::Destack)
}

fn common_command_options_from_run_options(
    options: &RunOptions,
    input: daemon::CommandInput,
) -> napi::Result<daemon::CommonCommandOptions> {
    let target_overrides = parse_target_overrides_json(options.target_overrides_json.as_deref())?;
    let runtime_overrides =
        parse_runtime_overrides_json(options.runtime_overrides_json.as_deref())?;
    let cache_dir = options.cache_dir.as_ref().map(PathBuf::from);
    let config_path = options.config_path.as_ref().map(PathBuf::from);
    let profile = options.profile.clone();
    let env = options
        .env
        .iter()
        .map(|entry| daemon::CommandEnvVar {
            key: entry.key.clone(),
            value: entry.value.clone(),
        })
        .collect();
    let overrides = options
        .overrides
        .iter()
        .map(run_config_override_to_core)
        .collect::<napi::Result<Vec<_>>>()?;

    Ok(daemon::CommonCommandOptions {
        inputs: vec![input],
        allow_dsconfig_fallback: options.allow_dsconfig_fallback,
        cache_dir,
        config_path,
        target: options.target.clone(),
        target_overrides,
        runtime_overrides,
        profile,
        diagnostic: Some(options.compiler.diagnostic.clone().into()),
        env,
        overrides,
        watch: false,
        dry_run: options.dry_run,
    })
}

fn run_config_override_to_core(
    override_entry: &RunConfigOverride,
) -> napi::Result<daemon::ConfigOverride> {
    let value = serde_json::from_str(&override_entry.value_json).map_err(|error| {
        Error::from_reason(format!(
            "invalid config override json for '{}': {error}",
            override_entry.path
        ))
    })?;
    Ok(daemon::ConfigOverride {
        path: override_entry.path.clone(),
        value,
    })
}

fn parse_target_overrides_json(
    input: Option<&str>,
) -> napi::Result<Option<daemon::CommandTargetOverrides>> {
    input
        .map(|input| {
            serde_json::from_str(input).map_err(|error| {
                Error::from_reason(format!("invalid target overrides json: {error}"))
            })
        })
        .transpose()
}

fn parse_runtime_overrides_json(
    input: Option<&str>,
) -> napi::Result<Option<workspace::DsConfigRuntimeOptionsJson>> {
    input
        .map(|input| {
            serde_json::from_str(input).map_err(|error| {
                Error::from_reason(format!("invalid runtime overrides json: {error}"))
            })
        })
        .transpose()
}

fn run_result_from_command_response(
    response: daemon::DaemonCommandResult,
) -> napi::Result<RunResult> {
    let diagnostics = run_diagnostics_from_collection(&response.diagnostics);
    let payload = response
        .data
        .as_ref()
        .map(run_payload_from_value)
        .transpose()?;

    let has_errors = diagnostics_have_errors(&diagnostics);
    let has_warnings = diagnostics_have_warnings(&diagnostics);

    Ok(RunResult {
        success: response.success,
        exit_code: response.exit_code,
        diagnostic_count: diagnostics.len() as u32,
        has_errors,
        has_warnings,
        diagnostics,
        output: response
            .output
            .into_iter()
            .map(run_output_chunk_binding)
            .collect(),
        payload,
        module_count: saturating_u32_from_usize(response.module_count),
        profile_count: saturating_u32_from_usize(response.profile_count),
        target_count: saturating_u32_from_usize(response.target_count),
        stats: response.stats.as_ref().map(run_stats_binding),
    })
}

fn run_diagnostics_from_collection(diagnostics: &[source::Diagnostic]) -> Vec<Diagnostic> {
    diagnostics.iter().cloned().map(Into::into).collect()
}

fn run_output_chunk_binding(chunk: daemon::DaemonCommandOutputChunk) -> RunOutputChunk {
    RunOutputChunk {
        stream: chunk.stream.into(),
        bytes: chunk.bytes,
    }
}

fn run_payload_from_value(payload: &serde_json::Value) -> napi::Result<RunPayload> {
    let payload_result = serde_json::from_value::<daemon::CommandRunPayload>(payload.clone());

    if let Ok(payload) = payload_result {
        return Ok(match payload {
            daemon::CommandRunPayload::Value { value } => RunPayload {
                kind: RunPayloadKind::Value,
                value_json: Some(value.to_string()),
                message: None,
            },
            daemon::CommandRunPayload::RuntimeError { message } => RunPayload {
                kind: RunPayloadKind::RuntimeError,
                value_json: None,
                message: Some(message),
            },
        });
    }

    Ok(RunPayload {
        kind: RunPayloadKind::Unknown,
        value_json: Some(payload.to_string()),
        message: Some("unrecognized run payload shape".to_string()),
    })
}

fn run_stats_binding(stats: &daemon::DaemonCommandStats) -> RunStats {
    RunStats {
        elapsed_ms: saturating_u32_from_u64(stats.elapsed_ms),
        tasks_completed: saturating_u32_from_u64(stats.tasks_completed),
        tasks_failed: saturating_u32_from_u64(stats.tasks_failed),
        tasks_skipped: saturating_u32_from_u64(stats.tasks_skipped),
        modules_processed: saturating_u32_from_u64(stats.modules_processed),
        lines_processed: saturating_u32_from_u64(stats.lines_processed),
        slow_tasks: saturating_u32_from_u64(stats.slow_tasks),
        cache: stats.cache.as_ref().map(|cache| RunCacheStats {
            hits_memory: saturating_u32_from_u64(cache.hits_memory),
            hits_disk: saturating_u32_from_u64(cache.hits_disk),
            misses: saturating_u32_from_u64(cache.misses),
            writes_memory: saturating_u32_from_u64(cache.writes_memory),
            writes_disk: saturating_u32_from_u64(cache.writes_disk),
            errors: saturating_u32_from_u64(cache.errors),
            hit_rate: cache.hit_rate.into(),
        }),
        timings: stats.timings.as_ref().map(|timings| {
            timings
                .iter()
                .map(|entry| RunTimingTagStats {
                    name: entry.name.clone(),
                    duration_ms: saturating_u32_from_u64(entry.duration_ms),
                    sample_count: saturating_u32_from_u64(entry.sample_count),
                })
                .collect()
        }),
    }
}

fn saturating_u32_from_usize(value: usize) -> u32 {
    if value > u32::MAX as usize {
        u32::MAX
    } else {
        value as u32
    }
}

fn saturating_u32_from_u64(value: u64) -> u32 {
    if value > u64::from(u32::MAX) {
        u32::MAX
    } else {
        value as u32
    }
}

fn napi_error_from_run(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}
