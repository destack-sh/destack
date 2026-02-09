use std::path::PathBuf;
use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use destack_source::{File, FileContent, FileId, IndentStyle, LineEnding};

use crate::{
    ArrayTypeStyle, ArrowParentheses, BorrowMode, FilenameCase, FormatterOptions, ImportSortOrder,
    LintCategory, LintPreset, LintSeverity, LinterOptions, OrganizeImports, QuoteProperty,
    QuoteStyle, TrailingComma, TypeDefinitionStyle,
};

use super::target::{
    Allocator, BoundsCheckPolicy, CheckFailurePolicy, DebugInfoLevel, DebugMode, DeterminismPolicy,
    DivisionCheckPolicy, EmitArtifact, FloatMathPolicy, LinkMode, LtoMode, NullCheckPolicy,
    OptimizeLevel, OsrMode, OutputFormat, OutputMode, OverflowCheckPolicy, PanicPolicy, Platform,
    ProfilingMode, RelocationModel, ReplayMode, Runtime, SafepointMode, SafetyPreset,
    SandboxPolicy, ShiftCheckPolicy, ShrinkLevel, SpeculationMode, StripLevel, Target,
    TargetDiscovery, TrustPolicy, UnwindFormat,
};
use super::tsconfig::{EsTarget, ModuleTarget};
use super::{ProfileConfig, ProfileConfigJson};

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
        if compiler.types.is_empty() {
            compiler.types = parent_compiler.types.clone();
        }
        if compiler.profile.is_none() {
            compiler.profile = parent_compiler.profile.clone();
        }
        if compiler.comptime_env.is_none() {
            compiler.comptime_env = parent_compiler.comptime_env.clone();
        }

        // inherit TypeScript-compatible checking options (stricter wins)
        compiler.strict = compiler.strict || parent_compiler.strict;
        compiler.always_strict = compiler.always_strict || parent_compiler.always_strict;
        if parent_compiler
            .no_implicit_any
            .is_stricter_than(compiler.no_implicit_any)
        {
            compiler.no_implicit_any = parent_compiler.no_implicit_any;
        }
        compiler.strict_null_checks =
            compiler.strict_null_checks || parent_compiler.strict_null_checks;
        if parent_compiler
            .no_implicit_this
            .is_stricter_than(compiler.no_implicit_this)
        {
            compiler.no_implicit_this = parent_compiler.no_implicit_this;
        }
        compiler.strict_function_types =
            compiler.strict_function_types || parent_compiler.strict_function_types;
        compiler.strict_bind_call_apply =
            compiler.strict_bind_call_apply || parent_compiler.strict_bind_call_apply;
        compiler.strict_builtin_iterator_return = compiler.strict_builtin_iterator_return
            || parent_compiler.strict_builtin_iterator_return;
        compiler.strict_property_initialization = compiler.strict_property_initialization
            || parent_compiler.strict_property_initialization;
        compiler.use_unknown_in_catch_variables = compiler.use_unknown_in_catch_variables
            || parent_compiler.use_unknown_in_catch_variables;
        if parent_compiler
            .no_unused_locals
            .is_stricter_than(compiler.no_unused_locals)
        {
            compiler.no_unused_locals = parent_compiler.no_unused_locals;
        }
        if parent_compiler
            .no_unused_parameters
            .is_stricter_than(compiler.no_unused_parameters)
        {
            compiler.no_unused_parameters = parent_compiler.no_unused_parameters;
        }
        if parent_compiler
            .no_implicit_returns
            .is_stricter_than(compiler.no_implicit_returns)
        {
            compiler.no_implicit_returns = parent_compiler.no_implicit_returns;
        }
        if parent_compiler
            .allow_unreachable_code
            .is_stricter_than(compiler.allow_unreachable_code)
        {
            compiler.allow_unreachable_code = parent_compiler.allow_unreachable_code;
        }
        if parent_compiler
            .allow_unused_labels
            .is_stricter_than(compiler.allow_unused_labels)
        {
            compiler.allow_unused_labels = parent_compiler.allow_unused_labels;
        }
        if parent_compiler
            .no_implicit_override
            .is_stricter_than(compiler.no_implicit_override)
        {
            compiler.no_implicit_override = parent_compiler.no_implicit_override;
        }
        if parent_compiler
            .no_fallthrough_cases_in_switch
            .is_stricter_than(compiler.no_fallthrough_cases_in_switch)
        {
            compiler.no_fallthrough_cases_in_switch =
                parent_compiler.no_fallthrough_cases_in_switch;
        }
        compiler.exact_optional_property_types =
            compiler.exact_optional_property_types || parent_compiler.exact_optional_property_types;
        if parent_compiler
            .no_unchecked_indexed_access
            .is_stricter_than(compiler.no_unchecked_indexed_access)
        {
            compiler.no_unchecked_indexed_access = parent_compiler.no_unchecked_indexed_access;
        }
        if parent_compiler
            .no_property_access_from_index_signature
            .is_stricter_than(compiler.no_property_access_from_index_signature)
        {
            compiler.no_property_access_from_index_signature =
                parent_compiler.no_property_access_from_index_signature;
        }

        // inherit Destack-specific checking (stricter wins)
        if parent_compiler.no_any.is_stricter_than(compiler.no_any) {
            compiler.no_any = parent_compiler.no_any;
        }
        if parent_compiler
            .no_unknown
            .is_stricter_than(compiler.no_unknown)
        {
            compiler.no_unknown = parent_compiler.no_unknown;
        }
        if parent_compiler
            .no_imprecise_primitives
            .is_stricter_than(compiler.no_imprecise_primitives)
        {
            compiler.no_imprecise_primitives = parent_compiler.no_imprecise_primitives;
        }
        if parent_compiler
            .no_implicit_conversions
            .is_stricter_than(compiler.no_implicit_conversions)
        {
            compiler.no_implicit_conversions = parent_compiler.no_implicit_conversions;
        }
        if parent_compiler
            .implicit_collection_conversions
            .is_stricter_than(compiler.implicit_collection_conversions)
        {
            compiler.implicit_collection_conversions =
                parent_compiler.implicit_collection_conversions;
        }
        if parent_compiler
            .no_unsafe_type_assertions
            .is_stricter_than(compiler.no_unsafe_type_assertions)
        {
            compiler.no_unsafe_type_assertions = parent_compiler.no_unsafe_type_assertions;
        }
        if parent_compiler
            .no_must_assertions
            .is_stricter_than(compiler.no_must_assertions)
        {
            compiler.no_must_assertions = parent_compiler.no_must_assertions;
        }
        if parent_compiler
            .no_definite_assignment_assertions
            .is_stricter_than(compiler.no_definite_assignment_assertions)
        {
            compiler.no_definite_assignment_assertions =
                parent_compiler.no_definite_assignment_assertions;
        }
        if parent_compiler
            .no_custom_type_guards
            .is_stricter_than(compiler.no_custom_type_guards)
        {
            compiler.no_custom_type_guards = parent_compiler.no_custom_type_guards;
        }
        if parent_compiler
            .no_unsound_variance
            .is_stricter_than(compiler.no_unsound_variance)
        {
            compiler.no_unsound_variance = parent_compiler.no_unsound_variance;
        }
        if parent_compiler
            .no_unsound_narrowing
            .is_stricter_than(compiler.no_unsound_narrowing)
        {
            compiler.no_unsound_narrowing = parent_compiler.no_unsound_narrowing;
        }
        if parent_compiler
            .deep_readonly
            .is_stricter_than(compiler.deep_readonly)
        {
            compiler.deep_readonly = parent_compiler.deep_readonly;
            if !compiler.deep_readonly_explicit {
                compiler.deep_readonly_explicit = parent_compiler.deep_readonly_explicit;
            }
        }
        if parent_compiler
            .no_untrusted_declarations
            .is_stricter_than(compiler.no_untrusted_declarations)
        {
            compiler.no_untrusted_declarations = parent_compiler.no_untrusted_declarations;
        }
        if parent_compiler
            .no_redeclared_locals
            .is_stricter_than(compiler.no_redeclared_locals)
        {
            compiler.no_redeclared_locals = parent_compiler.no_redeclared_locals;
        }
        if parent_compiler
            .no_implicit_managed
            .is_stricter_than(compiler.no_implicit_managed)
        {
            compiler.no_implicit_managed = parent_compiler.no_implicit_managed;
        }
        if parent_compiler
            .no_managed
            .is_stricter_than(compiler.no_managed)
        {
            compiler.no_managed = parent_compiler.no_managed;
        }
        if parent_compiler
            .no_runtime
            .is_stricter_than(compiler.no_runtime)
        {
            compiler.no_runtime = parent_compiler.no_runtime;
        }
        if parent_compiler
            .no_referential_equality
            .is_stricter_than(compiler.no_referential_equality)
        {
            compiler.no_referential_equality = parent_compiler.no_referential_equality;
        }
        if parent_compiler
            .no_dynamic_evaluation
            .is_stricter_than(compiler.no_dynamic_evaluation)
        {
            compiler.no_dynamic_evaluation = parent_compiler.no_dynamic_evaluation;
        }
        if parent_compiler
            .no_global_this
            .is_stricter_than(compiler.no_global_this)
        {
            compiler.no_global_this = parent_compiler.no_global_this;
        }
        if parent_compiler
            .no_dynamic_import
            .is_stricter_than(compiler.no_dynamic_import)
        {
            compiler.no_dynamic_import = parent_compiler.no_dynamic_import;
        }
        if parent_compiler
            .no_dynamic_shapes
            .is_stricter_than(compiler.no_dynamic_shapes)
        {
            compiler.no_dynamic_shapes = parent_compiler.no_dynamic_shapes;
        }
        if parent_compiler
            .no_computed_property_access
            .is_stricter_than(compiler.no_computed_property_access)
        {
            compiler.no_computed_property_access = parent_compiler.no_computed_property_access;
        }
        if parent_compiler.no_proxy.is_stricter_than(compiler.no_proxy) {
            compiler.no_proxy = parent_compiler.no_proxy;
        }
        if parent_compiler
            .no_implicit_dynamic_dispatch
            .is_stricter_than(compiler.no_implicit_dynamic_dispatch)
        {
            compiler.no_implicit_dynamic_dispatch = parent_compiler.no_implicit_dynamic_dispatch;
        }
        if parent_compiler
            .no_exceptions
            .is_stricter_than(compiler.no_exceptions)
        {
            compiler.no_exceptions = parent_compiler.no_exceptions;
        }
        if parent_compiler
            .borrow_mode
            .is_stricter_than(compiler.borrow_mode)
        {
            compiler.borrow_mode = parent_compiler.borrow_mode;
        }

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
        if self.content.compiler_options.declaration_map.is_none() {
            compiler.declaration_map = parent_compiler.declaration_map;
        }
        if self.content.compiler_options.no_emit.is_none() {
            compiler.no_emit = parent_compiler.no_emit;
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

        // inherit cache options (child overrides if set)
        let child_cache_json = &self.content.cache;
        if child_cache_json.mode.is_none() {
            self.options.cache.mode = parent.cache.mode;
        }
        if child_cache_json.dir.is_none() {
            self.options.cache.dir = parent.cache.dir.clone();
        }
        if child_cache_json.max_size_mb.is_none() {
            self.options.cache.max_size_mb = parent.cache.max_size_mb;
        }
        if child_cache_json.policy.is_none() {
            self.options.cache.policy = parent.cache.policy;
        }
        if child_cache_json.validate.is_none() {
            self.options.cache.validate = parent.cache.validate;
        }
        if child_cache_json.scope.is_none() {
            self.options.cache.scope = parent.cache.scope;
        }

        // inherit compiler incremental settings (child overrides if explicitly set in JSON)
        if self.content.compiler_options.incremental.is_none() {
            self.options.compiler.incremental = parent.compiler.incremental;
        }

        // inherit watch options (child overrides if set)
        let child_watch_json = &self.content.watch;
        if child_watch_json.debounce_ms.is_none() {
            self.options.watch.debounce_ms = parent.watch.debounce_ms;
        }
        if child_watch_json.poll_interval_ms.is_none() {
            self.options.watch.poll_interval_ms = parent.watch.poll_interval_ms;
        }

        // extend targets (add missing targets from parent)
        for (name, target) in &parent.targets {
            if !self.options.targets.contains_key(name) {
                self.options.targets.insert(name.clone(), target.clone());
            }
        }

        // extend profiles (add missing profiles from parent)
        for (name, profile) in &parent.profiles {
            if !self.options.profiles.contains_key(name) {
                self.options.profiles.insert(name.clone(), profile.clone());
            }
        }
        if self.options.default_target.is_none() {
            self.options.default_target = parent.default_target.clone();
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
    /// Cache options.
    pub cache: DsConfigCacheOptions,
    /// Watch options.
    pub watch: DsConfigWatchOptions,
    /// Daemon options.
    pub daemon: DsConfigDaemonOptions,
    /// Build targets.
    pub targets: IndexMap<String, DsConfigTargetOptions>,
    /// Named profiles for semantic configuration.
    pub profiles: IndexMap<String, ProfileConfig>,
    /// Default target for workspace.
    pub default_target: Option<String>,
}

/// Cache configuration options.
#[derive(Debug, Clone, Default)]
pub struct DsConfigCacheOptions {
    /// Cache mode.
    pub mode: CacheMode,
    /// Cache directory path.
    pub dir: Option<PathBuf>,
    /// Maximum cache size in megabytes.
    pub max_size_mb: Option<u64>,
    /// Cache eviction policy.
    pub policy: CachePolicy,
    /// Cache validation strategy.
    pub validate: CacheValidate,
    /// Cache scope selection.
    pub scope: CacheScope,
}

/// Cache mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheMode {
    /// Disable caching.
    #[default]
    Off,
    /// Use in-memory caching only.
    Memory,
    /// Use on-disk caching.
    Disk,
}

