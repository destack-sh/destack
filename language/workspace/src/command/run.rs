use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use destack_artifact::ArtifactKey;
#[cfg(not(target_arch = "wasm32"))]
use destack_program::{Program, TypeId, Value, Word, WordLayout};
use destack_repository::TraceView;
#[cfg(not(target_arch = "wasm32"))]
use destack_repository::{Environment, Profile, Repository, Revision};
#[cfg(not(target_arch = "wasm32"))]
use destack_runtime::binding::BindingTable;
#[cfg(not(target_arch = "wasm32"))]
use destack_runtime::machine::{Engine, Entry};
#[cfg(not(target_arch = "wasm32"))]
use destack_runtime::world::World;
use destack_serde::Reflect;
#[cfg(target_arch = "wasm32")]
use destack_source::DiagnosticCollection;
#[cfg(not(target_arch = "wasm32"))]
use destack_source::{ModuleId, TargetId};
#[cfg(not(target_arch = "wasm32"))]
use destack_vm::MachineLimits;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
#[cfg(not(target_arch = "wasm32"))]
use super::context::SelectedTarget;
use super::outcome::CommandOutcome;
#[cfg(not(target_arch = "wasm32"))]
use super::output::OutputBuffer;

/// Run mode for the run command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, Default)]
pub enum RunMode {
    /// Execute an entry module.
    #[default]
    Program,
    /// Evaluate inline input and optionally print the result.
    Eval { print: bool },
}

/// Options for the run command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
pub struct RunOptions {
    /// Optional run entry function name.
    pub entry: Option<String>,
    /// Optional command arguments.
    pub args: Vec<String>,
    /// Optional run mode override.
    pub run_mode: RunMode,
}

/// Request to run a workspace target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RunInput {
    /// Revision selected for this run.
    pub revision: CommandRevision,
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether destack.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional Destack manifest path override.
    pub manifest: Option<PathBuf>,
    /// Optional target name override.
    pub target: Option<String>,
    /// Optional target overrides.
    pub target_overrides: Option<CommandTargetOverrides>,
    /// Optional profile name override.
    pub profile: Option<String>,
    /// Optional environment overrides.
    pub env: Vec<CommandEnvVar>,
    /// Optional manifest overrides.
    pub overrides: Vec<ManifestOverride>,
    /// Whether the command should watch for changes.
    pub watch: bool,
    /// Whether the command should skip writes.
    pub dry_run: bool,
    /// Trace detail returned for this command.
    pub trace: Option<TraceView>,
    /// Optional run entry function name.
    pub entry: Option<String>,
    /// Optional command arguments.
    pub args: Vec<String>,
    /// Optional run mode override.
    pub run_mode: RunMode,
}

impl_command_input_options!(RunInput {
    entry: None,
    args: Vec::new(),
    run_mode: RunMode::default(),
});

/// Payload for run command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunPayload {
    /// Run completed successfully with a return value.
    Value {
        /// Returned value from the entry function.
        value: serde_json::Value,
    },
    /// Run failed with a runtime error.
    RuntimeError {
        /// Runtime error string.
        message: String,
    },
}

/// Result of executing the entry module.
#[cfg(not(target_arch = "wasm32"))]
struct RunResult {
    /// Process exit code derived from the return value.
    exit_code: i32,
    /// Whether the return value directly defines a process exit code.
    is_exit_code: bool,
    /// Text representation for eval output.
    display: String,
    /// Serialized return value.
    payload: serde_json::Value,
}

