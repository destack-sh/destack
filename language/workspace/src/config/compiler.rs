use std::path::PathBuf;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::config::{
    BorrowMode, EsTarget, ModuleDetection, ModuleResolution, ModuleTarget, TsConfigOptions,
    normalize_typescript_lib_names, normalize_typescript_type_entries,
};

/// Path alias mapping (resolved from Destack config paths).
pub type DsPathAliases = IndexMap<String, Vec<String>>;

/// Normalized Destack compiler options.
///
/// **By default, strict mode is ON.**
/// Destack defaults to stricter type checking than TypeScript.
#[derive(Debug, Clone)]
pub struct CompilerOptions {
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
    /// Module resolution strategy.
    pub module_resolution: ModuleResolution,
    /// Use package.json exports field during module resolution.
    pub resolve_package_json_exports: bool,
    /// Use package.json imports field during module resolution.
    pub resolve_package_json_imports: bool,
    /// Custom package export conditions for module resolution.
    pub custom_conditions: Vec<String>,
    /// Package linker strategy for bare module resolution.
    pub node_linker: NodeLinker,
    /// How to detect modules versus scripts.
    pub module_detection: ModuleDetection,
    /// Library files to include (e.g., "es2024", "dom", "worker").
    pub lib: Vec<String>,
    /// Additional ambient type entries to include (e.g., "node", "@types/node", "dom.iterable").
    pub types: Vec<String>,
    /// Default environment for IDEs and CLI usage.
    pub environment: Option<String>,
    /// Default profile for IDEs and CLI usage.
    pub profile: Option<String>,
    /// Default mode for IDEs and CLI usage.
    pub mode: Option<String>,
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
    /// Policy for low level internal protocol imports (`platform:`).
    pub no_internal_import: DiagnosticPolicy,
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
    /// Default constructor naming policy for `@tagged` newtype unions.
    pub tagged_case: TaggedCase,

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
    /// Parse JavaScript files (.js, .mjs, .cjs) in JSX mode.
    pub js_as_jsx: bool,
    /// Type-check JavaScript files.
    pub check_js: bool,
    /// Allow arbitrary file extensions in import specifiers.
    pub allow_arbitrary_extensions: bool,
    /// Allow TypeScript file extensions in import specifiers.
    pub allow_importing_ts_extensions: bool,
    /// Preserve import and export syntax verbatim.
    pub verbatim_module_syntax: bool,
    /// Rewrite relative import extensions.
    pub rewrite_relative_import_extensions: bool,
    /// Skip type checking of declaration files.
    pub skip_lib_check: bool,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        // defaults to strict mode ON (stricter than TypeScript)
        let strict = true;
        Self {
            base_url: None,
            paths: None,
            module_resolution: ModuleResolution::default(),
            allow_arbitrary_extensions: false,
            allow_importing_ts_extensions: false,
            resolve_package_json_exports: true,
            resolve_package_json_imports: true,
            custom_conditions: Vec::new(),
            node_linker: NodeLinker::default(),
            module: ModuleTarget::default(),
            es_target: EsTarget::default(),
            module_detection: ModuleDetection::default(),
            lib: Vec::new(), // derived from runtime/platform if empty
            types: Vec::new(),
            environment: None,
            profile: None,
            mode: None,
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
            no_internal_import: DiagnosticPolicy::Allow,
            no_dynamic_shapes: DiagnosticPolicy::Allow,
            no_computed_property_access: DiagnosticPolicy::Allow,
            no_proxy: DiagnosticPolicy::Allow,
            no_implicit_dynamic_dispatch: DiagnosticPolicy::Allow,
            no_exceptions: DiagnosticPolicy::Allow,
            borrow_mode: BorrowMode::Hint,
            tagged_case: TaggedCase::default(),

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
            js_as_jsx: false,
            check_js: false,
            verbatim_module_syntax: false,
            rewrite_relative_import_extensions: false,
            skip_lib_check: false,
        }
    }
}

