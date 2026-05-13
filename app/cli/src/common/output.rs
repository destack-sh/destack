use std::path::PathBuf;

use clap::{Args, ValueEnum};
use destack_artifact::{EmitFormat, Platform, Runtime};
use destack_workspace::{
    DebugInfoLevel, LtoMode, OptimizeLevel, SourceMapMode, StripLevel, Target,
};

/// Emit format for CLI (maps to workspace EmitFormat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum EmitArg {
    /// JavaScript (.js).
    Js,
    /// TypeScript (.ts).
    Ts,
    /// HTML document.
    Html,
    /// WebAssembly (.wasm).
    Wasm,
    /// Native binary.
    Native,
}

impl From<EmitArg> for EmitFormat {
    fn from(kind: EmitArg) -> Self {
        match kind {
            EmitArg::Js => EmitFormat::Js,
            EmitArg::Ts => EmitFormat::Ts,
            EmitArg::Html => EmitFormat::Html,
            EmitArg::Wasm => EmitFormat::Wasm,
            EmitArg::Native => EmitFormat::Native,
        }
    }
}

/// Debug info emission for CLI (maps to workspace DebugInfoLevel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DebugInfoArg {
    /// No debug info.
    None,
    /// Line tables only.
    Line,
    /// Full debug info.
    Full,
}

impl From<DebugInfoArg> for DebugInfoLevel {
    fn from(level: DebugInfoArg) -> Self {
        match level {
            DebugInfoArg::None => DebugInfoLevel::None,
            DebugInfoArg::Line => DebugInfoLevel::Line,
            DebugInfoArg::Full => DebugInfoLevel::Full,
        }
    }
}

/// Symbol stripping for CLI (maps to workspace StripLevel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum StripArg {
    /// Keep all symbols.
    None,
    /// Strip local symbols.
    Partial,
    /// Strip all symbols.
    Full,
}

impl From<StripArg> for StripLevel {
    fn from(level: StripArg) -> Self {
        match level {
            StripArg::None => StripLevel::None,
            StripArg::Partial => StripLevel::Partial,
            StripArg::Full => StripLevel::Full,
        }
    }
}

/// Link time optimization for CLI (maps to workspace LtoMode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LtoArg {
    /// Auto select based on optimization level.
    Auto,
    /// Disable LTO.
    None,
    /// Thin LTO.
    Thin,
    /// Full LTO.
    Full,
}

impl From<LtoArg> for LtoMode {
    fn from(mode: LtoArg) -> Self {
        match mode {
            LtoArg::Auto => LtoMode::Auto,
            LtoArg::None => LtoMode::None,
            LtoArg::Thin => LtoMode::Thin,
            LtoArg::Full => LtoMode::Full,
        }
    }
}

/// Runtime environment for CLI (maps to workspace Runtime).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RuntimeArg {
    /// Web browser.
    Browser,
    /// Node.js.
    Node,
    /// Deno.
    Deno,
    /// Bun.
    Bun,
    /// Web Worker.
    Worker,
    /// WASM in JS host.
    WasmJs,
    /// WASM with WASI.
    WasmWasi,
}

impl From<RuntimeArg> for Runtime {
    fn from(kind: RuntimeArg) -> Self {
        match kind {
            RuntimeArg::Browser => Runtime::Browser,
            RuntimeArg::Node => Runtime::Node,
            RuntimeArg::Deno => Runtime::Deno,
            RuntimeArg::Bun => Runtime::Bun,
            RuntimeArg::Worker => Runtime::Worker,
            RuntimeArg::WasmJs => Runtime::WasmJs,
            RuntimeArg::WasmWasi => Runtime::WasmWasi,
        }
    }
}

/// Platform for CLI (maps to workspace Platform).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PlatformArg {
    /// Web browser.
    Web,
    /// Windows.
    Windows,
    /// macOS.
    Macos,
    /// Linux.
    Linux,
    /// WASI.
    Wasi,
    /// Universal/portable.
    Universal,
}

impl From<PlatformArg> for Platform {
    fn from(kind: PlatformArg) -> Self {
        match kind {
            PlatformArg::Web => Platform::Web,
            PlatformArg::Windows => Platform::Windows,
            PlatformArg::Macos => Platform::MacOS,
            PlatformArg::Linux => Platform::Linux,
            PlatformArg::Wasi => Platform::Wasi,
            PlatformArg::Universal => Platform::Universal,
        }
    }
}

/// Target configuration arguments for build commands.
#[derive(Args, Debug, Clone, Default)]
pub struct TargetArgs {
    /// Use a named target from destack.json.
    #[arg(long = "target", short = 't')]
    pub target: Option<String>,

    /// Emit format (js, ts, html, wasm, native). Default: js.
    #[arg(long = "emit", short = 'O')]
    pub emit: Option<EmitArg>,

    /// Runtime environment. Default: browser.
    #[arg(long)]
    pub runtime: Option<RuntimeArg>,

    /// Target platform. Default: web.
    #[arg(long)]
    pub platform: Option<PlatformArg>,

