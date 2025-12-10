use std::path::PathBuf;
use std::sync::Arc;

use indexmap::IndexMap;
use serde::Deserialize;

use destack_source::{
    File, FileContent, FileId, FormatterOptions, IndentStyle, LanguageFeature, LanguageFeatureSet,
    LineEnding, LinterOptions, LinterRules, RuleSeverity,
};

use super::target::{
    OptimizeLevel, OutputFormat, OutputMode, Platform, Runtime, ShrinkLevel, Target,
    TargetDiscovery,
};
use super::tsconfig::{EsTarget, ModuleKind};

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

/// DsConfig JSON (usually from `dsconfig.json`)
#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigJson {
    /// Extends other dsconfigs or tsconfigs.
    pub extends: Option<DsConfigExtendsField>,
    /// Specific files to include in the project.
    pub files: Option<Vec<String>>,
    /// Glob patterns for files to include.
    pub include: Option<Vec<String>>,
    /// Glob patterns for files to exclude.
    pub exclude: Option<Vec<String>>,
    /// Compiler options.
    #[serde(default)]
    pub compiler_options: DsConfigCompilerOptionsJson,
    /// Formatter options.
    #[serde(default)]
    pub formatter: DsConfigFormatterJson,
    /// Linter options.
    #[serde(default)]
    pub linter: DsConfigLinterJson,
    /// Build targets.
    pub targets: Option<IndexMap<String, DsConfigTargetJson>>,
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

        // inherit features from parent
        for feature in LanguageFeature::ALL {
            if parent_compiler.features.is_enabled(*feature) {
                compiler.features.enable(*feature);
            }
        }

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
        compiler.no_unchecked_indexed_access =
            compiler.no_unchecked_indexed_access || parent_compiler.no_unchecked_indexed_access;

        // inherit Destack-specific checking - umbrella flags (stricter wins)
        compiler.strict_types = compiler.strict_types || parent_compiler.strict_types;
        compiler.strict_portable = compiler.strict_portable || parent_compiler.strict_portable;

        // inherit Destack-specific checking - type strictness (stricter wins)
        compiler.no_implicit_self = compiler.no_implicit_self || parent_compiler.no_implicit_self;
        compiler.no_imprecise_primitives =
            compiler.no_imprecise_primitives || parent_compiler.no_imprecise_primitives;
        compiler.no_implicit_conversions =
            compiler.no_implicit_conversions || parent_compiler.no_implicit_conversions;

        // inherit Destack-specific checking - ownership (stricter wins)
        compiler.no_implicit_managed_type =
            compiler.no_implicit_managed_type || parent_compiler.no_implicit_managed_type;
        compiler.no_implicit_managed_value =
            compiler.no_implicit_managed_value || parent_compiler.no_implicit_managed_value;
        compiler.no_managed = compiler.no_managed || parent_compiler.no_managed;

        // inherit Destack-specific checking - shapes & dispatch (stricter wins)
        compiler.no_dynamic_shapes =
            compiler.no_dynamic_shapes || parent_compiler.no_dynamic_shapes;
        compiler.no_implicit_dynamic_dispatch =
            compiler.no_implicit_dynamic_dispatch || parent_compiler.no_implicit_dynamic_dispatch;

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
        if child_json.line_ending.is_none() {
            self.options.formatter.line_ending = parent.formatter.line_ending;
        }
        if child_json.indent_style.is_none() {
            self.options.formatter.indent_style = parent.formatter.indent_style;
        }
        if child_json.indent_width.is_none() {
            self.options.formatter.indent_width = parent.formatter.indent_width;
        }
        if child_json.line_width.is_none() {
            self.options.formatter.line_width = parent.formatter.line_width;
        }

        // inherit linter options (child overrides if explicitly set in JSON)
        let child_linter_json = &self.content.linter;
        if child_linter_json.enabled.is_none() {
            self.options.linter.enabled = parent.linter.enabled;
        }
        if child_linter_json.rules.recommended.is_none() {
            self.options.linter.rules.recommended = parent.linter.rules.recommended;
        }
        // merge rule overrides (child takes precedence)
        for (rule, severity) in &parent.linter.rules.overrides {
            if !self.options.linter.rules.overrides.contains_key(rule) {
                self.options
                    .linter
                    .rules
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

/// Value for the "extends" field of a dsconfig.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum DsConfigExtendsField {
    /// Extend a single dsconfig.
    Single(String),
    /// Extend multiple dsconfigs.
    Multiple(Vec<String>),
}

/// Normalized Destack configuration options (from `dsconfig.json`).
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

/// Path alias mapping (resolved from dsconfig paths).
pub type DsPathAliases = IndexMap<String, Vec<String>>;

/// Normalized Destack compiler options.
///
/// **By default, all language features are enabled and strict mode is ON.**
/// Set `allow*: false` in `dsconfig.json` to disable specific features.
/// Destack defaults to stricter type checking than TypeScript.
#[derive(Debug, Clone)]
pub struct DsConfigCompilerOptions {
    // language features
    /// Enabled language features. All features are enabled by default.
    pub features: LanguageFeatureSet,

    // module resolution
    /// Base URL for resolving non-relative module names.
    pub base_url: Option<PathBuf>,
    /// Path alias mappings (resolved relative to baseUrl).
    pub paths: Option<DsPathAliases>,

    // module & target
    /// Module format for output.
    pub module: ModuleKind,
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
    pub no_unchecked_indexed_access: bool,

    // Destack-specific checking
    /// Enable all Destack type strictness checks (no_implicit_self, no_imprecise_primitives, etc.).
    pub strict_types: bool,
    /// Require portable constructs that work on all targets (js, wasm, native).
    pub strict_portable: bool,
    /// Require explicit `self.` for member access in methods.
    pub no_implicit_self: bool,
    /// Require precise primitive types (int32 vs number, etc.).
    pub no_imprecise_primitives: bool,
    /// Require explicit widening/narrowing conversions.
    pub no_implicit_conversions: bool,
    /// Require `^T` or `&T` in type positions (no implicit managed types).
    pub no_implicit_managed_type: bool,
    /// Require explicit copy/borrow at call sites (no implicit managed values).
    pub no_implicit_managed_value: bool,
    /// Forbid managed runtime features entirely (no &T at all, pure value types only).
    pub no_managed: bool,
    /// Forbid defineProperty, prototype mutation, etc. (require static object shapes).
    pub no_dynamic_shapes: bool,
    /// Require overloads to be statically resolvable (no runtime dispatch).
    pub no_implicit_dynamic_dispatch: bool,

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
        let strict_types = false; // off by default, opt-in for Destack type strictness
        Self {
            features: LanguageFeatureSet::all(),
            base_url: None,
            paths: None,
            module: ModuleKind::default(),
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
            no_unchecked_indexed_access: false,

            // Destack-specific checking - umbrella flags
            strict_types,
            strict_portable: false,

            // Destack-specific checking - type strictness (under strict_types)
            no_implicit_self: strict_types,
            no_imprecise_primitives: strict_types,
            no_implicit_conversions: strict_types,

            // Destack-specific checking - ownership (standalone)
            no_implicit_managed_type: false,
            no_implicit_managed_value: false,
            no_managed: false,

            // Destack-specific checking - shapes & dispatch (standalone)
            no_dynamic_shapes: false,
            no_implicit_dynamic_dispatch: false,

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

impl From<&DsConfigCompilerOptionsJson> for DsConfigCompilerOptions {
    fn from(json: &DsConfigCompilerOptionsJson) -> Self {
        // start with all features enabled
        let mut features = LanguageFeatureSet::all();

        // apply feature flags from JSON (only explicit false disables)
        json.apply_features(&mut features);

        let strict = json.strict.unwrap_or(true);
        let strict_types = json.strict_types.unwrap_or(false);
        Self {
            features,
            base_url: json.base_url.as_ref().map(PathBuf::from),
            paths: json.paths.clone(),
            module: json
                .module
                .as_deref()
                .and_then(ModuleKind::parse)
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
            no_unchecked_indexed_access: json.no_unchecked_indexed_access.unwrap_or(false),

            // Destack-specific checking - umbrella flags
            strict_types,
            strict_portable: json.strict_portable.unwrap_or(false),

            // Destack-specific checking - type strictness (under strict_types)
            no_implicit_self: json.no_implicit_self.unwrap_or(strict_types),
            no_imprecise_primitives: json.no_imprecise_primitives.unwrap_or(strict_types),
            no_implicit_conversions: json.no_implicit_conversions.unwrap_or(strict_types),

            // Destack-specific checking - ownership (standalone)
            no_implicit_managed_type: json.no_implicit_managed_type.unwrap_or(false),
            no_implicit_managed_value: json.no_implicit_managed_value.unwrap_or(false),
            no_managed: json.no_managed.unwrap_or(false),

            // Destack-specific checking - shapes & dispatch (standalone)
            no_dynamic_shapes: json.no_dynamic_shapes.unwrap_or(false),
            no_implicit_dynamic_dispatch: json.no_implicit_dynamic_dispatch.unwrap_or(false),

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
    pub module: ModuleKind,
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
            declaration: false,
            source_map: false,
            out_dir: PathBuf::from(super::target::DEFAULT_OUT_DIR),
            out_file: None,
            declaration_dir: None,
            module: ModuleKind::default(),
            es_target: EsTarget::default(),
            lib: None,
            debug: true,
            optimize: false,
            optimize_level: OptimizeLevel::O0,
            shrink_level: ShrinkLevel::S0,
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
                .and_then(ModuleKind::parse)
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
pub struct DsConfigCompilerOptionsJson {
    // language features
    /// Allow expression-oriented features: implicit returns, `loop`, `defer`, ranges, tuples, patterns.
    pub allow_expressions: Option<bool>,
    /// Allow tree literals: TSX-like syntax generalized for any tree-shaped data.
    pub allow_trees: Option<bool>,
    /// Allow annotations: decorators (`@`) extended to any expression.
    pub allow_annotations: Option<bool>,
    /// Allow type system extensions: newtypes, primitives, structs, constraints.
    pub allow_types: Option<bool>,
    /// Allow reflection: types as values, runtime type descriptors, decorator metadata.
    pub allow_reflection: Option<bool>,
    /// Allow dispatch: extensions and overloading (type-based method/function dispatch).
    pub allow_dispatch: Option<bool>,
    /// Allow ownership: value ownership (`&T`, `^T`), mutability (`var`), and explicit dispatch.
    pub allow_ownership: Option<bool>,

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
    pub no_unchecked_indexed_access: Option<bool>,

    // Destack-specific checking
    /// Enable all Destack type strictness checks (noImplicitSelf, noImprecisePrimitives, etc.).
    pub strict_types: Option<bool>,
    /// Ensure code works on all targets (js, wasm, native).
    pub strict_portable: Option<bool>,
    /// Require explicit `self.` for member access in methods.
    pub no_implicit_self: Option<bool>,
    /// Require precise primitive types (int32 vs number, etc.).
    pub no_imprecise_primitives: Option<bool>,
    /// Require explicit widening/narrowing conversions.
    pub no_implicit_conversions: Option<bool>,
    /// Require `^T` or `&T` in type positions (no implicit managed types).
    pub no_implicit_managed_type: Option<bool>,
    /// Require explicit copy/borrow at call sites (no implicit managed values).
    pub no_implicit_managed_value: Option<bool>,
    /// Forbid managed runtime features entirely (no &T at all, pure value types only).
    pub no_managed: Option<bool>,
    /// Forbid defineProperty, prototype mutation, etc. (require static object shapes).
    pub no_dynamic_shapes: Option<bool>,
    /// Require overloads to be statically resolvable (no runtime dispatch).
    pub no_implicit_dynamic_dispatch: Option<bool>,

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

impl DsConfigCompilerOptionsJson {
    /// Apply feature flags from options to a feature set.
    pub fn apply_features(&self, features: &mut LanguageFeatureSet) {
        if let Some(enabled) = self.allow_expressions {
            features.set(LanguageFeature::Expressions, enabled);
        }
        if let Some(enabled) = self.allow_trees {
            features.set(LanguageFeature::Trees, enabled);
        }
        if let Some(enabled) = self.allow_annotations {
            features.set(LanguageFeature::Annotations, enabled);
        }
        if let Some(enabled) = self.allow_types {
            features.set(LanguageFeature::Types, enabled);
        }
        if let Some(enabled) = self.allow_reflection {
            features.set(LanguageFeature::Reflection, enabled);
        }
        if let Some(enabled) = self.allow_dispatch {
            features.set(LanguageFeature::Dispatch, enabled);
        }
        if let Some(enabled) = self.allow_ownership {
            features.set(LanguageFeature::Ownership, enabled);
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
}

/// Formatter options (top-level, like Biome/Deno).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigFormatterJson {
    /// Line ending style: "lf", "crlf", or "cr".
    pub line_ending: Option<LineEndingJson>,
    /// Indent style: "tab" or "space".
    pub indent_style: Option<IndentStyleJson>,
    /// Number of spaces per indent (when using spaces). Default: 4.
    pub indent_width: Option<u8>,
    /// Maximum line width (best effort). Default: 100.
    pub line_width: Option<u8>,
}

impl DsConfigFormatterJson {
    /// Apply formatter options to a FormatterOptions struct.
    pub fn apply(&self, options: &mut FormatterOptions) {
        if let Some(line_ending) = self.line_ending {
            options.line_ending = line_ending.into();
        }
        if let Some(indent_style) = self.indent_style {
            options.indent_style = indent_style.into();
        }
        if let Some(indent_width) = self.indent_width {
            options.indent_width = indent_width;
        }
        if let Some(line_width) = self.line_width {
            options.line_width = line_width;
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
}

impl DsConfigLinterJson {
    /// Apply linter options to a LinterOptions struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(enabled) = self.enabled {
            options.enabled = enabled;
        }
        self.rules.apply(&mut options.rules);
    }
}

/// Linter rules configuration.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigLinterRulesJson {
    /// Enable the recommended rule set. Default: true.
    pub recommended: Option<bool>,
    /// Enable all rules (stricter than recommended).
    pub all: Option<bool>,
    /// Individual rule overrides (rule name -> severity).
    /// Overrides take precedence over presets.
    #[serde(flatten)]
    pub overrides: IndexMap<String, RuleSeverityJson>,
}

impl DsConfigLinterRulesJson {
    /// Apply rules configuration to a LinterRules struct.
    pub fn apply(&self, rules: &mut LinterRules) {
        if let Some(recommended) = self.recommended {
            rules.recommended = recommended;
        }
        // "all" implies recommended + more
        if let Some(true) = self.all {
            rules.recommended = true;
        }
        for (rule, severity) in &self.overrides {
            rules.overrides.insert(rule.clone(), (*severity).into());
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

impl From<RuleSeverityJson> for RuleSeverity {
    fn from(value: RuleSeverityJson) -> Self {
        match value {
            RuleSeverityJson::Off => RuleSeverity::Off,
            RuleSeverityJson::Warn => RuleSeverity::Warn,
            RuleSeverityJson::Error => RuleSeverity::Error,
        }
    }
}
