use std::path::PathBuf;

use destack_artifact::{EmitFormat, Platform, Runtime};
use destack_source::FileType;
use destack_workspace::{
    DebugInfoLevel, LtoMode, OptimizeLevel, RuntimeOptionsJson, SourceMapMode, StripLevel, Target,
};
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
    /// Emit format override.
    pub emit: Option<EmitFormat>,
    /// Runtime override.
    pub runtime: Option<Runtime>,
    /// Platform override.
    pub platform: Option<Platform>,
    /// CPU name override.
    pub cpu: Option<String>,
    /// CPU feature overrides.
    pub cpu_features: Vec<String>,
    /// LTO mode override.
    pub lto: Option<LtoMode>,
    /// Custom linker override.
    pub linker: Option<String>,
    /// Extra linker arguments.
    pub link_args: Vec<String>,
    /// Sysroot override.
    pub sysroot: Option<PathBuf>,
    /// Output directory override.
    pub out_dir: Option<PathBuf>,
    /// Output file override.
    pub out_file: Option<PathBuf>,
    /// Emit declarations override.
    pub declaration: bool,
    /// Emit source maps override.
    pub source_map: bool,
    /// Enable optimization override.
    pub optimize: bool,
    /// Optimization level override.
    pub opt_level: Option<OptimizeLevel>,
    /// Force debug profile defaults.
    pub debug: bool,
    /// Force release profile defaults.
    pub release: bool,
    /// Debug info level override.
    pub debug_info: Option<DebugInfoLevel>,
    /// Strip symbols override.
    pub strip: Option<StripLevel>,
}

impl CommandTargetOverrides {
    /// Return true when overrides contain any values.
    pub fn is_empty(&self) -> bool {
        self.emit.is_none()
            && self.runtime.is_none()
            && self.platform.is_none()
            && self.cpu.is_none()
            && self.cpu_features.is_empty()
            && self.lto.is_none()
            && self.linker.is_none()
            && self.link_args.is_empty()
            && self.sysroot.is_none()
            && self.out_dir.is_none()
            && self.out_file.is_none()
            && !self.declaration
            && !self.source_map
            && !self.optimize
            && self.opt_level.is_none()
            && !self.debug
            && !self.release
            && self.debug_info.is_none()
            && self.strip.is_none()
    }

    /// Apply overrides to a target.
    pub fn apply_to_target(&self, target: &mut Target) {
        if let Some(emit) = self.emit {
            target.emit = emit;
        }
        if let Some(runtime) = self.runtime {
            target.runtime = runtime;
            target.runtime_options.host = runtime;
        }
        if let Some(platform) = self.platform {
            target.platform = platform;
        }
        if let Some(ref cpu) = self.cpu {
            target.cpu = Some(cpu.clone());
        }
        if !self.cpu_features.is_empty() {
            target.cpu_features = self.cpu_features.clone();
        }
        if let Some(lto) = self.lto {
            target.lto_mode = lto;
        }
        if let Some(ref linker) = self.linker {
            target.linker = Some(linker.clone());
        }
        if !self.link_args.is_empty() {
            target.link_args = self.link_args.clone();
        }
        if let Some(ref sysroot) = self.sysroot {
            target.sysroot = Some(sysroot.clone());
        }

        if let Some(ref out_dir) = self.out_dir {
            target.out_dir = out_dir.clone();
        }
        if let Some(ref out_file) = self.out_file {
            target.out_file = Some(out_file.clone());
        }

        target.declaration = self.declaration;
        target.source_map_mode = self.source_map.then_some(SourceMapMode::External);

        let profile = resolve_profile(self.debug, self.release);
        if let Some(profile) = profile {
            apply_profile_settings(
                target,
                profile,
                self.optimize,
                self.opt_level,
                self.debug_info.is_some(),
            );
        }
        if self.optimize {
            target.optimize = true;
        }
        if let Some(level) = self.opt_level {
            target.optimize_level = level;
        }
        if let Some(debug_info) = self.debug_info {
            target.debug_info = debug_info;
        }
        if let Some(strip) = self.strip {
            target.strip = strip;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildProfile {
    Debug,
    Release,
}

fn resolve_profile(debug: bool, release: bool) -> Option<BuildProfile> {
    if debug && !release {
        return Some(BuildProfile::Debug);
    }
    if release && !debug {
        return Some(BuildProfile::Release);
    }
    None
}

fn apply_profile_settings(
    target: &mut Target,
    profile: BuildProfile,
    optimize_override: bool,
    opt_level_override: Option<OptimizeLevel>,
    debug_info_override: bool,
) {
    match profile {
        BuildProfile::Debug => {
            target.debug = true;
            if !optimize_override && opt_level_override.is_none() {
                target.optimize = false;
                target.optimize_level = OptimizeLevel::O0;
            }
            if !debug_info_override {
                target.debug_info = DebugInfoLevel::Full;
            }
        }
        BuildProfile::Release => {
            target.debug = false;
            if !optimize_override && opt_level_override.is_none() {
                target.optimize = true;
                target.optimize_level = OptimizeLevel::O3;
            }
            if !debug_info_override {
                target.debug_info = DebugInfoLevel::None;
            }
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
    /// Optional runtime overrides.
    pub runtime_overrides: Option<RuntimeOptionsJson>,
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
