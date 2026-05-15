use std::collections::BTreeMap;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use destack_artifact::{
    EmitFormat, Host, Platform, Runtime, TargetAbi, TargetArch, TargetKey, TargetVendor,
};
use destack_core::StableHasher;
use destack_source::TargetId;
use serde::Deserialize;

use crate::{CompilerOptions, PolicyOptions, PolicyOptionsJson};

use super::super::runtime::{
    ExecutionMode, ExecutionModeJson, RuntimeConfigJson, RuntimeOptions, runtime_options_with_base,
};
use super::codegen::*;
use super::js::*;
use super::link::*;
use super::native::*;
use super::output::*;
use super::policy::*;

/// Default output directory for targets.
const DEFAULT_TARGET_OUT_DIR: &str = "dist";
/// Stable hash domain for target configuration identity.
const TARGET_CONFIGURATION_KEY_DOMAIN: &[u8] = b"destack.workspace.target-configuration.v1";

/// A build target configuration.
///
/// Can be constructed from `destack.json` or programmatically.
/// This is the type used by compiler/codegen, independent of config parsing.
#[derive(Debug, Clone, Hash)]
pub struct Target {
    /// Target name (e.g., "web", "wasm", "dev").
    pub name: String,

    // discovery
    /// How modules are discovered for this target.
    pub discovery: TargetDiscovery,
    /// Entry points for entry-based discovery (bundled/executable targets).
    pub entry: Vec<PathBuf>,
    /// Global modules added as discovery roots.
    pub globals: Vec<PathBuf>,
    /// Target tree tag builder override.
    pub tree: Option<String>,
    /// Target derive providers.
    pub derive: Vec<String>,
    /// Glob patterns for files to include (for include-based discovery).
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,

    // output generation
    /// JavaScript module format.
    pub module: JsModuleFormat,
    /// ECMAScript target version.
    pub es_target: EsTarget,
    /// Explicit profile name for this target.
    pub profile: Option<String>,
    /// Active source graph modes for this target.
    pub modes: Vec<String>,
    /// Assembly topology for this script target.
    pub assembly: BundleMode,
    /// Whether to preserve one emitted module file per reachable module.
    pub preserve_modules: bool,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<PathBuf>,
    /// Manual chunk assignments keyed by chunk name.
    pub manual_chunks: BTreeMap<String, Vec<String>>,
    /// Whether to only honor explicit manual chunk declarations.
    pub only_explicit_manual_chunks: bool,
    /// Dependency and resolution options.
    pub bundle_dependencies: BundleDependencyOptions,
    /// Asset handling options.
    pub bundle_assets: BundleAssetOptions,
    /// Output configuration for bundled products.
    pub bundle_output: BundleOutputOptions,
    /// Minification options.
    pub minify: BundleMinifyOptions,
    /// Policy declarations and rules for this target.
    pub policy: PolicyOptions,
    /// Emitted artifact family (js, ts, html, wasm, native).
    pub emit: EmitFormat,
    /// Runtime execution contract.
    pub runtime: Runtime,
    /// Target operating system.
    pub platform: Platform,
    /// Target host environment.
    pub host: Host,
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
    /// Native output kind for this target.
    pub native_output: NativeOutputKind,
    /// Explicit linker executable for native targets.
    pub linker: Option<String>,
    /// Linker driver family.
    pub linker_flavor: LinkerFlavor,
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
    pub symbol_visibility: SymbolVisibility,
    /// Version script for exported symbols.
    pub version_script: Option<PathBuf>,
    /// Linker script for the final link.
    pub linker_script: Option<PathBuf>,
    /// Position independent code policy.
    pub position_independent: PositionIndependentMode,
    /// C runtime linkage policy.
    pub crt: CrtLinkage,
    /// Shared object soname.
    pub soname: Option<String>,
    /// Darwin install name.
    pub install_name: Option<String>,
    /// Emit declaration files (e.g., `.d.ts` alongside `.js` output).
    pub declaration: bool,
    /// Source map emission mode.
    pub source_map_mode: Option<SourceMapMode>,