#[cfg(not(target_arch = "wasm32"))]
impl RunResult {
    /// Interpret one returned Program value for command output.
    fn new(program: &Program, value: &Value) -> CommandResult<Self> {
        let ty = value.ty();
        let words = program
            .value_words(ty, value)
            .map_err(|error| error.to_string())?;
        let layout = program.word_layout(ty);

        // derive process semantics from scalar return values
        let exit_code = Self::exit_code(layout, words);
        let is_exit_code = matches!(
            layout,
            Some(
                WordLayout::Void
                    | WordLayout::Boolean
                    | WordLayout::Int { .. }
                    | WordLayout::Uint { .. }
            )
        );

        // render the exact value for terminal and protocol consumers
        let display = Self::display(ty, layout, words)?;
        let payload = Self::payload(ty, layout, words)?;

        Ok(Self {
            exit_code,
            is_exit_code,
            display,
            payload,
        })
    }

    /// Convert one word layout into a process exit code.
    fn exit_code(layout: Option<WordLayout>, words: &[Word]) -> i32 {
        match layout {
            Some(WordLayout::Void) => 0,
            Some(WordLayout::Boolean) => i32::from(!words[0].as_boolean()),
            Some(WordLayout::Int { width }) => {
                Self::signed(words, width).clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32
            }
            Some(WordLayout::Uint { width }) => {
                Self::unsigned(words, width).min(i32::MAX as u128) as i32
            }
            _ => 0,
        }
    }

    /// Format one exact returned value for eval output.
    fn display(ty: TypeId, layout: Option<WordLayout>, words: &[Word]) -> CommandResult<String> {
        let display = match layout {
            Some(WordLayout::Void) => "void".to_string(),
            Some(WordLayout::Boolean) => words[0].as_boolean().to_string(),
            Some(WordLayout::Character) => words[0]
                .as_character()
                .ok_or_else(|| format!("invalid character value 0x{:x}", words[0].bits()))?
                .to_string(),
            Some(WordLayout::Int { width }) => Self::signed(words, width).to_string(),
            Some(WordLayout::Uint { width }) => Self::unsigned(words, width).to_string(),
            Some(WordLayout::Float16 | WordLayout::Bfloat16) => {
                format!("0x{:04x}", words[0].bits() as u16)
            }
            Some(WordLayout::Float32) => words[0].as_f32().to_string(),
            Some(WordLayout::Float64) => words[0].as_f64().to_string(),
            _ => Self::display_words(ty, words),
        };

        Ok(display)
    }

    /// Serialize one exact returned value for command protocol output.
    fn payload(
        ty: TypeId,
        layout: Option<WordLayout>,
        words: &[Word],
    ) -> CommandResult<serde_json::Value> {
        let payload = match layout {
            Some(WordLayout::Void) => serde_json::Value::Null,
            Some(WordLayout::Boolean) => serde_json::Value::Bool(words[0].as_boolean()),
            Some(WordLayout::Character) => {
                let value = words[0]
                    .as_character()
                    .ok_or_else(|| format!("invalid character value 0x{:x}", words[0].bits()))?;

                serde_json::Value::String(value.to_string())
            }
            Some(WordLayout::Int { width }) => Self::signed_payload(Self::signed(words, width)),
            Some(WordLayout::Uint { width }) => {
                Self::unsigned_payload(Self::unsigned(words, width))
            }
            Some(WordLayout::Float16 | WordLayout::Bfloat16) => {
                serde_json::Value::String(format!("0x{:04x}", words[0].bits() as u16))
            }
            Some(WordLayout::Float32) => Self::float_payload(words[0].as_f32().into()),
            Some(WordLayout::Float64) => Self::float_payload(words[0].as_f64()),
            _ => Self::words_payload(ty, words),
        };

        Ok(payload)
    }

    /// Format an aggregate or opaque value without discarding its type or words.
    fn display_words(ty: TypeId, words: &[Word]) -> String {
        let words = words
            .iter()
            .map(|word| format!("0x{:016x}", word.bits()))
            .collect::<Vec<_>>()
            .join(", ");

        format!("t{}({words})", ty.0)
    }

