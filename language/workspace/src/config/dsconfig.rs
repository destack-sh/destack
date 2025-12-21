use std::path::PathBuf;
use std::sync::Arc;

use indexmap::IndexMap;
use serde::Deserialize;

use destack_source::{File, FileContent, FileId, IndentStyle, LineEnding};

use crate::{
    ArrayTypeStyle, ArrowParentheses, FilenameCase, FormatterOptions, ImportSortOrder,
    LintCategory, LintPreset, LintSeverity, LinterOptions, OrganizeImports, QuoteProperty,
    QuoteStyle, TrailingComma, TypeDefinitionStyle,
};

use super::target::{
    Allocator, BoundsCheckPolicy, DebugInfoLevel, LinkMode, OptimizeLevel, OutputFormat,
    OutputMode, OverflowCheckPolicy, PanicStrategy, Platform, RelocationModel, Runtime,
    ShrinkLevel, StripLevel, Target, TargetDiscovery, UnwindFormat,
};
use super::tsconfig::{EsTarget, ModuleTarget};

/// Destack configuration (from `dsconfig.json`, 1:1 with Package).
#[derive(Debug, Clone)]
pub struct DsConfig {
    /// The id of the `dsconfig.json` file.
    pub file_id: FileId,
    /// Path to the `dsconfig.json` file.
    pub path: PathBuf,
    /// The directory containing the `dsconfig.json` file.
    pub directory: PathBuf,
    /// The normalized/resolved configuration options.
    pub options: DsConfigOptions,
    /// The raw JSON content of the `dsconfig.json` file.
    pub content: DsConfigJson,
}

impl DsConfig {
    /// Parse a dsconfig from a File with JSON content.
    pub fn parse(file: &Arc<File>) -> Result<Self, serde_json::Error> {
        // extract the JSON value from file content
        let FileContent::Json { value, .. } = &file.content else {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file is not JSON",
            )));
        };

        // parse the dsconfig from the JSON value
        let dsconfig_json: DsConfigJson = serde_json::from_value(value.clone())?;

        // extract path from file (prefer file.path, fall back to URI conversion)
        let path = file
            .path
            .clone()
            .or_else(|| file.uri.to_path_buf())
            .expect("dsconfig file must have a valid path");
        let directory = path
            .parent()
            .expect("dsconfig.json must have a parent directory")
            .to_path_buf();

        // create initial options from JSON
        let options = DsConfigOptions::from(&dsconfig_json);

        let dsconfig = Self {
            file_id: file.id,
            path,
            directory,
            content: dsconfig_json,
            options,
        };
        Ok(dsconfig)
    }

    /// Inherits settings from the given dsconfig into `self`.
    ///
    /// Type checking options use "most restrictive wins" semantics:
    /// if parent is stricter, child inherits it unless explicitly overridden.
    pub fn extend_from(&mut self, dsconfig: &Self) {
        let parent = &dsconfig.options;

        // extend files/include/exclude (child overrides if non-empty)
        if self.options.files.is_empty() {
            self.options.files = parent.files.clone();
        }
        if self.options.include.is_empty() {
            self.options.include = parent.include.clone();
        }
        if self.options.exclude.is_empty() {
            self.options.exclude = parent.exclude.clone();
        }

        // extend compiler options
        let parent_compiler = &parent.compiler;
        let compiler = &mut self.options.compiler;

        // inherit module resolution (child overrides if set)
        if compiler.base_url.is_none() {
            compiler.base_url = parent_compiler.base_url.clone();
        }
        if compiler.paths.is_none() {
            compiler.paths = parent_compiler.paths.clone();
        }

        // inherit lib (child overrides if set)
        if compiler.lib.is_empty() {
            compiler.lib = parent_compiler.lib.clone();
        }

        // inherit TypeScript-compatible checking options (stricter wins)
        compiler.strict = compiler.strict || parent_compiler.strict;
        compiler.no_implicit_any = compiler.no_implicit_any || parent_compiler.no_implicit_any;
        compiler.strict_null_checks =
            compiler.strict_null_checks || parent_compiler.strict_null_checks;
        compiler.no_implicit_this = compiler.no_implicit_this || parent_compiler.no_implicit_this;
        compiler.strict_function_types =
            compiler.strict_function_types || parent_compiler.strict_function_types;
        compiler.strict_bind_call_apply =
            compiler.strict_bind_call_apply || parent_compiler.strict_bind_call_apply;
        compiler.strict_property_initialization = compiler.strict_property_initialization
            || parent_compiler.strict_property_initialization;
        compiler.use_unknown_in_catch_variables = compiler.use_unknown_in_catch_variables
            || parent_compiler.use_unknown_in_catch_variables;
        compiler.no_unused_locals = compiler.no_unused_locals || parent_compiler.no_unused_locals;
        compiler.no_unused_parameters =
            compiler.no_unused_parameters || parent_compiler.no_unused_parameters;
        compiler.no_implicit_returns =
            compiler.no_implicit_returns || parent_compiler.no_implicit_returns;
        // allow_unreachable_code: false is stricter (disallows), so AND them
        compiler.allow_unreachable_code =
            compiler.allow_unreachable_code && parent_compiler.allow_unreachable_code;
        compiler.no_implicit_override =
            compiler.no_implicit_override || parent_compiler.no_implicit_override;
        compiler.no_fallthrough_cases_in_switch = compiler.no_fallthrough_cases_in_switch
            || parent_compiler.no_fallthrough_cases_in_switch;
        compiler.exact_optional_property_types =
            compiler.exact_optional_property_types || parent_compiler.exact_optional_property_types;
        compiler.no_indexed_access_unchecked =
            compiler.no_indexed_access_unchecked || parent_compiler.no_indexed_access_unchecked;

        // inherit Destack-specific checking (stricter wins)
        compiler.no_any = compiler.no_any || parent_compiler.no_any;
        compiler.no_unknown = compiler.no_unknown || parent_compiler.no_unknown;
        compiler.no_imprecise_primitives =
            compiler.no_imprecise_primitives || parent_compiler.no_imprecise_primitives;
        compiler.no_implicit_conversions =
            compiler.no_implicit_conversions || parent_compiler.no_implicit_conversions;
        compiler.no_unsafe_type_assertions =
            compiler.no_unsafe_type_assertions || parent_compiler.no_unsafe_type_assertions;
        compiler.no_implicit_self = compiler.no_implicit_self || parent_compiler.no_implicit_self;
        compiler.no_arguments = compiler.no_arguments || parent_compiler.no_arguments;
        compiler.no_redeclared_locals =
            compiler.no_redeclared_locals || parent_compiler.no_redeclared_locals;
        compiler.no_implicit_managed_type =
            compiler.no_implicit_managed_type || parent_compiler.no_implicit_managed_type;
        compiler.no_implicit_managed_value =
            compiler.no_implicit_managed_value || parent_compiler.no_implicit_managed_value;
        compiler.no_managed = compiler.no_managed || parent_compiler.no_managed;
        compiler.no_dynamic_evaluation =
            compiler.no_dynamic_evaluation || parent_compiler.no_dynamic_evaluation;
        compiler.no_global_this = compiler.no_global_this || parent_compiler.no_global_this;
        compiler.no_dynamic_import =
            compiler.no_dynamic_import || parent_compiler.no_dynamic_import;
        compiler.no_dynamic_shapes =
            compiler.no_dynamic_shapes || parent_compiler.no_dynamic_shapes;
        compiler.no_computed_property_access =
            compiler.no_computed_property_access || parent_compiler.no_computed_property_access;
        compiler.no_proxy = compiler.no_proxy || parent_compiler.no_proxy;
        compiler.no_implicit_dynamic_dispatch =
            compiler.no_implicit_dynamic_dispatch || parent_compiler.no_implicit_dynamic_dispatch;
        compiler.no_exceptions = compiler.no_exceptions || parent_compiler.no_exceptions;

        // inherit emit settings (child overrides if set)
        if compiler.root_dir.is_none() {
            compiler.root_dir = parent_compiler.root_dir.clone();
        }
        if compiler.out_dir.is_none() {
            compiler.out_dir = parent_compiler.out_dir.clone();
        }
        if compiler.declaration_dir.is_none() {
            compiler.declaration_dir = parent_compiler.declaration_dir.clone();
        }

        // inherit interop settings (child overrides if set)
        if compiler.tsconfig.is_none() {
            compiler.tsconfig = parent_compiler.tsconfig.clone();
        }

        // inherit formatter options (child overrides if explicitly set in JSON)
        let child_json = &self.content.formatter;
        // layout
        if child_json.line_ending.is_none() {
            self.options.formatter.line_ending = parent.formatter.line_ending;
        }
        if child_json.indent_style.is_none() && child_json.use_tabs.is_none() {
            self.options.formatter.indent_style = parent.formatter.indent_style;
        }
        if child_json.indent_width.is_none() {
            self.options.formatter.indent_width = parent.formatter.indent_width;
        }
        if child_json.line_width.is_none() {
            self.options.formatter.line_width = parent.formatter.line_width;
        }
        // syntax
        if child_json.quote_style.is_none() && child_json.single_quote.is_none() {
            self.options.formatter.quote_style = parent.formatter.quote_style;
        }
        if child_json.trailing_comma.is_none() {
            self.options.formatter.trailing_comma = parent.formatter.trailing_comma;
        }
        if child_json.bracket_spacing.is_none() {
            self.options.formatter.bracket_spacing = parent.formatter.bracket_spacing;
        }
        if child_json.arrow_parens.is_none() {
            self.options.formatter.arrow_parentheses = parent.formatter.arrow_parentheses;
        }
        if child_json.quote_props.is_none() {
            self.options.formatter.quote_property = parent.formatter.quote_property;
        }
        // tree/jsx
        if child_json.bracket_same_line.is_none() {
            self.options.formatter.bracket_same_line = parent.formatter.bracket_same_line;
        }
        if child_json.single_attribute_per_line.is_none() {
            self.options.formatter.single_attribute_per_line =
                parent.formatter.single_attribute_per_line;
        }

        // inherit linter options (child overrides if explicitly set in JSON)
        let child_linter_json = &self.content.linter;
        if child_linter_json.enabled.is_none() {
            self.options.linter.enabled = parent.linter.enabled;
        }
        if child_linter_json.rules.preset.is_none()
            && child_linter_json.rules.recommended.is_none()
            && child_linter_json.rules.all.is_none()
        {
            self.options.linter.preset = parent.linter.preset;
        }
        // merge category overrides (child takes precedence)
        for (category, severity) in &parent.linter.categories {
            if !self.options.linter.categories.contains_key(category) {
                self.options.linter.categories.insert(*category, *severity);
            }
        }
        // merge rule overrides (child takes precedence)
        for (rule, severity) in &parent.linter.overrides {
            if !self.options.linter.overrides.contains_key(rule) {
                self.options
                    .linter
                    .overrides
                    .insert(rule.clone(), *severity);
            }
        }

        // extend targets (add missing targets from parent)
        for (name, target) in &parent.targets {
            if !self.options.targets.contains_key(name) {
                self.options.targets.insert(name.clone(), target.clone());
            }
        }
    }

    /// "Build" the root dsconfig in place, finalizing options.
    pub fn build(&mut self) {
        // currently no special build steps needed for dsconfig
        // this is here for symmetry with TsConfig::build()
    }
}

