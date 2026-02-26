use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use destack_builtin::{BuiltinOutputFormat, BuiltinPlatform, BuiltinRuntime};

use super::policy::{
    BoundsCheckPolicy, BoundsCheckPolicyJson, CheckFailurePolicy, CheckFailurePolicyJson,
    DivisionCheckPolicy, DivisionCheckPolicyJson, ExecutionMode, ExecutionModeJson,
    FloatMathPolicy, FloatMathPolicyJson, NullCheckPolicy, NullCheckPolicyJson,
    OverflowCheckPolicy, OverflowCheckPolicyJson, PanicPolicy, PanicPolicyJson, SafetyPreset,
    SafetyPresetJson, SandboxPolicy, SandboxPolicyJson, ShiftCheckPolicy, ShiftCheckPolicyJson,
    TrustPolicy, TrustPolicyJson, UnwindFormat, UnwindFormatJson,
};
use super::runtime::{DsConfigRuntimeOptionsJson, RuntimeOptions, runtime_options_with_base};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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

impl From<OutputFormat> for BuiltinOutputFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Js => BuiltinOutputFormat::Js,
            OutputFormat::Ts => BuiltinOutputFormat::Ts,
            OutputFormat::Wasm => BuiltinOutputFormat::Wasm,
            OutputFormat::Native => BuiltinOutputFormat::Native,
        }
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

/// Extra artifacts to emit for debugging or inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmitArtifact {
    /// Lowered MIR for the module.
    Mir,
    /// Backend IR (LLVM/Cranelift).
    Ir,
    /// Assembly output.
    Asm,
    /// Object file output.
    Object,
    /// Symbol table output.
    Symbols,
}

impl std::str::FromStr for EmitArtifact {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "mir" => Ok(Self::Mir),
            "ir" | "llvm_ir" | "llvm" | "cranelift_ir" | "clif" => Ok(Self::Ir),
            "asm" | "assembly" => Ok(Self::Asm),
            "object" | "obj" => Ok(Self::Object),
            "symbols" | "sym" | "symtab" => Ok(Self::Symbols),
            _ => Err(()),
        }
    }
}

impl EmitArtifact {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Optimization level for builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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
    /// Maximal optimization (O4).
    O4,
}

/// Link time optimization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LtoMode {
    /// Use defaults based on optimization level.
    /// Auto enables Thin LTO at O4 and disables LTO at lower levels.
    #[default]
    Auto,
    /// Disable link time optimization.
    None,
    /// Enable Thin LTO at package scope.
    Thin,
    /// Enable Full LTO at program scope.
    Full,
}

impl std::str::FromStr for LtoMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "auto" => Ok(Self::Auto),
            "none" | "off" | "disabled" => Ok(Self::None),
            "thin" | "thinlto" | "thin_lto" => Ok(Self::Thin),
            "full" | "lto" => Ok(Self::Full),
            _ => Err(()),
        }
    }
}

impl LtoMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl From<u8> for OptimizeLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => Self::O0,
            1 => Self::O1,
            2 => Self::O2,
            3 => Self::O3,
            _ => Self::O4,
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
            OptimizeLevel::O4 => 4,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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

/// Debug execution mode for VM/native targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DebugMode {
    /// Choose mode based on target debug settings.
    #[default]
    Auto,
    /// Always run the interpreter.
    Vm,
    /// Run native with deopt-first debugging.
    Deopt,
    /// Run native only (no deopt).
    Native,
}

impl std::str::FromStr for DebugMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "auto" => Ok(Self::Auto),
            "vm" | "interpreter" => Ok(Self::Vm),
            "deopt" => Ok(Self::Deopt),
            "native" => Ok(Self::Native),
            _ => Err(()),
        }
    }
}

impl DebugMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// OSR entry mode for native execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OsrMode {
    /// OSR disabled.
    Disabled,
    /// OSR at loop headers.
    #[default]
    LoopHeaders,
    /// OSR only at explicitly marked sites.
    Explicit,
}

impl std::str::FromStr for OsrMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "disabled" | "off" => Ok(Self::Disabled),
            "loop_headers" | "loops" => Ok(Self::LoopHeaders),
            "explicit" => Ok(Self::Explicit),
            _ => Err(()),
        }
    }
}

impl OsrMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Safepoint insertion mode for native execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SafepointMode {
    /// Call sites, allocation points, and loop back-edges only.
    #[default]
    CallsAllocBackEdges,
    /// Add instruction-budget safepoints for bounded latency.
    Budgeted,
}

impl std::str::FromStr for SafepointMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "calls_alloc_backedges" | "standard" => Ok(Self::CallsAllocBackEdges),
            "budgeted" | "budget" => Ok(Self::Budgeted),
            _ => Err(()),
        }
    }
}

impl SafepointMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Speculation mode for native optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SpeculationMode {
    /// Disable speculative optimizations.
    None,
    /// Guarded speculations with explicit deopt metadata.
    #[default]
    Guarded,
    /// Aggressive speculation across more sites.
    Aggressive,
}

impl std::str::FromStr for SpeculationMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" | "off" => Ok(Self::None),
            "guarded" => Ok(Self::Guarded),
            "aggressive" => Ok(Self::Aggressive),
            _ => Err(()),
        }
    }
}

impl SpeculationMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Profiling mode for tiering and optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProfilingMode {
    /// Disable runtime profiling collection.
    None,
    /// Counters only (calls, branches, allocations).
    Counters,
    /// Sampling only (periodic opcode and site sampling).
    Sampling,
    /// Counters + sampling + inline caches.
    #[default]
    Hybrid,
}

impl std::str::FromStr for ProfilingMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" | "off" => Ok(Self::None),
            "counters" => Ok(Self::Counters),
            "sampling" => Ok(Self::Sampling),
            "hybrid" | "full" => Ok(Self::Hybrid),
            _ => Err(()),
        }
    }
}

impl ProfilingMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Symbol stripping policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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

/// Global allocator selection for native targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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

/// Borrow checking mode for ownership references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowMode {
    /// Hint mode with warnings only.
    Hint,
    /// Strict mode with errors on violations.
    Strict,
}

impl BorrowMode {
    /// Whether this mode is strict (errors on violations, `&mut T` has noalias semantics).
    pub fn is_strict(self) -> bool {
        matches!(self, BorrowMode::Strict)
    }

    /// Whether this mode is stricter than another mode.
    pub fn is_stricter_than(self, other: BorrowMode) -> bool {
        matches!((self, other), (BorrowMode::Strict, BorrowMode::Hint))
    }
}

/// Runtime environment that actually executes the compiled code (at runtime).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Runtime {
    // JS runtimes (for output=JS/TS)
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

impl From<Runtime> for BuiltinRuntime {
    fn from(value: Runtime) -> Self {
        match value {
            Runtime::Browser => BuiltinRuntime::Browser,
            Runtime::Node => BuiltinRuntime::Node,
            Runtime::Deno => BuiltinRuntime::Deno,
            Runtime::Bun => BuiltinRuntime::Bun,
            Runtime::Worker => BuiltinRuntime::Worker,
            Runtime::WasmJs => BuiltinRuntime::WasmJs,
            Runtime::WasmWasi => BuiltinRuntime::WasmWasi,
            Runtime::NativeHosted => BuiltinRuntime::NativeHosted,
            Runtime::NativeFreestanding => BuiltinRuntime::NativeFreestanding,
            Runtime::NativeEmbedded => BuiltinRuntime::NativeEmbedded,
        }
    }
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
            Self::Browser | Self::Node | Self::Deno | Self::Bun | Self::Worker
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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
    /// FreeBSD
    FreeBsd,
    /// OpenBSD
    OpenBsd,
    /// NetBSD
    NetBsd,
    /// DragonFly BSD
    DragonFly,
    /// Solaris
    Solaris,
    /// Illumos
    Illumos,
    /// Haiku
    Haiku,
    /// Fuchsia
    Fuchsia,
    /// Redox
    Redox,
    /// Hermit
    Hermit,

    // Mobile
    /// iOS
    IOS,
    /// Android
    Android,