/// Cache eviction policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CachePolicy {
    /// Least recently used eviction.
    #[default]
    Lru,
    /// Time to live eviction.
    Ttl,
}

/// Cache validation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CacheValidate {
    /// Always validate cache entries strictly.
    #[default]
    Strict,
    /// Validate only on mismatched metadata or changes.
    Fast,
}

/// Cache scope selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheScope {
    /// Cache entries are workspace-local.
    #[default]
    Workspace,
    /// Cache entries are stored in a global shared cache.
    Global,
}

/// Daemon configuration options.
#[derive(Debug, Clone)]
pub struct DsConfigDaemonOptions {
    /// Idle shutdown timeout in milliseconds, or None to disable.
    pub idle_shutdown_ms: Option<u64>,
}

/// Default idle shutdown timeout for the daemon.
pub const DEFAULT_DAEMON_IDLE_SHUTDOWN_MS: u64 = 600_000;

impl Default for DsConfigDaemonOptions {
    /// Return default daemon options.
    fn default() -> Self {
        Self {
            idle_shutdown_ms: Some(DEFAULT_DAEMON_IDLE_SHUTDOWN_MS),
        }
    }
}

/// Watch configuration options.
#[derive(Debug, Clone)]
pub struct DsConfigWatchOptions {
    /// Debounce interval in milliseconds.
    pub debounce_ms: u64,
    /// Poll interval in milliseconds for polling watchers.
    pub poll_interval_ms: Option<u64>,
}

impl Default for DsConfigWatchOptions {
    fn default() -> Self {
        Self {
            debounce_ms: 30,
            poll_interval_ms: None,
        }
    }
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
    /// Additional library types to include (e.g., "node", "dom.iterable").
    pub types: Vec<String>,
    /// Default profile for IDEs and CLI usage.
    pub profile: Option<String>,
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,
    /// Enable incremental compilation for this project.
    pub incremental: bool,