/// Normalized Destack package configuration options (from `dsconfig.json`).
#[derive(Debug, Clone, Default)]
pub struct DsConfigOptions {
    /// Specific files to include in the project.
    pub files: Vec<String>,
    /// Glob patterns for files to include.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,
    /// Compiler options.
    pub compiler: DsConfigCompilerOptions,
    /// Formatter options.
    pub formatter: FormatterOptions,
    /// Linter options.
    pub linter: LinterOptions,
    /// Build targets.
    pub targets: IndexMap<String, DsConfigTargetOptions>,
}

/// Path alias mapping (resolved from dsconfig paths).
pub type DsPathAliases = IndexMap<String, Vec<String>>;

/// Normalized Destack compiler options.
///
/// **By default, strict mode is ON.**
/// Destack defaults to stricter type checking than TypeScript.
#[derive(Debug, Clone)]
pub struct DsConfigCompilerOptions {
    // module resolution
    /// Base URL for resolving non-relative module names.
    pub base_url: Option<PathBuf>,
    /// Path alias mappings (resolved relative to baseUrl).
    pub paths: Option<DsPathAliases>,

    // module & target
    /// Module format for output.
    pub module: ModuleTarget,
    /// ECMAScript target version.
    pub es_target: EsTarget,
    /// Library files to include (e.g., "es2024", "dom", "worker").
    pub lib: Vec<String>,

    // TypeScript-compatible checking
    /// Enable all TypeScript-compatible strict type-checking options.
    pub strict: bool,
    /// Error on implicit `any`.
    pub no_implicit_any: bool,
    /// Strict null checks.
    pub strict_null_checks: bool,
    /// Error on implicit `this`.
    pub no_implicit_this: bool,
    /// Strict function types.
    pub strict_function_types: bool,
    /// Strict bind/call/apply.
    pub strict_bind_call_apply: bool,
    /// Strict property initialization.
    pub strict_property_initialization: bool,
    /// Use `unknown` in catch variables.
    pub use_unknown_in_catch_variables: bool,
    /// Report errors on unused locals.
    pub no_unused_locals: bool,
    /// Report errors on unused parameters.
    pub no_unused_parameters: bool,
    /// Require explicit returns.
    pub no_implicit_returns: bool,
    /// Allow unreachable code.
    pub allow_unreachable_code: bool,
    /// Require `override` keyword.
    pub no_implicit_override: bool,
    /// No switch fallthrough.
    pub no_fallthrough_cases_in_switch: bool,
    /// Exact optional property types.
    pub exact_optional_property_types: bool,
    /// Add `undefined` to index access.
    pub no_indexed_access_unchecked: bool,