    // Other
    /// WASI (WebAssembly System Interface)
    Wasi,
    /// Emscripten (WebAssembly with Emscripten ABI)
    Emscripten,
    /// Bare metal (no operating system).
    BareMetal,
    /// Unknown or portable (no platform-specific APIs)
    Universal,
}

impl From<Platform> for BuiltinPlatform {
    fn from(value: Platform) -> Self {
        match value {
            Platform::Web => BuiltinPlatform::Web,
            Platform::Windows => BuiltinPlatform::Windows,
            Platform::MacOS => BuiltinPlatform::MacOS,
            Platform::Linux => BuiltinPlatform::Linux,
            Platform::FreeBsd => BuiltinPlatform::FreeBsd,
            Platform::OpenBsd => BuiltinPlatform::OpenBsd,
            Platform::NetBsd => BuiltinPlatform::NetBsd,
            Platform::DragonFly => BuiltinPlatform::DragonFly,
            Platform::Solaris => BuiltinPlatform::Solaris,
            Platform::Illumos => BuiltinPlatform::Illumos,
            Platform::Haiku => BuiltinPlatform::Haiku,
            Platform::Fuchsia => BuiltinPlatform::Fuchsia,
            Platform::Redox => BuiltinPlatform::Redox,
            Platform::Hermit => BuiltinPlatform::Hermit,
            Platform::IOS => BuiltinPlatform::IOS,
            Platform::Android => BuiltinPlatform::Android,
            Platform::Wasi => BuiltinPlatform::Wasi,
            Platform::Emscripten => BuiltinPlatform::Emscripten,
            Platform::BareMetal => BuiltinPlatform::BareMetal,
            Platform::Universal => BuiltinPlatform::Universal,
        }
    }
}

impl std::str::FromStr for Platform {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "web" | "browser" => Ok(Self::Web),
            "windows" | "win32" | "win" => Ok(Self::Windows),
            "macos" | "darwin" | "mac" => Ok(Self::MacOS),
            "linux" => Ok(Self::Linux),
            "freebsd" => Ok(Self::FreeBsd),
            "openbsd" => Ok(Self::OpenBsd),
            "netbsd" => Ok(Self::NetBsd),
            "dragonfly" | "dragonflybsd" => Ok(Self::DragonFly),
            "solaris" => Ok(Self::Solaris),
            "illumos" => Ok(Self::Illumos),
            "haiku" => Ok(Self::Haiku),
            "fuchsia" => Ok(Self::Fuchsia),
            "redox" => Ok(Self::Redox),
            "hermit" | "hermitos" => Ok(Self::Hermit),
            "ios" => Ok(Self::IOS),
            "android" => Ok(Self::Android),
            "wasi" => Ok(Self::Wasi),
            "emscripten" | "emscripten-wasm" => Ok(Self::Emscripten),
            "bare_metal" | "bare-metal" | "baremetal" | "none" => Ok(Self::BareMetal),
            "universal" | "portable" | "any" => Ok(Self::Universal),
            _ => Err(()),
        }
    }
}

impl Platform {
    /// Return the canonical lowercase tag for this platform.
    pub fn canonical_tag(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Windows => "windows",
            Self::MacOS => "macos",
            Self::Linux => "linux",
            Self::FreeBsd => "freebsd",
            Self::OpenBsd => "openbsd",
            Self::NetBsd => "netbsd",
            Self::DragonFly => "dragonfly",
            Self::Solaris => "solaris",
            Self::Illumos => "illumos",
            Self::Haiku => "haiku",
            Self::Fuchsia => "fuchsia",
            Self::Redox => "redox",
            Self::Hermit => "hermit",
            Self::IOS => "ios",
            Self::Android => "android",
            Self::Wasi => "wasi",
            Self::Emscripten => "emscripten",
            Self::BareMetal => "baremetal",
            Self::Universal => "universal",
        }
    }

    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Whether this platform is a web platform.
    pub fn is_web(&self) -> bool {
        matches!(self, Self::Web)
    }

    /// Whether this platform is a WASM target.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::Wasi | Self::Emscripten)
    }

    /// Whether this is a desktop platform.
    pub fn is_desktop(&self) -> bool {
        matches!(
            self,
            Self::Windows
                | Self::MacOS
                | Self::Linux
                | Self::FreeBsd
                | Self::OpenBsd
                | Self::NetBsd
                | Self::DragonFly
                | Self::Solaris
                | Self::Illumos
                | Self::Haiku
                | Self::Fuchsia
                | Self::Redox
                | Self::Hermit
        )
    }

    /// Whether this is a mobile platform.
    pub fn is_mobile(&self) -> bool {
        matches!(self, Self::IOS | Self::Android)
    }

    /// Whether this is a Unix-style platform.
    pub fn is_unix(&self) -> bool {
        matches!(
            self,
            Self::MacOS
                | Self::Linux
                | Self::FreeBsd
                | Self::OpenBsd
                | Self::NetBsd
                | Self::DragonFly
                | Self::Solaris
                | Self::Illumos
                | Self::Haiku
                | Self::IOS
                | Self::Android
        )
    }

    /// Whether this is a bare metal platform.
    pub fn is_bare_metal(&self) -> bool {
        matches!(self, Self::BareMetal)
    }

    /// Return the target family tag used by `import.meta.target.family`.
    pub fn family_tag(&self) -> &'static str {
        if self.is_web() {
            return "web";
        }

        if matches!(self, Self::Windows) {
            return "windows";
        }

        if self.is_unix() {
            return "unix";
        }

        if self.is_wasm() {
            return "wasm";
        }

        if self.is_bare_metal() {
            return "bare-metal";
        }

        if matches!(self, Self::Universal) {
            return "universal";
        }

        "other"
    }

    /// Resolve the target triple OS component for this platform.
    pub fn triple_os_component(&self) -> Option<&'static str> {
        match self {
            Self::Windows => Some("windows"),
            Self::MacOS => Some("darwin"),
            Self::Linux => Some("linux"),
            Self::FreeBsd => Some("freebsd"),
            Self::OpenBsd => Some("openbsd"),
            Self::NetBsd => Some("netbsd"),
            Self::DragonFly => Some("dragonfly"),
            Self::Solaris => Some("solaris"),
            Self::Illumos => Some("illumos"),
            Self::Haiku => Some("haiku"),
            Self::Fuchsia => Some("fuchsia"),
            Self::Redox => Some("redox"),
            Self::Hermit => Some("hermit"),
            Self::IOS => Some("ios"),
            Self::Android => Some("android"),
            Self::Wasi => Some("wasi"),
            Self::Emscripten => Some("emscripten"),
            Self::BareMetal => Some("none"),
            Self::Web | Self::Universal => None,
        }
    }
}

/// CPU architecture for native targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetArch {
    /// x86_64 (amd64)
    X86_64,
    /// x86 (i686)
    X86,
    /// AArch64 (arm64)
    Aarch64,
    /// ARMv7 (32-bit)
    Armv7,
    /// ARMv6 (32-bit)
    Armv6,
    /// RISC-V 64-bit
    Riscv64,
    /// RISC-V 32-bit
    Riscv32,
    /// PowerPC 64-bit
    PowerPc64,
    /// PowerPC 64-bit (little-endian)
    PowerPc64le,
    /// s390x
    S390x,
    /// MIPS64
    Mips64,
    /// MIPS64 (little-endian)
    Mips64el,
    /// LoongArch 64-bit
    LoongArch64,
    /// WebAssembly 32-bit
    Wasm32,
    /// WebAssembly 64-bit
    Wasm64,
    /// Other architecture (verbatim string)
    Other(String),
}

