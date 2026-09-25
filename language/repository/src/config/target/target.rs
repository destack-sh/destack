use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tspp_artifact::{Code, Host, Output, Platform, Runtime, TargetAbi, TargetArch, TargetVendor};
use tspp_source::TargetId;

use crate::{CompilerOptions, ExecutionOptions, Policy};

use super::compiler::*;
use super::condition::*;
use super::destination::Destination;
use super::js::*;
use super::native::*;
use super::output::*;

/// A build target configuration.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    /// Entry points for entry-rooted targets.
    pub entry: Vec<PathBuf>,
    /// User callable launched by this target.
    pub entrypoint: Option<Entrypoint>,
    /// Global modules added to every target root set.
    pub globals: Vec<PathBuf>,
    /// Glob patterns for files to include when no entry is declared.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,

    /// Policy declarations and rules for this target.
    pub policy: Policy,
    /// Output produced by this target.
    pub output: Output,
    /// Executable representations included in a Program artifact.
    pub code: Vec<Code>,
    /// Target architecture.
    pub architecture: Option<TargetArch>,
    /// Target vendor.
    pub vendor: Option<TargetVendor>,
    /// Target application binary interface.
    pub abi: Option<TargetAbi>,
    /// Target operating system.
    pub platform: Platform,
    /// Target host environment.
    pub host: Host,
    /// Source graph conditions for this target.
    pub conditions: TargetConditionSet,
    /// Compiler behavior for this target.
    pub compiler: TargetCompilerOptions,
    /// Filesystem output destination.
    pub destination: Destination,
    /// Source map emission mode.
    pub source_map: Option<SourceMapMode>,
    /// JavaScript output configuration.
    pub js: TargetJsOptions,
    /// Native codegen and linking configuration.
    pub native: TargetNativeOptions,
    /// World and Runtime execution configuration.
    pub execution: ExecutionOptions,
}

impl Default for Target {
    fn default() -> Self {
        Self::js()
    }
}

impl Target {
    /// Create a target with explicit output, code, and host axes.
    fn new(output: Output, code: Vec<Code>, host: Host) -> Self {
        Self {
            entry: Vec::new(),
            entrypoint: None,
            globals: Vec::new(),
            include: Vec::new(),
            exclude: Vec::new(),
            policy: Policy::default(),
            output,
            code,
            architecture: None,
            vendor: None,
            abi: None,
            platform: Platform::Unknown,
            host,
            conditions: TargetConditionSet::default(),
            compiler: TargetCompilerOptions::default(),
            destination: Destination::default(),
            source_map: None,
            js: TargetJsOptions::default(),
            native: TargetNativeOptions::default(),
            execution: ExecutionOptions::default(),
        }
    }

