use std::path::{Path, PathBuf};

use destack_artifact::{EmitFormat, TargetOutputKind, TargetOutputName};
use destack_source::TargetId;
use indexmap::IndexMap;
use serde::Deserialize;

use crate::CompilerOptions;

use super::super::policy::{
    BoundsCheckPolicy, BoundsCheckPolicyJson, CheckFailurePolicy, CheckFailurePolicyJson,
    DivisionCheckPolicy, DivisionCheckPolicyJson, ExecutionMode, ExecutionModeJson,
    FloatMathPolicy, FloatMathPolicyJson, NullCheckPolicy, NullCheckPolicyJson,
    OverflowCheckPolicy, OverflowCheckPolicyJson, PanicPolicy, PanicPolicyJson, SafetyPreset,
    SafetyPresetJson, SandboxPolicy, SandboxPolicyJson, ShiftCheckPolicy, ShiftCheckPolicyJson,
    TrustPolicy, TrustPolicyJson, UnwindFormat, UnwindFormatJson,
};
use super::super::runtime::{
    RuntimeAppDeclaration, RuntimeConfigJson, RuntimeOptions, runtime_options_with_base,
};
use super::super::tsconfig::{EsTarget, ModuleTarget};

use super::app::*;
use super::bundle::*;
use super::execution::*;
use super::native::*;
use super::optimization::*;
use super::output::*;

/// Default output directory for targets.
const DEFAULT_TARGET_OUT_DIR: &str = "dist";

/// A build target configuration.
///
/// Can be constructed from `destack.json` or programmatically.
/// This is the type used by compiler/codegen, independent of config parsing.
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
    pub module: ModuleTarget,
    /// ECMAScript target version.
    pub es_target: EsTarget,
    /// Library files for this target. If `None`, derived automatically from runtime and platform.
    pub lib: Option<Vec<String>>,
    /// Additional library types for this target.
    pub types: Option<Vec<String>>,
    /// Explicit profile name for this target.
    pub profile: Option<String>,
    /// Explicit mode name for this target.
    pub mode: Option<String>,
    /// Assembly topology for this script target.
    pub assembly: BundleMode,
    /// Whether to preserve one emitted module file per reachable module.
    pub preserve_modules: bool,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<PathBuf>,
    /// Manual chunk assignments keyed by chunk name.
    pub manual_chunks: IndexMap<String, Vec<String>>,
    /// Whether to only honor explicit manual chunk declarations.
    pub only_explicit_manual_chunks: bool,
    /// Dependency and resolution options.
    pub bundle_dependencies: TargetDependencyOptions,
    /// Asset handling options.
    pub bundle_assets: TargetAssetOptions,
    /// Tree shaking options.
    pub treeshake: TargetTreeshakeOptions,
    /// Output configuration for bundled products.
    pub bundle_output: TargetOutputPolicy,
    /// Compile time define replacements.
    pub define: IndexMap<String, String>,
    /// Minification options.
    pub minify: TargetMinifyOptions,
    /// App declaration for packaging and runtime capability planning.
    pub app: TargetAppDeclaration,
    /// Formal named outputs published by this target.
    pub outputs: TargetOutputs,
    /// Emitted artifact family (js, ts, html, wasm, native).
    pub emit: EmitFormat,
    /// Runtime environment (browser, node, wasm-wasi, native-managed, etc.).
    pub runtime: Runtime,
    /// Runtime version for selecting versioned libs.
    pub runtime_version: Option<String>,
    /// Target platform / operating system.
    pub platform: Platform,
    /// Target architecture for native codegen (e.g., "x86_64", "aarch64").
    pub target_arch: Option<TargetArch>,
    /// Target vendor for native codegen (e.g., "apple", "pc", "unknown").
    pub target_vendor: Option<TargetVendor>,
    /// Target ABI for native codegen (e.g., "gnu", "musl", "msvc").
    pub target_abi: Option<TargetAbi>,
    /// CPU name for native codegen (e.g., "native", "x86-64", "znver3").
    pub cpu: Option<String>,
    /// CPU feature flags for native codegen (e.g., "+sse4.2", "+aes").
    pub cpu_features: Vec<String>,
    /// Relocation model for native codegen.
    pub relocation_model: RelocationModel,
    /// Native output kind for this target.
    pub native_output: TargetNativeOutputKind,
    /// Link mode for native targets.
    pub link_mode: LinkMode,
    /// Explicit linker executable for native targets.
    pub linker: Option<String>,
    /// Linker driver family.
    pub linker_flavor: TargetLinkerFlavor,
    /// Extra linker arguments for native targets.
    pub link_args: Vec<String>,
    /// Sysroot path for native toolchains.
    pub sysroot: Option<PathBuf>,
    /// Additional library search paths.
    pub library_search_paths: Vec<PathBuf>,
    /// Additional libraries to link.
    pub libraries: Vec<String>,
    /// Additional framework search paths.
    pub framework_search_paths: Vec<PathBuf>,
    /// Additional frameworks to link.
    pub frameworks: Vec<String>,
    /// Runtime search paths embedded into the final output.
    pub rpath: Vec<String>,
    /// Runtime search paths emitted as runpath entries.
    pub runpath: Vec<String>,
    /// Explicit entry symbol override.
    pub entry_symbol: Option<String>,
    /// Explicitly exported symbol names.
    pub export_symbols: Vec<String>,
    /// Symbol visibility policy.
    pub symbol_visibility: TargetSymbolVisibility,
    /// Version script for exported symbols.
    pub version_script: Option<PathBuf>,
    /// Linker script for the final link.
    pub linker_script: Option<PathBuf>,
    /// Position independent code policy.
    pub position_independent: TargetPositionIndependentMode,
    /// C runtime linkage policy.
    pub crt: TargetCrtLinkage,
    /// Shared object soname.
    pub soname: Option<String>,
    /// Darwin install name.
    pub install_name: Option<String>,
    /// Emit declaration files (e.g., `.d.ts` alongside `.js` output).
    pub declaration: bool,
    /// Source map emission mode.
    pub source_map_mode: Option<SourceMapMode>,
    /// Extra sidecar artifacts to emit.
    pub artifacts: Vec<EmitArtifact>,

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