impl std::str::FromStr for TargetArch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase().replace('-', "_");
        Ok(match value.as_str() {
            "x86_64" | "amd64" => Self::X86_64,
            "x86" | "i686" | "i586" | "i386" => Self::X86,
            "aarch64" | "arm64" => Self::Aarch64,
            "armv7" | "armv7l" => Self::Armv7,
            "armv6" | "armv6l" => Self::Armv6,
            "riscv64" => Self::Riscv64,
            "riscv32" => Self::Riscv32,
            "powerpc64" | "ppc64" => Self::PowerPc64,
            "powerpc64le" | "ppc64le" => Self::PowerPc64le,
            "s390x" => Self::S390x,
            "mips64" => Self::Mips64,
            "mips64el" | "mips64le" => Self::Mips64el,
            "loongarch64" | "loong64" => Self::LoongArch64,
            "wasm32" => Self::Wasm32,
            "wasm64" => Self::Wasm64,
            other => Self::Other(other.to_string()),
        })
    }
}

impl TargetArch {
    /// Parse a target architecture from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Format this architecture as a target triple component.
    pub fn triple_component(&self) -> String {
        match self {
            Self::X86_64 => "x86_64".to_string(),
            Self::X86 => "i686".to_string(),
            Self::Aarch64 => "aarch64".to_string(),
            Self::Armv7 => "armv7".to_string(),
            Self::Armv6 => "armv6".to_string(),
            Self::Riscv64 => "riscv64".to_string(),
            Self::Riscv32 => "riscv32".to_string(),
            Self::PowerPc64 => "powerpc64".to_string(),
            Self::PowerPc64le => "powerpc64le".to_string(),
            Self::S390x => "s390x".to_string(),
            Self::Mips64 => "mips64".to_string(),
            Self::Mips64el => "mips64el".to_string(),
            Self::LoongArch64 => "loongarch64".to_string(),
            Self::Wasm32 => "wasm32".to_string(),
            Self::Wasm64 => "wasm64".to_string(),
            Self::Other(value) => value.clone(),
        }
    }
}

/// Target vendor for native targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetVendor {
    /// Unknown vendor.
    Unknown,
    /// Apple.
    Apple,
    /// PC.
    Pc,
    /// IBM.
    Ibm,
    /// Nintendo.
    Nintendo,
    /// Other vendor (verbatim string).
    Other(String),
}

impl std::str::FromStr for TargetVendor {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase().replace('-', "_");
        Ok(match value.as_str() {
            "unknown" => Self::Unknown,
            "apple" => Self::Apple,
            "pc" => Self::Pc,
            "ibm" => Self::Ibm,
            "nintendo" => Self::Nintendo,
            other => Self::Other(other.to_string()),
        })
    }
}

impl TargetVendor {
    /// Parse a target vendor from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Resolve the default vendor for a platform when none is specified.
    pub fn default_for_platform(platform: Platform) -> Self {
        match platform {
            Platform::Windows => Self::Pc,
            Platform::MacOS | Platform::IOS => Self::Apple,
            _ => Self::Unknown,
        }
    }

    /// Format this vendor as a target triple component.
    pub fn triple_component(&self) -> String {
        match self {
            Self::Unknown => "unknown".to_string(),
            Self::Apple => "apple".to_string(),
            Self::Pc => "pc".to_string(),
            Self::Ibm => "ibm".to_string(),
            Self::Nintendo => "nintendo".to_string(),
            Self::Other(value) => value.clone(),
        }
    }
}

/// Target environment / ABI flavor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetEnv {
    /// GNU environment.
    Gnu,
    /// Musl environment.
    Musl,
    /// MSVC environment.
    Msvc,
    /// GNU + LLVM environment.
    GnuLlvm,
    /// EABI (bare-metal).
    Eabi,
    /// EABI with hard-float.
    Eabihf,
    /// Musl + EABI.
    MuslEabi,
    /// Musl + EABI hard-float.
    MuslEabihf,
    /// Android.
    Android,
    /// Other environment (verbatim string).
    Other(String),
}

impl std::str::FromStr for TargetEnv {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase().replace('-', "_");
        Ok(match value.as_str() {
            "gnu" => Self::Gnu,
            "musl" => Self::Musl,
            "msvc" => Self::Msvc,
            "gnullvm" | "gnu_llvm" => Self::GnuLlvm,
            "eabi" => Self::Eabi,
            "eabihf" => Self::Eabihf,
            "musleabi" => Self::MuslEabi,
            "musleabihf" => Self::MuslEabihf,
            "android" => Self::Android,
            other => Self::Other(other.to_string()),
        })
    }
}

impl TargetEnv {
    /// Parse a target environment from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Resolve the default environment for a platform when none is specified.
    pub fn default_for_platform(platform: Platform) -> Option<Self> {
        match platform {
            Platform::Linux => Some(Self::Gnu),
            Platform::Windows => Some(Self::Msvc),
            Platform::Android => Some(Self::Android),
            _ => None,
        }
    }

    /// Format this environment as a target triple component.
    pub fn triple_component(&self) -> String {
        match self {
            Self::Gnu => "gnu".to_string(),
            Self::Musl => "musl".to_string(),
            Self::Msvc => "msvc".to_string(),
            Self::GnuLlvm => "gnullvm".to_string(),
            Self::Eabi => "eabi".to_string(),
            Self::Eabihf => "eabihf".to_string(),
            Self::MuslEabi => "musleabi".to_string(),
            Self::MuslEabihf => "musleabihf".to_string(),
            Self::Android => "android".to_string(),
            Self::Other(value) => value.clone(),
        }
    }
}

/// Default output directory for targets.
pub const DEFAULT_OUT_DIR: &str = "dist";

/// A build target configuration.
///
/// Can be constructed from dsconfig.json or programmatically.
/// This is the type used by compiler/codegen - independent of dsconfig parsing.
#[derive(Debug, Clone, Hash, Default)]
pub struct Target {
    /// Target name (e.g., "npm", "wasm", "dev").
    pub name: String,
    /// Whether this target exists only for synthetic purposes.
    pub synthetic: bool,

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
    /// Target platform / operating system.
    pub platform: Platform,
    /// Target triple for native codegen (e.g., "x86_64-unknown-linux-gnu").
    pub target_triple: Option<String>,
    /// Target architecture for native codegen (e.g., "x86_64", "aarch64").
    pub target_arch: Option<TargetArch>,
    /// Target vendor for native codegen (e.g., "apple", "pc", "unknown").
    pub target_vendor: Option<TargetVendor>,
    /// Target environment / ABI for native codegen (e.g., "gnu", "musl", "msvc").
    pub target_env: Option<TargetEnv>,
    /// CPU name for native codegen (e.g., "native", "x86-64", "znver3").
    pub cpu: Option<String>,
    /// CPU feature flags for native codegen (e.g., "+sse4.2", "+aes").
    pub cpu_features: Vec<String>,
    /// Relocation model for native codegen.
    pub relocation_model: RelocationModel,
    /// Link mode for native targets.
    pub link_mode: LinkMode,
    /// Explicit linker executable for native targets.
    pub linker: Option<String>,
    /// Extra linker arguments for native targets.
    pub link_args: Vec<String>,
    /// Sysroot path for native targets.
    pub sysroot: Option<PathBuf>,
    /// Emit declaration files (e.g., `.d.ts` alongside `.js` output).
    pub declaration: bool,
    /// Emit source maps.
    pub source_map: bool,
    /// Extra artifacts to emit.
    pub emit: Vec<EmitArtifact>,