    /// Return the known built-in target names.
    pub fn builtin_target_names() -> &'static [&'static str] {
        &[
            "default",
            "js",
            "bytecode",
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

    /// Create a target with default JavaScript output.
    pub fn js() -> Self {
        Self::new(Output::Bundle, Vec::new(), Host::Browser)
    }

    /// Create a target with Destack bytecode output.
    pub fn bytecode() -> Self {
        let mut target = Self::new(Output::Program, vec![Code::Bytecode], Host::Native);
        target.compiler.optimize = OptimizeLevel::O2;

        target
    }

    /// Create a target with WASM output for JavaScript hosts.
    pub fn wasm_js() -> Self {
        let mut target = Self::new(Output::Program, vec![Code::Wasm], Host::Browser);
        target.architecture = Some(TargetArch::Wasm32);
        target.compiler.optimize = OptimizeLevel::O2;

        target
    }

    /// Create a target with WASM output for WASI.
    pub fn wasm_wasi() -> Self {
        let mut target = Self::new(Output::Program, vec![Code::Wasm], Host::Wasi);
        target.architecture = Some(TargetArch::Wasm32);
        target.compiler.optimize = OptimizeLevel::O2;

        target
    }

    /// Create a target with native output.
    pub fn native() -> Self {
        let mut target = Self::new(Output::Program, vec![Code::Native], Host::Native);
        target.compiler.optimize = OptimizeLevel::O2;

        target
    }

    /// Create a new target for const evaluation.
    pub fn const_evaluation() -> Self {
        Self::bytecode()
    }

    /// Create a target with native freestanding output.
    pub fn native_freestanding() -> Self {
        let mut target = Self::native();
        target.platform = Platform::None;
        target.host = Host::Freestanding;

        target
    }

    /// Create a target with native embedded output.
    pub fn native_embedded() -> Self {
        let mut target = Self::native();
        target.platform = Platform::None;
        target.host = Host::Freestanding;

        target
    }

    /// Create a built-in target configuration for a known target name.
    pub fn builtin_for_name(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::default()),
            "js" => Some(Self::js()),
            "bytecode" => Some(Self::bytecode()),
            "wasm" => Some(Self::wasm_js()),
            "wasm-wasi" | "wasi" => Some(Self::wasm_wasi()),
            "native" => Some(Self::native()),
            _ => None,
        }
    }

    /// Derive the output mode from the target configuration.
    pub fn output_mode(&self) -> OutputMode {
        if self.destination.file.is_some() || matches!(self.output, Output::Program) {
            return OutputMode::File;
        }

        OutputMode::Directory
    }

    /// Return how this target chooses its root module set.
    pub fn root(&self) -> TargetRoot {
        if self.entry.is_empty() {
            TargetRoot::Include
        } else {
            TargetRoot::Entry
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

    /// Build compiler options adjusted for this target.
    pub fn compiler_options(&self, compiler_options: &CompilerOptions) -> CompilerOptions {
        let mut compiler_options = compiler_options.clone();

        // target defaults
        if self.compiler.tree.is_some() {
            compiler_options.tree = self.compiler.tree.clone();
        }
        compiler_options.modes.extend(self.conditions.modes.clone());
        compiler_options.roles.extend(self.conditions.roles.clone());
        compiler_options
            .features
            .extend(self.conditions.features.clone());
        compiler_options.tags.extend(self.conditions.tags.clone());
        compiler_options.globals.extend(self.globals.clone());
        compiler_options.derive.extend(self.compiler.derive.clone());
        compiler_options
            .restrictions
            .tighten_with(&self.compiler.restrictions);

        compiler_options
    }

    /// Return the runtime selected by this target.
    pub fn runtime(&self) -> Runtime {
        match self.output {
            Output::Bundle => Runtime::Js,
            Output::Program => Runtime::Tspp,
        }
    }

    /// Return whether this target assembles one target level output shape.
    pub fn emits_assembled_output(&self) -> bool {
        is_assembled_target(
            Some(self.js.mode),
            self.root(),
            self.entry.len(),
            self.output,
            self.js.preserve_modules,
            !self.js.manual_chunks.is_empty(),
            self.destination.file.is_some(),
        )
    }

    /// Return whether this target emits one file per source module.
    pub fn emits_per_module_output(&self) -> bool {
        !self.emits_assembled_output()
    }

    /// Return whether this target should minify assembled JavaScript, HTML, or CSS output text.
    pub fn should_minify_js_output(&self) -> bool {
        self.js.minify.minifies_output()
    }

    /// Return whether this target should compact printed bundled JavaScript output.
    pub fn should_minify_js_text(&self) -> bool {
        self.js.minify.enabled || self.js.minify.whitespace
    }

    /// Return whether this target should structurally minify bundled JavaScript syntax.
    pub fn should_minify_js_syntax(&self) -> bool {
        self.js.minify.enabled || self.js.minify.syntax
    }

    /// Return whether this target emits any source map data.
    pub fn emits_source_maps(&self) -> bool {
        self.source_map.is_some()
    }

    /// Return whether this target emits standalone source map outputs.
    pub fn emits_source_map_output(&self) -> bool {
        self.source_map.is_some_and(SourceMapMode::emits_output)
    }

    /// Return whether this target inlines source maps into text outputs.
    pub fn uses_inline_source_maps(&self) -> bool {
        self.source_map.is_some_and(SourceMapMode::is_inline)
    }

    /// Resolve a target triple string from the target configuration.
    pub fn resolved_target_triple(&self) -> Option<String> {
        let architecture = self.architecture.as_ref()?;
        let os = self
            .platform
            .triple_os_component()
            .or_else(|| self.host.triple_system_component())
            .unwrap_or("unknown");

        let vendor = self
            .vendor
            .clone()
            .unwrap_or_else(|| TargetVendor::default_for_platform(self.platform));
        let env = self
            .abi
            .clone()
            .or_else(|| TargetAbi::default_for_platform(self.platform));

        let mut triple = format!(
            "{}-{}-{os}",
            architecture.triple_component(),
            vendor.triple_component()
        );
        if let Some(env) = env {
            triple.push('-');
            triple.push_str(env.triple_component());
        }

        Some(triple)
    }

    /// Resolve the absolute output directory for this target.
    ///
    /// Relative destinations resolve from the package directory.
    pub fn resolve_output_directory(&self, package_directory: &Path) -> PathBuf {
        if self.destination.directory.is_absolute() {
            self.destination.directory.clone()
        } else {
            package_directory.join(&self.destination.directory)
        }
    }

    /// Resolve the output file path for a module.
    ///
    /// Output structure mirrors source structure below the compiler root directory.
    pub fn resolve_output_file(
        &self,
        package_directory: &Path,
        root_directory: Option<&Path>,
        module_path: &Path,
        extension: &str,
    ) -> PathBuf {
        let output_directory = self.resolve_output_directory(package_directory);

        // module path
        let relative =
            self.relative_module_output_path(package_directory, root_directory, module_path);

        // change extension and join with output directory
        output_directory.join(relative.with_extension(extension))
    }

    /// Return the relative output path for one emitted module artifact.
    fn relative_module_output_path(
        &self,
        package_directory: &Path,
        root_directory: Option<&Path>,
        module_path: &Path,
    ) -> PathBuf {
        let absolute_module_path = if module_path.is_absolute() {
            module_path.to_path_buf()
        } else {
            package_directory.join(module_path)
        };
        let relative = absolute_module_path
            .strip_prefix(package_directory)
            .unwrap_or(module_path);

        // preserve modules root
        if let Some(relative) =
            self.strip_preserve_modules_root(package_directory, &absolute_module_path, relative)
        {
            return relative.to_path_buf();
        }

        // compiler root dir
        if let Some(root_directory) = root_directory {
            return absolute_module_path
                .strip_prefix(root_directory)
                .unwrap_or(relative)
                .to_path_buf();
        }

        relative.to_path_buf()
    }

    /// Strip the configured preserve-modules root from one module path when possible.
    fn strip_preserve_modules_root<'a>(
        &self,
        package_directory: &Path,
        module_path: &'a Path,
        package_relative_path: &'a Path,
    ) -> Option<&'a Path> {
        let preserve_modules_root = self.js.preserve_modules_root.as_deref()?;

        // package relative root
        if let Ok(relative) = package_relative_path.strip_prefix(preserve_modules_root) {
            return Some(relative);
        }

        // absolute root
        if preserve_modules_root.is_absolute() {
            return module_path.strip_prefix(preserve_modules_root).ok();
        }

        let absolute_root = package_directory.join(preserve_modules_root);

        module_path.strip_prefix(&absolute_root).ok()
    }
}