    // Destack-specific checking
    /// Forbid use of `any` type.
    pub no_any: bool,
    /// Forbid use of `unknown` type.
    pub no_unknown: bool,
    /// Require precise primitive types (int32 vs number, etc.).
    pub no_imprecise_primitives: bool,
    /// Require explicit widening/narrowing conversions.
    pub no_implicit_conversions: bool,
    /// Forbid unsafe type assertions (`as T`).
    pub no_unsafe_type_assertions: bool,
    /// Require explicit `self.` for member access in methods.
    pub no_implicit_self: bool,
    /// Forbid `arguments` object (use rest parameters instead).
    pub no_arguments: bool,
    /// Forbid re-declaration of local variables.
    pub no_redeclared_locals: bool,
    /// Require `^T` or `&T` in type positions (no implicit managed types).
    pub no_implicit_managed_type: bool,
    /// Require explicit copy/borrow at call sites (no implicit managed values).
    pub no_implicit_managed_value: bool,
    /// Forbid managed memory features entirely (no unowned `T` at all, pure value types only).
    pub no_managed: bool,
    /// Forbid runtime entirely (no managed memory, no Promise, no exceptions, ...)
    pub no_runtime: bool,
    /// Forbid referential equality.
    pub no_referential_equality: bool,
    /// Forbid `eval()` and `Function` constructor.
    pub no_dynamic_evaluation: bool,
    /// Forbid `globalThis` access.
    pub no_global_this: bool,
    /// Forbid dynamic `import()` and `require()` expressions.
    pub no_dynamic_import: bool,
    /// Forbid defineProperty, prototype mutation, delete (require static object shapes).
    pub no_dynamic_shapes: bool,
    /// Forbid computed property access `obj[expr]` where expr isn't constant.
    pub no_computed_property_access: bool,
    /// Forbid `Proxy`.
    pub no_proxy: bool,
    /// Require overloads to be statically resolvable (no implicit runtime dispatch).
    pub no_implicit_dynamic_dispatch: bool,
    /// Forbid `throw` and `try`/`catch` (use Result types instead).
    pub no_exceptions: bool,

    // emit
    /// Root directory of source files (controls output directory structure, not module resolution).
    pub root_dir: Option<PathBuf>,
    /// Output directory for compiled files.
    pub out_dir: Option<PathBuf>,
    /// Output directory for declaration files. Defaults to out_dir.
    pub declaration_dir: Option<PathBuf>,

    // interop
    /// Path to tsconfig.json to inherit settings from.
    pub tsconfig: Option<PathBuf>,
    /// Allow TypeScript files (.ts, .tsx) in the project.
    pub allow_ts: bool,
    /// Type-check TypeScript files.
    pub check_ts: bool,
    /// Allow JavaScript files (.js, .jsx) in the project.
    pub allow_js: bool,
    /// Type-check JavaScript files.
    pub check_js: bool,
    /// Skip type checking of declaration files.
    pub skip_lib_check: bool,
}

impl Default for DsConfigCompilerOptions {
    fn default() -> Self {
        // defaults to strict mode ON (stricter than TypeScript)
        let strict = true;
        Self {
            base_url: None,
            paths: None,
            module: ModuleTarget::default(),
            es_target: EsTarget::default(),
            lib: Vec::new(), // derived from runtime/platform if empty

            // TypeScript-compatible checking
            strict,
            no_implicit_any: strict,
            strict_null_checks: strict,
            no_implicit_this: strict,
            strict_function_types: strict,
            strict_bind_call_apply: strict,
            strict_property_initialization: strict,
            use_unknown_in_catch_variables: strict,
            no_unused_locals: false,
            no_unused_parameters: false,
            no_implicit_returns: true,
            allow_unreachable_code: false,
            no_implicit_override: true,
            no_fallthrough_cases_in_switch: true,
            exact_optional_property_types: false,
            no_indexed_access_unchecked: false,

            // Destack-specific checking (all off by default, opt-in)
            no_any: false,
            no_unknown: false,
            no_imprecise_primitives: false,
            no_implicit_conversions: false,
            no_unsafe_type_assertions: false,
            no_implicit_self: false,
            no_arguments: false,
            no_redeclared_locals: false,
            no_implicit_managed_type: false,
            no_implicit_managed_value: false,
            no_managed: false,
            no_runtime: false,
            no_referential_equality: false,
            no_dynamic_evaluation: false,
            no_global_this: false,
            no_dynamic_import: false,
            no_dynamic_shapes: false,
            no_computed_property_access: false,
            no_proxy: false,
            no_implicit_dynamic_dispatch: false,
            no_exceptions: false,

            // emit
            root_dir: None,
            out_dir: None,
            declaration_dir: None,

            // interop
            tsconfig: None,
            allow_ts: true,
            check_ts: false,
            allow_js: true,
            check_js: false,
            skip_lib_check: false,
        }
    }
}

/// DsConfig JSON (usually from `dsconfig.json`)
#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigJson {
    /// Extends other dsconfigs or tsconfigs.
    pub extends: Option<ExtendsFieldJson>,
    /// Specific files to include in the project.
    pub files: Option<Vec<String>>,
    /// Glob patterns for files to include.
    pub include: Option<Vec<String>>,
    /// Glob patterns for files to exclude.
    pub exclude: Option<Vec<String>>,
    /// Compiler options.
    #[serde(default)]
    pub compiler_options: CompilerOptionsJson,
    /// Formatter options.
    #[serde(default)]
    pub formatter: DsConfigFormatterJson,
    /// Linter options.
    #[serde(default)]
    pub linter: DsConfigLinterJson,
    /// Build targets.
    pub targets: Option<IndexMap<String, DsConfigTargetJson>>,
}