    // optimization
    /// Whether this is a debug build.
    pub debug: bool,
    /// Whether optimization is enabled.
    pub optimize: bool,
    /// Optimization level.
    pub optimize_level: OptimizeLevel,
    /// Loop unroll threshold in instructions.
    pub unroll_threshold: Option<u64>,
    /// Inline budget scaling in percent.
    pub inline_budget_scale_percent: Option<u64>,
    /// Link time optimization mode.
    pub lto_mode: LtoMode,
    /// Shrink level (code size reduction).
    pub shrink_level: ShrinkLevel,
    /// Floating point math optimization policy.
    pub float_math: FloatMathPolicy,
    /// Debug info emission policy.
    pub debug_info: DebugInfoLevel,
    /// Debug execution mode for VM/native targets.
    pub debug_mode: DebugMode,
    /// OSR policy for native execution.
    pub osr_mode: OsrMode,
    /// Safepoint insertion mode for native execution.
    pub safepoint_mode: SafepointMode,
    /// Instruction interval for safepoint polling (when enabled).
    pub safepoint_interval: Option<u64>,
    /// Speculation mode for native optimization.
    pub speculation_mode: SpeculationMode,
    /// Profiling mode for tiering and optimization.
    pub profiling_mode: ProfilingMode,
    /// Runtime execution options.
    pub runtime_options: RuntimeOptions,
    /// Trust policy for runtime execution.
    pub trust_policy: TrustPolicy,
    /// Sandbox policy for runtime isolation.
    pub sandbox_policy: SandboxPolicy,
    /// Symbol stripping policy.
    pub strip: StripLevel,
    /// Panic policy for unrecoverable errors.
    pub panic: PanicPolicy,
    /// Unwind info format for native targets.
    pub unwind: UnwindFormat,
    /// Safety preset that configures runtime checks.
    pub safety_preset: Option<SafetyPreset>,
    /// Integer overflow checking policy.
    pub overflow_checks: OverflowCheckPolicy,
    /// Bounds check policy for array and slice accesses.
    pub bounds_checks: BoundsCheckPolicy,
    /// Null check policy for reference operations.
    pub null_checks: NullCheckPolicy,
    /// Division check policy for divide and remainder operations.
    pub division_checks: DivisionCheckPolicy,
    /// Shift range check policy.
    pub shift_checks: ShiftCheckPolicy,
    /// Check failure behavior.
    pub check_failure: CheckFailurePolicy,
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
            trust_policy: TrustPolicy::Internal,
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