    // TypeScript-compatible checking
    /// Enable all strict type checking options.
    pub strict: bool,
    /// Parse in strict mode.
    pub always_strict: bool,
    /// Policy for expressions and declarations with implied `any` type.
    pub no_implicit_any: DiagnosticPolicy,
    /// Enable strict null checks (`null` and `undefined` are distinct types).
    pub strict_null_checks: bool,
    /// Policy for `this` expressions with implied `any` type.
    pub no_implicit_this: DiagnosticPolicy,
    /// Enable strict checking of function types.
    pub strict_function_types: bool,
    /// Enable strict checking of `bind`, `call`, and `apply`.
    pub strict_bind_call_apply: bool,
    /// Enable strict checking of built in iterator return types.
    pub strict_builtin_iterator_return: bool,
    /// Enable strict checking of property initialization in classes.
    pub strict_property_initialization: bool,
    /// Use `unknown` instead of `any` for catch clause variables.
    pub use_unknown_in_catch_variables: bool,
    /// Policy for unused local variables.
    pub no_unused_locals: DiagnosticPolicy,
    /// Policy for unused parameters.
    pub no_unused_parameters: DiagnosticPolicy,
    /// Policy for missing return paths.
    pub no_implicit_returns: DiagnosticPolicy,
    /// Policy for unreachable code.
    pub allow_unreachable_code: DiagnosticPolicy,
    /// Policy for unused labels.
    pub allow_unused_labels: DiagnosticPolicy,
    /// Policy for missing `override` on class members.
    pub no_implicit_override: DiagnosticPolicy,
    /// Policy for fallthrough cases in switch statements.
    pub no_fallthrough_cases_in_switch: DiagnosticPolicy,
    /// Interpret optional property types as written without implicit `undefined`.
    pub exact_optional_property_types: bool,
    /// Policy for unchecked index signature access results.
    pub no_unchecked_indexed_access: DiagnosticPolicy,
    /// Policy for property access from index signatures without explicit index access.
    pub no_property_access_from_index_signature: DiagnosticPolicy,

    // Destack-specific checking
    /// Policy for forbidding `any` types.
    pub no_any: DiagnosticPolicy,
    /// Policy for forbidding `unknown` types.
    pub no_unknown: DiagnosticPolicy,
    /// Policy for imprecise primitive types (int32 vs number, etc.).
    pub no_imprecise_primitives: DiagnosticPolicy,
    /// Policy for implicit widening/narrowing conversions.
    pub no_implicit_conversions: DiagnosticPolicy,
    /// Policy for implicit collection conversions (record-like and sized arrays).
    pub implicit_collection_conversions: ImplicitCollectionConversionPolicy,
    /// Policy for unsafe type assertions (`as T`).
    pub no_unsafe_type_assertions: DiagnosticPolicy,
    /// Policy for must assertions (`x!`).
    pub no_must_assertions: DiagnosticPolicy,
    /// Policy for definite assignment assertions (`field!: T`).
    pub no_definite_assignment_assertions: DiagnosticPolicy,
    /// Policy for custom type guards (`x is T`).
    pub no_custom_type_guards: DiagnosticPolicy,
    /// Policy for unsound variance (invariant mutable positions).
    pub no_unsound_variance: DiagnosticPolicy,
    /// Policy for unsound narrowing (`instanceof`, `in`, and predicates).
    pub no_unsound_narrowing: DiagnosticPolicy,
    /// Policy for shallow readonly behavior.
    pub deep_readonly: DiagnosticPolicy,
    /// Whether deepReadonly was explicitly configured in JSON.
    pub deep_readonly_explicit: bool,
    /// Policy for untrusted declaration files.
    pub no_untrusted_declarations: DiagnosticPolicy,
    /// Policy for re-declaration of local variables.
    pub no_redeclared_locals: DiagnosticPolicy,
    /// Policy for implicit ownership of managed types and values.
    pub no_implicit_managed: DiagnosticPolicy,
    /// Policy for GC-managed defaults and allocations.
    pub no_managed: DiagnosticPolicy,
    /// Policy for runtime usage (no managed memory, no Promise, no exceptions, ...).
    pub no_runtime: DiagnosticPolicy,
    /// Policy for referential equality.
    pub no_referential_equality: DiagnosticPolicy,
    /// Policy for `eval()` and `Function` constructor.
    pub no_dynamic_evaluation: DiagnosticPolicy,
    /// Policy for `globalThis` access.
    pub no_global_this: DiagnosticPolicy,
    /// Policy for dynamic `import()` and `require()` expressions.
    pub no_dynamic_import: DiagnosticPolicy,
    /// Policy for defineProperty, prototype mutation, delete, and declaration expressions.
    pub no_dynamic_shapes: DiagnosticPolicy,
    /// Policy for computed property access `obj[expr]` where expr isn't constant.
    pub no_computed_property_access: DiagnosticPolicy,
    /// Policy for `Proxy`.
    pub no_proxy: DiagnosticPolicy,
    /// Policy for overloads that are not statically resolvable.
    pub no_implicit_dynamic_dispatch: DiagnosticPolicy,
    /// Policy for `throw` and `try`/`catch` (use Result types instead).
    pub no_exceptions: DiagnosticPolicy,
    /// Borrow checking mode for `&T` and `&mut T`.
    pub borrow_mode: BorrowMode,

    // emit
    /// Root directory of source files (controls output directory structure, not module resolution).
    pub root_dir: Option<PathBuf>,
    /// Output directory for compiled files.
    pub out_dir: Option<PathBuf>,
    /// Output directory for declaration files. Defaults to out_dir.
    pub declaration_dir: Option<PathBuf>,
    /// Generate declaration maps for `.d.ts` output.
    pub declaration_map: bool,
    /// Do not emit output files.
    pub no_emit: bool,

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
            types: Vec::new(),
            profile: None,
            comptime_env: None,
            incremental: true,

            // TypeScript-compatible checking
            strict,
            always_strict: strict,
            no_implicit_any: if strict {
                DiagnosticPolicy::Deny
            } else {
                DiagnosticPolicy::Allow
            },
            strict_null_checks: strict,
            no_implicit_this: if strict {
                DiagnosticPolicy::Deny
            } else {
                DiagnosticPolicy::Allow
            },
            strict_function_types: strict,
            strict_bind_call_apply: strict,
            strict_builtin_iterator_return: strict,
            strict_property_initialization: strict,
            use_unknown_in_catch_variables: strict,
            no_unused_locals: DiagnosticPolicy::Allow,
            no_unused_parameters: DiagnosticPolicy::Allow,
            no_implicit_returns: DiagnosticPolicy::Deny,
            allow_unreachable_code: DiagnosticPolicy::Deny,
            allow_unused_labels: DiagnosticPolicy::Deny,
            no_implicit_override: DiagnosticPolicy::Deny,
            no_fallthrough_cases_in_switch: DiagnosticPolicy::Allow,
            exact_optional_property_types: true,
            no_unchecked_indexed_access: DiagnosticPolicy::Deny,
            no_property_access_from_index_signature: DiagnosticPolicy::Allow,