    /// Serialize an aggregate or opaque value without discarding its type or words.
    fn words_payload(ty: TypeId, words: &[Word]) -> serde_json::Value {
        let words = words
            .iter()
            .map(|word| format!("0x{:016x}", word.bits()))
            .collect::<Vec<_>>();

        serde_json::json!({
            "type": ty.0,
            "words": words,
        })
    }

    /// Convert one floating-point value to command JSON.
    fn float_payload(value: f64) -> serde_json::Value {
        match serde_json::Number::from_f64(value) {
            Some(value) => serde_json::Value::Number(value),
            None => serde_json::Value::String(value.to_string()),
        }
    }

    /// Decode one little-endian unsigned integer from execution words.
    fn unsigned(words: &[Word], width: u8) -> u128 {
        let low = words[0].as_u64() as u128;
        let high = if width > Word::BIT_LEN {
            words[1].as_u64() as u128
        } else {
            0
        };
        let value = low | high << Word::BIT_LEN;

        if width >= u128::BITS as u8 {
            value
        } else {
            value & ((1u128 << width) - 1)
        }
    }

    /// Decode one little-endian signed integer from execution words.
    fn signed(words: &[Word], width: u8) -> i128 {
        let value = Self::unsigned(words, width);
        if width >= i128::BITS as u8 {
            value as i128
        } else {
            let sign = 1u128 << (width - 1);
            let mask = (1u128 << width) - 1;

            if value & sign == 0 {
                value as i128
            } else {
                (value | !mask) as i128
            }
        }
    }

    /// Convert one signed integer to command JSON without losing precision.
    fn signed_payload(value: i128) -> serde_json::Value {
        match i64::try_from(value) {
            Ok(value) => serde_json::Value::Number(value.into()),
            Err(_) => serde_json::Value::String(value.to_string()),
        }
    }

    /// Convert one unsigned integer to command JSON without losing precision.
    fn unsigned_payload(value: u128) -> serde_json::Value {
        match u64::try_from(value) {
            Ok(value) => serde_json::Value::Number(value.into()),
            Err(_) => serde_json::Value::String(value.to_string()),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl CommandContext<'_> {
    /// Execute a run command.
    pub(crate) async fn execute_run_command(
        &mut self,
        input: &RunInput,
    ) -> CommandResult<CommandOutcome<Option<RunPayload>>> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision()?;
        let entry_module = modules
            .first()
            .copied()
            .ok_or_else(|| "run requires an entry module".to_string())?;

        // resolve the target configuration
        let target_overrides = self.common.target_overrides.as_ref();
        let target = self.resolve_target_for_module(revision, entry_module, target_overrides)?;

        // collect run roots
        let artifact_keys = vec![ArtifactKey::program(target.id.package_id(), target.id)];

        // provide the requested roots
        self.provide(revision, &artifact_keys).await?;
        let diagnostics = self.command_diagnostics(revision, &artifact_keys)?;
        let exit_code = diagnostics.get_status_code();
        let profile_count = self
            .target_profile_id(revision, entry_module, target.id)
            .map(|_| 1)?;
        if exit_code != 0 {
            return Ok(CommandOutcome::new(
                diagnostics,
                exit_code,
                modules.len(),
                profile_count,
                1,
            )
            .with_data(None));
        }

        // execute the entry module
        let entry_name = input.entry.clone().unwrap_or_else(|| "main".to_string());
        let run_result = match run_entry_module(
            &self.repository,
            revision,
            &inputs,
            entry_module,
            &target,
            &entry_name,
            &input.args,
            input.run_mode,
            self.output,
        ) {
            Ok(result) => result,
            Err(error) => {
                let payload = RunPayload::RuntimeError {
                    message: error.to_string(),
                };
                return Ok(
                    CommandOutcome::new(diagnostics, 1, modules.len(), profile_count, 1)
                        .with_data(Some(payload)),
                );
            }
        };

        let payload = RunPayload::Value {
            value: run_result.payload,
        };

        Ok(CommandOutcome::new(
            diagnostics,
            run_result.exit_code,
            modules.len(),
            profile_count,
            1,
        )
        .with_data(Some(payload)))
    }
}

#[cfg(target_arch = "wasm32")]
impl CommandContext<'_> {
    /// Execute a run command.
    pub(crate) async fn execute_run_command(
        &mut self,
        _input: &RunInput,
    ) -> CommandResult<CommandOutcome<Option<RunPayload>>> {
        Ok(CommandOutcome::new(DiagnosticCollection::default(), 1, 0, 0, 0).with_data(None))
    }
}