impl From<&DsConfigJson> for DsConfigOptions {
    fn from(json: &DsConfigJson) -> Self {
        let compiler = DsConfigCompilerOptions::from(&json.compiler_options);

        let mut formatter = FormatterOptions::default();
        json.formatter.apply(&mut formatter);

        let mut linter = LinterOptions::default();
        json.linter.apply(&mut linter);

        Self {
            files: json.files.clone().unwrap_or_default(),
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            compiler,
            formatter,
            linter,
            targets: json
                .targets
                .as_ref()
                .map(|t| {
                    t.iter()
                        .map(|(k, v)| (k.clone(), DsConfigTargetOptions::from(v)))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// Value for the "extends" field of a dsconfig.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ExtendsFieldJson {
    /// Extend a single dsconfig.
    Single(String),
    /// Extend multiple dsconfigs.
    Multiple(Vec<String>),
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
    /// Runtime environment (browser, node, wasm-wasi, destack, etc.).
    pub runtime: Runtime,
    /// Target platform (web, windows, macos, linux, ios, android, etc.).
    pub platform: Platform,
    /// Target triple for native codegen.
    /// This selects the ABI and CPU architecture for native targets.
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
    /// Emit declaration files (.d.ts) alongside JS output.
    pub declaration: bool,
    /// Emit source maps.
    pub source_map: bool,

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
    /// Panic strategy for unrecoverable errors.
    pub panic: PanicStrategy,
    /// Unwind info format for native targets.
    pub unwind: UnwindFormat,
    /// Integer overflow checking policy.
    pub overflow_checks: OverflowCheckPolicy,
    /// Bounds check policy for array and slice accesses.
    pub bounds_checks: BoundsCheckPolicy,
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
            platform: Platform::default(),
            target_triple: None,
            cpu: None,
            cpu_features: Vec::new(),
            relocation_model: RelocationModel::default(),
            link_mode: LinkMode::default(),
            declaration: false,
            source_map: false,
            out_dir: PathBuf::from(super::target::DEFAULT_OUT_DIR),
            out_file: None,
            declaration_dir: None,
            module: ModuleTarget::default(),
            es_target: EsTarget::default(),
            lib: None,
            debug: true,
            optimize: false,
            optimize_level: OptimizeLevel::O0,
            shrink_level: ShrinkLevel::S0,
            debug_info: DebugInfoLevel::default(),
            strip: StripLevel::default(),
            panic: PanicStrategy::default(),
            unwind: UnwindFormat::default(),
            overflow_checks: OverflowCheckPolicy::default(),
            bounds_checks: BoundsCheckPolicy::default(),
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
            discovery: self.discovery,
            entry: self.entry.clone(),
            include: self.include.clone(),
            exclude: self.exclude.clone(),
            output: self.output,
            runtime: self.runtime,
            platform: self.platform,
            target_triple: self.target_triple.clone(),
            cpu: self.cpu.clone(),
            cpu_features: self.cpu_features.clone(),
            relocation_model: self.relocation_model,
            link_mode: self.link_mode,
            declaration: self.declaration,
            source_map: self.source_map,
            out_dir: self.out_dir.clone(),
            out_file: self.out_file.clone(),
            declaration_dir: self.declaration_dir.clone(),
            module: self.module,
            es_target: self.es_target,
            lib: self.lib.clone(),
            debug: self.debug,
            optimize: self.optimize,
            optimize_level: self.optimize_level,
            shrink_level: self.shrink_level,
            debug_info: self.debug_info,
            strip: self.strip,
            panic: self.panic,
            unwind: self.unwind,
            overflow_checks: self.overflow_checks,
            bounds_checks: self.bounds_checks,
            allocator: self.allocator,
        }
    }
}

impl From<&DsConfigTargetJson> for DsConfigTargetOptions {
    fn from(json: &DsConfigTargetJson) -> Self {
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
            platform: json
                .platform
                .as_deref()
                .and_then(Platform::parse)
                .unwrap_or_default(),
            target_triple: json.target_triple.clone(),
            cpu: json.cpu.clone(),
            cpu_features: json.cpu_features.clone().unwrap_or_default(),
            relocation_model: json
                .relocation_model
                .map(RelocationModel::from)
                .unwrap_or_default(),
            link_mode: json.link_mode.map(LinkMode::from).unwrap_or_default(),
            declaration: json.declaration,
            source_map: json.source_map,
            out_dir: json
                .out_dir
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(super::target::DEFAULT_OUT_DIR)),
            out_file: json.out_file.as_ref().map(PathBuf::from),
            declaration_dir: json.declaration_dir.as_ref().map(PathBuf::from),
            module: json
                .module
                .as_deref()
                .and_then(ModuleTarget::parse)
                .unwrap_or_default(),
            es_target: json
                .es_target
                .as_deref()
                .and_then(EsTarget::parse)
                .unwrap_or_default(),
            lib: json.lib.clone(),
            debug: json.debug,
            optimize: json.optimize,
            optimize_level: json
                .optimize_level
                .map(OptimizeLevel::from)
                .unwrap_or_default(),
            shrink_level: json.shrink_level.map(ShrinkLevel::from).unwrap_or_default(),
            debug_info: json
                .debug_info
                .map(DebugInfoLevel::from)
                .unwrap_or_default(),
            strip: json.strip.map(StripLevel::from).unwrap_or_default(),
            panic: json.panic.map(PanicStrategy::from).unwrap_or_default(),
            unwind: json.unwind.map(UnwindFormat::from).unwrap_or_default(),
            overflow_checks: json
                .overflow_checks
                .map(OverflowCheckPolicy::from)
                .unwrap_or_default(),
            bounds_checks: json
                .bounds_checks
                .map(BoundsCheckPolicy::from)
                .unwrap_or_default(),
            allocator: json.allocator.map(Allocator::from).unwrap_or_default(),
        }
    }
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

/// Panic strategy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PanicStrategyJson {
    /// Abort immediately.
    #[serde(alias = "abort")]
    Abort,
    /// Unwind the stack.
    #[serde(alias = "unwind")]
    Unwind,
}

impl From<PanicStrategyJson> for PanicStrategy {
    fn from(value: PanicStrategyJson) -> Self {
        match value {
            PanicStrategyJson::Abort => PanicStrategy::Abort,
            PanicStrategyJson::Unwind => PanicStrategy::Unwind,
        }
    }
}

/// Unwind info format for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum UnwindFormatJson {
    /// No unwind info.
    #[serde(alias = "none")]
    None,
    /// DWARF unwind info.
    #[serde(alias = "dwarf")]
    Dwarf,
    /// Windows SEH unwind info.
    #[serde(alias = "seh")]
    Seh,
}

impl From<UnwindFormatJson> for UnwindFormat {
    fn from(value: UnwindFormatJson) -> Self {
        match value {
            UnwindFormatJson::None => UnwindFormat::None,
            UnwindFormatJson::Dwarf => UnwindFormat::Dwarf,
            UnwindFormatJson::Seh => UnwindFormat::Seh,
        }
    }
}

/// Overflow checking policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OverflowCheckPolicyJson {
    /// Always emit overflow checks.
    #[serde(alias = "always")]
    Always,
    /// Emit overflow checks only in debug builds.
    #[serde(alias = "debug")]
    Debug,
    /// Never emit overflow checks.
    #[serde(alias = "never")]
    #[serde(alias = "off")]
    Never,
}

impl From<OverflowCheckPolicyJson> for OverflowCheckPolicy {
    fn from(value: OverflowCheckPolicyJson) -> Self {
        match value {
            OverflowCheckPolicyJson::Always => OverflowCheckPolicy::Always,
            OverflowCheckPolicyJson::Debug => OverflowCheckPolicy::Debug,
            OverflowCheckPolicyJson::Never => OverflowCheckPolicy::Never,
        }
    }
}

/// Bounds check policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BoundsCheckPolicyJson {
    /// Always emit bounds checks.
    #[serde(alias = "always")]
    Always,
    /// Emit bounds checks only in debug builds.
    #[serde(alias = "debug")]
    Debug,
    /// Never emit bounds checks.
    #[serde(alias = "never")]
    #[serde(alias = "off")]
    Never,
}

impl From<BoundsCheckPolicyJson> for BoundsCheckPolicy {
    fn from(value: BoundsCheckPolicyJson) -> Self {
        match value {
            BoundsCheckPolicyJson::Always => BoundsCheckPolicy::Always,
            BoundsCheckPolicyJson::Debug => BoundsCheckPolicy::Debug,
            BoundsCheckPolicyJson::Never => BoundsCheckPolicy::Never,
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

/// Line ending style for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LineEndingJson {
    /// Unix-style line endings (LF).
    #[serde(rename = "lf")]
    Lf,
    /// Windows-style line endings (CRLF).
    #[serde(rename = "crlf")]
    Crlf,
    /// Classic Mac-style line endings (CR).
    #[serde(rename = "cr")]
    Cr,
}

impl From<LineEndingJson> for LineEnding {
    fn from(value: LineEndingJson) -> Self {
        match value {
            LineEndingJson::Lf => LineEnding::LineFeed,
            LineEndingJson::Crlf => LineEnding::CarriageReturnLineFeed,
            LineEndingJson::Cr => LineEnding::CarriageReturn,
        }
    }
}

/// Indent style for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum IndentStyleJson {
    #[serde(alias = "tabs")]
    Tab,
    #[serde(alias = "spaces")]
    Space,
}

impl From<IndentStyleJson> for IndentStyle {
    fn from(value: IndentStyleJson) -> Self {
        match value {
            IndentStyleJson::Tab => IndentStyle::Tab,
            IndentStyleJson::Space => IndentStyle::Space,
        }
    }
}
/// Destack configuration compiler options.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptionsJson {
    // module resolution
    /// Base URL for resolving non-relative module names.
    pub base_url: Option<String>,
    /// Path alias mappings (resolved relative to baseUrl, like tsconfig paths).
    pub paths: Option<IndexMap<String, Vec<String>>>,