            // Destack-specific checking (all off by default, opt-in)
            no_any: if strict {
                DiagnosticPolicy::Deny
            } else {
                DiagnosticPolicy::Allow
            },
            no_unknown: DiagnosticPolicy::Allow,
            no_imprecise_primitives: DiagnosticPolicy::Allow,
            no_implicit_conversions: DiagnosticPolicy::Allow,
            implicit_collection_conversions: ImplicitCollectionConversionPolicy::Allow,
            no_unsafe_type_assertions: DiagnosticPolicy::Allow,
            no_must_assertions: DiagnosticPolicy::Allow,
            no_definite_assignment_assertions: DiagnosticPolicy::Allow,
            no_custom_type_guards: DiagnosticPolicy::Allow,
            no_unsound_variance: DiagnosticPolicy::Allow,
            no_unsound_narrowing: DiagnosticPolicy::Allow,
            deep_readonly: DiagnosticPolicy::Allow,
            deep_readonly_explicit: false,
            no_untrusted_declarations: DiagnosticPolicy::Allow,
            no_redeclared_locals: DiagnosticPolicy::Allow,
            no_implicit_managed: DiagnosticPolicy::Allow,
            no_managed: DiagnosticPolicy::Allow,
            no_runtime: DiagnosticPolicy::Allow,
            no_referential_equality: DiagnosticPolicy::Allow,
            no_dynamic_evaluation: DiagnosticPolicy::Allow,
            no_global_this: DiagnosticPolicy::Allow,
            no_dynamic_import: DiagnosticPolicy::Allow,
            no_dynamic_shapes: DiagnosticPolicy::Allow,
            no_computed_property_access: DiagnosticPolicy::Allow,
            no_proxy: DiagnosticPolicy::Allow,
            no_implicit_dynamic_dispatch: DiagnosticPolicy::Allow,
            no_exceptions: DiagnosticPolicy::Allow,
            borrow_mode: BorrowMode::Hint,

            // emit
            root_dir: None,
            out_dir: None,
            declaration_dir: None,
            declaration_map: false,
            no_emit: false,

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

impl DsConfigCompilerOptions {
    /// Enable strict-mode defaults.
    pub fn apply_strict_defaults(&mut self) {
        // lock the strict umbrella flag and parser strictness
        self.strict = true;
        self.always_strict = true;

        // enable strict checks under the TypeScript umbrella
        self.no_implicit_any = DiagnosticPolicy::Deny;
        self.no_implicit_this = DiagnosticPolicy::Deny;
        self.strict_null_checks = true;
        self.strict_function_types = true;
        self.strict_bind_call_apply = true;
        self.strict_builtin_iterator_return = true;
        self.strict_property_initialization = true;
        self.use_unknown_in_catch_variables = true;

        // enable stricter checking defaults beyond the TS strict umbrella
        self.no_any = DiagnosticPolicy::Deny;
        self.no_implicit_returns = DiagnosticPolicy::Deny;
        self.no_implicit_override = DiagnosticPolicy::Deny;
        self.exact_optional_property_types = true;
        self.no_unchecked_indexed_access = DiagnosticPolicy::Deny;

        // enable strict diagnostics by default
        self.no_unused_locals = DiagnosticPolicy::Deny;
        self.no_unused_parameters = DiagnosticPolicy::Deny;
        self.no_fallthrough_cases_in_switch = DiagnosticPolicy::Deny;
        self.allow_unreachable_code = DiagnosticPolicy::Deny;
        self.allow_unused_labels = DiagnosticPolicy::Deny;
    }

    /// Enable native-only restrictions for native and wasm targets.
    pub fn apply_native_restrictions(&mut self) {
        // require strict checks for native targets
        self.apply_strict_defaults();

        // enforce soundness defaults for native targets
        self.no_any = DiagnosticPolicy::Deny;
        self.no_imprecise_primitives = DiagnosticPolicy::Deny;
        self.no_implicit_conversions = DiagnosticPolicy::Deny;
        self.no_unsafe_type_assertions = DiagnosticPolicy::Deny;
        self.no_must_assertions = DiagnosticPolicy::Deny;
        self.no_definite_assignment_assertions = DiagnosticPolicy::Deny;
        self.no_custom_type_guards = DiagnosticPolicy::Deny;
        self.no_unsound_variance = DiagnosticPolicy::Deny;
        self.no_unsound_narrowing = DiagnosticPolicy::Deny;
        self.deep_readonly = DiagnosticPolicy::Deny;
        self.no_untrusted_declarations = DiagnosticPolicy::Deny;
        self.no_implicit_managed = DiagnosticPolicy::Deny;
        self.no_managed = DiagnosticPolicy::Deny;
        self.no_property_access_from_index_signature = DiagnosticPolicy::Deny;
        self.borrow_mode = BorrowMode::Strict;
        self.implicit_collection_conversions = ImplicitCollectionConversionPolicy::Warn;

        // disable runtime features that native backends cannot support
        self.no_dynamic_evaluation = DiagnosticPolicy::Deny;
        self.no_dynamic_import = DiagnosticPolicy::Deny;
        self.no_proxy = DiagnosticPolicy::Deny;
        self.no_dynamic_shapes = DiagnosticPolicy::Deny;
        self.no_exceptions = DiagnosticPolicy::Deny;
        self.no_global_this = DiagnosticPolicy::Deny;
    }

    /// Enable explicit ownership defaults for managed memory control.
    pub fn apply_no_managed_defaults(&mut self) {
        // require explicit ownership markers for managed types and values
        self.no_implicit_managed = DiagnosticPolicy::Deny;
    }

    /// Enable runtime-free restrictions for compile-time only targets.
    pub fn apply_no_runtime_restrictions(&mut self) {
        // force runtime control flags on when runtime is disabled
        self.no_runtime = DiagnosticPolicy::Deny;
        self.no_managed = DiagnosticPolicy::Deny;
        self.no_exceptions = DiagnosticPolicy::Deny;
        self.no_dynamic_evaluation = DiagnosticPolicy::Deny;
        self.no_dynamic_import = DiagnosticPolicy::Deny;
        self.no_dynamic_shapes = DiagnosticPolicy::Deny;
        self.no_computed_property_access = DiagnosticPolicy::Deny;
        self.no_proxy = DiagnosticPolicy::Deny;
        self.no_global_this = DiagnosticPolicy::Deny;
        self.no_implicit_dynamic_dispatch = DiagnosticPolicy::Deny;

        // require explicit ownership markers for managed types and values
        self.apply_no_managed_defaults();
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
    /// Cache options.
    #[serde(default)]
    pub cache: DsConfigCacheJson,
    /// Watch options.
    #[serde(default)]
    #[serde(alias = "watchOptions")]
    pub watch: DsConfigWatchJson,
    /// Daemon options.
    #[serde(default)]
    pub daemon: DsConfigDaemonJson,
    /// Build targets.
    pub targets: Option<IndexMap<String, DsConfigTargetJson>>,
    /// Named profiles for semantic configuration.
    pub profiles: Option<IndexMap<String, ProfileConfigJson>>,
    /// Default target for workspace.
    pub default_target: Option<String>,
}

impl From<&DsConfigJson> for DsConfigOptions {
    fn from(json: &DsConfigJson) -> Self {
        let compiler = DsConfigCompilerOptions::from(&json.compiler_options);

        let mut formatter = FormatterOptions::default();
        json.formatter.apply(&mut formatter);

        let mut linter = LinterOptions::default();
        json.linter.apply(&mut linter);

        let cache = DsConfigCacheOptions::from(&json.cache);
        let watch = DsConfigWatchOptions::from(&json.watch);
        let daemon = DsConfigDaemonOptions::from(&json.daemon);

        Self {
            files: json.files.clone().unwrap_or_default(),
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            compiler,
            formatter,
            linter,
            cache,
            watch,
            daemon,
            targets: json
                .targets
                .as_ref()
                .map(|t| {
                    t.iter()
                        .map(|(k, v)| (k.clone(), DsConfigTargetOptions::from(v)))
                        .collect()
                })
                .unwrap_or_default(),
            profiles: json
                .profiles
                .as_ref()
                .map(|profiles| {
                    profiles
                        .iter()
                        .map(|(name, profile)| (name.clone(), ProfileConfig::from_json(profile)))
                        .collect()
                })
                .unwrap_or_default(),
            default_target: json.default_target.clone(),
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

/// Cache options (top-level).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigCacheJson {
    /// Cache mode.
    pub mode: Option<CacheModeJson>,
    /// Cache directory path.
    pub dir: Option<String>,
    /// Maximum cache size in megabytes.
    pub max_size_mb: Option<u64>,
    /// Cache eviction policy.
    pub policy: Option<CachePolicyJson>,
    /// Cache validation policy.
    pub validate: Option<CacheValidateJson>,
    /// Cache scope selection.
    pub scope: Option<CacheScopeJson>,
}

impl From<&DsConfigCacheJson> for DsConfigCacheOptions {
    fn from(json: &DsConfigCacheJson) -> Self {
        Self {
            mode: json.mode.map(CacheMode::from).unwrap_or_default(),
            dir: json.dir.as_ref().map(PathBuf::from),
            max_size_mb: json.max_size_mb,
            policy: json.policy.map(CachePolicy::from).unwrap_or_default(),
            validate: json.validate.map(CacheValidate::from).unwrap_or_default(),
            scope: json.scope.map(CacheScope::from).unwrap_or_default(),
        }
    }
}

/// Watch options (top-level).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigWatchJson {
    /// Debounce interval in milliseconds.
    pub debounce_ms: Option<u64>,
    /// Poll interval in milliseconds for polling watchers.
    pub poll_interval_ms: Option<u64>,
}

/// Daemon options (top-level).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigDaemonJson {
    /// Idle shutdown timeout in milliseconds.
    pub idle_shutdown_ms: Option<u64>,
}

impl From<&DsConfigWatchJson> for DsConfigWatchOptions {
    fn from(json: &DsConfigWatchJson) -> Self {
        Self {
            debounce_ms: json.debounce_ms.unwrap_or(30),
            poll_interval_ms: json.poll_interval_ms,
        }
    }
}

impl From<&DsConfigDaemonJson> for DsConfigDaemonOptions {
    /// Convert daemon JSON options into normalized options.
    fn from(json: &DsConfigDaemonJson) -> Self {
        // normalize the idle shutdown setting
        let idle_shutdown_ms = match json.idle_shutdown_ms {
            Some(0) => None,
            Some(value) => Some(value),
            None => Some(DEFAULT_DAEMON_IDLE_SHUTDOWN_MS),
        };

        Self { idle_shutdown_ms }
    }
}

/// Cache mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CacheModeJson {
    /// Disable caching.
    Off,
    /// Use in-memory caching only.
    Memory,
    /// Use on-disk caching.
    Disk,
}

impl From<CacheModeJson> for CacheMode {
    fn from(value: CacheModeJson) -> Self {
        match value {
            CacheModeJson::Off => CacheMode::Off,
            CacheModeJson::Memory => CacheMode::Memory,
            CacheModeJson::Disk => CacheMode::Disk,
        }
    }
}

/// Cache eviction policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CachePolicyJson {
    /// Least recently used eviction.
    Lru,
    /// Time to live eviction.
    Ttl,
}