    /// Create an implicit target configuration for a known target name.
    pub fn implicit_for_name(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::js(name)),
            "js" => Some(Self::js(name)),
            "ts" => Some(Self::ts(name)),
            "node" => Some(Self::node(name)),
            "wasm" => Some(Self::wasm_js(name)),
            "wasm-wasi" | "wasi" => Some(Self::wasm_wasi(name)),
            "native" => Some(Self::native(name)),
            _ => None,
        }
    }

    /// Create a synthetic target based on an existing target.
    pub fn synthetic_for(base: &Target, name: impl Into<String>) -> Self {
        let mut target = base.clone();
        target.name = name.into();
        target.output = OutputFormat::Native;
        target.optimize = false;
        target.optimize_level = OptimizeLevel::O0;
        target.lto_mode = LtoMode::None;
        target.declaration = false;
        target.source_map = false;
        target.emit.clear();
        target.synthetic = true;
        target
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

    /// Resolve a target triple string from the target configuration.
    pub fn resolved_target_triple(&self) -> Option<String> {
        if let Some(triple) = self.target_triple.as_ref() {
            return Some(triple.clone());
        }

        let target_arch = self.target_arch.as_ref()?;
        let os = self.platform.triple_os_component()?;

        let vendor = self
            .target_vendor
            .clone()
            .unwrap_or_else(|| TargetVendor::default_for_platform(self.platform));
        let env = self
            .target_env
            .clone()
            .or_else(|| TargetEnv::default_for_platform(self.platform));

        let mut components = vec![
            target_arch.triple_component(),
            vendor.triple_component(),
            os.to_string(),
        ];
        if let Some(env) = env {
            components.push(env.triple_component());
        }

        Some(components.join("-"))
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

    /// Set the trust policy for runtime execution.
    pub fn with_trust_policy(mut self, trust_policy: TrustPolicy) -> Self {
        self.trust_policy = trust_policy;
        self
    }

    /// Set the sandbox policy for runtime isolation.
    pub fn with_sandbox_policy(mut self, sandbox_policy: SandboxPolicy) -> Self {
        self.sandbox_policy = sandbox_policy;
        self
    }

    /// Set target triple for native codegen.
    pub fn with_target_triple(mut self, target_triple: impl Into<String>) -> Self {
        self.target_triple = Some(target_triple.into());
        self
    }

    /// Set target architecture for native codegen.
    pub fn with_target_arch(mut self, target_arch: TargetArch) -> Self {
        self.target_arch = Some(target_arch);
        self
    }

    /// Set target vendor for native codegen.
    pub fn with_target_vendor(mut self, target_vendor: TargetVendor) -> Self {
        self.target_vendor = Some(target_vendor);
        self
    }

    /// Set target environment / ABI for native codegen.
    pub fn with_target_env(mut self, target_env: TargetEnv) -> Self {
        self.target_env = Some(target_env);
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

    /// Set null check policy for reference operations.
    pub fn with_null_checks(mut self, null_checks: NullCheckPolicy) -> Self {
        self.null_checks = null_checks;
        self
    }

    /// Set division check policy for divide and remainder operations.
    pub fn with_division_checks(mut self, division_checks: DivisionCheckPolicy) -> Self {
        self.division_checks = division_checks;
        self
    }

    /// Set shift range check policy.
    pub fn with_shift_checks(mut self, shift_checks: ShiftCheckPolicy) -> Self {
        self.shift_checks = shift_checks;
        self
    }

    /// Set check failure behavior.
    pub fn with_check_failure(mut self, check_failure: CheckFailurePolicy) -> Self {
        self.check_failure = check_failure;
        self
    }

    /// Set overflow check policy for integer operations.
    pub fn with_overflow_checks(mut self, overflow_checks: OverflowCheckPolicy) -> Self {
        self.overflow_checks = overflow_checks;
        self
    }

    /// Set floating point math optimization policy.
    pub fn with_float_math(mut self, float_math: FloatMathPolicy) -> Self {
        self.float_math = float_math;
        self
    }

    /// Set safety preset and update runtime check policies.
    pub fn with_safety_preset(mut self, safety_preset: SafetyPreset) -> Self {
        let policies = safety_preset.runtime_check_policies();
        self.safety_preset = Some(safety_preset);
        self.overflow_checks = policies.overflow;
        self.bounds_checks = policies.bounds;
        self.null_checks = policies.null;
        self.division_checks = policies.division;
        self.shift_checks = policies.shift;
        self.float_math = safety_preset.float_math_policy();
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
            Runtime::Worker => {
                libs.push("worker".to_string());
                libs.push("worker.iterable".to_string());
                libs.push("worker.asynciterable".to_string());
            }
            Runtime::WasmJs
            | Runtime::WasmWasi
            | Runtime::NativeHosted
            | Runtime::NativeFreestanding
            | Runtime::NativeEmbedded => {
                libs.push("native".to_string());
            }
        }

        if self.output.is_js() || self.output.is_ts() {
            libs.push("js".to_string());
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

    /// Set the loop unroll threshold.
    pub fn with_unroll_threshold(mut self, unroll_threshold: u64) -> Self {
        self.unroll_threshold = Some(unroll_threshold);
        self
    }

    /// Set the inline budget scale percent.
    pub fn with_inline_budget_scale_percent(mut self, inline_budget_scale_percent: u64) -> Self {
        self.inline_budget_scale_percent = Some(inline_budget_scale_percent);
        self
    }

    /// Set the link time optimization mode.
    pub fn with_lto_mode(mut self, mode: LtoMode) -> Self {
        self.lto_mode = mode;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Safety presets map to runtime check policies.
    #[test]
    fn test_safety_preset_policies() {
        let debug = SafetyPreset::Debug.runtime_check_policies();
        assert_eq!(debug.overflow, OverflowCheckPolicy::Always);
        assert_eq!(debug.bounds, BoundsCheckPolicy::Always);
        assert_eq!(debug.null, NullCheckPolicy::Always);
        assert_eq!(debug.division, DivisionCheckPolicy::Always);
        assert_eq!(debug.shift, ShiftCheckPolicy::Always);

        let fast = SafetyPreset::ReleaseFast.runtime_check_policies();
        assert_eq!(fast.overflow, OverflowCheckPolicy::Never);
        assert_eq!(fast.bounds, BoundsCheckPolicy::Never);
        assert_eq!(fast.null, NullCheckPolicy::Never);
        assert_eq!(fast.division, DivisionCheckPolicy::Never);
        assert_eq!(fast.shift, ShiftCheckPolicy::Never);

        let debug_float = SafetyPreset::Debug.float_math_policy();
        let fast_float = SafetyPreset::ReleaseFast.float_math_policy();
        assert_eq!(debug_float, FloatMathPolicy::Strict);
        assert_eq!(fast_float, FloatMathPolicy::Fast);
    }
}

/// Normalized Destack build target options (from dsconfig.json).
#[derive(Debug, Clone)]
pub struct DsConfigTargetOptions {
    // discovery
    /// How modules are discovered for this target.
    pub discovery: TargetDiscovery,
    /// Entry points for entry-based discovery.
    pub entry: Vec<PathBuf>,
    /// Glob patterns for files to include (for include-based discovery).
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,

    // output format
    /// Output format (js, ts, wasm, native).
    pub output: OutputFormat,
    /// Runtime environment (browser, node, wasm-wasi, native-hosted, etc.).
    pub runtime: Runtime,
    /// Runtime version for selecting versioned libs.
    pub runtime_version: Option<String>,
    /// Target platform / operating system.
    pub platform: Platform,
    /// Target triple for native codegen.
    /// This selects the ABI and CPU architecture for native targets.
    /// Target triple for native codegen (e.g., "x86_64-unknown-linux-gnu").
    pub target_triple: Option<String>,
    /// Target architecture for native codegen (e.g., "x86_64", "aarch64").
    pub target_arch: Option<TargetArch>,
    /// Target vendor for native codegen (e.g., "apple", "pc", "unknown").
    pub target_vendor: Option<TargetVendor>,
    /// Target environment / ABI for native codegen (e.g., "gnu", "musl", "msvc").
    pub target_env: Option<TargetEnv>,
    /// CPU name for native codegen (e.g., "native", "x86-64", "znver3").
    pub cpu: Option<String>,
    /// CPU feature flags for native codegen (e.g., "+sse4.2", "+aes").
    pub cpu_features: Vec<String>,
    /// Relocation model for native codegen.
    pub relocation_model: RelocationModel,
    /// Link mode for native targets.
    pub link_mode: LinkMode,
    /// Explicit linker executable for native targets.
    pub linker: Option<String>,
    /// Extra linker arguments for native targets.
    pub link_args: Vec<String>,
    /// Sysroot path for native targets.
    pub sysroot: Option<PathBuf>,
    /// Emit declaration files (e.g., `.d.ts` alongside `.js` output).
    pub declaration: bool,
    /// Emit source maps.
    pub source_map: bool,
    /// Extra artifacts to emit.
    pub emit: Vec<EmitArtifact>,

    // output paths
    /// Output directory for this target (defaults to "dist").
    pub out_dir: PathBuf,
    /// Output file for single-file targets like wasm.
    pub out_file: Option<PathBuf>,
    /// Separate directory for declaration files.
    pub declaration_dir: Option<PathBuf>,

    // JS/TS specific
    /// Module format for this target.
    pub module: ModuleTarget,
    /// ECMAScript target for this target.
    pub es_target: EsTarget,
    /// Library files for this target. If `None`, derived automatically from runtime and platform.
    pub lib: Option<Vec<String>>,
    /// Additional library types for this target.
    pub types: Option<Vec<String>>,
    /// Explicit profile name for this target.
    pub profile: Option<String>,

    // optimization
    /// Whether this is a debug build.
    pub debug: bool,
    /// Whether optimization is enabled.
    pub optimize: bool,
    /// Optimization level.
    pub optimize_level: OptimizeLevel,
    /// Loop unroll threshold in instructions.
    pub unroll_threshold: Option<u64>,
    /// Inline budget scaling in percent.
    pub inline_budget_scale_percent: Option<u64>,
    /// Link time optimization mode.
    pub lto_mode: LtoMode,
    /// Shrink level (code size reduction).
    pub shrink_level: ShrinkLevel,
    /// Floating point math optimization policy.
    pub float_math: FloatMathPolicy,
    /// Debug info emission policy.
    pub debug_info: DebugInfoLevel,
    /// Debug execution mode for VM/native targets.
    pub debug_mode: DebugMode,
    /// OSR mode for native execution.
    pub osr_mode: OsrMode,
    /// Safepoint insertion mode for native execution.
    pub safepoint_mode: SafepointMode,
    /// Instruction interval for safepoint polling (when enabled).
    pub safepoint_interval: Option<u64>,
    /// Speculation mode for native optimization.
    pub speculation_mode: SpeculationMode,
    /// Profiling mode for tiering and optimization.
    pub profiling_mode: ProfilingMode,
    /// Runtime execution options.
    pub runtime_options: RuntimeOptions,
    /// Trust policy for runtime execution.
    pub trust_policy: TrustPolicy,
    /// Sandbox policy for runtime isolation.
    pub sandbox_policy: SandboxPolicy,
    /// Symbol stripping policy.
    pub strip: StripLevel,
    /// Panic policy for unrecoverable errors.
    pub panic: PanicPolicy,
    /// Unwind info format for native targets.
    pub unwind: UnwindFormat,
    /// Safety preset that configures runtime checks.
    pub safety_preset: Option<SafetyPreset>,
    /// Integer overflow checking policy.
    pub overflow_checks: OverflowCheckPolicy,
    /// Bounds check policy for array and slice accesses.
    pub bounds_checks: BoundsCheckPolicy,
    /// Null check policy for reference operations.
    pub null_checks: NullCheckPolicy,
    /// Division check policy for divide and remainder operations.
    pub division_checks: DivisionCheckPolicy,
    /// Shift range check policy.
    pub shift_checks: ShiftCheckPolicy,
    /// Check failure behavior.
    pub check_failure: CheckFailurePolicy,
    /// Global allocator selection for native targets.
    pub allocator: Allocator,
}

impl Default for DsConfigTargetOptions {
    fn default() -> Self {
        Self {
            discovery: TargetDiscovery::default(),
            entry: Vec::new(),
            include: Vec::new(),
            exclude: Vec::new(),
            output: OutputFormat::default(),
            runtime: Runtime::default(),
            runtime_version: None,
            platform: Platform::default(),
            target_triple: None,
            target_arch: None,
            target_vendor: None,
            target_env: None,
            cpu: None,
            cpu_features: Vec::new(),
            relocation_model: RelocationModel::default(),
            link_mode: LinkMode::default(),
            linker: None,
            link_args: Vec::new(),
            sysroot: None,
            declaration: false,
            source_map: false,
            emit: Vec::new(),
            out_dir: PathBuf::from(DEFAULT_OUT_DIR),
            out_file: None,
            declaration_dir: None,
            module: ModuleTarget::default(),
            es_target: EsTarget::default(),
            lib: None,
            types: None,
            profile: None,
            debug: true,
            optimize: false,
            optimize_level: OptimizeLevel::O0,
            unroll_threshold: None,
            inline_budget_scale_percent: None,
            lto_mode: LtoMode::default(),
            shrink_level: ShrinkLevel::S0,
            float_math: FloatMathPolicy::default(),
            debug_info: DebugInfoLevel::default(),
            debug_mode: DebugMode::default(),
            osr_mode: OsrMode::default(),
            safepoint_mode: SafepointMode::default(),
            safepoint_interval: None,
            speculation_mode: SpeculationMode::default(),
            profiling_mode: ProfilingMode::default(),
            runtime_options: RuntimeOptions::default(),
            trust_policy: TrustPolicy::default(),
            sandbox_policy: SandboxPolicy::default(),
            strip: StripLevel::default(),
            panic: PanicPolicy::default(),
            unwind: UnwindFormat::default(),
            safety_preset: None,
            overflow_checks: OverflowCheckPolicy::default(),
            bounds_checks: BoundsCheckPolicy::default(),
            null_checks: NullCheckPolicy::default(),
            division_checks: DivisionCheckPolicy::default(),
            shift_checks: ShiftCheckPolicy::default(),
            check_failure: CheckFailurePolicy::default(),
            allocator: Allocator::default(),
        }
    }
}

impl DsConfigTargetOptions {
    /// Derive the output mode from the target configuration.
    pub fn output_mode(&self) -> OutputMode {
        if self.out_file.is_some() || self.output.is_single_file() {
            OutputMode::File
        } else {
            OutputMode::Directory
        }
    }

    /// Whether this target produces single-file output.
    pub fn is_out_file(&self) -> bool {
        self.output_mode() == OutputMode::File
    }

    /// Whether this target produces directory output (one file per source file).
    pub fn is_out_dir(&self) -> bool {
        self.output_mode() == OutputMode::Directory
    }

    /// Convert to a standalone Target with the given name.
    pub fn to_target(&self, name: &str) -> Target {
        Target {
            name: name.to_string(),
            synthetic: false,
            discovery: self.discovery,
            entry: self.entry.clone(),
            include: self.include.clone(),
            exclude: self.exclude.clone(),
            output: self.output,
            runtime: self.runtime,
            runtime_version: self.runtime_version.clone(),
            platform: self.platform,
            target_triple: self.target_triple.clone(),
            target_arch: self.target_arch.clone(),
            target_vendor: self.target_vendor.clone(),
            target_env: self.target_env.clone(),
            cpu: self.cpu.clone(),
            cpu_features: self.cpu_features.clone(),
            relocation_model: self.relocation_model,
            link_mode: self.link_mode,
            linker: self.linker.clone(),
            link_args: self.link_args.clone(),
            sysroot: self.sysroot.clone(),
            declaration: self.declaration,
            source_map: self.source_map,
            emit: self.emit.clone(),
            out_dir: self.out_dir.clone(),
            out_file: self.out_file.clone(),
            declaration_dir: self.declaration_dir.clone(),
            module: self.module,
            es_target: self.es_target,
            lib: self.lib.clone(),
            types: self.types.clone(),
            profile: self.profile.clone(),
            debug: self.debug,
            optimize: self.optimize,
            optimize_level: self.optimize_level,
            unroll_threshold: self.unroll_threshold,
            inline_budget_scale_percent: self.inline_budget_scale_percent,
            lto_mode: self.lto_mode,
            shrink_level: self.shrink_level,
            float_math: self.float_math,
            debug_info: self.debug_info,
            debug_mode: self.debug_mode,
            osr_mode: self.osr_mode,
            safepoint_mode: self.safepoint_mode,
            safepoint_interval: self.safepoint_interval,
            speculation_mode: self.speculation_mode,
            profiling_mode: self.profiling_mode,
            runtime_options: self.runtime_options.clone(),
            trust_policy: self.trust_policy,
            sandbox_policy: self.sandbox_policy,
            strip: self.strip,
            panic: self.panic,
            unwind: self.unwind,
            safety_preset: self.safety_preset,
            overflow_checks: self.overflow_checks,
            bounds_checks: self.bounds_checks,
            null_checks: self.null_checks,
            division_checks: self.division_checks,
            shift_checks: self.shift_checks,
            check_failure: self.check_failure,
            allocator: self.allocator,
        }
    }

    /// Derive target options from JSON and base runtime options.
    pub fn from_json_with_runtime(
        json: &DsConfigTargetJson,
        base_runtime: &RuntimeOptions,
    ) -> Self {
        // derive entry points
        let entry: Vec<PathBuf> = json
            .entry
            .as_ref()
            .map(|e| e.iter().map(PathBuf::from).collect())
            .unwrap_or_default();

        // derive discovery mode: Entry if entry points are set, otherwise Include
        let discovery = if !entry.is_empty() {
            TargetDiscovery::Entry
        } else {
            TargetDiscovery::Include
        };

        // apply runtime overrides on top of the base runtime options
        let mut runtime_options =
            runtime_options_with_base(base_runtime, json.runtime_options.as_ref());

        // align execution mode field with runtime options
        if let Some(execution_mode) = json.execution.map(ExecutionMode::from) {
            runtime_options.execution = execution_mode;
        }

        let safety_preset = json.safety_preset.map(SafetyPreset::from);
        let default_checks = safety_preset
            .map(|preset| preset.runtime_check_policies())
            .unwrap_or_default();
        let default_float_math = safety_preset
            .map(|preset| preset.float_math_policy())
            .unwrap_or_default();

        Self {
            discovery,
            entry,
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            output: json.output.map(OutputFormat::from).unwrap_or_default(),
            runtime: json
                .runtime
                .as_deref()
                .and_then(Runtime::parse)
                .unwrap_or_default(),
            runtime_version: json.runtime_version.clone(),
            platform: json
                .platform
                .as_deref()
                .and_then(Platform::parse)
                .unwrap_or_default(),
            target_triple: json.target_triple.clone(),
            target_arch: json.arch.as_deref().and_then(TargetArch::parse),
            target_vendor: json.vendor.as_deref().and_then(TargetVendor::parse),
            target_env: json.env.as_deref().and_then(TargetEnv::parse),
            cpu: json.cpu.clone(),
            cpu_features: json.cpu_features.clone().unwrap_or_default(),
            relocation_model: json
                .relocation_model
                .map(RelocationModel::from)
                .unwrap_or_default(),
            link_mode: json.link_mode.map(LinkMode::from).unwrap_or_default(),
            linker: json.linker.clone(),
            link_args: json.link_args.clone().unwrap_or_default(),
            sysroot: json.sysroot.as_ref().map(PathBuf::from),
            declaration: json.declaration,
            source_map: json.source_map,
            emit: json
                .emit
                .as_ref()
                .map(|emit| emit.iter().copied().map(EmitArtifact::from).collect())
                .unwrap_or_default(),
            out_dir: json
                .out_dir
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_OUT_DIR)),
            out_file: json.out_file.as_ref().map(PathBuf::from),
            declaration_dir: json.declaration_dir.as_ref().map(PathBuf::from),
            module: json
                .module
                .as_deref()
                .and_then(ModuleTarget::parse)
                .unwrap_or_default(),
            es_target: json
                .target
                .as_deref()
                .and_then(EsTarget::parse)
                .unwrap_or_default(),
            lib: json.lib.clone(),
            types: json.types.clone(),
            profile: json.profile.clone(),
            debug: json.debug,
            optimize: json.optimize,
            optimize_level: json
                .optimize_level
                .map(OptimizeLevel::from)
                .unwrap_or_default(),
            unroll_threshold: json.unroll_threshold,
            inline_budget_scale_percent: json.inline_budget_scale_percent,
            lto_mode: json.lto_mode.map(LtoMode::from).unwrap_or_default(),
            shrink_level: json.shrink_level.map(ShrinkLevel::from).unwrap_or_default(),
            float_math: json
                .float_math
                .map(FloatMathPolicy::from)
                .unwrap_or(default_float_math),
            debug_info: json
                .debug_info
                .map(DebugInfoLevel::from)
                .unwrap_or_default(),
            debug_mode: json.debug_mode.map(DebugMode::from).unwrap_or_default(),
            osr_mode: json.osr_mode.map(OsrMode::from).unwrap_or_default(),
            safepoint_mode: json
                .safepoint_mode
                .map(SafepointMode::from)
                .unwrap_or_default(),
            safepoint_interval: json.safepoint_interval,
            speculation_mode: json
                .speculation_mode
                .map(SpeculationMode::from)
                .unwrap_or_default(),
            profiling_mode: json
                .profiling_mode
                .map(ProfilingMode::from)
                .unwrap_or_default(),
            runtime_options,
            trust_policy: json.trust_policy.map(TrustPolicy::from).unwrap_or_default(),
            sandbox_policy: json
                .sandbox_policy
                .map(SandboxPolicy::from)
                .unwrap_or_default(),
            strip: json.strip.map(StripLevel::from).unwrap_or_default(),
            panic: json.panic.map(PanicPolicy::from).unwrap_or_default(),
            unwind: json.unwind.map(UnwindFormat::from).unwrap_or_default(),
            safety_preset,
            overflow_checks: json
                .overflow_checks
                .map(OverflowCheckPolicy::from)
                .unwrap_or(default_checks.overflow),
            bounds_checks: json
                .bounds_checks
                .map(BoundsCheckPolicy::from)
                .unwrap_or(default_checks.bounds),
            null_checks: json
                .null_checks
                .map(NullCheckPolicy::from)
                .unwrap_or(default_checks.null),
            division_checks: json
                .division_checks
                .map(DivisionCheckPolicy::from)
                .unwrap_or(default_checks.division),
            shift_checks: json
                .shift_checks
                .map(ShiftCheckPolicy::from)
                .unwrap_or(default_checks.shift),
            check_failure: json
                .check_failure
                .map(CheckFailurePolicy::from)
                .unwrap_or_default(),
            allocator: json.allocator.map(Allocator::from).unwrap_or_default(),
        }
    }
}

impl From<&DsConfigTargetJson> for DsConfigTargetOptions {
    fn from(json: &DsConfigTargetJson) -> Self {
        Self::from_json_with_runtime(json, &RuntimeOptions::default())
    }
}

/// Destack build target configuration.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigTargetJson {
    // discovery
    /// Entry points for entry-based discovery (bundled/executable targets).
    /// If set, discovery mode is Entry; otherwise it's Include.
    pub entry: Option<Vec<String>>,
    /// Glob patterns for files to include (for include-based discovery).
    pub include: Option<Vec<String>>,
    /// Glob patterns for files to exclude.
    pub exclude: Option<Vec<String>>,

    // output format
    /// Output format (e.g., JavaScript, TypeScript, WebAssembly, Native).
    pub output: Option<OutputFormatJson>,
    /// Runtime environment (e.g., Browser, Node, Deno, Bun, Worker, Workerd).
    pub runtime: Option<String>,
    /// Runtime version for selecting versioned libs.
    pub runtime_version: Option<String>,
    /// Target platform / operating system.
    pub platform: Option<String>,
    /// Target triple for native codegen (e.g., "x86_64-unknown-linux-gnu").
    pub target_triple: Option<String>,
    /// Target architecture for native codegen (e.g., "x86_64", "aarch64").
    pub arch: Option<String>,
    /// Target vendor for native codegen (e.g., "apple", "pc", "unknown").
    pub vendor: Option<String>,
    /// Target environment / ABI for native codegen (e.g., "gnu", "musl", "msvc").
    pub env: Option<String>,
    /// CPU name for native codegen (e.g., "native", "x86-64", "znver3").
    pub cpu: Option<String>,
    /// CPU feature flags for native codegen (e.g., "+sse4.2", "+aes").
    pub cpu_features: Option<Vec<String>>,
    /// Relocation model.
    pub relocation_model: Option<RelocationModelJson>,
    /// Link mode.
    pub link_mode: Option<LinkModeJson>,
    /// Explicit linker executable.
    pub linker: Option<String>,
    /// Extra linker arguments.
    pub link_args: Option<Vec<String>>,
    /// Sysroot path for native toolchains.
    pub sysroot: Option<String>,
    /// Emit declaration files (e.g., `.d.ts` alongside `.js` output).
    #[serde(default)]
    pub declaration: bool,
    /// Emit source maps.
    #[serde(default)]
    pub source_map: bool,
    /// Extra artifacts to emit.
    pub emit: Option<Vec<EmitArtifactJson>>,

    // output paths
    /// Output directory for this target (overrides compilerOptions.outDir).
    pub out_dir: Option<String>,
    /// Output file for single-file targets like wasm (e.g., "./dist/core.wasm").
    pub out_file: Option<String>,
    /// Separate directory for declaration files (overrides compilerOptions.declarationDir).
    pub declaration_dir: Option<String>,

    // JS/TS specific
    /// Module format for this target (overrides compilerOptions.module).
    pub module: Option<String>,
    /// ECMAScript target for this target (overrides compilerOptions.target).
    pub target: Option<String>,
    /// Library files for this target (overrides derived libs).
    pub lib: Option<Vec<String>>,
    /// Additional library types for this target.
    pub types: Option<Vec<String>>,
    /// Explicit profile name for this target.
    pub profile: Option<String>,

    // optimization
    /// Whether this is a debug build.
    #[serde(default)]
    pub debug: bool,
    /// Whether optimization is enabled.
    #[serde(default)]
    pub optimize: bool,
    /// Optimization level (0-4).
    pub optimize_level: Option<u8>,
    /// Loop unroll threshold in instructions.
    pub unroll_threshold: Option<u64>,
    /// Inline budget scaling in percent.
    pub inline_budget_scale_percent: Option<u64>,
    /// Link time optimization mode.
    pub lto_mode: Option<LtoModeJson>,
    /// Shrink level (0-3).
    pub shrink_level: Option<u8>,
    /// Floating point math optimization policy.
    pub float_math: Option<FloatMathPolicyJson>,
    /// Debug info emission policy.
    pub debug_info: Option<DebugInfoLevelJson>,
    /// Debug execution mode for VM/native targets.
    pub debug_mode: Option<DebugModeJson>,
    /// OSR mode for native execution.
    pub osr_mode: Option<OsrModeJson>,
    /// Safepoint insertion mode for native execution.
    pub safepoint_mode: Option<SafepointModeJson>,
    /// Instruction interval for safepoint polling (when enabled).
    pub safepoint_interval: Option<u64>,
    /// Speculation mode for native optimization.
    pub speculation_mode: Option<SpeculationModeJson>,
    /// Profiling mode for tiering and optimization.
    pub profiling_mode: Option<ProfilingModeJson>,
    /// Execution mode for runtime scheduling and replay.
    #[serde(alias = "executionMode")]
    #[serde(alias = "execution_mode")]
    pub execution: Option<ExecutionModeJson>,
    /// Runtime options overrides for this target.
    #[serde(alias = "runtimeOptions")]
    pub runtime_options: Option<DsConfigRuntimeOptionsJson>,
    /// Trust policy for runtime execution.
    pub trust_policy: Option<TrustPolicyJson>,
    /// Sandbox policy for runtime isolation.
    pub sandbox_policy: Option<SandboxPolicyJson>,
    /// Symbol stripping policy.
    pub strip: Option<StripLevelJson>,
    /// Panic policy.
    pub panic: Option<PanicPolicyJson>,
    /// Unwind info format.
    pub unwind: Option<UnwindFormatJson>,
    /// Safety preset that configures runtime checks.
    pub safety_preset: Option<SafetyPresetJson>,
    /// Overflow checking policy.
    pub overflow_checks: Option<OverflowCheckPolicyJson>,
    /// Bounds check policy.
    pub bounds_checks: Option<BoundsCheckPolicyJson>,
    /// Null check policy.
    pub null_checks: Option<NullCheckPolicyJson>,
    /// Division check policy.
    pub division_checks: Option<DivisionCheckPolicyJson>,
    /// Shift range check policy.
    pub shift_checks: Option<ShiftCheckPolicyJson>,
    /// Check failure behavior.
    pub check_failure: Option<CheckFailurePolicyJson>,
    /// Global allocator selection.
    pub allocator: Option<AllocatorJson>,
}

/// Output format for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OutputFormatJson {
    /// JavaScript (.js).
    #[serde(alias = "javascript")]
    Js,
    /// TypeScript (.ts).
    #[serde(alias = "typescript")]
    Ts,
    /// WebAssembly (.wasm).
    #[serde(alias = "webassembly")]
    Wasm,
    /// Native binary.
    #[serde(alias = "binary")]
    Native,
}