impl std::hash::Hash for Target {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.discovery.hash(state);
        self.entry.hash(state);
        self.include.hash(state);
        self.exclude.hash(state);
        self.module.hash(state);
        self.es_target.hash(state);
        self.lib.hash(state);
        self.types.hash(state);
        self.profile.hash(state);
        self.mode.hash(state);
        self.assembly.hash(state);
        self.preserve_modules.hash(state);
        self.preserve_modules_root.hash(state);
        self.only_explicit_manual_chunks.hash(state);
        self.bundle_dependencies.hash(state);
        self.bundle_assets.hash(state);
        self.treeshake.hash(state);
        self.bundle_output.hash(state);
        self.minify.hash(state);
        self.app.hash(state);
        self.outputs.hash(state);
        self.emit.hash(state);
        self.runtime.hash(state);
        self.runtime_version.hash(state);
        self.platform.hash(state);
        self.target_arch.hash(state);
        self.target_vendor.hash(state);
        self.target_abi.hash(state);
        self.cpu.hash(state);
        self.cpu_features.hash(state);
        self.relocation_model.hash(state);
        self.native_output.hash(state);
        self.link_mode.hash(state);
        self.linker.hash(state);
        self.linker_flavor.hash(state);
        self.link_args.hash(state);
        self.sysroot.hash(state);
        self.library_search_paths.hash(state);
        self.libraries.hash(state);
        self.framework_search_paths.hash(state);
        self.frameworks.hash(state);
        self.rpath.hash(state);
        self.runpath.hash(state);
        self.entry_symbol.hash(state);
        self.export_symbols.hash(state);
        self.symbol_visibility.hash(state);
        self.version_script.hash(state);
        self.linker_script.hash(state);
        self.position_independent.hash(state);
        self.crt.hash(state);
        self.soname.hash(state);
        self.install_name.hash(state);
        self.declaration.hash(state);
        self.source_map_mode.hash(state);
        self.artifacts.hash(state);
        self.debug.hash(state);
        self.optimize.hash(state);
        self.optimize_level.hash(state);
        self.unroll_threshold.hash(state);
        self.inline_budget_scale_percent.hash(state);
        self.lto_mode.hash(state);
        self.shrink_level.hash(state);
        self.float_math.hash(state);
        self.debug_info.hash(state);
        self.debug_mode.hash(state);
        self.safepoint_mode.hash(state);
        self.safepoint_interval.hash(state);
        self.speculation_mode.hash(state);
        self.profiling_mode.hash(state);
        self.runtime_options.hash(state);
        self.trust_policy.hash(state);
        self.sandbox_policy.hash(state);
        self.strip.hash(state);
        self.panic.hash(state);
        self.unwind.hash(state);
        self.safety_preset.hash(state);
        self.overflow_checks.hash(state);
        self.bounds_checks.hash(state);
        self.null_checks.hash(state);
        self.division_checks.hash(state);
        self.shift_checks.hash(state);
        self.check_failure.hash(state);
        self.allocator.hash(state);
        self.out_dir.hash(state);
        self.out_file.hash(state);
        self.declaration_dir.hash(state);

        self.manual_chunks.len().hash(state);
        for (name, modules) in &self.manual_chunks {
            name.hash(state);
            modules.hash(state);
        }

        self.define.len().hash(state);
        for (name, value) in &self.define {
            name.hash(state);
            value.hash(state);
        }
    }
}

impl Default for Target {
    fn default() -> Self {
        Self::native("default")
    }
}

impl Target {
    /// Create a target from default target options.
    fn from_default_options(name: impl Into<String>) -> Self {
        let name = name.into();

        TargetOptions::default().to_target(name.as_str())
    }