/// User callable launched by a Destack target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Entrypoint {
    /// Module containing the exported entrypoint function.
    pub module: PathBuf,
    /// Exported function invoked after runtime bootstrap.
    pub export: String,
}

/// Resolve the JavaScript output mode for a target.
fn resolved_js_output_mode(
    explicit_mode: Option<JsOutputMode>,
    root: TargetRoot,
    entry_count: usize,
    output: Output,
    has_output_file: bool,
    preserves_modules: bool,
    has_manual_chunks: bool,
) -> JsOutputMode {
    if has_output_file || matches!(output, Output::Program) {
        return JsOutputMode::SingleFile;
    }

    if let Some(explicit_mode) = explicit_mode {
        return explicit_mode;
    }

    if preserves_modules {
        return JsOutputMode::PreserveModules;
    }

    if has_manual_chunks {
        return JsOutputMode::Chunked;
    }

    if root == TargetRoot::Entry && entry_count > 1 {
        return JsOutputMode::Chunked;
    }

    match root {
        TargetRoot::Entry => JsOutputMode::SingleFile,
        TargetRoot::Include => JsOutputMode::PreserveModules,
    }
}

/// Returns `true` if the target is an assembled target (i.e. it uses entry output layout).
fn is_assembled_target(
    explicit_mode: Option<JsOutputMode>,
    root: TargetRoot,
    entry_count: usize,
    output: Output,
    preserves_modules: bool,
    has_manual_chunks: bool,
    has_output_file: bool,
) -> bool {
    let mode = resolved_js_output_mode(
        explicit_mode,
        root,
        entry_count,
        output,
        has_output_file,
        preserves_modules,
        has_manual_chunks,
    );

    if has_output_file || matches!(output, Output::Program) {
        return true;
    }

    mode.uses_entry_output_layout()
}
