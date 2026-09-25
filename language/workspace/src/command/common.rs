use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use futures::channel::mpsc::{Receiver, Sender, channel};
use parking_lot::Mutex;
use tspp_serde::{Reflect, Value, ValueError};
use tspp_session::{SessionEvent, SessionEventHandler};

use serde::{Deserialize, Serialize};
use tspp_repository::{Revision, Target, TraceView};
use tspp_source::{FileType, TargetId};

use super::DEFAULT_PROGRESS_INTERVAL;

/// Maximum pending progress event count for one command.
const PROGRESS_CAPACITY: usize = 1;

/// Command input sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
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
            target.destination.directory = out_dir.clone();
        }

        if let Some(out_file) = self.out_file.as_ref() {
            target.destination.file = Some(out_file.clone());
        }
    }
}

/// Revision selection for command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, Default)]
pub enum CommandRevision {
    /// Execute from the current physical workspace revision.
    #[default]
    Current,
    /// Execute only if physical workspace state remains at this revision.
    Exact(Revision),
}

/// Environment variable override for commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CommandEnvVar {
    /// Environment variable name.
    pub key: String,
    /// Environment variable value.
    pub value: String,
}

/// Manifest override applied to one command invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ManifestOverride {
    /// Manifest path, such as `compiler.target`.
    pub path: String,
    /// Override payload value.
    pub value: Value,
}

impl ManifestOverride {
    /// Convert this override into a repository manifest override.
    pub fn to_repository(&self) -> Result<tspp_repository::ManifestOverride, ValueError> {
        Ok(tspp_repository::ManifestOverride {
            path: self.path.clone(),
            value: self.value.clone().into_json()?,
        })
    }
}

impl From<tspp_repository::ManifestOverride> for ManifestOverride {
    /// Convert a repository manifest override into a command manifest override.
    fn from(value: tspp_repository::ManifestOverride) -> Self {
        Self {
            path: value.path,
            value: Value::from(value.value),
        }
    }
}

/// Standard payload for unimplemented command responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CommandMessagePayload {
    /// Message describing the command response.
    pub message: String,
    /// Whether the command is implemented.
    pub implemented: bool,
}

/// Output chunk from command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CommandOutputChunk {
    /// Output stream kind.
    pub stream: OutputStream,
    /// Output bytes.
    pub bytes: Vec<u8>,
}

/// Generated output file produced by a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum OutputStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// Progress event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ProgressEvent {
    /// Identifier for the ongoing task.
    pub task: String,
    /// Stage message for the task.
    pub message: Option<String>,
    /// Optional progress percent between 0 and 100.
    pub percent: Option<u8>,
}

/// Progress sender for one command execution.
#[derive(Clone)]
pub struct CommandProgress {
    /// Progress event sender.
    sender: Sender<ProgressEvent>,
    /// Minimum interval between throttled progress events.
    interval: Duration,
}

impl std::fmt::Debug for CommandProgress {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CommandProgress")
            .field("interval", &self.interval)
            .finish_non_exhaustive()
    }
}

impl CommandProgress {
    /// Create one progress sender and receiver with the default delivery policy.
    pub fn channel() -> (Self, ProgressEvents) {
        let (sender, receiver) = channel(PROGRESS_CAPACITY);
        let progress = Self {
            sender,
            interval: DEFAULT_PROGRESS_INTERVAL,
        };

        (progress, ProgressEvents { receiver })
    }

    /// Return progress with a custom throttle interval.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Try to queue one progress event without stalling command workers.
    pub fn try_emit(&self, event: ProgressEvent) -> bool {
        self.sender.clone().try_send(event).is_ok()
    }

    /// Return the progress throttle interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Convert this progress sender into a throttled session event handler.
    pub(crate) fn event_handler(self) -> SessionEventHandler {
        let interval = self.interval();
        let throttle = Mutex::new((None::<Instant>, 0usize));

        Arc::new(move |event| match event {
            SessionEvent::TaskFinished { artifact_key, .. }
            | SessionEvent::TaskFailed { artifact_key, .. } => {
                // count every completed artifact
                let mut throttle = throttle.lock();
                throttle.1 += 1;
                let is_due = throttle.0.is_none_or(|last| last.elapsed() >= interval);

                // emit accumulated progress when the interval elapses
                if is_due {
                    throttle.0 = Some(Instant::now());
                    let _is_queued = self.try_emit(ProgressEvent {
                        task: artifact_key.stage().name().to_string(),
                        message: Some(format!("{} artifacts", throttle.1)),
                        percent: None,
                    });
                }
            }
            _ => {}
        })
    }
}

/// Progress events received from one command execution.
#[derive(Debug)]
pub struct ProgressEvents {
    /// Progress event receiver.
    receiver: Receiver<ProgressEvent>,
}

impl ProgressEvents {
    /// Receive the next progress event until the command closes its sender.
    pub async fn receive(&mut self) -> Option<ProgressEvent> {
        self.receiver.next().await
    }
}

/// Common command options shared across command payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
    /// Trace detail returned for this command.
    pub trace: Option<TraceView>,
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
            trace: None,
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
                    trace: self.trace,
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
                    trace: options.trace,
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