impl From<CachePolicyJson> for CachePolicy {
    fn from(value: CachePolicyJson) -> Self {
        match value {
            CachePolicyJson::Lru => CachePolicy::Lru,
            CachePolicyJson::Ttl => CachePolicy::Ttl,
        }
    }
}

/// Cache validation policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CacheValidateJson {
    /// Always validate cache entries strictly.
    Strict,
    /// Validate only on mismatched metadata or changes.
    Fast,
}

impl From<CacheValidateJson> for CacheValidate {
    fn from(value: CacheValidateJson) -> Self {
        match value {
            CacheValidateJson::Strict => CacheValidate::Strict,
            CacheValidateJson::Fast => CacheValidate::Fast,
        }
    }
}

/// Cache scope selection for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CacheScopeJson {
    /// Cache entries are workspace-local.
    Workspace,
    /// Cache entries are stored in a global shared cache.
    Global,
}

impl From<CacheScopeJson> for CacheScope {
    fn from(value: CacheScopeJson) -> Self {
        match value {
            CacheScopeJson::Workspace => CacheScope::Workspace,
            CacheScopeJson::Global => CacheScope::Global,
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
    /// Runtime environment (browser, node, wasm-wasi, native-hosted, etc.).
    pub runtime: Runtime,
    /// Runtime version for selecting versioned libs.
    pub runtime_version: Option<String>,
    /// Target platform (web, windows, macos, linux, ios, android, bare-metal, etc.).
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
    /// Determinism policy for runtime scheduling and I/O.
    pub determinism: DeterminismPolicy,
    /// Replay policy for external effects.
    pub replay: ReplayMode,
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
            out_dir: PathBuf::from(super::target::DEFAULT_OUT_DIR),
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
            determinism: DeterminismPolicy::default(),
            replay: ReplayMode::default(),
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
            determinism: self.determinism,
            replay: self.replay,
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
                .unwrap_or_else(|| PathBuf::from(super::target::DEFAULT_OUT_DIR)),
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
            determinism: json
                .determinism
                .map(DeterminismPolicy::from)
                .unwrap_or_default(),
            replay: json.replay.map(ReplayMode::from).unwrap_or_default(),
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

/// Determinism policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DeterminismPolicyJson {
    /// Best-effort execution without determinism guarantees.
    #[serde(alias = "off")]
    BestEffort,
    /// Deterministic scheduling with controlled randomness.
    #[serde(alias = "determinism")]
    Deterministic,
}

impl From<DeterminismPolicyJson> for DeterminismPolicy {
    fn from(value: DeterminismPolicyJson) -> Self {
        match value {
            DeterminismPolicyJson::BestEffort => DeterminismPolicy::BestEffort,
            DeterminismPolicyJson::Deterministic => DeterminismPolicy::Deterministic,
        }
    }
}

/// Replay policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ReplayModeJson {
    /// Disable record/replay.
    #[serde(alias = "none")]
    Off,
    /// Record external effects for replay.
    Record,
    /// Replay external effects from the log.
    Replay,
}

impl From<ReplayModeJson> for ReplayMode {
    fn from(value: ReplayModeJson) -> Self {
        match value {
            ReplayModeJson::Off => ReplayMode::Off,
            ReplayModeJson::Record => ReplayMode::Record,
            ReplayModeJson::Replay => ReplayMode::Replay,
        }
    }
}

/// Trust policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TrustPolicyJson {
    /// Untrusted code with strict limits and validation.
    #[serde(alias = "sandboxed")]
    Untrusted,
    /// Trusted code with relaxed limits.
    Trusted,
    /// Internal toolchain code with full privileges.
    Internal,
}

impl From<TrustPolicyJson> for TrustPolicy {
    fn from(value: TrustPolicyJson) -> Self {
        match value {
            TrustPolicyJson::Untrusted => TrustPolicy::Untrusted,
            TrustPolicyJson::Trusted => TrustPolicy::Trusted,
            TrustPolicyJson::Internal => TrustPolicy::Internal,
        }
    }
}

/// Sandbox policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SandboxPolicyJson {
    /// In-process isolation with VM guardrails.
    #[serde(alias = "in_process", alias = "inprocess")]
    InProcess,
    /// Process isolation with OS sandboxing.
    Process,
    /// Container or VM isolation.
    #[serde(alias = "vm")]
    Container,
    /// Forbid execution without an external sandbox.
    #[serde(alias = "forbid", alias = "deny")]
    Forbidden,
}