    /// Return the known implicit target names.
    pub fn implicit_target_names() -> &'static [&'static str] {
        &[
            "default",
            "js",
            "ts",
            "html",
            "node",
            "wasm",
            "wasm-wasi",
            "wasi",
            "native",
        ]
    }

    /// Create an implicit target configuration for a known target id.
    pub fn implicit_for_id(target_id: TargetId) -> Option<Self> {
        let package_id = target_id.package_id();

        for name in Self::implicit_target_names() {
            let implicit_target_id = TargetId::new(package_id, name);

            if implicit_target_id == target_id {
                return Self::implicit_for_name(name);
            }
        }

        None
    }

    /// Create a new target with the given name and default JS output.
    pub fn js(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.outputs = default_target_outputs(EmitFormat::Js, false, true, None, false);
        target.emit = EmitFormat::Js;
        target.runtime = Runtime::Node;
        target.runtime_options.host = Runtime::Node;
        target.platform = Platform::Web;
        target.declaration = true;

        target
    }

    /// Create a new target with the given name and TypeScript output.
    pub fn ts(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.outputs = default_target_outputs(EmitFormat::Ts, false, false, None, false);
        target.emit = EmitFormat::Ts;
        target.runtime = Runtime::Node;
        target.runtime_options.host = Runtime::Node;
        target.platform = Platform::Web;

        target
    }

    /// Create a new target with the given name and HTML document output.
    pub fn html(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.outputs = default_target_outputs(EmitFormat::Html, true, false, None, false);
        target.emit = EmitFormat::Html;
        target.runtime = Runtime::Browser;
        target.runtime_options.host = Runtime::Browser;
        target.platform = Platform::Web;

        target
    }

    /// Create a new target with the given name and JS output for Node.js.
    pub fn node(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.outputs = default_target_outputs(EmitFormat::Js, false, true, None, false);
        target.emit = EmitFormat::Js;
        target.runtime = Runtime::Node;
        target.runtime_options.host = Runtime::Node;
        target.platform = Platform::Universal;
        target.declaration = true;

        target
    }

    /// Create a new target with the given name and WASM output for JS host.
    pub fn wasm_js(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.outputs = default_target_outputs(EmitFormat::Wasm, true, false, None, false);
        target.emit = EmitFormat::Wasm;
        target.runtime = Runtime::WasmJs;
        target.runtime_options.host = Runtime::WasmJs;
        target.platform = Platform::Web;
        target.optimize = true;

        target
    }

    /// Create a new target with the given name and WASM output for WASI.
    pub fn wasm_wasi(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.outputs = default_target_outputs(EmitFormat::Wasm, true, false, None, false);
        target.emit = EmitFormat::Wasm;
        target.runtime = Runtime::WasmWasi;
        target.runtime_options.host = Runtime::WasmWasi;
        target.platform = Platform::Wasi;
        target.optimize = true;

        target
    }

    /// Create a new target with the given name and native output.
    pub fn native(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.outputs = default_target_outputs(EmitFormat::Native, true, false, None, false);
        target.emit = EmitFormat::Native;
        target.runtime = Runtime::NativeManaged;
        target.runtime_options.host = Runtime::NativeManaged;
        target.platform = Platform::Universal;
        target.optimize = true;

        target
    }

    /// Create a new target for comptime execution.
    pub fn comptime(name: impl Into<String>) -> Self {
        let mut target = Self::native(name);
        target.trust_policy = TrustPolicy::Internal;

        target
    }

    /// Create a new target with the given name and native freestanding output.
    pub fn native_freestanding(name: impl Into<String>) -> Self {
        let mut target = Self::native(name);
        target.runtime = Runtime::NativeFreestanding;
        target.runtime_options.host = Runtime::NativeFreestanding;
        target.platform = Platform::BareMetal;

        target
    }

    /// Create a new target with the given name and native embedded output.
    pub fn native_embedded(name: impl Into<String>) -> Self {
        let mut target = Self::native(name);
        target.runtime = Runtime::NativeEmbedded;
        target.runtime_options.host = Runtime::NativeEmbedded;
        target.platform = Platform::BareMetal;

        target
    }

    /// Create an implicit target configuration for a known target name.
    pub fn implicit_for_name(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::default()),
            "js" => Some(Self::js(name)),
            "ts" => Some(Self::ts(name)),
            "html" => Some(Self::html(name)),
            "node" => Some(Self::node(name)),
            "wasm" => Some(Self::wasm_js(name)),
            "wasm-wasi" | "wasi" => Some(Self::wasm_wasi(name)),
            "native" => Some(Self::native(name)),
            _ => None,
        }
    }

    /// Derive the output mode from the target configuration.
    pub fn output_mode(&self) -> OutputMode {
        if self.out_file.is_some() || self.emit.is_single_file() {
            return OutputMode::File;
        }

        OutputMode::Directory
    }

    /// Whether this target produces single-file output.
    pub fn is_single_file(&self) -> bool {
        self.output_mode() == OutputMode::File
    }

    /// Whether this target produces directory output (one file per source file).
    pub fn is_directory(&self) -> bool {
        self.output_mode() == OutputMode::Directory
    }

    /// Build compiler options adjusted for this target.
    pub fn compiler_options(&self, compiler_options: &CompilerOptions) -> CompilerOptions {
        let mut compiler_options = compiler_options.clone();
        let is_native_output = self.emit.is_wasm() || self.emit.is_native();

        // native outputs force stricter semantics
        if is_native_output {
            compiler_options.apply_native_restrictions();
        }

        compiler_options
    }

    /// Return whether this target uses the JavaScript generation pipeline.
    pub fn uses_js_generate_pipeline(&self) -> bool {
        self.emit.is_js_family()
    }

    /// Return whether this target uses the native generation pipeline.
    pub fn uses_native_generate_pipeline(&self) -> bool {
        self.emit.is_native_family()
    }

    /// Return whether this target assembles one target level output shape.
    pub fn emits_assembled_output(&self) -> bool {
        is_assembled_target(
            Some(self.assembly),
            self.discovery,
            self.entry.len(),
            &self.app,
            self.emit,
            self.preserve_modules,
            self.manual_chunks.is_empty(),
            self.out_file.is_some(),
        )
    }

    /// Return whether this target emits one file per source module.
    pub fn emits_per_module_output(&self) -> bool {
        !self.emits_assembled_output()
    }

    /// Return whether this target should minify assembled JavaScript, HTML, or CSS output text.
    pub fn should_minify_bundle_output(&self) -> bool {
        self.minify.minifies_output()
    }

    /// Return whether this target should compact printed bundled JavaScript output.
    pub fn should_minify_bundle_script_output(&self) -> bool {
        self.minify.enabled || self.minify.whitespace
    }

    /// Return whether this target should structurally minify bundled JavaScript syntax.
    pub fn should_minify_bundle_script_syntax(&self) -> bool {
        self.minify.enabled || self.minify.syntax
    }

    /// Return whether this target should structurally minify bundled CSS output.
    pub fn should_minify_bundle_css_syntax(&self) -> bool {
        self.minify.enabled || self.minify.syntax
    }

    /// Return whether this target should compact bundled CSS output whitespace.
    pub fn should_minify_bundle_css_whitespace(&self) -> bool {
        self.minify.enabled || self.minify.whitespace
    }

    /// Return whether this target should structurally minify bundled HTML output.
    pub fn should_minify_bundle_html_output(&self) -> bool {
        self.minify.minifies_output()
    }

    /// Return whether this target emits any source map data.
    pub fn emits_source_maps(&self) -> bool {
        self.source_map_mode().is_some()
    }

    /// Return whether this target emits standalone source map outputs.
    pub fn emits_source_map_output(&self) -> bool {
        self.source_map_mode()
            .is_some_and(SourceMapMode::emits_output)
    }

    /// Return whether this target inlines source maps into text outputs.
    pub fn uses_inline_source_maps(&self) -> bool {
        self.source_map_mode().is_some_and(SourceMapMode::is_inline)
    }

    /// Return whether this target publishes one binary payload.
    pub fn publishes_binary_output(&self) -> bool {
        self.outputs.contains_kind(TargetOutputKind::Binary)
    }

    /// Return the normalized source map mode for this target.
    pub fn source_map_mode(&self) -> Option<SourceMapMode> {
        self.bundle_output.sourcemap.or(self.source_map_mode)
    }

    /// Resolve a target triple string from the target configuration.
    pub fn resolved_target_triple(&self) -> Option<String> {
        let target_arch = self.target_arch.as_ref()?;
        let os = self.platform.triple_os_component()?;

        let vendor = self
            .target_vendor
            .clone()
            .unwrap_or_else(|| TargetVendor::default_for_platform(self.platform));
        let env = self
            .target_abi
            .clone()
            .or_else(|| TargetAbi::default_for_platform(self.platform));

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
        self.source_map_mode = source_map.then_some(SourceMapMode::External);
        self
    }

    /// Set the source map emission mode.
    pub fn with_source_map_mode(mut self, source_map_mode: Option<SourceMapMode>) -> Self {
        self.source_map_mode = source_map_mode;
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

    /// Set an explicit mode name for this target.
    pub fn with_mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = Some(mode.into());
        self
    }

    /// Set the runtime.
    pub fn with_runtime(mut self, runtime: Runtime) -> Self {
        self.runtime = runtime;
        self.runtime_options.host = runtime;
        self
    }

    /// Set the runtime version.
    pub fn with_runtime_version(mut self, runtime_version: impl Into<String>) -> Self {
        let runtime_version = runtime_version.into();
        self.runtime_version = Some(runtime_version.clone());
        self.runtime_options.version = Some(runtime_version);
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

    /// Set target ABI for native codegen.
    pub fn with_target_abi(mut self, target_abi: TargetAbi) -> Self {
        self.target_abi = Some(target_abi);
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
            | Runtime::NativeManaged
            | Runtime::NativeFreestanding
            | Runtime::NativeEmbedded => {
                libs.push("native".to_string());
            }
        }

        if self.emit.is_js() || self.emit.is_ts() || self.emit.is_html() {
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

        // module path
        let relative = self.relative_module_output_path(package_dir, root_dir, module_path);

        // change extension and join with output directory
        out_dir.join(relative.with_extension(extension))
    }

    /// Return the relative output path for one emitted module artifact.
    fn relative_module_output_path(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        module_path: &Path,
    ) -> PathBuf {
        let relative = module_path.strip_prefix(package_dir).unwrap_or(module_path);

        // preserve modules root
        if let Some(relative) = self.strip_preserve_modules_root(package_dir, module_path, relative)
        {
            return relative.to_path_buf();
        }

        // compiler root dir
        if let Some(root_dir) = root_dir {
            return relative
                .strip_prefix(root_dir)
                .unwrap_or(relative)
                .to_path_buf();
        }

        relative.to_path_buf()
    }

    /// Strip the configured preserve-modules root from one module path when possible.
    fn strip_preserve_modules_root<'a>(
        &self,
        package_dir: &Path,
        module_path: &'a Path,
        package_relative_path: &'a Path,
    ) -> Option<&'a Path> {
        let preserve_modules_root = self.preserve_modules_root.as_deref()?;

        // package relative root
        if let Ok(relative) = package_relative_path.strip_prefix(preserve_modules_root) {
            return Some(relative);
        }

        // absolute root
        if preserve_modules_root.is_absolute() {
            return module_path.strip_prefix(preserve_modules_root).ok();
        }

        let absolute_root = package_dir.join(preserve_modules_root);

        module_path.strip_prefix(&absolute_root).ok()
    }
}

