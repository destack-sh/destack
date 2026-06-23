use std::path::PathBuf;
use std::time::Duration;

use destack_serde::Schema;

use destack_repository::{Revision, Target};
use destack_source::{FileType, TargetId};
use serde::{Deserialize, Serialize};

use super::DEFAULT_PROGRESS_INTERVAL;

/// Command input sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub enum CommandInput {
    /// A file path input.
    File { path: PathBuf },
    /// Inline source input.
    Inline {
        /// Input label.
        name: String,
        /// Inline content.
        content: String,
        /// Explicit file type.
        file_type: FileType,
    },
    /// Stdin source input.
    Stdin {
        /// Input label.
        name: String,
        /// Stdin content.
        content: String,
        /// Explicit file type.
        file_type: FileType,
    },
}

/// Target overrides for command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema, Default)]
pub struct CommandTargetOverrides {
    /// Output directory override.
    pub out_dir: Option<PathBuf>,
    /// Output file override.
    pub out_file: Option<PathBuf>,
}

impl CommandTargetOverrides {
    /// Return true when overrides contain any values.
    pub fn is_empty(&self) -> bool {
        self.out_dir.is_none() && self.out_file.is_none()
    }

    /// Apply overrides to a target.
    pub fn apply_to_target(&self, target: &mut Target) {
        if let Some(out_dir) = self.out_dir.as_ref() {
            target.output.directory = out_dir.clone();
        }

        if let Some(out_file) = self.out_file.as_ref() {
            target.output.file = Some(out_file.clone());
        }
    }
}

/// Revision selection for command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema, Default)]
pub enum CommandRevision {
    /// Execute from the current root revision.
    #[default]
    Current,
    /// Execute only if the root is still at this revision.
    Exact(Revision),
}

/// Environment variable override for commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct CommandEnvVar {
    /// Environment variable name.
    pub key: String,
    /// Environment variable value.
    pub value: String,
}

/// Manifest override applied to one command invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct ManifestOverride {
    /// Manifest path, such as `compiler.target`.
    pub path: String,
    /// Override payload value.
    pub value: JsonValue,
}

impl ManifestOverride {
    /// Convert this override into a repository manifest override.
    pub fn to_repository(&self) -> Result<destack_repository::ManifestOverride, JsonValueError> {
        Ok(destack_repository::ManifestOverride {
            path: self.path.clone(),
            value: self.value.clone().into_json()?,
        })
    }
}

impl From<destack_repository::ManifestOverride> for ManifestOverride {
    /// Convert a repository manifest override into a command manifest override.
    fn from(value: destack_repository::ManifestOverride) -> Self {
        Self {
            path: value.path,
            value: JsonValue::from_json(value.value),
        }
    }
}

/// JSON-compatible value carried by command protocol messages.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Schema)]
pub enum JsonValue {
    /// Null value.
    #[default]
    Null,
    /// Boolean value.
    Bool(bool),
    /// Signed integer value.
    I64(i64),
    /// Unsigned integer value.
    U64(u64),
    /// Floating point value.
    F64(f64),
    /// String value.
    String(String),
    /// Array value.
    Array(Vec<JsonValue>),
    /// Object entries in source order.
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// Convert one JSON value into a command JSON value.
    pub fn from_json(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(value),
            serde_json::Value::Number(value) => json_number(value),
            serde_json::Value::String(value) => Self::String(value),
            serde_json::Value::Array(values) => {
                Self::Array(values.into_iter().map(Self::from_json).collect())
            }
            serde_json::Value::Object(values) => Self::Object(
                values
                    .into_iter()
                    .map(|(key, value)| (key, Self::from_json(value)))
                    .collect(),
            ),
        }
    }

    /// Convert this command JSON value into a JSON value.
    pub fn into_json(self) -> Result<serde_json::Value, JsonValueError> {
        match self {
            Self::Null => Ok(serde_json::Value::Null),
            Self::Bool(value) => Ok(serde_json::Value::Bool(value)),
            Self::I64(value) => Ok(serde_json::Value::Number(value.into())),
            Self::U64(value) => Ok(serde_json::Value::Number(value.into())),
            Self::F64(value) => serde_json::Number::from_f64(value)
                .map(serde_json::Value::Number)
                .ok_or(JsonValueError::NonFiniteFloat),
            Self::String(value) => Ok(serde_json::Value::String(value)),
            Self::Array(values) => {
                let values = values
                    .into_iter()
                    .map(Self::into_json)
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(serde_json::Value::Array(values))
            }
            Self::Object(values) => {
                let values = values
                    .into_iter()
                    .map(|(key, value)| value.into_json().map(|value| (key, value)))
                    .collect::<Result<serde_json::Map<_, _>, _>>()?;

                Ok(serde_json::Value::Object(values))
            }
        }
    }
}

impl From<serde_json::Value> for JsonValue {
    /// Convert a JSON value into a command JSON value.
    fn from(value: serde_json::Value) -> Self {
        Self::from_json(value)
    }
}

