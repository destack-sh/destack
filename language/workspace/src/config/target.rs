use std::path::{Path, PathBuf};

use super::tsconfig::{EsTarget, ModuleTarget};

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

/// Relocation model for native codegen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RelocationModel {
    /// Static relocation model.
    Static,
    /// Position-independent code.
    #[default]
    Pic,
    /// Position-independent executable.
    Pie,
}

impl std::str::FromStr for RelocationModel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "static" => Ok(Self::Static),
            "pic" => Ok(Self::Pic),
            "pie" => Ok(Self::Pie),
            _ => Err(()),
        }
    }
}

impl RelocationModel {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Link mode for native targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkMode {
    /// Prefer static linking.
    Static,
    /// Prefer dynamic linking.
    #[default]
    Dynamic,
}

impl std::str::FromStr for LinkMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "static" => Ok(Self::Static),
            "dynamic" | "shared" => Ok(Self::Dynamic),
            _ => Err(()),
        }
    }
}

impl LinkMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Debug info emission policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DebugInfoLevel {
    /// No debug info.
    #[default]
    None,
    /// Line tables only.
    Line,
    /// Full debug info.
    Full,
}

impl std::str::FromStr for DebugInfoLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" => Ok(Self::None),
            "line" | "lines" => Ok(Self::Line),
            "full" => Ok(Self::Full),
            _ => Err(()),
        }
    }
}

impl DebugInfoLevel {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Symbol stripping policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StripLevel {
    /// Keep all symbols.
    #[default]
    None,
    /// Strip local symbols.
    Partial,
    /// Strip all symbols.
    Full,
}

impl std::str::FromStr for StripLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" => Ok(Self::None),
            "partial" => Ok(Self::Partial),
            "full" | "all" => Ok(Self::Full),
            _ => Err(()),
        }
    }
}

impl StripLevel {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Panic policy for unrecoverable errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanicPolicy {
    /// Abort immediately.
    #[default]
    Abort,
    /// Unwind the stack.
    Unwind,
}

impl std::str::FromStr for PanicPolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "abort" => Ok(Self::Abort),
            "unwind" => Ok(Self::Unwind),
            _ => Err(()),
        }
    }
}

impl PanicPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Unwind info format for native targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnwindFormat {
    /// No unwind info.
    #[default]
    None,
    /// DWARF unwind info.
    Dwarf,
    /// Windows SEH unwind info.
    Seh,
}

impl std::str::FromStr for UnwindFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" => Ok(Self::None),
            "dwarf" => Ok(Self::Dwarf),
            "seh" => Ok(Self::Seh),
            _ => Err(()),
        }
    }
}

impl UnwindFormat {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Integer overflow checking policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverflowCheckPolicy {
    /// Always emit overflow checks.
    Always,
    /// Emit overflow checks only in debug builds.
    #[default]
    Debug,
    /// Never emit overflow checks.
    Never,
}

impl std::str::FromStr for OverflowCheckPolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "always" => Ok(Self::Always),
            "debug" => Ok(Self::Debug),
            "never" | "off" => Ok(Self::Never),
            _ => Err(()),
        }
    }
}

impl OverflowCheckPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Global allocator selection for native targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Allocator {
    /// Use the platform default allocator.
    #[default]
    System,
    /// Use mimalloc.
    MiMalloc,
    /// Use jemalloc.
    JeMalloc,
    /// Use a custom allocator provided by the runtime.
    Custom,
}

impl std::str::FromStr for Allocator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "system" => Ok(Self::System),
            "mimalloc" => Ok(Self::MiMalloc),
            "jemalloc" => Ok(Self::JeMalloc),
            "custom" => Ok(Self::Custom),
            _ => Err(()),
        }
    }
}

impl Allocator {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
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

/// Bounds check policy for array and slice accesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoundsCheckPolicy {
    /// Always emit bounds checks.
    Always,
    /// Emit bounds checks only in debug builds.
    #[default]
    Debug,
    /// Never emit bounds checks (unsafe, fastest).
    Never,
}