/// Execute the entry module in the VM.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
fn run_entry_module(
    repository: &Arc<Repository>,
    revision: Revision,
    inputs: &[CommandInput],
    entry_module: ModuleId,
    target: &SelectedTarget,
    entry_name: &str,
    args: &[String],
    run_mode: RunMode,
    output: &mut OutputBuffer,
) -> CommandResult<RunResult> {
    // resolve the runtime profile
    let target_id = target.id;
    let profile = target_profile(repository, revision, entry_module, target_id)?;
    let runtime_options = target.target.execution.clone();
    let conditions = profile.conditions().clone();

    // load the linked Program
    let artifacts = repository.artifact_reader(revision);
    let program = artifacts
        .read::<Program>((target_id.package_id(), target_id))
        .map_err(|error| error.to_string())?;

    // runtime launch
    let entry_source = inputs
        .first()
        .ok_or_else(|| "run requires an entry module".to_string())?;
    let environment = environment_for_source(entry_source, args);
    let mut world =
        World::new(&runtime_options, environment.clone()).map_err(|error| format!("{error}"))?;
    let binding_table = Arc::new(BindingTable::new().with_fiber_bindings());
    let engine = Engine::new(program.clone(), MachineLimits::default());
    let runtime_id = world
        .spawn_runtime(
            environment,
            &runtime_options,
            conditions,
            binding_table,
            engine,
        )
        .map_err(|error| format!("{error}"))?;

    let entry = Entry::new(entry_name);
    let value = world
        .run_entrypoint(runtime_id, &entry, &[])
        .map_err(|error| format!("{error}"))?;
    let result = RunResult::new(&program, &value)?;

    if matches!(run_mode, RunMode::Program) && !result.is_exit_code {
        output.push_stderr(b"non-integer return value, defaulting to exit code 0\n".to_vec());
    }

    if matches!(run_mode, RunMode::Eval { print: true }) {
        output.push_stdout(format!("{}\n", result.display).into_bytes());
    }

    if result.exit_code != 0 {
        output.push_stderr(format!("process exited with code {}\n", result.exit_code).into_bytes());
    }

    Ok(result)
}

/// Build the launch environment for the entry source.
#[cfg(not(target_arch = "wasm32"))]
fn environment_for_source(source: &CommandInput, args: &[String]) -> Environment {
    let mut arguments = Vec::with_capacity(args.len().saturating_add(1));
    arguments.push(command_input_display_name(source));
    arguments.extend(args.iter().cloned());
    let mut environment = Environment::capture_process();
    environment.args = arguments;

    environment
}

/// Get a display name for the entry source.
#[cfg(not(target_arch = "wasm32"))]
fn command_input_display_name(source: &CommandInput) -> String {
    match source {
        CommandInput::File { path } => path.to_string_lossy().into_owned(),
        CommandInput::Inline { name, .. } => name.clone(),
        CommandInput::Stdin { name, .. } => name.clone(),
    }
}

/// Return the profile selected for one module target.
#[cfg(not(target_arch = "wasm32"))]
fn target_profile(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    target_id: TargetId,
) -> Result<Arc<Profile>, String> {
    let profile = repository
        .profile_for_module_target(revision, module_id, target_id)
        .map_err(|error| format!("failed to resolve target profile: {error}"))?;

    Ok(profile)
}