/// Errors produced while converting command JSON values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonValueError {
    /// JSON cannot represent a non-finite floating point value.
    NonFiniteFloat,
}

impl std::fmt::Display for JsonValueError {
    /// Format a command JSON value error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteFloat => write!(formatter, "json value contains a non-finite float"),
        }
    }
}

impl std::error::Error for JsonValueError {}

/// Standard payload for unimplemented command responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct CommandMessagePayload {
    /// Message describing the command response.
    pub message: String,
    /// Whether the command is implemented.
    pub implemented: bool,
}

/// Output chunk from command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct CommandOutputChunk {
    /// Output stream kind.
    pub stream: OutputStream,
    /// Output bytes.
    pub bytes: Vec<u8>,
}

/// Generated output file produced by a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct CommandOutputFile {
    /// Output id.
    pub id: u64,
    /// Target id.
    pub target: TargetId,
    /// File type for the output.
    pub file_type: FileType,
    /// Output path for the generated file.
    pub path: PathBuf,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Optional content hash.
    pub content_hash: Option<u64>,
}

/// Command output stream kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub enum OutputStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// Convert one JSON number into a command JSON value.
fn json_number(value: serde_json::Number) -> JsonValue {
    if let Some(value) = value.as_i64() {
        JsonValue::I64(value)
    } else if let Some(value) = value.as_u64() {
        JsonValue::U64(value)
    } else if let Some(value) = value.as_f64() {
        JsonValue::F64(value)
    } else {
        unreachable!("serde_json numbers are signed, unsigned, or floating point")
    }
}

/// Progress event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct ProgressEvent {
    /// Identifier for the ongoing task.
    pub task: String,
    /// Stage message for the task.
    pub message: Option<String>,
    /// Optional progress percent between 0 and 100.
    pub percent: Option<u8>,
    /// Whether this event signals completion.
    pub done: bool,
}

/// Progress callback and delivery policy for one command execution.
#[derive(Clone, Copy)]
pub struct CommandProgress<'a> {
    /// Progress callback.
    notify: &'a (dyn Fn(ProgressEvent) + Sync),
    /// Minimum interval between throttled progress events.
    interval: Duration,
}

impl std::fmt::Debug for CommandProgress<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CommandProgress")
            .field("interval", &self.interval)
            .finish_non_exhaustive()
    }
}

impl<'a> CommandProgress<'a> {
    /// Create command progress with the default delivery policy.
    pub fn new(notify: &'a (dyn Fn(ProgressEvent) + Sync)) -> Self {
        Self {
            notify,
            interval: DEFAULT_PROGRESS_INTERVAL,
        }
    }

    /// Return progress with a custom throttle interval.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Emit one progress event.
    pub fn emit(&self, event: ProgressEvent) {
        (self.notify)(event);
    }

    /// Return the progress throttle interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }
}

/// Common command options shared across command payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct CommandOptions {
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
}

impl Default for CommandOptions {
    /// Return command options for a root-level command.
    fn default() -> Self {
        Self {
            inputs: Vec::new(),
            config_inputs: true,
            cwd: None,
            manifest: None,
            target: None,
            target_overrides: None,
            profile: None,
            env: Vec::new(),
            overrides: Vec::new(),
            watch: false,
            dry_run: false,
        }
    }
}

/// Implement command option extraction for one flat command input.
macro_rules! impl_command_input_options {
    ($input:ty { $($field:ident: $value:expr),* $(,)? }) => {
        impl $input {
            /// Rebuild shared command options from this command input.
            pub fn command_options(&self) -> CommandOptions {
                CommandOptions {
                    inputs: self.inputs.clone(),
                    config_inputs: self.config_inputs,
                    cwd: self.cwd.clone(),
                    manifest: self.manifest.clone(),
                    target: self.target.clone(),
                    target_overrides: self.target_overrides.clone(),
                    profile: self.profile.clone(),
                    env: self.env.clone(),
                    overrides: self.overrides.clone(),
                    watch: self.watch,
                    dry_run: self.dry_run,
                }
            }
        }

        impl From<(CommandRevision, CommandOptions)> for $input {
            fn from(value: (CommandRevision, CommandOptions)) -> Self {
                let (revision, options) = value;

                Self {
                    revision,
                    inputs: options.inputs,
                    config_inputs: options.config_inputs,
                    cwd: options.cwd,
                    manifest: options.manifest,
                    target: options.target,
                    target_overrides: options.target_overrides,
                    profile: options.profile,
                    env: options.env,
                    overrides: options.overrides,
                    watch: options.watch,
                    dry_run: options.dry_run,
                    $($field: $value,)*
                }
            }
        }

        impl Default for $input {
            fn default() -> Self {
                Self::from((CommandRevision::default(), CommandOptions::default()))
            }
        }
    };
}

pub(crate) use impl_command_input_options;