impl std::str::FromStr for BoundsCheckPolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "always" => Ok(Self::Always),
            "debug" => Ok(Self::Debug),
            "never" | "off" => Ok(Self::Never),
            _ => Err(()),
        }
    }
}

impl BoundsCheckPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Borrow checking mode for ownership references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowMode {
    /// Hint mode with warnings only.
    Hint,
    /// Strict mode with errors on violations.
    Strict,
}

impl BorrowMode {
    /// Whether this mode is stricter than another mode.
    pub fn is_stricter_than(self, other: BorrowMode) -> bool {
        matches!((self, other), (BorrowMode::Strict, BorrowMode::Hint))
    }
}

/// Runtime environment that actually executes the compiled code (at runtime).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Runtime {
    // JS runtimes (for output=js/ts)
    /// Web browser (Chrome, Firefox, Safari, etc.)
    Browser,
    /// Node.js
    #[default]
    Node,
    /// Deno
    Deno,
    /// Bun
    Bun,
    /// Web Worker / Service Worker / Shared Worker
    Worker,
    /// Cloudflare Workers (workerd)
    Workerd,

    // WASM runtimes (for output=wasm)
    /// WASM running in a JS host (browser or Node)
    WasmJs,
    /// WASM with WASI (wasmtime, wasmer, etc.)
    WasmWasi,

    // Native runtimes (for output=native)
    /// Native hosted runtime (OS services available).
    NativeHosted,
    /// Native freestanding runtime (no OS services assumed).
    NativeFreestanding,
    /// Native embedded runtime (freestanding with tight constraints).
    NativeEmbedded,
}

impl std::str::FromStr for Runtime {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "browser" => Ok(Self::Browser),
            "node" => Ok(Self::Node),
            "deno" => Ok(Self::Deno),
            "bun" => Ok(Self::Bun),
            "worker" => Ok(Self::Worker),
            "workerd" => Ok(Self::Workerd),
            "wasm_js" | "wasm-js" | "wasmjs" => Ok(Self::WasmJs),
            "wasm_wasi" | "wasm-wasi" | "wasmwasi" | "wasi" => Ok(Self::WasmWasi),
            "native" | "native_hosted" | "native-hosted" => Ok(Self::NativeHosted),
            "native_freestanding" | "native-freestanding" => Ok(Self::NativeFreestanding),
            "native_embedded" | "native-embedded" => Ok(Self::NativeEmbedded),
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
            Self::Browser | Self::Node | Self::Deno | Self::Bun | Self::Worker | Self::Workerd
        )
    }

    /// Whether this runtime is a WASM host.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::WasmJs | Self::WasmWasi)
    }

    /// Whether this runtime is a native runtime.
    pub fn is_native(&self) -> bool {
        matches!(
            self,
            Self::NativeHosted | Self::NativeFreestanding | Self::NativeEmbedded
        )
    }

    /// Whether this runtime runs in a browser-like environment (has DOM potential).
    pub fn is_browser_like(&self) -> bool {
        matches!(self, Self::Browser | Self::WasmJs)
    }

    /// Whether this runtime is server-side.
    pub fn is_server(&self) -> bool {
        matches!(
            self,
            Self::Node
                | Self::Deno
                | Self::Bun
                | Self::WasmWasi
                | Self::NativeHosted
                | Self::NativeFreestanding
                | Self::NativeEmbedded
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
    /// Bare metal (no operating system).
    BareMetal,
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
            "bare_metal" | "bare-metal" | "baremetal" | "none" => Ok(Self::BareMetal),
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

    /// Whether this is a bare metal platform.
    pub fn is_bare_metal(&self) -> bool {
        matches!(self, Self::BareMetal)
    }
}

/// Default output directory for targets.
pub const DEFAULT_OUT_DIR: &str = "dist";

/// A build target configuration.
///
/// Can be constructed from dsconfig.json or programmatically.
/// This is the type used by compiler/codegen - independent of dsconfig parsing.
#[derive(Debug, Clone, Default)]
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
    pub module: ModuleTarget,
    /// ECMAScript target version.
    pub es_target: EsTarget,
    /// Library files for this target. If `None`, derived automatically from runtime and platform.
    pub lib: Option<Vec<String>>,
    /// Additional library types for this target.
    pub types: Option<Vec<String>>,
    /// Explicit profile name for this target.
    pub profile: Option<String>,
    /// Output format (js, ts, wasm, native).
    pub output: OutputFormat,
    /// Runtime environment (browser, node, wasm-wasi, native-hosted, etc.).
    pub runtime: Runtime,
    /// Runtime version for selecting versioned libs.
    pub runtime_version: Option<String>,
    /// Target platform (web, windows, macos, linux, ios, android, bare-metal, etc.).
    pub platform: Platform,
    /// Target triple for native codegen (e.g., "x86_64-unknown-linux-gnu").
    pub target_triple: Option<String>,
    /// CPU name for native codegen (e.g., "native", "x86-64", "znver3").
    pub cpu: Option<String>,
    /// CPU feature flags for native codegen (e.g., "+sse4.2", "+aes").
    pub cpu_features: Vec<String>,
    /// Relocation model for native codegen.
    pub relocation_model: RelocationModel,
    /// Link mode for native targets.
    pub link_mode: LinkMode,
    /// Emit declaration files (e.g., `.d.ts` alongside `.js` output).
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
    /// Debug info emission policy.
    pub debug_info: DebugInfoLevel,
    /// Symbol stripping policy.
    pub strip: StripLevel,
    /// Panic policy for unrecoverable errors.
    pub panic: PanicPolicy,
    /// Unwind info format for native targets.
    pub unwind: UnwindFormat,
    /// Integer overflow checking policy.
    pub overflow_checks: OverflowCheckPolicy,
    /// Bounds check policy for array and slice accesses.
    pub bounds_checks: BoundsCheckPolicy,
    /// Global allocator selection for native targets.
    pub allocator: Allocator,

    // output paths
    /// Output directory for this target (relative to package, defaults to "dist").
    pub out_dir: PathBuf,
    /// Output file for single-file targets.
    pub out_file: Option<PathBuf>,
    /// Separate directory for declaration files.
    pub declaration_dir: Option<PathBuf>,
}