impl CompilerOptions {
    /// Apply compiler options from one normalized TypeScript configuration.
    pub fn apply_tsconfig_options(&mut self, tsconfig_options: &TsConfigOptions) {
        let compiler = &tsconfig_options.compiler;

        // module resolution
        self.base_url = compiler.base_url.clone();
        self.paths = compiler.paths.as_ref().map(|paths| {
            let mut mapped_paths = DsPathAliases::default();

            for (key, values) in paths {
                mapped_paths.insert(key.clone(), values.clone());
            }

            mapped_paths
        });

        // language and module semantics
        self.module = compiler.module;
        self.es_target = compiler.es_target;
        self.allow_js = compiler.allow_js;
        self.check_js = compiler.check_js;
        self.skip_lib_check = compiler.skip_lib_check;

        // ambient libraries
        if !compiler.lib.is_empty() {
            self.lib = normalize_typescript_lib_names(&compiler.lib);
        }

        if !compiler.types.is_empty() {
            self.types = normalize_typescript_type_entries(&compiler.types);
        }
    }

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

        // keep lint-style diagnostics configurable under native targets
        self.no_unused_locals = DiagnosticPolicy::Allow;
        self.no_unused_parameters = DiagnosticPolicy::Allow;
        self.no_fallthrough_cases_in_switch = DiagnosticPolicy::Allow;
        self.allow_unreachable_code = DiagnosticPolicy::Allow;
        self.allow_unused_labels = DiagnosticPolicy::Allow;

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

/// Node package linker mode for module resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeLinker {
    /// Detect linker mode automatically from workspace files.
    #[default]
    Auto,
    /// Resolve packages through node_modules directory traversal.
    NodeModules,
    /// Resolve packages through Yarn Plug'n'Play manifests.
    Pnp,
}

impl NodeLinker {
    /// Parse one node linker value from config text.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "node-modules" | "node_modules" | "nodeModules" => Some(Self::NodeModules),
            "pnp" => Some(Self::Pnp),
            _ => None,
        }
    }
}

/// Constructor naming policy for `@tagged` newtype unions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaggedCase {
    /// Preserve discriminant spelling.
    Preserve,
    /// Convert discriminants to camelCase.
    CamelCase,
    /// Convert discriminants to UpperCamelCase.
    #[default]
    UpperCamelCase,
    /// Convert discriminants to snake_case.
    SnakeCase,
    /// Convert discriminants to SCREAMING_SNAKE_CASE.
    ScreamingSnakeCase,
}

impl TaggedCase {
    /// Return the spelling used in configuration and `@tagged`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preserve => "preserve",
            Self::CamelCase => "camelCase",
            Self::UpperCamelCase => "UpperCamelCase",
            Self::SnakeCase => "snake_case",
            Self::ScreamingSnakeCase => "SCREAMING_SNAKE_CASE",
        }
    }
}

/// Constructor naming policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum TaggedCaseJson {
    #[serde(rename = "preserve")]
    Preserve,
    #[serde(rename = "camelCase")]
    CamelCase,
    #[serde(rename = "UpperCamelCase")]
    UpperCamelCase,
    #[serde(rename = "snake_case")]
    SnakeCase,
    #[serde(rename = "SCREAMING_SNAKE_CASE")]
    ScreamingSnakeCase,
}

