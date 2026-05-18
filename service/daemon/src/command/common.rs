use std::path::PathBuf;

use destack_source::FileType;
use destack_workspace::Target;
use serde::{Deserialize, Serialize};

pub use destack_workspace::ConfigPatch;

/// Command input sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
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
            target.out_dir = out_dir.clone();
        }

        if let Some(out_file) = self.out_file.as_ref() {
            target.out_file = Some(out_file.clone());
        }
    }
}

/// Environment variable override for commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandEnvVar {
    /// Environment variable name.
    pub key: String,
    /// Environment variable value.
    pub value: String,
}

/// Standard payload for unimplemented command responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandMessagePayload {
    /// Message describing the command response.
    pub message: String,
    /// Whether the command is implemented.
    pub implemented: bool,
}

/// Common command options shared across command payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommonCommandOptions {
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether Destack config fallback should resolve inputs when none are provided.
    pub allow_destack_config_fallback: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional cache directory override.
    pub cache_dir: Option<PathBuf>,
    /// Optional Destack config path override.
    pub config_path: Option<PathBuf>,
    /// Optional target name override.
    pub target: Option<String>,
    /// Optional target overrides.
    pub target_overrides: Option<CommandTargetOverrides>,
    /// Optional profile name override.
    pub profile: Option<String>,
    /// Optional environment overrides.
    pub env: Vec<CommandEnvVar>,
    /// Optional config patches.
    pub config_patches: Vec<ConfigPatch>,
    /// Whether the command should watch for changes.
    pub watch: bool,
    /// Whether the command should skip writes.
    pub dry_run: bool,
}