    /// Output directory.
    #[arg(long = "out-dir", short = 'o')]
    pub out_dir: Option<PathBuf>,

    /// Output file (for single-file targets like wasm).
    #[arg(long = "out-file")]
    pub out_file: Option<PathBuf>,

    /// Emit declaration files (.d.ts).
    #[arg(long)]
    pub declaration: bool,

    /// Emit source maps.
    #[arg(long = "source-map")]
    pub source_map: bool,

    /// Enable optimization.
    #[arg(long)]
    pub optimize: bool,

    /// Optimization level (0-4).
    #[arg(long = "opt-level", value_parser = clap::value_parser!(u8).range(0..=4))]
    pub opt_level: Option<u8>,

    /// Force a debug build profile (disables optimization unless overridden).
    #[arg(long, conflicts_with = "release")]
    pub debug: bool,

    /// Force a release build profile (enables optimization unless overridden).
    #[arg(long, conflicts_with = "debug")]
    pub release: bool,

    /// Debug info emission level.
    #[arg(long = "debug-info", value_enum)]
    pub debug_info: Option<DebugInfoArg>,

    /// Symbol stripping level.
    #[arg(long = "strip", value_enum)]
    pub strip: Option<StripArg>,

    /// Link time optimization mode.
    #[arg(long = "lto", value_enum)]
    pub lto: Option<LtoArg>,

    /// CPU name for native codegen.
    #[arg(long = "cpu")]
    pub cpu: Option<String>,

    /// CPU feature flags (comma-separated).
    #[arg(long = "cpu-features", value_delimiter = ',')]
    pub cpu_features: Vec<String>,

    /// Custom linker for native targets.
    #[arg(long = "linker")]
    pub linker: Option<String>,

    /// Extra linker arguments (comma-separated).
    #[arg(long = "link-arg", value_delimiter = ',')]
    pub link_args: Vec<String>,

    /// Sysroot path for native targets.
    #[arg(long = "sysroot")]
    pub sysroot: Option<PathBuf>,
}

impl TargetArgs {
    /// Check if this specifies a named target from destack.json.
    pub fn is_named_target(&self) -> bool {
        self.target.is_some()
    }

    /// Check if any ad-hoc target options are specified.
    pub fn has_adhoc_options(&self) -> bool {
        self.emit.is_some()
            || self.runtime.is_some()
            || self.platform.is_some()
            || self.cpu.is_some()
            || !self.cpu_features.is_empty()
            || self.linker.is_some()
            || !self.link_args.is_empty()
            || self.sysroot.is_some()
            || self.out_dir.is_some()
            || self.out_file.is_some()
            || self.declaration
            || self.source_map
            || self.optimize
            || self.opt_level.is_some()
            || self.debug
            || self.release
            || self.debug_info.is_some()
            || self.strip.is_some()
            || self.lto.is_some()
    }

    /// Get the target name (for named targets).
    pub fn target_name(&self) -> Option<&str> {
        self.target.as_deref()
    }

    /// Create an ad-hoc Target from CLI arguments.
    pub fn to_target(&self, name: &str) -> Target {
        let mut target = Target::js(name);
        self.apply_to_target(&mut target);
        target
    }

    /// Apply CLI arguments to an existing target.
    pub fn apply_to_target(&self, target: &mut Target) {
        // apply emit and runtime overrides
        if let Some(emit) = self.emit {
            target.emit = emit.into();
        }
        if let Some(runtime) = self.runtime {
            target.runtime = runtime.into();
            target.runtime_options.host = runtime.into();
        }
        if let Some(platform) = self.platform {
            target.platform = platform.into();
        }
        if let Some(ref cpu) = self.cpu {
            target.cpu = Some(cpu.clone());
        }
        if !self.cpu_features.is_empty() {
            target.cpu_features = self.cpu_features.clone();
        }
        if let Some(lto) = self.lto {
            target.lto_mode = lto.into();
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

        // apply output paths
        if let Some(ref out_dir) = self.out_dir {
            target.out_dir = out_dir.clone();
        }
        if let Some(ref out_file) = self.out_file {
            target.out_file = Some(out_file.clone());
        }

        // apply emission flags
        target.declaration = self.declaration;
        target.source_map_mode = self.source_map.then_some(SourceMapMode::External);

        // apply optimization settings
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
            target.optimize_level = level.into();
        }
        if let Some(debug_info) = self.debug_info {
            target.debug_info = debug_info.into();
        }
        if let Some(strip) = self.strip {
            target.strip = strip.into();
        }
    }
}

/// Build profile selection for targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildProfile {
    Debug,
    Release,
}

/// Resolve the requested build profile, if any.
fn resolve_profile(debug: bool, release: bool) -> Option<BuildProfile> {
    if debug && !release {
        return Some(BuildProfile::Debug);
    }
    if release && !debug {
        return Some(BuildProfile::Release);
    }
    None
}

/// Apply profile defaults to the target.
fn apply_profile_settings(
    target: &mut Target,
    profile: BuildProfile,
    optimize_override: bool,
    opt_level_override: Option<u8>,
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