impl From<OutputFormatJson> for OutputFormat {
    fn from(value: OutputFormatJson) -> Self {
        match value {
            OutputFormatJson::Js => OutputFormat::Js,
            OutputFormatJson::Ts => OutputFormat::Ts,
            OutputFormatJson::Wasm => OutputFormat::Wasm,
            OutputFormatJson::Native => OutputFormat::Native,
        }
    }
}

/// Extra artifacts to emit for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EmitArtifactJson {
    /// Lowered MIR.
    Mir,
    /// Backend IR.
    Ir,
    /// Assembly output.
    Asm,
    /// Object file output.
    Object,
    /// Symbol table output.
    Symbols,
}

impl From<EmitArtifactJson> for EmitArtifact {
    fn from(value: EmitArtifactJson) -> Self {
        match value {
            EmitArtifactJson::Mir => EmitArtifact::Mir,
            EmitArtifactJson::Ir => EmitArtifact::Ir,
            EmitArtifactJson::Asm => EmitArtifact::Asm,
            EmitArtifactJson::Object => EmitArtifact::Object,
            EmitArtifactJson::Symbols => EmitArtifact::Symbols,
        }
    }
}

/// Relocation model for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RelocationModelJson {
    /// Static relocation model.
    #[serde(alias = "static")]
    Static,
    /// Position-independent code.
    #[serde(alias = "position_independent")]
    #[serde(alias = "position_independent_code")]
    #[serde(alias = "position_independent_executable")]
    Pic,
    /// Position-independent executable.
    #[serde(alias = "pie")]
    Pie,
}