/// Normalized Destack build target options.
#[derive(Debug, Clone)]
pub struct TargetOptions {
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
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
    /// Emitted artifact family (js, ts, html, wasm, native).
    pub emit: EmitFormat,
    /// Runtime environment (browser, node, wasm-wasi, native-managed, etc.).
    pub runtime: Runtime,
    /// Runtime version for selecting versioned libs.
    pub runtime_version: Option<String>,
    /// Target platform / operating system.
    pub platform: Platform,
    /// Target architecture for native codegen.
    pub target_arch: Option<TargetArch>,
    /// Target vendor for native codegen.
    pub target_vendor: Option<TargetVendor>,
    /// Target ABI for native codegen.
    pub target_abi: Option<TargetAbi>,
    /// CPU name for native codegen.
    pub cpu: Option<String>,
    /// CPU feature flags for native codegen.
    pub cpu_features: Vec<String>,
    /// Relocation model for native codegen.
    pub relocation_model: RelocationModel,
    /// Native output kind for this target.
    pub native_output: TargetNativeOutputKind,
    /// Link mode for native targets.
    pub link_mode: LinkMode,
    /// Explicit linker executable for native targets.
    pub linker: Option<String>,
    /// Linker driver family.
    pub linker_flavor: TargetLinkerFlavor,
    /// Extra linker arguments for native targets.
    pub link_args: Vec<String>,
    /// Sysroot path for native toolchains.
    pub sysroot: Option<PathBuf>,
    /// Additional library search paths.
    pub library_search_paths: Vec<PathBuf>,
    /// Additional libraries to link.
    pub libraries: Vec<String>,
    /// Additional framework search paths.
    pub framework_search_paths: Vec<PathBuf>,
    /// Additional frameworks to link.
    pub frameworks: Vec<String>,
    /// Runtime search paths embedded into the final output.
    pub rpath: Vec<String>,
    /// Runtime search paths emitted as runpath entries.
    pub runpath: Vec<String>,
    /// Explicit entry symbol override.
    pub entry_symbol: Option<String>,
    /// Explicitly exported symbol names.
    pub export_symbols: Vec<String>,
    /// Symbol visibility policy.
    pub symbol_visibility: TargetSymbolVisibility,
    /// Version script for exported symbols.
    pub version_script: Option<PathBuf>,
    /// Linker script for the final link.
    pub linker_script: Option<PathBuf>,
    /// Position independent code policy.
    pub position_independent: TargetPositionIndependentMode,
    /// C runtime linkage policy.
    pub crt: TargetCrtLinkage,
    /// Shared object soname.
    pub soname: Option<String>,
    /// Darwin install name.
    pub install_name: Option<String>,
    /// Emit declaration files (e.g., `.d.ts` alongside `.js` output).
    pub declaration: bool,
    /// Source map emission mode.
    pub source_map_mode: Option<SourceMapMode>,
    /// Extra sidecar artifacts to emit.
    pub artifacts: Vec<EmitArtifact>,