impl From<SandboxPolicyJson> for SandboxPolicy {
    fn from(value: SandboxPolicyJson) -> Self {
        match value {
            SandboxPolicyJson::InProcess => SandboxPolicy::InProcess,
            SandboxPolicyJson::Process => SandboxPolicy::Process,
            SandboxPolicyJson::Container => SandboxPolicy::Container,
            SandboxPolicyJson::Forbidden => SandboxPolicy::Forbidden,
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

/// Panic policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PanicPolicyJson {
    /// Abort immediately.
    #[serde(alias = "abort")]
    Abort,
    /// Unwind the stack.
    #[serde(alias = "unwind")]
    Unwind,
}

impl From<PanicPolicyJson> for PanicPolicy {
    fn from(value: PanicPolicyJson) -> Self {
        match value {
            PanicPolicyJson::Abort => PanicPolicy::Abort,
            PanicPolicyJson::Unwind => PanicPolicy::Unwind,
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

/// Safety preset for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SafetyPresetJson {
    /// Debug safety mode with checks enabled.
    #[serde(alias = "debug")]
    Debug,
    /// Release mode with checks enabled.
    #[serde(alias = "releaseSafe")]
    #[serde(alias = "safe")]
    ReleaseSafe,
    /// Release mode with checks disabled.
    #[serde(alias = "releaseFast")]
    #[serde(alias = "fast")]
    ReleaseFast,
    /// Release mode with checks disabled and size focused settings.
    #[serde(alias = "releaseSmall")]
    #[serde(alias = "small")]
    ReleaseSmall,
}

impl From<SafetyPresetJson> for SafetyPreset {
    fn from(value: SafetyPresetJson) -> Self {
        match value {
            SafetyPresetJson::Debug => SafetyPreset::Debug,
            SafetyPresetJson::ReleaseSafe => SafetyPreset::ReleaseSafe,
            SafetyPresetJson::ReleaseFast => SafetyPreset::ReleaseFast,
            SafetyPresetJson::ReleaseSmall => SafetyPreset::ReleaseSmall,
        }
    }
}

/// Floating point math policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum FloatMathPolicyJson {
    /// Strict IEEE semantics.
    #[serde(alias = "strict")]
    Strict,
    /// Allow reassociation and algebraic simplifications.
    #[serde(alias = "reassoc")]
    #[serde(alias = "reassociate")]
    #[serde(alias = "relaxed")]
    Reassociate,
    /// Enable fast math optimizations (assume no NaN, inf, or signed zero).
    #[serde(alias = "fast")]
    #[serde(alias = "fast-math")]
    #[serde(alias = "fast_math")]
    #[serde(alias = "fastmath")]
    Fast,
}

impl From<FloatMathPolicyJson> for FloatMathPolicy {
    fn from(value: FloatMathPolicyJson) -> Self {
        match value {
            FloatMathPolicyJson::Strict => FloatMathPolicy::Strict,
            FloatMathPolicyJson::Reassociate => FloatMathPolicy::Reassociate,
            FloatMathPolicyJson::Fast => FloatMathPolicy::Fast,
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

/// Null check policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum NullCheckPolicyJson {
    /// Always emit null checks.
    #[serde(alias = "always")]
    Always,
    /// Emit null checks only in debug builds.
    #[serde(alias = "debug")]
    Debug,
    /// Never emit null checks.
    #[serde(alias = "never")]
    #[serde(alias = "off")]
    Never,
}

impl From<NullCheckPolicyJson> for NullCheckPolicy {
    fn from(value: NullCheckPolicyJson) -> Self {
        match value {
            NullCheckPolicyJson::Always => NullCheckPolicy::Always,
            NullCheckPolicyJson::Debug => NullCheckPolicy::Debug,
            NullCheckPolicyJson::Never => NullCheckPolicy::Never,
        }
    }
}

/// Division check policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DivisionCheckPolicyJson {
    /// Always emit division checks.
    #[serde(alias = "always")]
    Always,
    /// Emit division checks only in debug builds.
    #[serde(alias = "debug")]
    Debug,
    /// Never emit division checks.
    #[serde(alias = "never")]
    #[serde(alias = "off")]
    Never,
}

impl From<DivisionCheckPolicyJson> for DivisionCheckPolicy {
    fn from(value: DivisionCheckPolicyJson) -> Self {
        match value {
            DivisionCheckPolicyJson::Always => DivisionCheckPolicy::Always,
            DivisionCheckPolicyJson::Debug => DivisionCheckPolicy::Debug,
            DivisionCheckPolicyJson::Never => DivisionCheckPolicy::Never,
        }
    }
}

/// Shift range check policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ShiftCheckPolicyJson {
    /// Always emit shift range checks.
    #[serde(alias = "always")]
    Always,
    /// Emit shift range checks only in debug builds.
    #[serde(alias = "debug")]
    Debug,
    /// Never emit shift range checks.
    #[serde(alias = "never")]
    #[serde(alias = "off")]
    Never,
}

impl From<ShiftCheckPolicyJson> for ShiftCheckPolicy {
    fn from(value: ShiftCheckPolicyJson) -> Self {
        match value {
            ShiftCheckPolicyJson::Always => ShiftCheckPolicy::Always,
            ShiftCheckPolicyJson::Debug => ShiftCheckPolicy::Debug,
            ShiftCheckPolicyJson::Never => ShiftCheckPolicy::Never,
        }
    }
}

/// Check failure behavior for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CheckFailurePolicyJson {
    /// Trap immediately on a failed check.
    #[serde(alias = "trap")]
    Trap,
    /// Trigger a panic on a failed check.
    #[serde(alias = "panic")]
    Panic,
    /// Abort execution on a failed check.
    #[serde(alias = "abort")]
    Abort,
}

impl From<CheckFailurePolicyJson> for CheckFailurePolicy {
    fn from(value: CheckFailurePolicyJson) -> Self {
        match value {
            CheckFailurePolicyJson::Trap => CheckFailurePolicy::Trap,
            CheckFailurePolicyJson::Panic => CheckFailurePolicy::Panic,
            CheckFailurePolicyJson::Abort => CheckFailurePolicy::Abort,
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

/// Borrow checking mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BorrowModeJson {
    Hint,
    Strict,
}

impl From<BorrowModeJson> for BorrowMode {
    fn from(value: BorrowModeJson) -> Self {
        match value {
            BorrowModeJson::Hint => BorrowMode::Hint,
            BorrowModeJson::Strict => BorrowMode::Strict,
        }
    }
}

/// Diagnostic policy for allow/warn/deny enforcement.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticPolicyValueJson {
    Allow,
    Warn,
    #[serde(alias = "error")]
    Deny,
}

/// Diagnostic policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum DiagnosticPolicyJson {
    Bool(bool),
    Value(DiagnosticPolicyValueJson),
}

impl From<DiagnosticPolicyJson> for DiagnosticPolicy {
    fn from(value: DiagnosticPolicyJson) -> Self {
        match value {
            DiagnosticPolicyJson::Bool(true) => DiagnosticPolicy::Deny,
            DiagnosticPolicyJson::Bool(false) => DiagnosticPolicy::Allow,
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Allow) => {
                DiagnosticPolicy::Allow
            }
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Warn) => DiagnosticPolicy::Warn,
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Deny) => DiagnosticPolicy::Deny,
        }
    }
}

impl DiagnosticPolicyJson {
    /// Convert allow-style policies where `true` means allow.
    pub fn into_allow_policy(self) -> DiagnosticPolicy {
        match self {
            DiagnosticPolicyJson::Bool(true) => DiagnosticPolicy::Allow,
            DiagnosticPolicyJson::Bool(false) => DiagnosticPolicy::Deny,
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Allow) => {
                DiagnosticPolicy::Allow
            }
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Warn) => DiagnosticPolicy::Warn,
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Deny) => DiagnosticPolicy::Deny,
        }
    }
}

/// Diagnostic policy for allow/warn/deny enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticPolicy {
    /// Allow without diagnostics.
    Allow,
    /// Allow with a warning.
    Warn,
    /// Forbid with an error.
    Deny,
}

impl DiagnosticPolicy {
    /// Return whether this policy is stricter than another.
    pub fn is_stricter_than(self, other: Self) -> bool {
        self.rank() > other.rank()
    }

    /// Return whether this policy is Allow.
    pub fn is_allow(self) -> bool {
        matches!(self, DiagnosticPolicy::Allow)
    }

    /// Return whether this policy is Warn.
    pub fn is_warn(self) -> bool {
        matches!(self, DiagnosticPolicy::Warn)
    }

    /// Return whether this policy is Deny.
    pub fn is_deny(self) -> bool {
        matches!(self, DiagnosticPolicy::Deny)
    }

    /// Return a stable numeric rank for ordering.
    pub fn rank(self) -> u8 {
        match self {
            DiagnosticPolicy::Allow => 0,
            DiagnosticPolicy::Warn => 1,
            DiagnosticPolicy::Deny => 2,
        }
    }
}

/// Implicit collection conversion policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ImplicitCollectionConversionPolicyJson {
    Allow,
    Warn,
    Deny,
}