    // module & target
    /// Module format for output (e.g., "esnext", "commonjs").
    pub module: Option<String>,
    /// ECMAScript target version (e.g., "es2022", "esnext").
    pub target: Option<String>,
    /// Library files to include (e.g., ["es2024", "dom"]).
    pub lib: Option<Vec<String>>,

    // TypeScript-compatible checking
    /// Enable all TypeScript-derived strict type-checking options. Default: true for .ds files.
    pub strict: Option<bool>,
    /// Error on expressions and declarations with implied `any` type.
    pub no_implicit_any: Option<bool>,
    /// Enable strict null checks (`null` and `undefined` are distinct types).
    pub strict_null_checks: Option<bool>,
    /// Error on `this` expressions with implied `any` type.
    pub no_implicit_this: Option<bool>,
    /// Enable strict checking of function types (contravariant parameters).
    pub strict_function_types: Option<bool>,
    /// Enable strict checking of `bind`, `call`, and `apply` methods.
    pub strict_bind_call_apply: Option<bool>,
    /// Require class properties to be initialized in constructor.
    pub strict_property_initialization: Option<bool>,
    /// Use `unknown` instead of `any` for catch clause variables.
    pub use_unknown_in_catch_variables: Option<bool>,
    /// Report errors on unused local variables.
    pub no_unused_locals: Option<bool>,
    /// Report errors on unused function parameters.
    pub no_unused_parameters: Option<bool>,
    /// Report error when not all code paths return a value.
    pub no_implicit_returns: Option<bool>,
    /// Allow unreachable code (disables dead code warnings).
    pub allow_unreachable_code: Option<bool>,
    /// Require `override` keyword when overriding class members.
    pub no_implicit_override: Option<bool>,
    /// Report errors for fallthrough cases in switch statements.
    pub no_fallthrough_cases_in_switch: Option<bool>,
    /// Interpret optional property types as written (no implicit `undefined`).
    pub exact_optional_property_types: Option<bool>,
    /// Add `undefined` to index signature results (safer array access).
    pub no_indexed_access_unchecked: Option<bool>,

    // Destack-specific checking
    /// Forbid use of `any` type.
    pub no_any: Option<bool>,
    /// Forbid use of `unknown` type.
    pub no_unknown: Option<bool>,
    /// Require precise primitive types (int32 vs number, etc.).
    pub no_imprecise_primitives: Option<bool>,
    /// Require explicit widening/narrowing conversions.
    pub no_implicit_conversions: Option<bool>,
    /// Forbid unsafe type assertions (`as T`).
    pub no_unsafe_type_assertions: Option<bool>,
    /// Require explicit `self.` for member access in methods.
    pub no_implicit_self: Option<bool>,
    /// Forbid `arguments` object (use rest parameters instead).
    pub no_arguments: Option<bool>,
    /// Forbid re-declaration of local variables.
    pub no_redeclared_locals: Option<bool>,
    /// Require `^T` or `&T` in type positions (no implicit managed types).
    pub no_implicit_managed_type: Option<bool>,
    /// Require explicit copy/borrow at call sites (no implicit managed values).
    pub no_implicit_managed_value: Option<bool>,
    /// Forbid managed runtime features entirely (no &T at all, pure value types only).
    pub no_managed: Option<bool>,
    /// Forbid runtime entirely (no managed memory, no Promise, no exceptions, ...).
    pub no_runtime: Option<bool>,
    /// Forbid referential equality.
    pub no_referential_equality: Option<bool>,
    /// Forbid `eval()` and `Function` constructor.
    pub no_dynamic_evaluation: Option<bool>,
    /// Forbid `globalThis` access.
    pub no_global_this: Option<bool>,
    /// Forbid dynamic `import()` and `require()` expressions.
    pub no_dynamic_import: Option<bool>,
    /// Forbid defineProperty, prototype mutation, delete (require static object shapes).
    pub no_dynamic_shapes: Option<bool>,
    /// Forbid computed property access `obj[expr]` where expr isn't constant.
    pub no_computed_property_access: Option<bool>,
    /// Forbid `Proxy`.
    pub no_proxy: Option<bool>,
    /// Require overloads to be statically resolvable (no implicit runtime dispatch).
    pub no_implicit_dynamic_dispatch: Option<bool>,
    /// Forbid `throw` and `try`/`catch` (use Result types instead).
    pub no_exceptions: Option<bool>,

    // emit
    /// Root directory of source files (controls output directory structure, not module resolution).
    pub root_dir: Option<String>,
    /// Output directory for compiled files.
    pub out_dir: Option<String>,
    /// Output directory for declaration files (.d.ts). Defaults to outDir.
    pub declaration_dir: Option<String>,

    // interop
    /// Path to tsconfig.json to inherit settings from.
    pub tsconfig: Option<String>,
    /// Allow TypeScript files (.ts, .tsx) in the project.
    pub allow_ts: Option<bool>,
    /// Type-check TypeScript files.
    pub check_ts: Option<bool>,
    /// Allow JavaScript files (.js, .jsx) in the project.
    pub allow_js: Option<bool>,
    /// Type-check JavaScript files.
    pub check_js: Option<bool>,
    /// Skip type checking of declaration files (.d.ts, .d.ds).
    pub skip_lib_check: Option<bool>,
}

