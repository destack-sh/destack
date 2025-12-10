use std::path::{Path, PathBuf};

use super::tsconfig::{EsTarget, ModuleKind};

/// How modules are discovered for a build target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TargetDiscovery {
    /// Start from entry points and follow imports.
    /// Requires `entry` to be set. Used for bundles/executables.
    Entry,
    /// Compile all files matching `include` patterns.
    /// Each file becomes a separate output. Used for libraries.
    #[default]
    Include,
}

/// Output format for a build target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OutputFormat {
    /// JavaScript (.js).
    #[default]
    Js,
    /// TypeScript (.ts).
    Ts,
    /// WebAssembly (.wasm).
    Wasm,
    /// Native binary.
    Native,
}

impl OutputFormat {
    /// Whether this format produces JavaScript output.
    pub fn is_js(&self) -> bool {
        matches!(self, Self::Js)
    }

    /// Whether this format produces TypeScript output.
    pub fn is_ts(&self) -> bool {
        matches!(self, Self::Ts)
    }

    /// Whether this format produces WebAssembly output.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::Wasm)
    }

    /// Whether this format produces native binary output.
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Native)
    }

    /// Whether this format typically produces a single output file.
    pub fn is_single_file(&self) -> bool {
        matches!(self, Self::Wasm | Self::Native)
    }
}

/// Output mode for build targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OutputMode {
    /// One output file per source file, preserving directory structure.
    /// Uses `out_dir` for the output directory.
    #[default]
    Directory,
    /// Single bundled/compiled output file.
    /// Uses `out_file` for the output path.
    File,
}

/// Optimization level for builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptimizeLevel {
    /// No optimization (O0).
    #[default]
    O0,
    /// Basic optimization (O1).
    O1,
    /// Standard optimization (O2).
    O2,
    /// Aggressive optimization (O3).
    O3,
}

impl From<u8> for OptimizeLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => Self::O0,
            1 => Self::O1,
            2 => Self::O2,
            _ => Self::O3,
        }
    }
}

impl From<OptimizeLevel> for u8 {
    fn from(level: OptimizeLevel) -> Self {
        match level {
            OptimizeLevel::O0 => 0,
            OptimizeLevel::O1 => 1,
            OptimizeLevel::O2 => 2,
            OptimizeLevel::O3 => 3,
        }
    }
}

/// Shrink level for builds (code size reduction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShrinkLevel {
    /// No shrinking (S0).
    #[default]
    S0,
    /// Basic shrinking (S1).
    S1,
    /// Standard shrinking (S2).
    S2,
    /// Aggressive shrinking (S3).
    S3,
}

impl From<u8> for ShrinkLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => Self::S0,
            1 => Self::S1,
            2 => Self::S2,
            _ => Self::S3,
        }
    }
}

impl From<ShrinkLevel> for u8 {
    fn from(level: ShrinkLevel) -> Self {
        match level {
            ShrinkLevel::S0 => 0,
            ShrinkLevel::S1 => 1,
            ShrinkLevel::S2 => 2,
            ShrinkLevel::S3 => 3,
        }
    }
}

/// Runtime environment that executes the compiled code.
///
/// This determines what APIs are available and what semantic behaviors apply.
/// For JS output, this is the JS engine/environment. For WASM, this is the WASM host.
/// For native, this is the Destack runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Runtime {
    // JS runtimes (for output=js/ts)
    /// Web browser (Chrome, Firefox, Safari, etc.)
    #[default]
    Browser,
    /// Node.js
    Node,
    /// Deno
    Deno,
    /// Bun
    Bun,
    /// Web Worker / Service Worker / Shared Worker
    Worker,
    /// Cloudflare Workers (workerd)
    Workerd,
    /// Embedded JS engine (QuickJS, Hermes, JavaScriptCore)
    Embedded,

    // WASM runtimes (for output=wasm)
    /// WASM running in a JS host (browser or Node)
    WasmJs,
    /// WASM with WASI (wasmtime, wasmer, etc.)
    WasmWasi,
    /// Standalone WASM runtime without WASI
    WasmStandalone,

    // Native runtime (for output=native)
    /// Destack native runtime
    Destack,
}

