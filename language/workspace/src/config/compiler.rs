use std::path::PathBuf;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::{BorrowMode, EsTarget, ModuleTarget};

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
    /// Error on expressions and declarations with implied `any` type.
    pub no_implicit_any: bool,
    /// Enable strict null checks (`null` and `undefined` are distinct types).
    pub strict_null_checks: bool,
    /// Error on `this` expressions with implied `any` type.
    pub no_implicit_this: bool,
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
    /// Report errors on unused local variables.
    pub no_unused_locals: bool,
    /// Report errors on unused parameters.
    pub no_unused_parameters: bool,
    /// Report errors when not all code paths return a value.
    pub no_implicit_returns: bool,
    /// Allow unreachable code.
    pub allow_unreachable_code: bool,
    /// Allow unused labels.
    pub allow_unused_labels: bool,
    /// Require `override` on class members that override base members.
    pub no_implicit_override: bool,
    /// Report errors for fallthrough cases in switch statements.
    pub no_fallthrough_cases_in_switch: bool,
    /// Interpret optional property types as written without implicit `undefined`.
    pub exact_optional_property_types: bool,
    /// Add `undefined` to index signature access results.
    pub no_unchecked_indexed_access: bool,
    /// Disallow property access from index signatures without explicit index access.
    pub no_property_access_from_index_signature: bool,

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
    /// Forbid re-declaration of local variables.
    pub no_redeclared_locals: bool,
    /// Require explicit ownership for managed types and values.
    pub no_implicit_managed: bool,
    /// Forbid GC-managed defaults and allocations (explicit ownership still allowed).
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
    /// Forbid defineProperty, prototype mutation, delete, and declaration expressions.
    pub no_dynamic_shapes: bool,
    /// Forbid computed property access `obj[expr]` where expr isn't constant.
    pub no_computed_property_access: bool,
    /// Forbid `Proxy`.
    pub no_proxy: bool,
    /// Require overloads to be statically resolvable (no implicit runtime dispatch).
    pub no_implicit_dynamic_dispatch: bool,
    /// Forbid `throw` and `try`/`catch` (use Result types instead).
    pub no_exceptions: bool,
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
            no_implicit_any: strict,
            strict_null_checks: strict,
            no_implicit_this: strict,
            strict_function_types: strict,
            strict_bind_call_apply: strict,
            strict_builtin_iterator_return: strict,
            strict_property_initialization: strict,
            use_unknown_in_catch_variables: strict,
            no_unused_locals: false,
            no_unused_parameters: false,
            no_implicit_returns: true,
            allow_unreachable_code: false,
            allow_unused_labels: false,
            no_implicit_override: true,
            no_fallthrough_cases_in_switch: false,
            exact_optional_property_types: true,
            no_unchecked_indexed_access: true,
            no_property_access_from_index_signature: false,

            // Destack-specific checking (all off by default, opt-in)
            no_any: strict,
            no_unknown: false,
            no_imprecise_primitives: false,
            no_implicit_conversions: false,
            no_unsafe_type_assertions: false,
            no_redeclared_locals: false,
            no_implicit_managed: false,
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
        self.no_implicit_any = true;
        self.no_implicit_this = true;
        self.strict_null_checks = true;
        self.strict_function_types = true;
        self.strict_bind_call_apply = true;
        self.strict_builtin_iterator_return = true;
        self.strict_property_initialization = true;
        self.use_unknown_in_catch_variables = true;

        // enable stricter checking defaults beyond the TS strict umbrella
        self.no_any = true;
        self.no_implicit_returns = true;
        self.no_implicit_override = true;
        self.exact_optional_property_types = true;
        self.no_unchecked_indexed_access = true;

        // enable strict diagnostics by default
        self.no_unused_locals = true;
        self.no_unused_parameters = true;
        self.no_fallthrough_cases_in_switch = true;
        self.allow_unreachable_code = false;
        self.allow_unused_labels = false;
    }

    /// Enable native-only restrictions for native and wasm targets.
    pub fn apply_native_restrictions(&mut self) {
        // strict TypeScript checks are required for native targets
        self.apply_strict_defaults();

        // enforce soundness defaults for native targets
        self.no_any = true;
        self.no_imprecise_primitives = true;
        self.no_implicit_conversions = true;
        self.no_unsafe_type_assertions = true;
        self.no_implicit_managed = true;
        self.no_managed = true;
        self.no_property_access_from_index_signature = true;
        self.borrow_mode = BorrowMode::Strict;

        // disable runtime features that native backends cannot support
        self.no_dynamic_evaluation = true;
        self.no_dynamic_import = true;
        self.no_proxy = true;
        self.no_dynamic_shapes = true;
        self.no_exceptions = true;
        self.no_global_this = true;
    }

    /// Enable explicit ownership defaults for managed memory control.
    pub fn apply_no_managed_defaults(&mut self) {
        // require explicit ownership markers for managed types and values
        self.no_implicit_managed = true;
    }

    /// Enable runtime-free restrictions for compile-time only targets.
    pub fn apply_no_runtime_restrictions(&mut self) {
        // force runtime control flags on when runtime is disabled
        self.no_runtime = true;
        self.no_managed = true;
        self.no_exceptions = true;
        self.no_dynamic_evaluation = true;
        self.no_dynamic_import = true;
        self.no_dynamic_shapes = true;
        self.no_computed_property_access = true;
        self.no_proxy = true;
        self.no_global_this = true;
        self.no_implicit_dynamic_dispatch = true;

        // require explicit ownership markers for managed types and values
        self.apply_no_managed_defaults();
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
    /// Error on expressions and declarations with implied `any` type.
    pub no_implicit_any: Option<bool>,
    /// Enable strict null checks (`null` and `undefined` are distinct types).
    pub strict_null_checks: Option<bool>,
    /// Error on `this` expressions with implied `any` type.
    pub no_implicit_this: Option<bool>,
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
    /// Report errors on unused local variables.
    pub no_unused_locals: Option<bool>,
    /// Report errors on unused function parameters.
    pub no_unused_parameters: Option<bool>,
    /// Report errors when not all code paths return a value.
    pub no_implicit_returns: Option<bool>,
    /// Allow unreachable code.
    pub allow_unreachable_code: Option<bool>,
    /// Allow unused labels.
    pub allow_unused_labels: Option<bool>,
    /// Require `override` on class members that override base members.
    pub no_implicit_override: Option<bool>,
    /// Report errors for fallthrough cases in switch statements.
    pub no_fallthrough_cases_in_switch: Option<bool>,
    /// Interpret optional property types as written without implicit `undefined`.
    pub exact_optional_property_types: Option<bool>,
    /// Add `undefined` to index signature access results.
    pub no_unchecked_indexed_access: Option<bool>,
    /// Disallow property access from index signatures without explicit index access.
    pub no_property_access_from_index_signature: Option<bool>,

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
    /// Forbid re-declaration of local variables.
    pub no_redeclared_locals: Option<bool>,
    /// Require explicit ownership for managed types and values.
    pub no_implicit_managed: Option<bool>,
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
    /// Forbid defineProperty, prototype mutation, delete, and declaration expressions.
    pub no_dynamic_shapes: Option<bool>,
    /// Forbid computed property access `obj[expr]` where expr isn't constant.
    pub no_computed_property_access: Option<bool>,
    /// Forbid `Proxy`.
    pub no_proxy: Option<bool>,
    /// Require overloads to be statically resolvable (no implicit runtime dispatch).
    pub no_implicit_dynamic_dispatch: Option<bool>,
    /// Forbid `throw` and `try`/`catch` (use Result types instead).
    pub no_exceptions: Option<bool>,
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
            no_implicit_any: json.no_implicit_any.unwrap_or(strict),
            strict_null_checks: json.strict_null_checks.unwrap_or(strict),
            no_implicit_this: json.no_implicit_this.unwrap_or(strict),
            strict_function_types: json.strict_function_types.unwrap_or(strict),
            strict_bind_call_apply: json.strict_bind_call_apply.unwrap_or(strict),
            strict_builtin_iterator_return: json.strict_builtin_iterator_return.unwrap_or(strict),
            strict_property_initialization: json.strict_property_initialization.unwrap_or(strict),
            use_unknown_in_catch_variables: json.use_unknown_in_catch_variables.unwrap_or(strict),
            no_unused_locals: json.no_unused_locals.unwrap_or(false),
            no_unused_parameters: json.no_unused_parameters.unwrap_or(false),
            no_implicit_returns: json.no_implicit_returns.unwrap_or(true),
            allow_unreachable_code: json.allow_unreachable_code.unwrap_or(false),
            allow_unused_labels: json.allow_unused_labels.unwrap_or(false),
            no_implicit_override: json.no_implicit_override.unwrap_or(true),
            no_fallthrough_cases_in_switch: json.no_fallthrough_cases_in_switch.unwrap_or(false),
            exact_optional_property_types: json.exact_optional_property_types.unwrap_or(true),
            no_unchecked_indexed_access: json.no_unchecked_indexed_access.unwrap_or(true),
            no_property_access_from_index_signature: json
                .no_property_access_from_index_signature
                .unwrap_or(true),

            // Destack-specific checking
            no_any: json.no_any.unwrap_or(strict),
            no_unknown: json.no_unknown.unwrap_or(false),
            no_imprecise_primitives: json.no_imprecise_primitives.unwrap_or(false),
            no_implicit_conversions: json.no_implicit_conversions.unwrap_or(false),
            no_unsafe_type_assertions: json.no_unsafe_type_assertions.unwrap_or(false),
            no_redeclared_locals: json.no_redeclared_locals.unwrap_or(false),
            no_implicit_managed: json.no_implicit_managed.unwrap_or(false),
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
        if options.no_managed {
            options.apply_no_managed_defaults();
        }

        // apply runtime-free restrictions when requested
        if options.no_runtime {
            options.apply_no_runtime_restrictions();
        }

        options
    }
}