impl From<ImplicitCollectionConversionPolicyJson> for ImplicitCollectionConversionPolicy {
    fn from(value: ImplicitCollectionConversionPolicyJson) -> Self {
        match value {
            ImplicitCollectionConversionPolicyJson::Allow => {
                ImplicitCollectionConversionPolicy::Allow
            }
            ImplicitCollectionConversionPolicyJson::Warn => {
                ImplicitCollectionConversionPolicy::Warn
            }
            ImplicitCollectionConversionPolicyJson::Deny => {
                ImplicitCollectionConversionPolicy::Deny
            }
        }
    }
}

/// Policy for implicit collection conversions (record-like and sized arrays).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplicitCollectionConversionPolicy {
    /// Allow implicit reification without diagnostics.
    Allow,
    /// Allow implicit reification with a warning.
    Warn,
    /// Forbid implicit reification with an error.
    Deny,
}

impl ImplicitCollectionConversionPolicy {
    /// Return whether this policy is stricter than another.
    pub fn is_stricter_than(self, other: Self) -> bool {
        self.rank() > other.rank()
    }

    /// Return a stable numeric rank for ordering.
    fn rank(self) -> u8 {
        match self {
            ImplicitCollectionConversionPolicy::Allow => 0,
            ImplicitCollectionConversionPolicy::Warn => 1,
            ImplicitCollectionConversionPolicy::Deny => 2,
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
    /// Additional library types to include (e.g., ["node", "dom.iterable"]).
    pub types: Option<Vec<String>>,
    /// Default profile for IDEs and CLI usage.
    pub profile: Option<String>,
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,
    /// Enable incremental compilation for this project.
    pub incremental: Option<bool>,

    // TypeScript-compatible checking
    /// Enable all strict type checking options. Default: true for .ds files.
    pub strict: Option<bool>,
    /// Parse in strict mode.
    pub always_strict: Option<bool>,
    /// Policy for expressions and declarations with implied `any` type.
    pub no_implicit_any: Option<DiagnosticPolicyJson>,
    /// Enable strict null checks (`null` and `undefined` are distinct types).
    pub strict_null_checks: Option<bool>,
    /// Policy for `this` expressions with implied `any` type.
    pub no_implicit_this: Option<DiagnosticPolicyJson>,
    /// Enable strict checking of function types.
    pub strict_function_types: Option<bool>,
    /// Enable strict checking of `bind`, `call`, and `apply` methods.
    pub strict_bind_call_apply: Option<bool>,
    /// Enable strict checking for built in iterator return types.
    pub strict_builtin_iterator_return: Option<bool>,
    /// Enable strict checking of property initialization in classes.
    pub strict_property_initialization: Option<bool>,
    /// Use `unknown` instead of `any` for catch clause variables.
    pub use_unknown_in_catch_variables: Option<bool>,
    /// Policy for unused local variables.
    pub no_unused_locals: Option<DiagnosticPolicyJson>,
    /// Policy for unused function parameters.
    pub no_unused_parameters: Option<DiagnosticPolicyJson>,
    /// Policy for missing return paths.
    pub no_implicit_returns: Option<DiagnosticPolicyJson>,
    /// Policy for unreachable code.
    pub allow_unreachable_code: Option<DiagnosticPolicyJson>,
    /// Policy for unused labels.
    pub allow_unused_labels: Option<DiagnosticPolicyJson>,
    /// Policy for missing `override` on class members.
    pub no_implicit_override: Option<DiagnosticPolicyJson>,
    /// Policy for fallthrough cases in switch statements.
    pub no_fallthrough_cases_in_switch: Option<DiagnosticPolicyJson>,
    /// Interpret optional property types as written without implicit `undefined`.
    pub exact_optional_property_types: Option<bool>,
    /// Policy for unchecked index signature access results.
    pub no_unchecked_indexed_access: Option<DiagnosticPolicyJson>,
    /// Policy for property access from index signatures without explicit index access.
    pub no_property_access_from_index_signature: Option<DiagnosticPolicyJson>,

    // Destack-specific checking
    /// Policy for forbidding `any` types.
    pub no_any: Option<DiagnosticPolicyJson>,
    /// Policy for forbidding `unknown` types.
    pub no_unknown: Option<DiagnosticPolicyJson>,
    /// Policy for imprecise primitive types (int32 vs number, etc.).
    pub no_imprecise_primitives: Option<DiagnosticPolicyJson>,
    /// Policy for implicit widening/narrowing conversions.
    pub no_implicit_conversions: Option<DiagnosticPolicyJson>,
    /// Policy for implicit collection conversions (record-like and sized arrays).
    pub implicit_collection_conversions: Option<ImplicitCollectionConversionPolicyJson>,
    /// Policy for unsafe type assertions (`as T`).
    pub no_unsafe_type_assertions: Option<DiagnosticPolicyJson>,
    /// Policy for must assertions (`x!`).
    #[serde(
        rename = "noMustAssertions",
        alias = "noNonNullAssertions",
        alias = "no_non_null_assertions"
    )]
    pub no_must_assertions: Option<DiagnosticPolicyJson>,
    /// Policy for definite assignment assertions (`field!: T`).
    pub no_definite_assignment_assertions: Option<DiagnosticPolicyJson>,
    /// Policy for custom type guards (`x is T`).
    #[serde(alias = "noUserDefinedTypeGuards")]
    pub no_custom_type_guards: Option<DiagnosticPolicyJson>,
    /// Policy for unsound variance (invariant mutable positions).
    pub no_unsound_variance: Option<DiagnosticPolicyJson>,
    /// Policy for unsound narrowing (`instanceof`, `in`, and predicates).
    pub no_unsound_narrowing: Option<DiagnosticPolicyJson>,
    /// Policy for shallow readonly behavior.
    pub deep_readonly: Option<DiagnosticPolicyJson>,
    /// Policy for untrusted declaration files.
    pub no_untrusted_declarations: Option<DiagnosticPolicyJson>,
    /// Policy for re-declaration of local variables.
    pub no_redeclared_locals: Option<DiagnosticPolicyJson>,
    /// Policy for implicit ownership of managed types and values.
    pub no_implicit_managed: Option<DiagnosticPolicyJson>,
    /// Policy for GC-managed defaults and allocations.
    pub no_managed: Option<DiagnosticPolicyJson>,
    /// Policy for runtime usage (no managed memory, no Promise, no exceptions, ...).
    pub no_runtime: Option<DiagnosticPolicyJson>,
    /// Policy for referential equality.
    pub no_referential_equality: Option<DiagnosticPolicyJson>,
    /// Policy for `eval()` and `Function` constructor.
    pub no_dynamic_evaluation: Option<DiagnosticPolicyJson>,
    /// Policy for `globalThis` access.
    pub no_global_this: Option<DiagnosticPolicyJson>,
    /// Policy for dynamic `import()` and `require()` expressions.
    pub no_dynamic_import: Option<DiagnosticPolicyJson>,
    /// Policy for defineProperty, prototype mutation, delete, and declaration expressions.
    pub no_dynamic_shapes: Option<DiagnosticPolicyJson>,
    /// Policy for computed property access `obj[expr]` where expr isn't constant.
    pub no_computed_property_access: Option<DiagnosticPolicyJson>,
    /// Policy for `Proxy`.
    pub no_proxy: Option<DiagnosticPolicyJson>,
    /// Policy for overloads that are not statically resolvable.
    pub no_implicit_dynamic_dispatch: Option<DiagnosticPolicyJson>,
    /// Policy for `throw` and `try`/`catch` (use Result types instead).
    pub no_exceptions: Option<DiagnosticPolicyJson>,
    /// Borrow checking mode for `&T` and `&mut T`.
    pub borrow_mode: Option<BorrowModeJson>,