impl From<RelocationModelJson> for RelocationModel {
    fn from(value: RelocationModelJson) -> Self {
        match value {
            RelocationModelJson::Static => RelocationModel::Static,
            RelocationModelJson::Pic => RelocationModel::Pic,
            RelocationModelJson::Pie => RelocationModel::Pie,
        }
    }
}

/// Link mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LinkModeJson {
    /// Prefer static linking.
    #[serde(alias = "static")]
    Static,
    /// Prefer dynamic linking.
    #[serde(alias = "shared")]
    Dynamic,
}

impl From<LinkModeJson> for LinkMode {
    fn from(value: LinkModeJson) -> Self {
        match value {
            LinkModeJson::Static => LinkMode::Static,
            LinkModeJson::Dynamic => LinkMode::Dynamic,
        }
    }
}

/// Debug info emission policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DebugInfoLevelJson {
    /// No debug info.
    #[serde(alias = "off")]
    None,
    /// Line tables only.
    #[serde(alias = "lines")]
    #[serde(alias = "line_tables")]
    Line,
    /// Full debug info.
    #[serde(alias = "full")]
    Full,
}

impl From<DebugInfoLevelJson> for DebugInfoLevel {
    fn from(value: DebugInfoLevelJson) -> Self {
        match value {
            DebugInfoLevelJson::None => DebugInfoLevel::None,
            DebugInfoLevelJson::Line => DebugInfoLevel::Line,
            DebugInfoLevelJson::Full => DebugInfoLevel::Full,
        }
    }
}