impl From<TaggedCaseJson> for TaggedCase {
    fn from(value: TaggedCaseJson) -> Self {
        match value {
            TaggedCaseJson::Preserve => Self::Preserve,
            TaggedCaseJson::CamelCase => Self::CamelCase,
            TaggedCaseJson::UpperCamelCase => Self::UpperCamelCase,
            TaggedCaseJson::SnakeCase => Self::SnakeCase,
            TaggedCaseJson::ScreamingSnakeCase => Self::ScreamingSnakeCase,
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
    /// Module resolution strategy (e.g., "bundler", "node16", "nodenext").
    pub module_resolution: Option<String>,
    /// Use package.json exports field during module resolution.
    pub resolve_package_json_exports: Option<bool>,
    /// Use package.json imports field during module resolution.
    pub resolve_package_json_imports: Option<bool>,
    /// Custom package export conditions for module resolution.
    pub custom_conditions: Option<Vec<String>>,
    /// Package linker strategy for bare module resolution (`auto`, `node-modules`, `pnp`).
    pub node_linker: Option<String>,
    /// How to detect modules versus scripts (e.g., "auto", "force", "legacy").
    pub module_detection: Option<String>,
    /// Library files to include (e.g., ["es2024", "dom"]).
    pub lib: Option<Vec<String>>,
    /// Additional ambient type entries to include (e.g., ["node", "@types/node", "dom.iterable"]).
    pub types: Option<Vec<String>>,
    /// Default environment for IDEs and CLI usage.
    pub environment: Option<String>,
    /// Default profile for IDEs and CLI usage.
    pub profile: Option<String>,
    /// Default mode for IDEs and CLI usage.
    pub mode: Option<String>,
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
    /// Policy for low level internal protocol imports (`platform:`).
    pub no_internal_import: Option<DiagnosticPolicyJson>,
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
    /// Default constructor naming policy for `@tagged` newtype unions.
    pub tagged_case: Option<TaggedCaseJson>,

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
    /// Parse JavaScript files (.js, .mjs, .cjs) in JSX mode.
    pub js_as_jsx: Option<bool>,
    /// Type-check JavaScript files.
    pub check_js: Option<bool>,
    /// Allow arbitrary file extensions in import specifiers.
    pub allow_arbitrary_extensions: Option<bool>,
    /// Allow TypeScript file extensions in import specifiers.
    pub allow_importing_ts_extensions: Option<bool>,
    /// Preserve import and export syntax verbatim.
    pub verbatim_module_syntax: Option<bool>,
    /// Rewrite relative import extensions.
    pub rewrite_relative_import_extensions: Option<bool>,
    /// Skip type checking of declaration files (.d.ts, .d.ds).
    pub skip_lib_check: Option<bool>,
}

impl From<&CompilerOptionsJson> for CompilerOptions {
    fn from(json: &CompilerOptionsJson) -> Self {
        let module_resolution = json
            .module_resolution
            .as_deref()
            .and_then(ModuleResolution::parse)
            .unwrap_or_default();
        let resolve_package_json_default = matches!(
            module_resolution,
            ModuleResolution::Node16 | ModuleResolution::NodeNext | ModuleResolution::Bundler
        );
        let strict = json.strict.unwrap_or(true);
        let mut options = Self {
            base_url: json.base_url.as_ref().map(PathBuf::from),
            paths: json.paths.clone(),
            module_resolution,
            allow_arbitrary_extensions: json.allow_arbitrary_extensions.unwrap_or(false),
            allow_importing_ts_extensions: json.allow_importing_ts_extensions.unwrap_or(false),
            resolve_package_json_exports: json
                .resolve_package_json_exports
                .unwrap_or(resolve_package_json_default),
            resolve_package_json_imports: json
                .resolve_package_json_imports
                .unwrap_or(resolve_package_json_default),
            custom_conditions: json.custom_conditions.clone().unwrap_or_default(),
            node_linker: json
                .node_linker
                .as_deref()
                .and_then(NodeLinker::parse)
                .unwrap_or_default(),
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
            module_detection: json
                .module_detection
                .as_deref()
                .and_then(ModuleDetection::parse)
                .unwrap_or_default(),
            lib: json.lib.clone().unwrap_or_default(),
            types: json.types.clone().unwrap_or_default(),
            environment: json.environment.clone(),
            profile: json.profile.clone(),
            mode: json.mode.clone(),
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
                .unwrap_or(DiagnosticPolicy::Allow),

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
            no_internal_import: json
                .no_internal_import
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
            tagged_case: json.tagged_case.map(TaggedCase::from).unwrap_or_default(),

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
            js_as_jsx: json.js_as_jsx.unwrap_or(false),
            check_js: json.check_js.unwrap_or(false),
            verbatim_module_syntax: json.verbatim_module_syntax.unwrap_or(false),
            rewrite_relative_import_extensions: json
                .rewrite_relative_import_extensions
                .unwrap_or(false),
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