    // optimization
    /// Optimization level.
    pub optimize_level: OptimizeLevel,
    /// Link time optimization mode.
    pub lto_mode: LtoMode,
    /// Floating point math optimization policy.
    pub float_math: FloatMathPolicy,
    /// Debug info emission policy.
    pub debug_info: DebugInfoLevel,
    /// Runtime execution options.
    pub runtime_options: RuntimeOptions,
    /// Panic behavior for unrecoverable program failures.
    pub panic: PanicPolicy,
    /// Symbol stripping policy.
    pub strip: StripLevel,
    /// Runtime checks.
    pub checks: RuntimeChecks,

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
        Self::native("default")
    }
}

impl Target {
    /// Create a target from default target options.
    fn from_default_options(name: impl Into<String>) -> Self {
        let name = name.into();

        TargetOptions::default().to_target(name.as_str())
    }

    /// Return the known built-in target names.
    pub fn builtin_target_names() -> &'static [&'static str] {
        &[
            "default",
            "js",
            "ts",
            "html",
            "wasm",
            "wasm-wasi",
            "wasi",
            "native",
        ]
    }

    /// Create a built-in target configuration for a known target id.
    pub fn builtin_for_id(target_id: TargetId) -> Option<Self> {
        let package_id = target_id.package_id();

        for name in Self::builtin_target_names() {
            let builtin_target_id = TargetId::new(package_id, name);

            if builtin_target_id == target_id {
                return Self::builtin_for_name(name);
            }
        }

        None
    }

    /// Create a new target with the given name and default JS output.
    pub fn js(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.emit = EmitFormat::Js;
        target.runtime = Runtime::Js;
        target.runtime_options.runtime = Runtime::Js;
        target.platform = Platform::Unknown;
        target.host = Host::Unknown;
        target.declaration = true;

        target
    }

    /// Create a new target with the given name and TypeScript output.
    pub fn ts(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.emit = EmitFormat::Ts;
        target.runtime = Runtime::Js;
        target.runtime_options.runtime = Runtime::Js;
        target.platform = Platform::Unknown;
        target.host = Host::Unknown;

        target
    }

    /// Create a new target with the given name and HTML document output.
    pub fn html(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.emit = EmitFormat::Html;
        target.runtime = Runtime::Js;
        target.runtime_options.runtime = Runtime::Js;
        target.platform = Platform::Unknown;
        target.host = Host::Browser;

        target
    }

    /// Create a new target with the given name and WASM output for JS host.
    pub fn wasm_js(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.emit = EmitFormat::Wasm;
        target.runtime = Runtime::Destack;
        target.runtime_options.runtime = Runtime::Destack;
        target.platform = Platform::Unknown;
        target.host = Host::Browser;
        target.optimize_level = OptimizeLevel::O2;

        target
    }

    /// Create a new target with the given name and WASM output for WASI.
    pub fn wasm_wasi(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.emit = EmitFormat::Wasm;
        target.runtime = Runtime::Destack;
        target.runtime_options.runtime = Runtime::Destack;
        target.platform = Platform::Unknown;
        target.host = Host::Wasi;
        target.optimize_level = OptimizeLevel::O2;

        target
    }

    /// Create a new target with the given name and native output.
    pub fn native(name: impl Into<String>) -> Self {
        let mut target = Self::from_default_options(name);
        target.emit = EmitFormat::Native;
        target.runtime = Runtime::Destack;
        target.runtime_options.runtime = Runtime::Destack;
        target.platform = Platform::Unknown;
        target.host = Host::Native;
        target.optimize_level = OptimizeLevel::O2;

        target
    }

    /// Create a new target for comptime execution.
    pub fn comptime(name: impl Into<String>) -> Self {
        Self::native(name)
    }

    /// Create a new target with the given name and native freestanding output.
    pub fn native_freestanding(name: impl Into<String>) -> Self {
        let mut target = Self::native(name);
        target.platform = Platform::None;
        target.host = Host::Freestanding;

        target
    }

    /// Create a new target with the given name and native embedded output.
    pub fn native_embedded(name: impl Into<String>) -> Self {
        let mut target = Self::native(name);
        target.platform = Platform::None;
        target.host = Host::Freestanding;

        target
    }

    /// Create a built-in target configuration for a known target name.
    pub fn builtin_for_name(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::default()),
            "js" => Some(Self::js(name)),
            "ts" => Some(Self::ts(name)),
            "html" => Some(Self::html(name)),
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

        // target semantic defaults
        if self.tree.is_some() {
            compiler_options.tree = self.tree.clone();
        }
        compiler_options.modes.extend(self.modes.clone());
        compiler_options.globals.extend(self.globals.clone());
        compiler_options.derive.extend(self.derive.clone());

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

    /// Return the artifact dependency key for this target configuration.
    pub fn configuration_key(&self) -> TargetKey {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(TARGET_CONFIGURATION_KEY_DOMAIN);
        self.hash(&mut hasher);

        TargetKey::new(hasher.finish_u128())
    }

    /// Return whether this target assembles one target level output shape.
    pub fn emits_assembled_output(&self) -> bool {
        is_assembled_target(
            Some(self.assembly),
            self.discovery,
            self.entry.len(),
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
        self.emit.is_native_family()
    }

    /// Return the normalized source map mode for this target.
    pub fn source_map_mode(&self) -> Option<SourceMapMode> {
        self.bundle_output.sourcemap.or(self.source_map_mode)
    }

    /// Resolve a target triple string from the target configuration.
    pub fn resolved_target_triple(&self) -> Option<String> {
        let target_arch = self.target_arch.as_ref()?;
        let os = self
            .platform
            .triple_os_component()
            .or_else(|| self.host.triple_system_component())?;

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
    pub fn with_module(mut self, module: JsModuleFormat) -> Self {
        self.module = module;
        self
    }

    /// Set the ES target.
    pub fn with_es_target(mut self, es_target: EsTarget) -> Self {
        self.es_target = es_target;
        self
    }

    /// Set an explicit profile name for this target.
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Set active source graph modes for this target.
    pub fn with_modes(mut self, modes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.modes = modes.into_iter().map(Into::into).collect();
        self
    }

    /// Set the runtime.
    pub fn with_runtime(mut self, runtime: Runtime) -> Self {
        self.runtime = runtime;
        self.runtime_options.runtime = runtime;
        self
    }

    /// Set the platform.
    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    /// Set the host environment.
    pub fn with_host(mut self, host: Host) -> Self {
        self.host = host;
        self
    }

    /// Set the panic behavior.
    pub fn with_panic(mut self, panic: PanicPolicy) -> Self {
        self.panic = panic;
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

    /// Set runtime checks.
    pub fn with_checks(mut self, checks: RuntimeChecks) -> Self {
        self.checks = checks;
        self
    }

    /// Set floating point math optimization policy.
    pub fn with_float_math(mut self, float_math: FloatMathPolicy) -> Self {
        self.float_math = float_math;
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

    /// Set the optimization level.
    pub fn with_optimize_level(mut self, level: OptimizeLevel) -> Self {
        self.optimize_level = level;
        self
    }

    /// Set the link time optimization mode.
    pub fn with_lto_mode(mut self, mode: LtoMode) -> Self {
        self.lto_mode = mode;
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

    /// Set global provider roots.
    pub fn with_globals(mut self, globals: Vec<PathBuf>) -> Self {
        self.globals = globals;
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
    /// The `root_dir` parameter (from compiler) specifies the root of source files.
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
    // discovery
    /// How modules are discovered for this target.
    pub discovery: TargetDiscovery,
    /// Entry points for entry-based discovery.
    pub entry: Vec<PathBuf>,
    /// Global provider modules added as discovery roots.
    pub globals: Vec<PathBuf>,
    /// Target tree tag builder override.
    pub tree: Option<String>,
    /// Target derive providers.
    pub derive: Vec<String>,
    /// Glob patterns for files to include (for include-based discovery).
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,

    // output format
    /// Emitted artifact family (js, ts, html, wasm, native).
    pub emit: EmitFormat,
    /// Runtime execution contract.
    pub runtime: Runtime,
    /// Target operating system.
    pub platform: Platform,
    /// Target host environment.
    pub host: Host,
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
    /// Native output kind for this target.
    pub native_output: NativeOutputKind,
    /// Explicit linker executable for native targets.
    pub linker: Option<String>,
    /// Linker driver family.
    pub linker_flavor: LinkerFlavor,
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
    pub symbol_visibility: SymbolVisibility,
    /// Version script for exported symbols.
    pub version_script: Option<PathBuf>,
    /// Linker script for the final link.
    pub linker_script: Option<PathBuf>,
    /// Position independent code policy.
    pub position_independent: PositionIndependentMode,
    /// C runtime linkage policy.
    pub crt: CrtLinkage,
    /// Shared object soname.
    pub soname: Option<String>,
    /// Darwin install name.
    pub install_name: Option<String>,
    /// Emit declaration files (e.g., `.d.ts` alongside `.js` output).
    pub declaration: bool,
    /// Source map emission mode.
    pub source_map_mode: Option<SourceMapMode>,

    // output paths
    /// Output directory for this target (defaults to "dist").
    pub out_dir: PathBuf,
    /// Output file for single-file targets like html or wasm.
    pub out_file: Option<PathBuf>,
    /// Separate directory for declaration files.
    pub declaration_dir: Option<PathBuf>,

    // JS/TS specific
    /// JavaScript module format for this target.
    pub module: JsModuleFormat,
    /// ECMAScript target for this target.
    pub es_target: EsTarget,
    /// Explicit profile name for this target.
    pub profile: Option<String>,
    /// Active source graph modes for this target.
    pub modes: Vec<String>,
    /// Assembly topology for this script target.
    pub assembly: BundleMode,
    /// Whether to preserve one emitted module file per reachable module.
    pub preserve_modules: bool,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<PathBuf>,
    /// Manual chunk assignments keyed by chunk name.
    pub manual_chunks: BTreeMap<String, Vec<String>>,
    /// Whether to only honor explicit manual chunk declarations.
    pub only_explicit_manual_chunks: bool,
    /// Dependency and resolution options.
    pub bundle_dependencies: BundleDependencyOptions,
    /// Asset handling options.
    pub bundle_assets: BundleAssetOptions,
    /// Output configuration for bundled products.
    pub bundle_output: BundleOutputOptions,
    /// Minification options.
    pub minify: BundleMinifyOptions,
    /// Policy declarations and rules for this target.
    pub policy: PolicyOptions,

    // optimization
    /// Optimization level.
    pub optimize_level: OptimizeLevel,
    /// Link time optimization mode.
    pub lto_mode: LtoMode,
    /// Floating point math optimization policy.
    pub float_math: FloatMathPolicy,
    /// Debug info emission policy.
    pub debug_info: DebugInfoLevel,
    /// Runtime execution options.
    pub runtime_options: RuntimeOptions,
    /// Panic behavior for unrecoverable program failures.
    pub panic: PanicPolicy,
    /// Symbol stripping policy.
    pub strip: StripLevel,
    /// Runtime checks.
    pub checks: RuntimeChecks,
}

impl Default for TargetOptions {
    fn default() -> Self {
        Self {
            discovery: TargetDiscovery::default(),
            entry: Vec::new(),
            globals: Vec::new(),
            tree: None,
            derive: Vec::new(),
            include: Vec::new(),
            exclude: Vec::new(),
            emit: EmitFormat::default(),
            runtime: Runtime::default(),
            platform: Platform::Unknown,
            host: Host::Unknown,
            target_arch: None,
            target_vendor: None,
            target_abi: None,
            cpu: None,
            cpu_features: Vec::new(),
            native_output: NativeOutputKind::default(),
            linker: None,
            linker_flavor: LinkerFlavor::default(),
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
            symbol_visibility: SymbolVisibility::default(),
            version_script: None,
            linker_script: None,
            position_independent: PositionIndependentMode::default(),
            crt: CrtLinkage::default(),
            soname: None,
            install_name: None,
            declaration: false,
            source_map_mode: None,
            out_dir: PathBuf::from(DEFAULT_TARGET_OUT_DIR),
            out_file: None,
            declaration_dir: None,
            module: JsModuleFormat::default(),
            es_target: EsTarget::default(),
            profile: None,
            modes: Vec::new(),
            assembly: BundleMode::default(),
            preserve_modules: false,
            preserve_modules_root: None,
            manual_chunks: BTreeMap::new(),
            only_explicit_manual_chunks: false,
            bundle_dependencies: BundleDependencyOptions::default(),
            bundle_assets: BundleAssetOptions::default(),
            bundle_output: BundleOutputOptions::default(),
            minify: BundleMinifyOptions::default(),
            policy: PolicyOptions::default(),
            optimize_level: OptimizeLevel::O0,
            lto_mode: LtoMode::default(),
            float_math: FloatMathPolicy::default(),
            debug_info: DebugInfoLevel::default(),
            runtime_options: RuntimeOptions::default(),
            panic: PanicPolicy::default(),
            strip: StripLevel::default(),
            checks: RuntimeChecks::default(),
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
            globals: self.globals.clone(),
            tree: self.tree.clone(),
            derive: self.derive.clone(),
            include: self.include.clone(),
            exclude: self.exclude.clone(),
            emit: self.emit,
            runtime: self.runtime,
            platform: self.platform,
            host: self.host,
            target_arch: self.target_arch.clone(),
            target_vendor: self.target_vendor.clone(),
            target_abi: self.target_abi.clone(),
            cpu: self.cpu.clone(),
            cpu_features: self.cpu_features.clone(),
            native_output: self.native_output,
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
            out_dir: self.out_dir.clone(),
            out_file: self.out_file.clone(),
            declaration_dir: self.declaration_dir.clone(),
            module: self.module,
            es_target: self.es_target,
            profile: self.profile.clone(),
            modes: self.modes.clone(),
            assembly: self.assembly,
            preserve_modules: self.preserve_modules,
            preserve_modules_root: self.preserve_modules_root.clone(),
            manual_chunks: self.manual_chunks.clone(),
            only_explicit_manual_chunks: self.only_explicit_manual_chunks,
            bundle_dependencies: self.bundle_dependencies.clone(),
            bundle_assets: self.bundle_assets.clone(),
            bundle_output: self.bundle_output.clone(),
            minify: self.minify.clone(),
            policy: self.policy.clone(),
            optimize_level: self.optimize_level,
            lto_mode: self.lto_mode,
            float_math: self.float_math,
            debug_info: self.debug_info,
            runtime_options: self.runtime_options.clone(),
            panic: self.panic,
            strip: self.strip,
            checks: self.checks,
        }
    }

    /// Derive target options from JSON and base runtime options.
    pub fn from_json_with_runtime(
        json: &TargetJson,
        base_runtime: &RuntimeOptions,
    ) -> Result<Self, String> {
        Self::from_json_with_runtime_and_policy(json, base_runtime, &PolicyOptions::default())
    }

    /// Derive target options from JSON, base runtime options, and base policy.
    pub fn from_json_with_runtime_and_policy(
        json: &TargetJson,
        base_runtime: &RuntimeOptions,
        base_policy: &PolicyOptions,
    ) -> Result<Self, String> {
        // derive entry points
        let entry: Vec<PathBuf> = json
            .entry
            .as_ref()
            .map(|e| e.iter().map(PathBuf::from).collect())
            .unwrap_or_default();
        let globals: Vec<PathBuf> = json
            .globals
            .as_ref()
            .map(|globals| globals.iter().map(PathBuf::from).collect())
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

        // merge target policy over package policy
        let policy = PolicyOptions::from_json_with_parent(json.policy.as_ref(), base_policy);

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
            .map(BundleDependencyOptions::from)
            .unwrap_or_default();
        let bundle_assets = json
            .assets
            .as_ref()
            .map(BundleAssetOptions::from)
            .unwrap_or_default();
        let bundle_output = json
            .output
            .as_ref()
            .map(BundleOutputOptions::from)
            .unwrap_or_default();
        let minify = json
            .minify
            .as_ref()
            .map(BundleMinifyOptions::from)
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
        let checks = json
            .checks
            .clone()
            .map(RuntimeChecks::from)
            .unwrap_or_default();
        let host = json
            .host
            .as_deref()
            .map(parse_target_host)
            .transpose()?
            .unwrap_or_default();
        let platform = json
            .platform
            .as_deref()
            .map(parse_target_platform)
            .transpose()?
            .unwrap_or_else(|| default_platform_for_host(host));

        Ok(Self {
            discovery,
            entry,
            globals,
            tree: json.tree.clone(),
            derive: json.derive.clone().unwrap_or_default(),
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            emit,
            runtime: runtime_options.runtime,
            platform,
            host,
            target_arch: json.arch.as_deref().and_then(TargetArch::parse),
            target_vendor: json.vendor.as_deref().and_then(TargetVendor::parse),
            target_abi: json.env.as_deref().and_then(TargetAbi::parse),
            cpu: json.cpu.clone(),
            cpu_features: json.cpu_features.clone().unwrap_or_default(),
            native_output: json.native_output.unwrap_or_default(),
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
                .and_then(JsModuleFormat::parse)
                .unwrap_or_default(),
            es_target: json
                .target
                .as_deref()
                .and_then(EsTarget::parse)
                .unwrap_or_default(),
            profile: json.profile.clone(),
            modes: json.modes.clone().unwrap_or_default(),
            assembly,
            preserve_modules,
            preserve_modules_root,
            manual_chunks,
            only_explicit_manual_chunks,
            bundle_dependencies,
            bundle_assets,
            bundle_output,
            minify,
            policy,
            optimize_level: json.optimize.map(OptimizeLevel::from).unwrap_or_default(),
            lto_mode: json.lto_mode.map(LtoMode::from).unwrap_or_default(),
            float_math: json.float.map(FloatMathPolicy::from).unwrap_or_default(),
            debug_info: json
                .debug_info
                .map(DebugInfoLevel::from)
                .unwrap_or_default(),
            runtime_options,
            panic: json.panic.map(PanicPolicy::from).unwrap_or_default(),
            strip: json.strip.map(StripLevel::from).unwrap_or_default(),
            checks,
        })
    }
}

impl TryFrom<&TargetJson> for TargetOptions {
    type Error = String;

    fn try_from(json: &TargetJson) -> Result<Self, Self::Error> {
        Self::from_json_with_runtime(json, &RuntimeOptions::default())
    }
}

/// Parse one target platform from a config value.
fn parse_target_platform(value: &str) -> Result<Platform, String> {
    Platform::parse(value).ok_or_else(|| format!("unsupported target platform '{value}'"))
}

/// Parse one target host from a config value.
fn parse_target_host(value: &str) -> Result<Host, String> {
    Host::parse(value).ok_or_else(|| format!("unsupported target host '{value}'"))
}

/// Resolve the implied operating system for one host.
fn default_platform_for_host(host: Host) -> Platform {
    match host {
        Host::Freestanding => Platform::None,
        _ => Platform::Unknown,
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
    // discovery
    /// Entry points for entry-based discovery (bundled/executable targets).
    /// If set, discovery mode is Entry; otherwise it's Include.
    pub entry: Option<Vec<String>>,
    /// Global modules added as discovery roots.
    pub globals: Option<Vec<String>>,
    /// Target tree tag builder override.
    pub tree: Option<String>,
    /// Target derive providers.
    pub derive: Option<Vec<String>>,
    /// Glob patterns for files to include (for include-based discovery).
    pub include: Option<Vec<String>>,
    /// Glob patterns for files to exclude.
    pub exclude: Option<Vec<String>>,

    // emit family
    /// Emitted artifact family (e.g., JavaScript, TypeScript, HTML, WebAssembly, Native).
    pub emit: Option<EmitFormatJson>,
    /// Runtime environment shorthand or full runtime configuration.
    pub runtime: Option<RuntimeConfigJson>,
    /// Target operating system.
    pub platform: Option<String>,
    /// Host environment that provides imports and ambient effects.
    pub host: Option<String>,
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
    /// Native output kind for this target.
    pub native_output: Option<NativeOutputKind>,
    /// Explicit linker executable for native targets.
    pub linker: Option<String>,
    /// Linker driver family.
    pub linker_flavor: Option<LinkerFlavor>,
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
    pub symbol_visibility: Option<SymbolVisibility>,
    /// Version script for exported symbols.
    pub version_script: Option<String>,
    /// Linker script for the final link.
    pub linker_script: Option<String>,
    /// Position independent code policy.
    pub position_independent: Option<PositionIndependentMode>,
    /// C runtime linkage policy.
    pub crt: Option<CrtLinkage>,
    /// Shared object soname.
    pub soname: Option<String>,
    /// Darwin install name.
    pub install_name: Option<String>,
    /// Emit declaration files (e.g., `.d.ts` alongside `.js` output).
    #[serde(default)]
    pub declaration: bool,

    // output paths
    /// Output directory for this target (overrides compiler.outDir).
    pub out_dir: Option<String>,
    /// Output file for single-file targets like html or wasm (e.g., "./dist/index.html").
    pub out_file: Option<String>,
    /// Separate directory for declaration files (overrides compiler.declarationDir).
    pub declaration_dir: Option<String>,

    // JS/TS specific
    /// Module format for this target (overrides compiler.module).
    pub module: Option<String>,
    /// ECMAScript target for this target (overrides compiler.target).
    pub target: Option<String>,
    /// Explicit profile name for this target.
    pub profile: Option<String>,
    /// Active source graph modes for this target.
    pub modes: Option<Vec<String>>,
    /// Assembly topology for this script target.
    pub assembly: Option<BundleMode>,
    /// Whether to preserve one emitted module file per reachable module.
    pub preserve_modules: Option<bool>,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<String>,
    /// Manual chunk assignments keyed by chunk name.
    pub manual_chunks: Option<BTreeMap<String, Vec<String>>>,
    /// Whether to only honor explicit manual chunk declarations.
    pub only_explicit_manual_chunks: Option<bool>,
    /// Dependency and resolution options.
    #[serde(alias = "deps")]
    pub dependencies: Option<BundleDependencyOptionsJson>,
    /// Asset handling options.
    pub assets: Option<BundleAssetOptionsJson>,
    /// Output options for assembled bundle files.
    pub output: Option<BundleOutputOptionsJson>,
    /// Minification options.
    pub minify: Option<BundleMinifyOptionsJson>,
    /// Policy declarations and rules for this target.
    pub policy: Option<PolicyOptionsJson>,
    // optimization
    /// Optimization setting.
    pub optimize: Option<OptimizeJson>,
    /// Link time optimization mode.
    pub lto_mode: Option<LtoModeJson>,
    /// Floating point math optimization policy.
    pub float: Option<FloatMathPolicyJson>,
    /// Debug info emission policy.
    pub debug_info: Option<DebugInfoLevelJson>,
    /// Execution mode for runtime scheduling and replay.
    #[serde(alias = "executionMode")]
    #[serde(alias = "execution_mode")]
    pub execution: Option<ExecutionModeJson>,
    /// Panic behavior for unrecoverable program failures.
    pub panic: Option<PanicPolicyJson>,
    /// Symbol stripping policy.
    pub strip: Option<StripLevelJson>,
    /// Runtime checks.
    pub checks: Option<RuntimeChecksJson>,
}

impl TargetJson {
    /// Validate explicit target declarations before they are normalized.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(policy) = &self.policy {
            policy.validate()?;
        }

        Ok(())
    }
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

    bundle_mode.uses_entry_output_layout()
}