impl From<&CompilerOptionsJson> for DsConfigCompilerOptions {
    fn from(json: &CompilerOptionsJson) -> Self {
        let strict = json.strict.unwrap_or(true);
        Self {
            base_url: json.base_url.as_ref().map(PathBuf::from),
            paths: json.paths.clone(),
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
            lib: json.lib.clone().unwrap_or_default(),

            // TypeScript-compatible checking
            strict,
            no_implicit_any: json.no_implicit_any.unwrap_or(strict),
            strict_null_checks: json.strict_null_checks.unwrap_or(strict),
            no_implicit_this: json.no_implicit_this.unwrap_or(strict),
            strict_function_types: json.strict_function_types.unwrap_or(strict),
            strict_bind_call_apply: json.strict_bind_call_apply.unwrap_or(strict),
            strict_property_initialization: json.strict_property_initialization.unwrap_or(strict),
            use_unknown_in_catch_variables: json.use_unknown_in_catch_variables.unwrap_or(strict),
            no_unused_locals: json.no_unused_locals.unwrap_or(false),
            no_unused_parameters: json.no_unused_parameters.unwrap_or(false),
            no_implicit_returns: json.no_implicit_returns.unwrap_or(true),
            allow_unreachable_code: json.allow_unreachable_code.unwrap_or(false),
            no_implicit_override: json.no_implicit_override.unwrap_or(true),
            no_fallthrough_cases_in_switch: json.no_fallthrough_cases_in_switch.unwrap_or(true),
            exact_optional_property_types: json.exact_optional_property_types.unwrap_or(false),
            no_indexed_access_unchecked: json.no_indexed_access_unchecked.unwrap_or(false),

            // Destack-specific checking
            no_any: json.no_any.unwrap_or(false),
            no_unknown: json.no_unknown.unwrap_or(false),
            no_imprecise_primitives: json.no_imprecise_primitives.unwrap_or(false),
            no_implicit_conversions: json.no_implicit_conversions.unwrap_or(false),
            no_unsafe_type_assertions: json.no_unsafe_type_assertions.unwrap_or(false),
            no_implicit_self: json.no_implicit_self.unwrap_or(false),
            no_arguments: json.no_arguments.unwrap_or(false),
            no_redeclared_locals: json.no_redeclared_locals.unwrap_or(false),
            no_implicit_managed_type: json.no_implicit_managed_type.unwrap_or(false),
            no_implicit_managed_value: json.no_implicit_managed_value.unwrap_or(false),
            no_managed: json.no_managed.unwrap_or(false),
            no_runtime: json.no_runtime.unwrap_or(false),
            no_referential_equality: json.no_referential_equality.unwrap_or(false),
            no_dynamic_evaluation: json.no_dynamic_evaluation.unwrap_or(false),
            no_global_this: json.no_global_this.unwrap_or(false),
            no_dynamic_import: json.no_dynamic_import.unwrap_or(false),
            no_dynamic_shapes: json.no_dynamic_shapes.unwrap_or(false),
            no_computed_property_access: json.no_computed_property_access.unwrap_or(false),
            no_proxy: json.no_proxy.unwrap_or(false),
            no_implicit_dynamic_dispatch: json.no_implicit_dynamic_dispatch.unwrap_or(false),
            no_exceptions: json.no_exceptions.unwrap_or(false),

            // emit
            root_dir: json.root_dir.as_ref().map(PathBuf::from),
            out_dir: json.out_dir.as_ref().map(PathBuf::from),
            declaration_dir: json.declaration_dir.as_ref().map(PathBuf::from),

            // interop
            tsconfig: json.tsconfig.as_ref().map(PathBuf::from),
            allow_ts: json.allow_ts.unwrap_or(true),
            check_ts: json.check_ts.unwrap_or(false),
            allow_js: json.allow_js.unwrap_or(true),
            check_js: json.check_js.unwrap_or(false),
            skip_lib_check: json.skip_lib_check.unwrap_or(false),
        }
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
    /// Target platform (e.g., Web, Windows, macOS, Linux, iOS, Android, WASI, Universal).
    pub platform: Option<String>,
    /// Target triple for native codegen (e.g., "x86_64-unknown-linux-gnu").
    pub target_triple: Option<String>,
    /// CPU name for native codegen (e.g., "native", "x86-64", "znver3").
    pub cpu: Option<String>,
    /// CPU feature flags for native codegen (e.g., "+sse4.2", "+aes").
    pub cpu_features: Option<Vec<String>>,
    /// Relocation model.
    pub relocation_model: Option<RelocationModelJson>,
    /// Link mode.
    pub link_mode: Option<LinkModeJson>,
    /// Emit declaration files (.d.ts) alongside JS output.
    #[serde(default)]
    pub declaration: bool,
    /// Emit source maps.
    #[serde(default)]
    pub source_map: bool,

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
    pub es_target: Option<String>,
    /// Library files for this target (overrides derived libs).
    pub lib: Option<Vec<String>>,

    // optimization
    /// Whether this is a debug build.
    #[serde(default)]
    pub debug: bool,
    /// Whether optimization is enabled.
    #[serde(default)]
    pub optimize: bool,
    /// Optimization level (0-3).
    pub optimize_level: Option<u8>,
    /// Shrink level (0-3).
    pub shrink_level: Option<u8>,
    /// Debug info emission policy.
    pub debug_info: Option<DebugInfoLevelJson>,
    /// Symbol stripping policy.
    pub strip: Option<StripLevelJson>,
    /// Panic strategy.
    pub panic: Option<PanicStrategyJson>,
    /// Unwind info format.
    pub unwind: Option<UnwindFormatJson>,
    /// Overflow checking policy.
    pub overflow_checks: Option<OverflowCheckPolicyJson>,
    /// Bounds check policy.
    pub bounds_checks: Option<BoundsCheckPolicyJson>,
    /// Global allocator selection.
    pub allocator: Option<AllocatorJson>,
}

/// Quote style for JSON deserialization (Prettier: `singleQuote`).
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum QuoteStyleJson {
    /// Use double quotes.
    Double,
    /// Use single quotes.
    Single,
    /// Use single quotes for single characters, double quotes for strings.
    #[serde(alias = "auto")]
    Semantic,
}

impl From<QuoteStyleJson> for QuoteStyle {
    fn from(value: QuoteStyleJson) -> Self {
        match value {
            QuoteStyleJson::Double => QuoteStyle::Double,
            QuoteStyleJson::Single => QuoteStyle::Single,
            QuoteStyleJson::Semantic => QuoteStyle::Semantic,
        }
    }
}

/// Trailing comma policy for JSON deserialization (Prettier: `trailingComma`).
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TrailingCommaJson {
    /// Trailing commas everywhere valid.
    All,
    /// Trailing commas where valid in ES5.
    Es5,
    /// No trailing commas.
    None,
}

impl From<TrailingCommaJson> for TrailingComma {
    fn from(value: TrailingCommaJson) -> Self {
        match value {
            TrailingCommaJson::All => TrailingComma::All,
            TrailingCommaJson::Es5 => TrailingComma::Es5,
            TrailingCommaJson::None => TrailingComma::None,
        }
    }
}

/// Arrow function parentheses for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ArrowParenthesesJson {
    /// Always include parentheses.
    Always,
    /// Omit when possible.
    Avoid,
}

impl From<ArrowParenthesesJson> for ArrowParentheses {
    fn from(value: ArrowParenthesesJson) -> Self {
        match value {
            ArrowParenthesesJson::Always => ArrowParentheses::Always,
            ArrowParenthesesJson::Avoid => ArrowParentheses::Avoid,
        }
    }
}

/// Object property quoting for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum QuotePropertyJson {
    /// Only quote when required.
    AsNeeded,
    /// Quote all if any require quotes.
    Consistent,
    /// Preserve original quoting.
    Preserve,
}

impl From<QuotePropertyJson> for QuoteProperty {
    fn from(value: QuotePropertyJson) -> Self {
        match value {
            QuotePropertyJson::AsNeeded => QuoteProperty::AsNeeded,
            QuotePropertyJson::Consistent => QuoteProperty::Consistent,
            QuotePropertyJson::Preserve => QuoteProperty::Preserve,
        }
    }
}

/// Whether to organize imports.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OrganizeImportsJson {
    /// Organize imports: sort statements by group and specifiers alphabetically.
    On,
    /// Don't reorder imports (preserve original order).
    Off,
}

impl From<OrganizeImportsJson> for OrganizeImports {
    fn from(value: OrganizeImportsJson) -> Self {
        match value {
            OrganizeImportsJson::On => OrganizeImports::On,
            OrganizeImportsJson::Off => OrganizeImports::Off,
        }
    }
}

/// Sort order for import specifiers.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ImportSortOrderJson {
    /// Natural sort: numbers ordered as integers (a1 < a2 < a10).
    Natural,
    /// Alphabetical/lexicographic sort (a1 < a10 < a2).
    Alphabetical,
}

impl From<ImportSortOrderJson> for ImportSortOrder {
    fn from(value: ImportSortOrderJson) -> Self {
        match value {
            ImportSortOrderJson::Natural => ImportSortOrder::Natural,
            ImportSortOrderJson::Alphabetical => ImportSortOrder::Alphabetical,
        }
    }
}