impl std::str::FromStr for Runtime {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "browser" => Ok(Self::Browser),
            "node" => Ok(Self::Node),
            "deno" => Ok(Self::Deno),
            "bun" => Ok(Self::Bun),
            "worker" => Ok(Self::Worker),
            "workerd" => Ok(Self::Workerd),
            "embedded" => Ok(Self::Embedded),
            "wasm_js" | "wasmjs" => Ok(Self::WasmJs),
            "wasm_wasi" | "wasmwasi" | "wasi" => Ok(Self::WasmWasi),
            "wasm_standalone" | "wasmstandalone" => Ok(Self::WasmStandalone),
            "destack" | "native" => Ok(Self::Destack),
            _ => Err(()),
        }
    }
}

impl Runtime {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Whether this runtime is a JS engine.
    pub fn is_js(&self) -> bool {
        matches!(
            self,
            Self::Browser
                | Self::Node
                | Self::Deno
                | Self::Bun
                | Self::Worker
                | Self::Workerd
                | Self::Embedded
        )
    }

    /// Whether this runtime is a WASM host.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::WasmJs | Self::WasmWasi | Self::WasmStandalone)
    }

    /// Whether this runtime is the Destack native runtime.
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Destack)
    }

    /// Whether this runtime runs in a browser-like environment (has DOM potential).
    pub fn is_browser_like(&self) -> bool {
        matches!(self, Self::Browser | Self::WasmJs)
    }

    /// Whether this runtime is server-side.
    pub fn is_server(&self) -> bool {
        matches!(
            self,
            Self::Node | Self::Deno | Self::Bun | Self::WasmWasi | Self::Destack
        )
    }
}

/// Operating system / target platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Platform {
    // Web
    /// Web browser
    #[default]
    Web,

    // Desktop
    /// Windows
    Windows,
    /// macOS
    MacOS,
    /// Linux
    Linux,

    // Mobile
    /// iOS
    IOS,
    /// Android
    Android,

    // Other
    /// WASI (WebAssembly System Interface)
    Wasi,
    /// Unknown or portable (no platform-specific APIs)
    Universal,
}

impl std::str::FromStr for Platform {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "web" | "browser" => Ok(Self::Web),
            "windows" | "win32" | "win" => Ok(Self::Windows),
            "macos" | "darwin" | "mac" => Ok(Self::MacOS),
            "linux" => Ok(Self::Linux),
            "ios" => Ok(Self::IOS),
            "android" => Ok(Self::Android),
            "wasi" => Ok(Self::Wasi),
            "universal" | "portable" | "any" => Ok(Self::Universal),
            _ => Err(()),
        }
    }
}

impl Platform {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Whether this platform is a web platform.
    pub fn is_web(&self) -> bool {
        matches!(self, Self::Web)
    }

    /// Whether this is a desktop platform.
    pub fn is_desktop(&self) -> bool {
        matches!(self, Self::Windows | Self::MacOS | Self::Linux)
    }

    /// Whether this is a mobile platform.
    pub fn is_mobile(&self) -> bool {
        matches!(self, Self::IOS | Self::Android)
    }
}

/// Default output directory for targets.
pub const DEFAULT_OUT_DIR: &str = "dist";

/// A build target configuration.
///
/// Can be constructed from dsconfig.json or programmatically.
/// This is the type used by compiler/codegen - independent of dsconfig parsing.
#[derive(Debug, Clone)]
pub struct Target {
    /// Target name (e.g., "npm", "wasm", "dev").
    pub name: String,

    // discovery
    /// How modules are discovered for this target.
    pub discovery: TargetDiscovery,
    /// Entry points for entry-based discovery (bundled/executable targets).
    pub entry: Vec<PathBuf>,
    /// Glob patterns for files to include (for include-based discovery).
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,

    // output generation
    /// Module format (esnext, commonjs, etc.).
    pub module: ModuleKind,
    /// ECMAScript target version.
    pub es_target: EsTarget,
    /// Library files for this target. If `None`, derived automatically from runtime and platform.
    pub lib: Option<Vec<String>>,
    /// Output format (js, ts, wasm, native).
    pub output: OutputFormat,
    /// Runtime environment (browser, node, wasm-wasi, destack, etc.).
    pub runtime: Runtime,
    /// Target platform (web, windows, macos, linux, ios, android, etc.).
    pub platform: Platform,
    /// Emit declaration files (.d.ts) alongside JS output.
    pub declaration: bool,
    /// Emit source maps.
    pub source_map: bool,

    // optimization
    /// Whether this is a debug build.
    pub debug: bool,
    /// Whether optimization is enabled.
    pub optimize: bool,
    /// Optimization level.
    pub optimize_level: OptimizeLevel,
    /// Shrink level (code size reduction).
    pub shrink_level: ShrinkLevel,

