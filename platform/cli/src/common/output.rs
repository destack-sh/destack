use std::path::PathBuf;

use clap::{Args, ValueEnum};
use destack_workspace::{OutputFormat, Platform, Runtime, Target};

/// Output format for CLI (maps to workspace OutputFormat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputArg {
    /// JavaScript (.js).
    Js,
    /// TypeScript (.ts).
    Ts,
    /// WebAssembly (.wasm).
    Wasm,
    /// Native binary.
    Native,
}

impl From<OutputArg> for OutputFormat {
    fn from(kind: OutputArg) -> Self {
        match kind {
            OutputArg::Js => OutputFormat::Js,
            OutputArg::Ts => OutputFormat::Ts,
            OutputArg::Wasm => OutputFormat::Wasm,
            OutputArg::Native => OutputFormat::Native,
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
    /// iOS.
    Ios,
    /// Android.
    Android,
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
            PlatformArg::Ios => Platform::IOS,
            PlatformArg::Android => Platform::Android,
            PlatformArg::Wasi => Platform::Wasi,
            PlatformArg::Universal => Platform::Universal,
        }
    }
}

/// Target configuration arguments for build commands.
#[derive(Args, Debug, Clone, Default)]
pub struct TargetArgs {
    /// Use a named target from dsconfig.json.
    #[arg(long = "target", short = 't')]
    pub target: Option<String>,

    /// Output format (js, ts, wasm, native). Default: js.
    #[arg(long = "output", short = 'O')]
    pub output: Option<OutputArg>,

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

    /// Optimization level (0-3).
    #[arg(long = "opt-level", value_parser = clap::value_parser!(u8).range(0..=3))]
    pub opt_level: Option<u8>,
}

impl TargetArgs {
    /// Check if this specifies a named target from dsconfig.
    pub fn is_named_target(&self) -> bool {
        self.target.is_some()
    }

    /// Check if any ad-hoc target options are specified.
    pub fn has_adhoc_options(&self) -> bool {
        self.output.is_some()
            || self.runtime.is_some()
            || self.platform.is_some()
            || self.out_dir.is_some()
            || self.out_file.is_some()
            || self.declaration
            || self.source_map
            || self.optimize
            || self.opt_level.is_some()
    }

    /// Get the target name (for named targets).
    pub fn target_name(&self) -> Option<&str> {
        self.target.as_deref()
    }

    /// Create an ad-hoc Target from CLI arguments.
    pub fn to_target(&self, name: &str) -> Target {
        let mut target = Target::js(name);

        if let Some(output) = self.output {
            target.output = output.into();
        }
        if let Some(runtime) = self.runtime {
            target.runtime = runtime.into();
        }
        if let Some(platform) = self.platform {
            target.platform = platform.into();
        }
        if let Some(ref out_dir) = self.out_dir {
            target.out_dir = out_dir.clone();
        }
        if let Some(ref out_file) = self.out_file {
            target.out_file = Some(out_file.clone());
        }
        target.declaration = self.declaration;
        target.source_map = self.source_map;
        target.optimize = self.optimize;
        if let Some(level) = self.opt_level {
            target.optimize_level = level.into();
        }

        target
    }
}