/// Formatter options (top-level, like Biome/Deno).
///
/// Field names use Prettier-compatible naming for familiarity.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigFormatterJson {
    /// Line ending style: "lf", "crlf", or "cr".
    #[serde(alias = "endOfLine")]
    pub line_ending: Option<LineEndingJson>,
    /// Use tabs instead of spaces.
    #[serde(alias = "useTabs")]
    pub use_tabs: Option<bool>,
    /// Indent style: "tab" or "space".
    pub indent_style: Option<IndentStyleJson>,
    /// Number of spaces per indent. Default: 4.
    #[serde(alias = "tabWidth")]
    pub indent_width: Option<u8>,
    /// Maximum line width (best effort). Default: 100.
    #[serde(alias = "printWidth")]
    pub line_width: Option<u16>,

    /// Quote style: "double", "single", or "semantic".
    pub quote_style: Option<QuoteStyleJson>,
    /// Use single quotes. Takes precedence over quoteStyle.
    pub single_quote: Option<bool>,
    /// Trailing comma policy: "all", "es5", or "none".
    pub trailing_comma: Option<TrailingCommaJson>,
    /// Spaces inside object braces: `{ foo }` (true) vs `{foo}` (false). Default: true.
    pub bracket_spacing: Option<bool>,
    /// Arrow function parentheses: "always" or "avoid".
    pub arrow_parens: Option<ArrowParenthesesJson>,
    /// Object property quoting: "as-needed", "consistent", or "preserve".
    pub quote_props: Option<QuotePropertyJson>,

    /// Put `>` of multi-line JSX on same line as last attribute.
    #[serde(alias = "jsxBracketSameLine")]
    pub bracket_same_line: Option<bool>,
    /// Force each JSX attribute onto its own line.
    pub single_attribute_per_line: Option<bool>,

    /// Whether to organize imports: "on" or "off". Default: off.
    pub organize_imports: Option<OrganizeImportsJson>,
    /// Sort order for import specifiers: "natural" or "alphabetical". Default: natural.
    pub import_sort_order: Option<ImportSortOrderJson>,
}

impl DsConfigFormatterJson {
    /// Apply formatter options to a FormatterOptions struct.
    pub fn apply(&self, options: &mut FormatterOptions) {
        // layout
        if let Some(line_ending) = self.line_ending {
            options.line_ending = line_ending.into();
        }
        if let Some(true) = self.use_tabs {
            options.indent_style = IndentStyle::Tab;
        } else if let Some(indent_style) = self.indent_style {
            options.indent_style = indent_style.into();
        }
        if let Some(indent_width) = self.indent_width {
            options.indent_width = indent_width;
        }
        if let Some(line_width) = self.line_width {
            options.line_width = line_width;
        }

        // syntax: singleQuote takes precedence over quoteStyle
        if let Some(single_quote) = self.single_quote {
            options.quote_style = if single_quote {
                QuoteStyle::Single
            } else {
                QuoteStyle::Double
            };
        } else if let Some(quote_style) = self.quote_style {
            options.quote_style = quote_style.into();
        }
        if let Some(trailing_comma) = self.trailing_comma {
            options.trailing_comma = trailing_comma.into();
        }
        if let Some(bracket_spacing) = self.bracket_spacing {
            options.bracket_spacing = bracket_spacing;
        }
        if let Some(arrow_parens) = self.arrow_parens {
            options.arrow_parentheses = arrow_parens.into();
        }
        if let Some(quote_props) = self.quote_props {
            options.quote_property = quote_props.into();
        }

        // tree/jsx
        if let Some(bracket_same_line) = self.bracket_same_line {
            options.bracket_same_line = bracket_same_line;
        }
        if let Some(single_attribute_per_line) = self.single_attribute_per_line {
            options.single_attribute_per_line = single_attribute_per_line;
        }

        // imports
        if let Some(organize_imports) = self.organize_imports {
            options.organize_imports = organize_imports.into();
        }
        if let Some(import_sort_order) = self.import_sort_order {
            options.import_sort_order = import_sort_order.into();
        }
    }
}

/// Linter options (top-level, like Biome/Deno).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigLinterJson {
    /// Whether linting is enabled. Default: true.
    pub enabled: Option<bool>,
    /// Rule configuration.
    #[serde(default)]
    pub rules: DsConfigLinterRulesJson,

    // complexity thresholds
    /// Maximum boolean parameters or fields.
    pub max_booleans: Option<usize>,
    /// Maximum cognitive complexity.
    pub max_cognitive_complexity: Option<usize>,
    /// Maximum cyclomatic complexity.
    pub max_cyclomatic_complexity: Option<usize>,
    /// Maximum nesting depth.
    pub max_depth: Option<usize>,
    /// Maximum lines per file.
    pub max_lines: Option<usize>,
    /// Maximum lines per function.
    pub max_lines_per_function: Option<usize>,
    /// Maximum callback nesting.
    pub max_nested_callbacks: Option<usize>,
    /// Maximum function parameters.
    pub max_params: Option<usize>,
    /// Maximum statements per function.
    pub max_statements: Option<usize>,
    /// Maximum return statements per function.
    pub max_return_statements: Option<usize>,
    /// Maximum switch cases per switch statement.
    pub max_switch_cases: Option<usize>,
    /// Maximum variants in a union type or enum.
    pub max_type_variants: Option<usize>,
    /// Maximum fields in a struct, class, or interface.
    pub max_type_fields: Option<usize>,
    /// Maximum type complexity (nesting depth of generics/unions/intersections).
    pub max_type_complexity: Option<usize>,
    /// Maximum occurrences of the same string literal before warning.
    pub max_duplicate_string_occurrences: Option<usize>,
    /// Maximum statements in a try block.
    pub max_try_block_statements: Option<usize>,

    // style options
    /// Preferred array type syntax: "array" or "generic".
    pub array_type: Option<ArrayTypeStyleJson>,
    /// Preferred type definition syntax: "type" or "interface".
    pub type_definition_style: Option<TypeDefinitionStyleJson>,
    /// Required catch clause error name.
    pub catch_error_name: Option<String>,
    /// Required filename case style.
    pub filename_case: Option<FilenameCaseJson>,

    // restriction options
    /// Magic numbers to allow.
    pub allowed_magic_numbers: Option<Vec<f64>>,
    /// Globals to restrict.
    pub restricted_globals: Option<Vec<String>>,
    /// Import paths to restrict.
    pub restricted_imports: Option<Vec<String>>,
    /// Comment terms to warn on.
    pub warning_comment_terms: Option<Vec<String>>,
}