    // output paths
    /// Output directory for this target (relative to package, defaults to "dist").
    pub out_dir: PathBuf,
    /// Output file for single-file targets.
    pub out_file: Option<PathBuf>,
    /// Separate directory for declaration files.
    pub declaration_dir: Option<PathBuf>,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            name: String::new(),

            // discovery
            discovery: TargetDiscovery::default(),
            entry: Vec::new(),
            include: Vec::new(),
            exclude: Vec::new(),

            // output generation
            module: ModuleKind::default(),
            es_target: EsTarget::default(),
            lib: None,
            output: OutputFormat::default(),
            runtime: Runtime::default(),
            platform: Platform::default(),
            declaration: false,
            source_map: false,

            // optimization
            debug: true,
            optimize: false,
            optimize_level: OptimizeLevel::O0,
            shrink_level: ShrinkLevel::S0,

            // output paths
            out_dir: PathBuf::from(DEFAULT_OUT_DIR),
            out_file: None,
            declaration_dir: None,
        }
    }
}

impl Target {
    /// Create a new target with the given name and default JS output for browser.
    pub fn js(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Js,
            runtime: Runtime::Browser,
            platform: Platform::Web,
            declaration: true, // default to emitting declarations for JS
            ..Default::default()
        }
    }

    /// Create a new target with the given name and TypeScript output.
    pub fn ts(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Ts,
            runtime: Runtime::Browser,
            platform: Platform::Web,
            ..Default::default()
        }
    }

    /// Create a new target with the given name and JS output for Node.js.
    pub fn node(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Js,
            runtime: Runtime::Node,
            platform: Platform::Universal,
            declaration: true,
            ..Default::default()
        }
    }

    /// Create a new target with the given name and WASM output for JS host.
    pub fn wasm(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Wasm,
            runtime: Runtime::WasmJs,
            platform: Platform::Web,
            optimize: true,
            ..Default::default()
        }
    }

    /// Create a new target with the given name and WASM output for WASI.
    pub fn wasm_wasi(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Wasm,
            runtime: Runtime::WasmWasi,
            platform: Platform::Wasi,
            optimize: true,
            ..Default::default()
        }
    }

    /// Create a new target with the given name and native output.
    pub fn native(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Native,
            runtime: Runtime::Destack,
            platform: Platform::Universal,
            optimize: true,
            ..Default::default()
        }
    }

    /// Derive the output mode from the target configuration.
    pub fn output_mode(&self) -> OutputMode {
        if self.out_file.is_some() || self.output.is_single_file() {
            OutputMode::File
        } else {
            OutputMode::Directory
        }
    }

    /// Whether this target produces single-file output.
    pub fn is_single_file(&self) -> bool {
        self.output_mode() == OutputMode::File
    }

    /// Whether this target produces directory output (one file per source file).
    pub fn is_directory(&self) -> bool {
        self.output_mode() == OutputMode::Directory
    }

    /// Set the output directory.
    pub fn with_out_dir(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.out_dir = out_dir.into();
        self
    }

    /// Set the output file.
    pub fn with_out_file(mut self, out_file: impl Into<PathBuf>) -> Self {
        self.out_file = Some(out_file.into());
        self
    }

    /// Set whether to emit declarations.
    pub fn with_declaration(mut self, declaration: bool) -> Self {
        self.declaration = declaration;
        self
    }

    /// Set whether to emit source maps.
    pub fn with_source_map(mut self, source_map: bool) -> Self {
        self.source_map = source_map;
        self
    }

    /// Set the module format.
    pub fn with_module(mut self, module: ModuleKind) -> Self {
        self.module = module;
        self
    }

    /// Set the ES target.
    pub fn with_es_target(mut self, es_target: EsTarget) -> Self {
        self.es_target = es_target;
        self
    }

    /// Set library files explicitly (overrides automatic derivation).
    pub fn with_lib(mut self, lib: Vec<String>) -> Self {
        self.lib = Some(lib);
        self
    }

    /// Set the runtime.
    pub fn with_runtime(mut self, runtime: Runtime) -> Self {
        self.runtime = runtime;
        self
    }

    /// Set the platform.
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    /// Derive library files from runtime and platform.
    /// If `lib` is explicitly set, returns it. Otherwise derives from runtime and platform:
    /// - ES version comes from runtime capabilities
    /// - Runtime-specific libs (dom, node, deno, worker, etc.)
    /// - Platform-specific libs for native targets (darwin, windows, linux)
    pub fn derived_lib(&self) -> Vec<String> {
        if let Some(lib) = &self.lib {
            return lib.clone();
        }

        let mut libs = Vec::new();

        // ES version based on runtime
        let es_lib = match self.runtime {
            // modern JS runtimes support esnext
            Runtime::Browser | Runtime::Node | Runtime::Deno | Runtime::Bun => "esnext",
            // workers typically support modern ES
            Runtime::Worker | Runtime::Workerd => "es2022",
            // embedded engines may be more limited
            Runtime::Embedded => "es2020",
            // WASM environments
            Runtime::WasmJs => "es2020",
            Runtime::WasmWasi | Runtime::WasmStandalone => "es2020",
            // native runtime supports full ES semantics
            Runtime::Destack => "esnext",
        };
        libs.push(es_lib.to_string());

        // runtime-specific libs
        match self.runtime {
            Runtime::Browser => {
                libs.push("dom".to_string());
                libs.push("dom.iterable".to_string());
            }
            Runtime::Node => {
                libs.push("node".to_string());
            }
            Runtime::Deno => {
                libs.push("deno".to_string());
            }
            Runtime::Bun => {
                libs.push("bun".to_string());
                libs.push("node".to_string());
            }
            Runtime::Worker | Runtime::Workerd => {
                libs.push("worker".to_string());
            }
            Runtime::WasmJs => {
                // WASM in browser may have limited DOM access
            }
            Runtime::WasmWasi => {
                libs.push("wasi".to_string());
            }
            Runtime::WasmStandalone | Runtime::Embedded => {
                // minimal environment
            }
            Runtime::Destack => {
                libs.push("destack".to_string());
            }
        }

        // platform-specific libs (primarily for native targets)
        if self.runtime.is_native() {
            match self.platform {
                Platform::MacOS | Platform::IOS => {
                    libs.push("darwin".to_string());
                }
                Platform::Windows => {
                    libs.push("windows".to_string());
                }
                Platform::Linux | Platform::Android => {
                    libs.push("linux".to_string());
                }
                _ => {}
            }
        }

        libs
    }

    /// Set whether optimization is enabled.
    pub fn with_optimize(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self
    }

    /// Set the optimization level.
    pub fn with_optimize_level(mut self, level: OptimizeLevel) -> Self {
        self.optimize_level = level;
        self
    }

    /// Set the shrink level.
    pub fn with_shrink_level(mut self, level: ShrinkLevel) -> Self {
        self.shrink_level = level;
        self
    }

    /// Set include patterns.
    pub fn with_include(mut self, include: Vec<String>) -> Self {
        self.include = include;
        self
    }

    /// Set exclude patterns.
    pub fn with_exclude(mut self, exclude: Vec<String>) -> Self {
        self.exclude = exclude;
        self
    }

    /// Set the discovery mode.
    pub fn with_discovery(mut self, discovery: TargetDiscovery) -> Self {
        self.discovery = discovery;
        self
    }

    /// Set entry points (also sets discovery mode to Entry).
    pub fn with_entry(mut self, entry: Vec<PathBuf>) -> Self {
        self.entry = entry;
        if !self.entry.is_empty() {
            self.discovery = TargetDiscovery::Entry;
        }
        self
    }

    /// Resolve the absolute output directory for this target.
    ///
    /// If out_dir is relative, it's resolved relative to the package directory.
    /// If out_dir is absolute, it's returned as-is.
    pub fn resolve_out_dir(&self, package_dir: &Path) -> PathBuf {
        if self.out_dir.is_absolute() {
            self.out_dir.clone()
        } else {
            package_dir.join(&self.out_dir)
        }
    }

    /// Compute the output path for a module artifact.
    ///
    /// Takes the module's path, computes its relative position within the package,
    /// and returns the corresponding output path with the given extension.
    /// Resolve the output file path for a module.
    ///
    /// The `root_dir` parameter (from compilerOptions) specifies the root of source files.
    /// Output structure mirrors source structure minus the root_dir prefix.
    pub fn resolve_out_file(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        module_path: &Path,
        extension: &str,
    ) -> PathBuf {
        let out_dir = self.resolve_out_dir(package_dir);

        // compute module's relative path within the package
        let relative = module_path.strip_prefix(package_dir).unwrap_or(module_path);

        // strip root_dir prefix if specified (e.g., "src/" → "")
        let relative = if let Some(root_dir) = root_dir {
            relative.strip_prefix(root_dir).unwrap_or(relative)
        } else {
            relative
        };

        // change extension and join with output directory
        out_dir.join(relative.with_extension(extension))
    }
}