/// Debug execution mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DebugModeJson {
    /// Choose mode based on target debug settings.
    #[serde(alias = "default")]
    Auto,
    /// Always run the interpreter.
    #[serde(alias = "interpreter")]
    Vm,
    /// Run native with deopt-first debugging.
    Deopt,
    /// Run native only (no deopt).
    Native,
}

impl From<DebugModeJson> for DebugMode {
    fn from(value: DebugModeJson) -> Self {
        match value {
            DebugModeJson::Auto => DebugMode::Auto,
            DebugModeJson::Vm => DebugMode::Vm,
            DebugModeJson::Deopt => DebugMode::Deopt,
            DebugModeJson::Native => DebugMode::Native,
        }
    }
}

/// OSR mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OsrModeJson {
    /// OSR disabled.
    #[serde(alias = "off")]
    Disabled,
    /// OSR at loop headers.
    #[serde(alias = "loops")]
    #[serde(alias = "loop-headers")]
    LoopHeaders,
    /// OSR only at explicit sites.
    Explicit,
}

impl From<OsrModeJson> for OsrMode {
    fn from(value: OsrModeJson) -> Self {
        match value {
            OsrModeJson::Disabled => OsrMode::Disabled,
            OsrModeJson::LoopHeaders => OsrMode::LoopHeaders,
            OsrModeJson::Explicit => OsrMode::Explicit,
        }
    }
}

/// Safepoint mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SafepointModeJson {
    /// Safepoints at calls, allocations, and loop back-edges.
    #[serde(rename = "calls-alloc-backedges")]
    #[serde(alias = "calls_alloc_backedges")]
    #[serde(alias = "standard")]
    CallsAllocBackEdges,
    /// Add instruction-budget safepoints for bounded latency.
    #[serde(alias = "budget")]
    Budgeted,
}

impl From<SafepointModeJson> for SafepointMode {
    fn from(value: SafepointModeJson) -> Self {
        match value {
            SafepointModeJson::CallsAllocBackEdges => SafepointMode::CallsAllocBackEdges,
            SafepointModeJson::Budgeted => SafepointMode::Budgeted,
        }
    }
}

/// Speculation mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SpeculationModeJson {
    /// Disable speculative optimizations.
    #[serde(alias = "off")]
    None,
    /// Guarded speculations with explicit deopt metadata.
    Guarded,
    /// Aggressive speculation across more sites.
    Aggressive,
}

impl From<SpeculationModeJson> for SpeculationMode {
    fn from(value: SpeculationModeJson) -> Self {
        match value {
            SpeculationModeJson::None => SpeculationMode::None,
            SpeculationModeJson::Guarded => SpeculationMode::Guarded,
            SpeculationModeJson::Aggressive => SpeculationMode::Aggressive,
        }
    }
}

/// Profiling mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ProfilingModeJson {
    /// Disable runtime profiling.
    #[serde(alias = "off")]
    None,
    /// Counters only.
    Counters,
    /// Sampling only.
    Sampling,
    /// Counters + sampling + inline caches.
    #[serde(alias = "full")]
    Hybrid,
}

impl From<ProfilingModeJson> for ProfilingMode {
    fn from(value: ProfilingModeJson) -> Self {
        match value {
            ProfilingModeJson::None => ProfilingMode::None,
            ProfilingModeJson::Counters => ProfilingMode::Counters,
            ProfilingModeJson::Sampling => ProfilingMode::Sampling,
            ProfilingModeJson::Hybrid => ProfilingMode::Hybrid,
        }
    }
}

/// Link time optimization mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LtoModeJson {
    /// Choose mode based on optimization level.
    #[serde(alias = "auto")]
    Auto,
    /// Disable link time optimization.
    #[serde(alias = "none")]
    #[serde(alias = "off")]
    #[serde(alias = "disabled")]
    None,
    /// Enable thin link time optimization.
    #[serde(alias = "thin")]
    #[serde(alias = "thinlto")]
    #[serde(alias = "thin_lto")]
    Thin,
    /// Enable full link time optimization.
    #[serde(alias = "full")]
    #[serde(alias = "lto")]
    Full,
}

impl From<LtoModeJson> for LtoMode {
    fn from(value: LtoModeJson) -> Self {
        match value {
            LtoModeJson::Auto => LtoMode::Auto,
            LtoModeJson::None => LtoMode::None,
            LtoModeJson::Thin => LtoMode::Thin,
            LtoModeJson::Full => LtoMode::Full,
        }
    }
}

/// Symbol stripping policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum StripLevelJson {
    /// Keep all symbols.
    #[serde(alias = "none")]
    None,
    /// Strip local symbols.
    #[serde(alias = "locals")]
    Partial,
    /// Strip all symbols.
    #[serde(alias = "all")]
    Full,
}

impl From<StripLevelJson> for StripLevel {
    fn from(value: StripLevelJson) -> Self {
        match value {
            StripLevelJson::None => StripLevel::None,
            StripLevelJson::Partial => StripLevel::Partial,
            StripLevelJson::Full => StripLevel::Full,
        }
    }
}

/// Allocator selection for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum AllocatorJson {
    /// Use the platform default allocator.
    #[serde(alias = "system")]
    System,
    /// Use mimalloc.
    #[serde(alias = "mimalloc")]
    #[serde(alias = "mi_malloc")]
    MiMalloc,
    /// Use jemalloc.
    #[serde(alias = "jemalloc")]
    #[serde(alias = "je_malloc")]
    JeMalloc,
    /// Use a custom allocator provided by the runtime.
    #[serde(alias = "custom")]
    Custom,
}

impl From<AllocatorJson> for Allocator {
    fn from(value: AllocatorJson) -> Self {
        match value {
            AllocatorJson::System => Allocator::System,
            AllocatorJson::MiMalloc => Allocator::MiMalloc,
            AllocatorJson::JeMalloc => Allocator::JeMalloc,
            AllocatorJson::Custom => Allocator::Custom,
        }
    }
}