impl DsConfigLinterJson {
    /// Apply linter options to a LinterOptions struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(enabled) = self.enabled {
            options.enabled = enabled;
        }
        self.rules.apply(options);

        // complexity thresholds
        if let Some(max_booleans) = self.max_booleans {
            options.max_booleans = max_booleans;
        }
        if let Some(max_cognitive_complexity) = self.max_cognitive_complexity {
            options.max_cognitive_complexity = max_cognitive_complexity;
        }
        if let Some(max_cyclomatic_complexity) = self.max_cyclomatic_complexity {
            options.max_cyclomatic_complexity = max_cyclomatic_complexity;
        }
        if let Some(max_depth) = self.max_depth {
            options.max_depth = max_depth;
        }
        if let Some(max_lines) = self.max_lines {
            options.max_lines = max_lines;
        }
        if let Some(max_lines_per_function) = self.max_lines_per_function {
            options.max_lines_per_function = max_lines_per_function;
        }
        if let Some(max_nested_callbacks) = self.max_nested_callbacks {
            options.max_nested_callbacks = max_nested_callbacks;
        }
        if let Some(max_params) = self.max_params {
            options.max_params = max_params;
        }
        if let Some(max_statements) = self.max_statements {
            options.max_statements = max_statements;
        }
        if let Some(max_return_statements) = self.max_return_statements {
            options.max_return_statements = max_return_statements;
        }
        if let Some(max_switch_cases) = self.max_switch_cases {
            options.max_switch_cases = max_switch_cases;
        }
        if let Some(max_type_variants) = self.max_type_variants {
            options.max_type_variants = max_type_variants;
        }
        if let Some(max_type_fields) = self.max_type_fields {
            options.max_type_fields = max_type_fields;
        }
        if let Some(max_type_complexity) = self.max_type_complexity {
            options.max_type_complexity = max_type_complexity;
        }
        if let Some(max_duplicate_string_occurrences) = self.max_duplicate_string_occurrences {
            options.max_duplicate_string_occurrences = max_duplicate_string_occurrences;
        }
        if let Some(max_try_block_statements) = self.max_try_block_statements {
            options.max_try_block_statements = max_try_block_statements;
        }

        // style options
        if let Some(array_type) = self.array_type {
            options.array_type = array_type.into();
        }
        if let Some(type_definition_style) = self.type_definition_style {
            options.type_definition_style = type_definition_style.into();
        }
        if let Some(ref catch_error_name) = self.catch_error_name {
            options.catch_error_name = catch_error_name.clone();
        }
        if let Some(filename_case) = self.filename_case {
            options.filename_case = filename_case.into();
        }

        // restriction options
        if let Some(ref allowed_magic_numbers) = self.allowed_magic_numbers {
            options.allowed_magic_numbers = allowed_magic_numbers.clone();
        }
        if let Some(ref restricted_globals) = self.restricted_globals {
            options.restricted_globals = restricted_globals.clone();
        }
        if let Some(ref restricted_imports) = self.restricted_imports {
            options.restricted_imports = restricted_imports.clone();
        }
        if let Some(ref warning_comment_terms) = self.warning_comment_terms {
            options.warning_comment_terms = warning_comment_terms.clone();
        }
    }
}

/// Linter rules configuration.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigLinterRulesJson {
    /// Preset: "none", "recommended", or "all".
    pub preset: Option<String>,
    /// Enable the recommended rule set (shorthand for preset: "recommended").
    pub recommended: Option<bool>,
    /// Enable all rules (shorthand for preset: "all").
    pub all: Option<bool>,
    /// Category-level severity overrides.
    pub categories: Option<IndexMap<LintCategoryJson, RuleSeverityJson>>,
    /// Individual rule overrides (rule name -> severity).
    #[serde(flatten)]
    pub overrides: IndexMap<String, RuleSeverityJson>,
}

impl DsConfigLinterRulesJson {
    /// Apply rules configuration to LinterOptions.
    pub fn apply(&self, options: &mut LinterOptions) {
        // preset field takes precedence
        if let Some(preset_str) = &self.preset {
            if let Some(preset) = LintPreset::parse(preset_str) {
                options.preset = preset;
            }
        } else if let Some(true) = self.all {
            options.preset = LintPreset::All;
        } else if let Some(recommended) = self.recommended {
            options.preset = if recommended {
                LintPreset::Recommended
            } else {
                LintPreset::None
            };
        }

        // category overrides
        if let Some(categories) = &self.categories {
            for (category, severity) in categories {
                options
                    .categories
                    .insert((*category).into(), (*severity).into());
            }
        }

        // rule overrides
        for (rule, severity) in &self.overrides {
            options.overrides.insert(rule.clone(), (*severity).into());
        }
    }
}

/// Rule severity for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverityJson {
    /// Rule is disabled.
    Off,
    /// Rule produces warnings.
    Warn,
    /// Rule produces errors.
    Error,
}

impl From<RuleSeverityJson> for LintSeverity {
    fn from(value: RuleSeverityJson) -> Self {
        match value {
            RuleSeverityJson::Off => LintSeverity::Off,
            RuleSeverityJson::Warn => LintSeverity::Warning,
            RuleSeverityJson::Error => LintSeverity::Error,
        }
    }
}

/// Lint category for JSON deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LintCategoryJson {
    /// Correctness lints detect likely bugs and logic errors.
    Correctness,
    /// Suspicious lints detect code that is likely unintentional.
    Suspicious,
    /// Performance lints detect inefficient patterns.
    Performance,
    /// Style lints enforce consistent coding style.
    Style,
    /// Security lints detect potential vulnerabilities.
    Security,
    /// Complexity lints detect overly complex code.
    Complexity,
    /// Restriction lints enforce project-specific restrictions.
    Restriction,
}

impl From<LintCategoryJson> for LintCategory {
    fn from(value: LintCategoryJson) -> Self {
        match value {
            LintCategoryJson::Correctness => LintCategory::Correctness,
            LintCategoryJson::Suspicious => LintCategory::Suspicious,
            LintCategoryJson::Performance => LintCategory::Performance,
            LintCategoryJson::Style => LintCategory::Style,
            LintCategoryJson::Security => LintCategory::Security,
            LintCategoryJson::Complexity => LintCategory::Complexity,
            LintCategoryJson::Restriction => LintCategory::Restriction,
        }
    }
}

/// Preferred array type syntax for the `array-type` rule.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ArrayTypeStyleJson {
    /// Prefer `T[]` syntax.
    Array,
    /// Prefer `Array<T>` syntax.
    Generic,
}

impl From<ArrayTypeStyleJson> for ArrayTypeStyle {
    fn from(value: ArrayTypeStyleJson) -> Self {
        match value {
            ArrayTypeStyleJson::Array => ArrayTypeStyle::Array,
            ArrayTypeStyleJson::Generic => ArrayTypeStyle::Generic,
        }
    }
}

/// Preferred type definition syntax for the `consistent-type-definitions` rule.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TypeDefinitionStyleJson {
    /// Prefer `type` aliases.
    Type,
    /// Prefer `interface` declarations.
    Interface,
}

impl From<TypeDefinitionStyleJson> for TypeDefinitionStyle {
    fn from(value: TypeDefinitionStyleJson) -> Self {
        match value {
            TypeDefinitionStyleJson::Type => TypeDefinitionStyle::Type,
            TypeDefinitionStyleJson::Interface => TypeDefinitionStyle::Interface,
        }
    }
}

/// Filename case style for the `filename-case` rule.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum FilenameCaseJson {
    /// kebab-case (e.g., `my-component.ts`).
    Kebab,
    /// snake_case (e.g., `my_component.ts`).
    Snake,
    /// camelCase (e.g., `myComponent.ts`).
    Camel,
    /// PascalCase (e.g., `MyComponent.ts`).
    Pascal,
}

impl From<FilenameCaseJson> for FilenameCase {
    fn from(value: FilenameCaseJson) -> Self {
        match value {
            FilenameCaseJson::Kebab => FilenameCase::Kebab,
            FilenameCaseJson::Snake => FilenameCase::Snake,
            FilenameCaseJson::Camel => FilenameCase::Camel,
            FilenameCaseJson::Pascal => FilenameCase::Pascal,
        }
    }
}