    // emit
    /// Root directory of source files (controls output directory structure, not module resolution).
    pub root_dir: Option<String>,
    /// Output directory for compiled files.
    pub out_dir: Option<String>,
    /// Output directory for declaration files (.d.ts). Defaults to outDir.
    pub declaration_dir: Option<String>,
    /// Generate declaration maps for `.d.ts` output.
    pub declaration_map: Option<bool>,
    /// Do not emit output files.
    pub no_emit: Option<bool>,

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
        let mut options = Self {
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
            types: json.types.clone().unwrap_or_default(),
            profile: json.profile.clone(),
            comptime_env: json.comptime_env.clone(),
            incremental: json.incremental.unwrap_or(true),

            // TypeScript-compatible checking
            strict,
            always_strict: json.always_strict.unwrap_or(strict),
            no_implicit_any: json.no_implicit_any.map(DiagnosticPolicy::from).unwrap_or(
                if strict {
                    DiagnosticPolicy::Deny
                } else {
                    DiagnosticPolicy::Allow
                },
            ),
            strict_null_checks: json.strict_null_checks.unwrap_or(strict),
            no_implicit_this: json.no_implicit_this.map(DiagnosticPolicy::from).unwrap_or(
                if strict {
                    DiagnosticPolicy::Deny
                } else {
                    DiagnosticPolicy::Allow
                },
            ),
            strict_function_types: json.strict_function_types.unwrap_or(strict),
            strict_bind_call_apply: json.strict_bind_call_apply.unwrap_or(strict),
            strict_builtin_iterator_return: json.strict_builtin_iterator_return.unwrap_or(strict),
            strict_property_initialization: json.strict_property_initialization.unwrap_or(strict),
            use_unknown_in_catch_variables: json.use_unknown_in_catch_variables.unwrap_or(strict),
            no_unused_locals: json
                .no_unused_locals
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_unused_parameters: json
                .no_unused_parameters
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_implicit_returns: json
                .no_implicit_returns
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Deny),
            allow_unreachable_code: json
                .allow_unreachable_code
                .map(DiagnosticPolicyJson::into_allow_policy)
                .unwrap_or(DiagnosticPolicy::Deny),
            allow_unused_labels: json
                .allow_unused_labels
                .map(DiagnosticPolicyJson::into_allow_policy)
                .unwrap_or(DiagnosticPolicy::Deny),
            no_implicit_override: json
                .no_implicit_override
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Deny),
            no_fallthrough_cases_in_switch: json
                .no_fallthrough_cases_in_switch
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            exact_optional_property_types: json.exact_optional_property_types.unwrap_or(true),
            no_unchecked_indexed_access: json
                .no_unchecked_indexed_access
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Deny),
            no_property_access_from_index_signature: json
                .no_property_access_from_index_signature
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Deny),

            // Destack-specific checking
            no_any: json
                .no_any
                .map(DiagnosticPolicy::from)
                .unwrap_or(if strict {
                    DiagnosticPolicy::Deny
                } else {
                    DiagnosticPolicy::Allow
                }),
            no_unknown: json
                .no_unknown
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_imprecise_primitives: json
                .no_imprecise_primitives
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_implicit_conversions: json
                .no_implicit_conversions
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            implicit_collection_conversions: json
                .implicit_collection_conversions
                .map(ImplicitCollectionConversionPolicy::from)
                .unwrap_or(ImplicitCollectionConversionPolicy::Allow),
            no_unsafe_type_assertions: json
                .no_unsafe_type_assertions
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_must_assertions: json
                .no_must_assertions
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_definite_assignment_assertions: json
                .no_definite_assignment_assertions
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_custom_type_guards: json
                .no_custom_type_guards
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_unsound_variance: json
                .no_unsound_variance
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_unsound_narrowing: json
                .no_unsound_narrowing
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            deep_readonly: json
                .deep_readonly
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            deep_readonly_explicit: json.deep_readonly.is_some(),
            no_untrusted_declarations: json
                .no_untrusted_declarations
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_redeclared_locals: json
                .no_redeclared_locals
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_implicit_managed: json
                .no_implicit_managed
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_managed: json
                .no_managed
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_runtime: json
                .no_runtime
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_referential_equality: json
                .no_referential_equality
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_dynamic_evaluation: json
                .no_dynamic_evaluation
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_global_this: json
                .no_global_this
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_dynamic_import: json
                .no_dynamic_import
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_dynamic_shapes: json
                .no_dynamic_shapes
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_computed_property_access: json
                .no_computed_property_access
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_proxy: json
                .no_proxy
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_implicit_dynamic_dispatch: json
                .no_implicit_dynamic_dispatch
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_exceptions: json
                .no_exceptions
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            borrow_mode: json
                .borrow_mode
                .map(BorrowMode::from)
                .unwrap_or(BorrowMode::Hint),

            // emit
            root_dir: json.root_dir.as_ref().map(PathBuf::from),
            out_dir: json.out_dir.as_ref().map(PathBuf::from),
            declaration_dir: json.declaration_dir.as_ref().map(PathBuf::from),
            declaration_map: json.declaration_map.unwrap_or(false),
            no_emit: json.no_emit.unwrap_or(false),

            // interop
            tsconfig: json.tsconfig.as_ref().map(PathBuf::from),
            allow_ts: json.allow_ts.unwrap_or(true),
            check_ts: json.check_ts.unwrap_or(false),
            allow_js: json.allow_js.unwrap_or(true),
            check_js: json.check_js.unwrap_or(false),
            skip_lib_check: json.skip_lib_check.unwrap_or(false),
        };

        // apply managed defaults when managed runtime is explicitly disabled
        if options.no_managed.is_deny() {
            options.apply_no_managed_defaults();
        }

        // apply runtime-free restrictions when requested
        if options.no_runtime.is_deny() {
            options.apply_no_runtime_restrictions();
        }

        options
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
    /// Target platform (e.g., Web, Windows, macOS, Linux, iOS, Android, WASI, BareMetal, Universal).
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
    /// Determinism policy for runtime scheduling and I/O.
    #[serde(alias = "determinismMode")]
    #[serde(alias = "determinism_mode")]
    pub determinism: Option<DeterminismPolicyJson>,
    /// Replay policy for external effects.
    #[serde(alias = "replayMode")]
    #[serde(alias = "replay_mode")]
    pub replay: Option<ReplayModeJson>,
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
    /// Allow explicit `void` to intentionally discard Promise results.
    pub allow_void_discard: Option<bool>,
    /// Check callback positions in `no-misused-promises`.
    pub check_misused_promises_in_callbacks: Option<bool>,
    /// Check conditionals in `no-misused-promises`.
    pub check_misused_promises_in_conditionals: Option<bool>,
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
    /// Minimum lines required to consider a block for duplicate code checks.
    pub min_duplicate_code_lines: Option<usize>,
    /// Minimum tokens required to consider a block for duplicate code checks.
    pub min_duplicate_code_tokens: Option<usize>,
    /// Minimum similarity percent for near duplicate code matching (0-100, 0=disabled).
    pub min_duplicate_code_near_similarity: Option<u8>,
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
        if let Some(allow_void_discard) = self.allow_void_discard {
            options.allow_void_discard = allow_void_discard;
        }
        if let Some(check_misused_promises_in_callbacks) = self.check_misused_promises_in_callbacks
        {
            options.check_misused_promises_in_callbacks = check_misused_promises_in_callbacks;
        }
        if let Some(check_misused_promises_in_conditionals) =
            self.check_misused_promises_in_conditionals
        {
            options.check_misused_promises_in_conditionals = check_misused_promises_in_conditionals;
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
        if let Some(min_duplicate_code_lines) = self.min_duplicate_code_lines {
            options.min_duplicate_code_lines = min_duplicate_code_lines;
        }
        if let Some(min_duplicate_code_tokens) = self.min_duplicate_code_tokens {
            options.min_duplicate_code_tokens = min_duplicate_code_tokens;
        }
        if let Some(min_duplicate_code_near_similarity) = self.min_duplicate_code_near_similarity {
            options.min_duplicate_code_near_similarity =
                min_duplicate_code_near_similarity.clamp(0, 100);
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
