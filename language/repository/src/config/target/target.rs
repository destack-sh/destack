use std::path::{Path, PathBuf};

use destack_artifact::{EmitFormat, Host, Platform, Runtime, TargetAbi, TargetVendor};
use destack_serde::Reflect;
use destack_source::TargetId;
use serde::{Deserialize, Serialize};

use crate::{CompilerOptions, Policy};

use super::super::runtime::RuntimeOptions;
use super::compiler::*;
use super::condition::*;
use super::js::*;
use super::native::*;
use super::output::*;

/// A build target configuration.
#[derive(Debug, Clone, Hash, Serialize, Deserialize, Reflect)]
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
    /// Emitted artifact family (js, ts, bytecode, wasm, native).
    pub emit: EmitFormat,
    /// Target operating system.
    pub platform: Platform,
    /// Target host environment.
    pub host: Host,
    /// Source graph conditions for this target.
    pub conditions: TargetConditionSet,
    /// Compiler behavior for this target.
    pub compiler: TargetCompilerOptions,
    /// Output paths and metadata options.
    pub output: TargetOutputOptions,
    /// JavaScript output configuration.
    pub js: TargetJsOptions,
    /// Native codegen and linking configuration.
    pub native: TargetNativeOptions,
    /// Runtime execution options.
    pub execution: RuntimeOptions,
}

impl Default for Target {
    fn default() -> Self {
        Self::js()
    }
}

impl Target {
    /// Create a target with explicit output and host axes.
    fn new(emit: EmitFormat, host: Host) -> Self {
        Self {
            entry: Vec::new(),
            entrypoint: None,
            globals: Vec::new(),
            include: Vec::new(),
            exclude: Vec::new(),
            policy: Policy::default(),
            emit,
            platform: Platform::Unknown,
            host,
            conditions: TargetConditionSet::default(),
            compiler: TargetCompilerOptions::default(),
            output: TargetOutputOptions::default(),
            js: TargetJsOptions::default(),
            native: TargetNativeOptions::default(),
            execution: RuntimeOptions::default(),
        }
    }

    /// Return the known built-in target names.
    pub fn builtin_target_names() -> &'static [&'static str] {
        &[
            "default",
            "js",
            "ts",
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
        Self::new(EmitFormat::Js, Host::Browser)
    }

    /// Create a target with TypeScript output.
    pub fn ts() -> Self {
        Self::new(EmitFormat::Ts, Host::Browser)
    }

    /// Create a target with Destack bytecode output.
    pub fn bytecode() -> Self {
        let mut target = Self::new(EmitFormat::Bytecode, Host::Native);
        target.compiler.optimize = OptimizeLevel::O2;

        target
    }

    /// Create a target with WASM output for JavaScript hosts.
    pub fn wasm_js() -> Self {
        let mut target = Self::new(EmitFormat::Wasm, Host::Browser);
        target.compiler.optimize = OptimizeLevel::O2;

        target
    }

    /// Create a target with WASM output for WASI.
    pub fn wasm_wasi() -> Self {
        let mut target = Self::new(EmitFormat::Wasm, Host::Wasi);
        target.compiler.optimize = OptimizeLevel::O2;

        target
    }

    /// Create a target with native output.
    pub fn native() -> Self {
        let mut target = Self::new(EmitFormat::Native, Host::Native);
        target.compiler.optimize = OptimizeLevel::O2;

        target
    }

    /// Create a new target for comptime execution.
    pub fn comptime() -> Self {
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
            "ts" => Some(Self::ts()),
            "bytecode" => Some(Self::bytecode()),
            "wasm" => Some(Self::wasm_js()),
            "wasm-wasi" | "wasi" => Some(Self::wasm_wasi()),
            "native" => Some(Self::native()),
            _ => None,
        }
    }

    /// Derive the output mode from the target configuration.
    pub fn output_mode(&self) -> OutputMode {
        if self.output.file.is_some() || self.emit.is_single_file() {
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
        if self.emit.is_script() {
            Runtime::Js
        } else {
            Runtime::Destack
        }
    }

    /// Return whether this target assembles one target level output shape.
    pub fn emits_assembled_output(&self) -> bool {
        is_assembled_target(
            Some(self.js.mode),
            self.root(),
            self.entry.len(),
            self.emit,
            self.js.preserve_modules,
            self.js.manual_chunks.is_empty(),
            self.output.file.is_some(),
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

    /// Return the normalized source map mode for this target.
    pub fn source_map_mode(&self) -> Option<SourceMapMode> {
        self.js.output.source_map.or(self.output.source_map)
    }

    /// Resolve a target triple string from the target configuration.
    pub fn resolved_target_triple(&self) -> Option<String> {
        let target_arch = self.native.arch.as_ref()?;
        let os = self
            .platform
            .triple_os_component()
            .or_else(|| self.host.triple_system_component())?;

        let vendor = self
            .native
            .vendor
            .clone()
            .unwrap_or_else(|| TargetVendor::default_for_platform(self.platform));
        let env = self
            .native
            .abi
            .clone()
            .or_else(|| TargetAbi::default_for_platform(self.platform));

        let mut triple = format!(
            "{}-{}-{os}",
            target_arch.triple_component(),
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
    /// If out_dir is relative, it's resolved relative to the package directory.
    /// If out_dir is absolute, it's returned as-is.
    pub fn resolve_out_dir(&self, package_dir: &Path) -> PathBuf {
        if self.output.directory.is_absolute() {
            self.output.directory.clone()
        } else {
            package_dir.join(&self.output.directory)
        }
    }

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
        let absolute_module_path = if module_path.is_absolute() {
            module_path.to_path_buf()
        } else {
            package_dir.join(module_path)
        };
        let relative = absolute_module_path
            .strip_prefix(package_dir)
            .unwrap_or(module_path);

        // preserve modules root
        if let Some(relative) =
            self.strip_preserve_modules_root(package_dir, &absolute_module_path, relative)
        {
            return relative.to_path_buf();
        }

        // compiler root dir
        if let Some(root_dir) = root_dir {
            return absolute_module_path
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
        let preserve_modules_root = self.js.preserve_modules_root.as_deref()?;

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

/// User callable launched by a Destack target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
    emit: EmitFormat,
    out_file: bool,
    preserve_modules: bool,
    manual_chunks_is_empty: bool,
) -> JsOutputMode {
    if out_file || emit.is_single_file() {
        return JsOutputMode::SingleFile;
    }

    if let Some(explicit_mode) = explicit_mode {
        return explicit_mode;
    }

    if preserve_modules {
        return JsOutputMode::PreserveModules;
    }

    if !manual_chunks_is_empty {
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
    emit: EmitFormat,
    preserve_modules: bool,
    manual_chunks_is_empty: bool,
    out_file: bool,
) -> bool {
    let mode = resolved_js_output_mode(
        explicit_mode,
        root,
        entry_count,
        emit,
        out_file,
        preserve_modules,
        manual_chunks_is_empty,
    );

    if out_file || emit.is_single_file() {
        return true;
    }

    mode.uses_entry_output_layout()
}