    // output paths
    /// Output directory for this target (defaults to "dist").
    pub out_dir: PathBuf,
    /// Output file for single-file targets like html or wasm.
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
    /// Explicit mode name for this target.
    pub mode: Option<String>,
    /// Assembly topology for this script target.
    pub assembly: BundleMode,
    /// Whether to preserve one emitted module file per reachable module.
    pub preserve_modules: bool,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<PathBuf>,
    /// Manual chunk assignments keyed by chunk name.
    pub manual_chunks: IndexMap<String, Vec<String>>,
    /// Whether to only honor explicit manual chunk declarations.
    pub only_explicit_manual_chunks: bool,
    /// Dependency and resolution options.
    pub bundle_dependencies: TargetDependencyOptions,
    /// Asset handling options.
    pub bundle_assets: TargetAssetOptions,
    /// Tree shaking options.
    pub treeshake: TargetTreeshakeOptions,
    /// Output configuration for bundled products.
    pub bundle_output: TargetOutputPolicy,
    /// Compile time define replacements.
    pub define: IndexMap<String, String>,
    /// Minification options.
    pub minify: TargetMinifyOptions,
    /// App declaration for packaging and runtime capability planning.
    pub app: TargetAppDeclaration,
    /// Formal named outputs published by this target.
    pub outputs: TargetOutputs,

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

impl Default for TargetOptions {
    fn default() -> Self {
        Self {
            labels: IndexMap::new(),
            annotations: IndexMap::new(),
            discovery: TargetDiscovery::default(),
            entry: Vec::new(),
            include: Vec::new(),
            exclude: Vec::new(),
            emit: EmitFormat::default(),
            runtime: Runtime::default(),
            runtime_version: None,
            platform: Platform::default(),
            target_arch: None,
            target_vendor: None,
            target_abi: None,
            cpu: None,
            cpu_features: Vec::new(),
            relocation_model: RelocationModel::default(),
            native_output: TargetNativeOutputKind::default(),
            link_mode: LinkMode::default(),
            linker: None,
            linker_flavor: TargetLinkerFlavor::default(),
            link_args: Vec::new(),
            sysroot: None,
            library_search_paths: Vec::new(),
            libraries: Vec::new(),
            framework_search_paths: Vec::new(),
            frameworks: Vec::new(),
            rpath: Vec::new(),
            runpath: Vec::new(),
            entry_symbol: None,
            export_symbols: Vec::new(),
            symbol_visibility: TargetSymbolVisibility::default(),
            version_script: None,
            linker_script: None,
            position_independent: TargetPositionIndependentMode::default(),
            crt: TargetCrtLinkage::default(),
            soname: None,
            install_name: None,
            declaration: false,
            source_map_mode: None,
            artifacts: Vec::new(),
            out_dir: PathBuf::from(DEFAULT_TARGET_OUT_DIR),
            out_file: None,
            declaration_dir: None,
            module: ModuleTarget::default(),
            es_target: EsTarget::default(),
            lib: None,
            types: None,
            profile: None,
            mode: None,
            assembly: BundleMode::default(),
            preserve_modules: false,
            preserve_modules_root: None,
            manual_chunks: IndexMap::new(),
            only_explicit_manual_chunks: false,
            bundle_dependencies: TargetDependencyOptions::default(),
            bundle_assets: TargetAssetOptions::default(),
            treeshake: TargetTreeshakeOptions::default(),
            bundle_output: TargetOutputPolicy::default(),
            define: IndexMap::new(),
            minify: TargetMinifyOptions::default(),
            app: TargetAppDeclaration::default(),
            outputs: TargetOutputs::new(),
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

impl TargetOptions {
    /// Derive the output mode from the target configuration.
    pub fn output_mode(&self) -> OutputMode {
        if self.out_file.is_some() || self.emit.is_single_file() {
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
            discovery: self.discovery,
            entry: self.entry.clone(),
            include: self.include.clone(),
            exclude: self.exclude.clone(),
            emit: self.emit,
            runtime: self.runtime,
            runtime_version: self.runtime_version.clone(),
            platform: self.platform,
            target_arch: self.target_arch.clone(),
            target_vendor: self.target_vendor.clone(),
            target_abi: self.target_abi.clone(),
            cpu: self.cpu.clone(),
            cpu_features: self.cpu_features.clone(),
            relocation_model: self.relocation_model,
            native_output: self.native_output,
            link_mode: self.link_mode,
            linker: self.linker.clone(),
            linker_flavor: self.linker_flavor,
            link_args: self.link_args.clone(),
            sysroot: self.sysroot.clone(),
            library_search_paths: self.library_search_paths.clone(),
            libraries: self.libraries.clone(),
            framework_search_paths: self.framework_search_paths.clone(),
            frameworks: self.frameworks.clone(),
            rpath: self.rpath.clone(),
            runpath: self.runpath.clone(),
            entry_symbol: self.entry_symbol.clone(),
            export_symbols: self.export_symbols.clone(),
            symbol_visibility: self.symbol_visibility,
            version_script: self.version_script.clone(),
            linker_script: self.linker_script.clone(),
            position_independent: self.position_independent,
            crt: self.crt,
            soname: self.soname.clone(),
            install_name: self.install_name.clone(),
            declaration: self.declaration,
            source_map_mode: self.source_map_mode,
            artifacts: self.artifacts.clone(),
            out_dir: self.out_dir.clone(),
            out_file: self.out_file.clone(),
            declaration_dir: self.declaration_dir.clone(),
            module: self.module,
            es_target: self.es_target,
            lib: self.lib.clone(),
            types: self.types.clone(),
            profile: self.profile.clone(),
            mode: self.mode.clone(),
            assembly: self.assembly,
            preserve_modules: self.preserve_modules,
            preserve_modules_root: self.preserve_modules_root.clone(),
            manual_chunks: self.manual_chunks.clone(),
            only_explicit_manual_chunks: self.only_explicit_manual_chunks,
            bundle_dependencies: self.bundle_dependencies.clone(),
            bundle_assets: self.bundle_assets.clone(),
            treeshake: self.treeshake.clone(),
            bundle_output: self.bundle_output.clone(),
            define: self.define.clone(),
            minify: self.minify.clone(),
            app: self.app.clone(),
            outputs: self.outputs.clone(),
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
    pub fn from_json_with_runtime(json: &TargetJson, base_runtime: &RuntimeOptions) -> Self {
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
        let mut runtime_options = runtime_options_with_base(base_runtime, json.runtime.as_ref());

        // align execution mode field with runtime options
        if let Some(execution_mode) = json.execution.map(ExecutionMode::from) {
            runtime_options.set_execution_mode(execution_mode);
        }

        // resolve one target app declaration for runtime host planning
        let app = json
            .app
            .as_ref()
            .map(TargetAppDeclaration::from)
            .unwrap_or_default();

        // derive the emit family early so output defaults can reuse the target shape
        let emit = json.emit.map(EmitFormat::from).unwrap_or_default();

        // resolve one assembled output decision before deriving output groups
        let preserve_modules = json.preserve_modules.unwrap_or_default();
        let preserve_modules_root = json.preserve_modules_root.as_ref().map(PathBuf::from);
        let manual_chunks = json.manual_chunks.clone().unwrap_or_default();
        let only_explicit_manual_chunks = json.only_explicit_manual_chunks.unwrap_or_default();
        let bundle_dependencies = json
            .dependencies
            .as_ref()
            .map(TargetDependencyOptions::from)
            .unwrap_or_default();
        let bundle_assets = json
            .assets
            .as_ref()
            .map(TargetAssetOptions::from)
            .unwrap_or_default();
        let treeshake = json
            .treeshake
            .as_ref()
            .map(TargetTreeshakeOptions::from)
            .unwrap_or_default();
        let bundle_output = json
            .output
            .as_ref()
            .map(TargetOutputPolicy::from)
            .unwrap_or_default();
        let define = json.define.clone().unwrap_or_default();
        let minify = json
            .minify
            .as_ref()
            .map(TargetMinifyOptions::from)
            .unwrap_or_default();
        let source_map_mode = bundle_output.sourcemap;
        let assembly = resolved_bundle_mode(
            json.assembly,
            discovery,
            entry.len(),
            emit,
            json.out_file.is_some(),
            preserve_modules,
            manual_chunks.is_empty(),
        );
        let is_assembled = is_assembled_target(
            json.assembly,
            discovery,
            entry.len(),
            &app,
            emit,
            preserve_modules,
            manual_chunks.is_empty(),
            json.out_file.is_some(),
        );

        // derive the formal target outputs from the target shape
        let outputs = default_target_outputs(
            emit,
            is_assembled,
            json.declaration,
            source_map_mode,
            bundle_output.manifest,
        );

        // seed runtime options with the resolved target app declaration
        runtime_options.app = RuntimeAppDeclaration::from(&app);

        let safety_preset = json.safety_preset.map(SafetyPreset::from);
        let default_checks = safety_preset
            .map(|preset| preset.runtime_check_policies())
            .unwrap_or_default();
        let default_float_math = safety_preset
            .map(|preset| preset.float_math_policy())
            .unwrap_or_default();

        Self {
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            discovery,
            entry,
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            emit,
            runtime: runtime_options.host,
            runtime_version: runtime_options.version.clone(),
            platform: json
                .platform
                .as_deref()
                .and_then(Platform::parse)
                .unwrap_or_default(),
            target_arch: json.arch.as_deref().and_then(TargetArch::parse),
            target_vendor: json.vendor.as_deref().and_then(TargetVendor::parse),
            target_abi: json.env.as_deref().and_then(TargetAbi::parse),
            cpu: json.cpu.clone(),
            cpu_features: json.cpu_features.clone().unwrap_or_default(),
            relocation_model: json
                .relocation_model
                .map(RelocationModel::from)
                .unwrap_or_default(),
            native_output: json.native_output.unwrap_or_default(),
            link_mode: json.link_mode.map(LinkMode::from).unwrap_or_default(),
            linker: json.linker.clone(),
            linker_flavor: json.linker_flavor.unwrap_or_default(),
            link_args: json.link_args.clone().unwrap_or_default(),
            sysroot: json.sysroot.as_ref().map(PathBuf::from),
            library_search_paths: json
                .library_search_paths
                .as_ref()
                .map(|paths| paths.iter().map(PathBuf::from).collect())
                .unwrap_or_default(),
            libraries: json.libraries.clone().unwrap_or_default(),
            framework_search_paths: json
                .framework_search_paths
                .as_ref()
                .map(|paths| paths.iter().map(PathBuf::from).collect())
                .unwrap_or_default(),
            frameworks: json.frameworks.clone().unwrap_or_default(),
            rpath: json.rpath.clone().unwrap_or_default(),
            runpath: json.runpath.clone().unwrap_or_default(),
            entry_symbol: json.entry_symbol.clone(),
            export_symbols: json.export_symbols.clone().unwrap_or_default(),
            symbol_visibility: json.symbol_visibility.unwrap_or_default(),
            version_script: json.version_script.as_ref().map(PathBuf::from),
            linker_script: json.linker_script.as_ref().map(PathBuf::from),
            position_independent: json.position_independent.unwrap_or_default(),
            crt: json.crt.unwrap_or_default(),
            soname: json.soname.clone(),
            install_name: json.install_name.clone(),
            declaration: json.declaration,
            source_map_mode,
            artifacts: json
                .artifacts
                .as_ref()
                .map(|artifacts| artifacts.iter().copied().map(EmitArtifact::from).collect())
                .unwrap_or_default(),
            out_dir: json
                .out_dir
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGET_OUT_DIR)),
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
            mode: json.mode.clone(),
            assembly,
            preserve_modules,
            preserve_modules_root,
            manual_chunks,
            only_explicit_manual_chunks,
            bundle_dependencies,
            bundle_assets,
            treeshake,
            bundle_output,
            define,
            minify,
            app,
            outputs,
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

impl From<&TargetJson> for TargetOptions {
    fn from(json: &TargetJson) -> Self {
        Self::from_json_with_runtime(json, &RuntimeOptions::default())
    }
}

/// A build artifact node.
///
/// Inputs: source discovery, build settings, runtime selection, and packaging declarations.
/// Outputs: built artifacts and target metadata consumed by deployment tooling.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetJson {
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    // discovery
    /// Entry points for entry-based discovery (bundled/executable targets).
    /// If set, discovery mode is Entry; otherwise it's Include.
    pub entry: Option<Vec<String>>,
    /// Glob patterns for files to include (for include-based discovery).
    pub include: Option<Vec<String>>,
    /// Glob patterns for files to exclude.
    pub exclude: Option<Vec<String>>,

    // emit family
    /// Emitted artifact family (e.g., JavaScript, TypeScript, HTML, WebAssembly, Native).
    pub emit: Option<EmitFormatJson>,
    /// Runtime host shorthand or full runtime configuration.
    pub runtime: Option<RuntimeConfigJson>,
    /// Host platform or packaging surface (e.g., browser, ios, android, macos, linux, windows).
    pub platform: Option<String>,
    /// Target architecture for native codegen (e.g., "x86_64", "aarch64").
    pub arch: Option<String>,
    /// Target vendor for native codegen (e.g., "apple", "pc", "unknown").
    pub vendor: Option<String>,
    /// Target ABI for native codegen (e.g., "gnu", "musl", "msvc").
    pub env: Option<String>,
    /// CPU name for native codegen (e.g., "native", "x86-64", "znver3").
    pub cpu: Option<String>,
    /// CPU feature flags for native codegen (e.g., "+sse4.2", "+aes").
    pub cpu_features: Option<Vec<String>>,
    /// Relocation model.
    pub relocation_model: Option<RelocationModelJson>,
    /// Native output kind for this target.
    pub native_output: Option<TargetNativeOutputKind>,
    /// Link mode for native targets.
    pub link_mode: Option<LinkModeJson>,
    /// Explicit linker executable for native targets.
    pub linker: Option<String>,
    /// Linker driver family.
    pub linker_flavor: Option<TargetLinkerFlavor>,
    /// Extra linker arguments for native targets.
    pub link_args: Option<Vec<String>>,
    /// Sysroot path for native toolchains.
    pub sysroot: Option<String>,
    /// Additional library search paths.
    pub library_search_paths: Option<Vec<String>>,
    /// Additional libraries to link.
    pub libraries: Option<Vec<String>>,
    /// Additional framework search paths.
    pub framework_search_paths: Option<Vec<String>>,
    /// Additional frameworks to link.
    pub frameworks: Option<Vec<String>>,
    /// Runtime search paths embedded into the final output.
    pub rpath: Option<Vec<String>>,
    /// Runtime search paths emitted as runpath entries.
    pub runpath: Option<Vec<String>>,
    /// Explicit entry symbol override.
    pub entry_symbol: Option<String>,
    /// Explicitly exported symbol names.
    pub export_symbols: Option<Vec<String>>,
    /// Symbol visibility policy.
    pub symbol_visibility: Option<TargetSymbolVisibility>,
    /// Version script for exported symbols.
    pub version_script: Option<String>,
    /// Linker script for the final link.
    pub linker_script: Option<String>,
    /// Position independent code policy.
    pub position_independent: Option<TargetPositionIndependentMode>,
    /// C runtime linkage policy.
    pub crt: Option<TargetCrtLinkage>,
    /// Shared object soname.
    pub soname: Option<String>,
    /// Darwin install name.
    pub install_name: Option<String>,
    /// Emit declaration files (e.g., `.d.ts` alongside `.js` output).
    #[serde(default)]
    pub declaration: bool,
    /// Extra sidecar artifacts to emit.
    pub artifacts: Option<Vec<EmitArtifactJson>>,

    // output paths
    /// Output directory for this target (overrides compilerOptions.outDir).
    pub out_dir: Option<String>,
    /// Output file for single-file targets like html or wasm (e.g., "./dist/index.html").
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
    /// Explicit mode name for this target.
    pub mode: Option<String>,
    /// Assembly topology for this script target.
    pub assembly: Option<BundleMode>,
    /// Whether to preserve one emitted module file per reachable module.
    pub preserve_modules: Option<bool>,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<String>,
    /// Manual chunk assignments keyed by chunk name.
    pub manual_chunks: Option<IndexMap<String, Vec<String>>>,
    /// Whether to only honor explicit manual chunk declarations.
    pub only_explicit_manual_chunks: Option<bool>,
    /// Dependency and resolution options.
    #[serde(alias = "deps")]
    pub dependencies: Option<TargetDependencyOptionsJson>,
    /// Asset handling options.
    pub assets: Option<TargetAssetOptionsJson>,
    /// Tree shaking options.
    pub treeshake: Option<TargetTreeshakeOptionsJson>,
    /// Output options for assembled bundle files.
    pub output: Option<TargetOutputPolicyJson>,
    /// Compile time define replacements.
    pub define: Option<IndexMap<String, String>>,
    /// Minification options.
    pub minify: Option<TargetMinifyOptionsJson>,
    /// App declaration for packaging and runtime capability planning.
    pub app: Option<TargetAppDeclarationJson>,
    // optimization
    /// Whether this is a debug build.
    #[serde(default)]
    pub debug: bool,
    /// Whether optimization is enabled.
    #[serde(default)]
    pub optimize: bool,
    /// Optimization level (0-4).
    #[cfg_attr(feature = "schema", schemars(range(min = 0, max = 4)))]
    pub optimize_level: Option<u8>,
    /// Loop unroll threshold in instructions.
    pub unroll_threshold: Option<u64>,
    /// Inline budget scaling in percent.
    pub inline_budget_scale_percent: Option<u64>,
    /// Link time optimization mode.
    pub lto_mode: Option<LtoModeJson>,
    /// Shrink level (0-3).
    #[cfg_attr(feature = "schema", schemars(range(min = 0, max = 3)))]
    pub shrink_level: Option<u8>,
    /// Floating point math optimization policy.
    pub float_math: Option<FloatMathPolicyJson>,
    /// Debug info emission policy.
    pub debug_info: Option<DebugInfoLevelJson>,
    /// Debug execution mode for VM/native targets.
    pub debug_mode: Option<DebugModeJson>,
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

/// Returns the default target outputs for a given emit configuration.
fn default_target_outputs(
    emit: EmitFormat,
    is_assembled: bool,
    declaration: bool,
    source_map_mode: Option<SourceMapMode>,
    has_manifest: bool,
) -> TargetOutputs {
    let mut outputs = TargetOutputs::new();

    // primary emitted surface
    let primary_name = match emit {
        EmitFormat::Native | EmitFormat::Wasm => TargetOutputName::Binary,
        EmitFormat::Html => TargetOutputName::Document,
        EmitFormat::Js | EmitFormat::Ts => {
            if is_assembled {
                TargetOutputName::Entry
            } else {
                TargetOutputName::Module
            }
        }
    };
    let primary_topology = match emit {
        EmitFormat::Native | EmitFormat::Wasm | EmitFormat::Html => TargetOutputTopology::File,
        EmitFormat::Js | EmitFormat::Ts => {
            if is_assembled {
                TargetOutputTopology::Collection
            } else {
                TargetOutputTopology::Directory
            }
        }
    };
    outputs.insert(
        primary_name.as_str(),
        TargetOutputOptions {
            kind: primary_name.kind(),
            topology: primary_topology,
            is_public: true,
        },
    );

    // declarations
    if declaration {
        outputs.insert(
            TargetOutputName::Types.as_str(),
            TargetOutputOptions {
                kind: TargetOutputKind::Types,
                topology: primary_topology,
                is_public: true,
            },
        );
    }

    // source maps
    if source_map_mode.is_some_and(SourceMapMode::emits_output) {
        outputs.insert(
            TargetOutputName::Maps.as_str(),
            TargetOutputOptions {
                kind: TargetOutputKind::Maps,
                topology: primary_topology,
                is_public: false,
            },
        );
    }

    // manifest sidecar
    if has_manifest {
        outputs.insert(
            TargetOutputName::Manifest.as_str(),
            TargetOutputOptions {
                kind: TargetOutputKind::Manifest,
                topology: TargetOutputTopology::Collection,
                is_public: true,
            },
        );
    }

    outputs
}

/// Resolves the bundle mode for a target.
fn resolved_bundle_mode(
    explicit_mode: Option<BundleMode>,
    discovery: TargetDiscovery,
    entry_count: usize,
    emit: EmitFormat,
    out_file: bool,
    preserve_modules: bool,
    manual_chunks_is_empty: bool,
) -> BundleMode {
    if out_file || emit.is_single_file() {
        return BundleMode::SingleFile;
    }

    if let Some(explicit_mode) = explicit_mode {
        return explicit_mode;
    }

    if preserve_modules {
        return BundleMode::PreserveModules;
    }

    if !manual_chunks_is_empty {
        return BundleMode::Chunked;
    }

    if discovery == TargetDiscovery::Entry && entry_count > 1 {
        return BundleMode::Chunked;
    }

    match discovery {
        TargetDiscovery::Entry => BundleMode::SingleFile,
        TargetDiscovery::Include => BundleMode::PreserveModules,
    }
}

/// Returns `true` if the target is an assembled target (i.e. it uses entry output layout).
fn is_assembled_target(
    explicit_bundle_mode: Option<BundleMode>,
    discovery: TargetDiscovery,
    entry_count: usize,
    app: &TargetAppDeclaration,
    emit: EmitFormat,
    preserve_modules: bool,
    manual_chunks_is_empty: bool,
    out_file: bool,
) -> bool {
    let bundle_mode = resolved_bundle_mode(
        explicit_bundle_mode,
        discovery,
        entry_count,
        emit,
        out_file,
        preserve_modules,
        manual_chunks_is_empty,
    );

    if out_file || emit.is_single_file() {
        return true;
    }

    if !app.is_empty() {
        return true;
    }

    bundle_mode.uses_entry_output_layout()
}