impl Target {
    /// Create a new target with the given name and default JS output.
    pub fn js(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Js,
            runtime: Runtime::Node,
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
            runtime: Runtime::Node,
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
    pub fn wasm_js(name: impl Into<String>) -> Self {
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
            runtime: Runtime::NativeHosted,
            platform: Platform::Universal,
            optimize: true,
            ..Default::default()
        }
    }

    /// Create a new target for comptime execution.
    pub fn comptime(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Native,
            runtime: Runtime::NativeHosted,
            platform: Platform::Universal,
            optimize: true,
            ..Default::default()
        }
    }

    /// Create a new target with the given name and native freestanding output.
    pub fn native_freestanding(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Native,
            runtime: Runtime::NativeFreestanding,
            platform: Platform::BareMetal,
            optimize: true,
            ..Default::default()
        }
    }

    /// Create a new target with the given name and native embedded output.
    pub fn native_embedded(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Native,
            runtime: Runtime::NativeEmbedded,
            platform: Platform::BareMetal,
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
    pub fn with_module(mut self, module: ModuleTarget) -> Self {
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

    /// Set additional library types (additive to derived libs).
    pub fn with_types(mut self, types: Vec<String>) -> Self {
        self.types = Some(types);
        self
    }

    /// Set an explicit profile name for this target.
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Set the runtime.
    pub fn with_runtime(mut self, runtime: Runtime) -> Self {
        self.runtime = runtime;
        self
    }

    /// Set the runtime version.
    pub fn with_runtime_version(mut self, runtime_version: impl Into<String>) -> Self {
        self.runtime_version = Some(runtime_version.into());
        self
    }

    /// Set the platform.
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    /// Set target triple for native codegen.
    pub fn with_target_triple(mut self, target_triple: impl Into<String>) -> Self {
        self.target_triple = Some(target_triple.into());
        self
    }

    /// Set CPU name for native codegen.
    pub fn with_cpu(mut self, cpu: impl Into<String>) -> Self {
        self.cpu = Some(cpu.into());
        self
    }

    /// Set CPU feature flags for native codegen.
    pub fn with_cpu_features(mut self, cpu_features: Vec<String>) -> Self {
        self.cpu_features = cpu_features;
        self
    }

    /// Set relocation model for native codegen.
    pub fn with_relocation_model(mut self, relocation_model: RelocationModel) -> Self {
        self.relocation_model = relocation_model;
        self
    }

    /// Set link mode for native targets.
    pub fn with_link_mode(mut self, link_mode: LinkMode) -> Self {
        self.link_mode = link_mode;
        self
    }

    /// Set bounds check policy for array and slice accesses.
    pub fn with_bounds_checks(mut self, bounds_checks: BoundsCheckPolicy) -> Self {
        self.bounds_checks = bounds_checks;
        self
    }

    /// Set overflow check policy for integer operations.
    pub fn with_overflow_checks(mut self, overflow_checks: OverflowCheckPolicy) -> Self {
        self.overflow_checks = overflow_checks;
        self
    }

    /// Set debug info emission policy.
    pub fn with_debug_info(mut self, debug_info: DebugInfoLevel) -> Self {
        self.debug_info = debug_info;
        self
    }

    /// Set symbol stripping policy.
    pub fn with_strip(mut self, strip: StripLevel) -> Self {
        self.strip = strip;
        self
    }

    /// Set panic strategy.
    pub fn with_panic(mut self, panic: PanicPolicy) -> Self {
        self.panic = panic;
        self
    }

    /// Set unwind info format.
    pub fn with_unwind(mut self, unwind: UnwindFormat) -> Self {
        self.unwind = unwind;
        self
    }

    /// Set global allocator selection.
    pub fn with_allocator(mut self, allocator: Allocator) -> Self {
        self.allocator = allocator;
        self
    }

    /// Derive library files from target settings.
    /// If `lib` is explicitly set, returns it. Otherwise derives from:
    /// - ES version from compiler target (JS/TS outputs only)
    /// - Runtime-specific libs (dom, node, deno, worker, etc.)
    pub fn derived_lib(&self) -> Vec<String> {
        if let Some(lib) = &self.lib {
            return lib.clone();
        }

        let mut libs = Vec::new();
        let runtime_version = self
            .runtime_version
            .as_deref()
            .map(str::trim)
            .filter(|version| !version.is_empty())
            .filter(|version| !version.eq_ignore_ascii_case("latest"));

        let versioned_lib = |name: &str, version: Option<&str>| {
            if let Some(version) = version {
                let version = version.strip_prefix('v').unwrap_or(version);
                format!("{name}.v{version}")
            } else {
                name.to_string()
            }
        };

        // runtime-specific libs
        match self.runtime {
            Runtime::Browser => {
                libs.push("dom".to_string());
                libs.push("dom.iterable".to_string());
                libs.push("dom.asynciterable".to_string());
            }
            Runtime::Node => {
                libs.push(versioned_lib("node", runtime_version));
            }
            Runtime::Deno => {
                libs.push(versioned_lib("deno", runtime_version));
            }
            Runtime::Bun => {
                libs.push(versioned_lib("bun", runtime_version));
                libs.push("node".to_string());
            }
            Runtime::Worker | Runtime::Workerd => {
                libs.push("worker".to_string());
                libs.push("worker.iterable".to_string());
                libs.push("worker.asynciterable".to_string());
            }
            Runtime::WasmJs
            | Runtime::WasmWasi
            | Runtime::NativeHosted
            | Runtime::NativeFreestanding
            | Runtime::NativeEmbedded => {
                // #Incomplete: map to Destack runtime libs
            }
        }

        if self.output.is_js() || self.output.is_ts() {
            libs.push(self.es_target.default_lib_name().to_string());
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
