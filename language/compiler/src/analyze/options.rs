use destack_dir::SymbolDecorators;
use destack_source::ModuleId;
use destack_workspace::{DsConfigCompilerOptions, TsCompilerOptions};

use std::sync::Arc;

use crate::{AnalyzeError, Compiler};
use destack_source::{File, FileType, Uri};
use destack_workspace::DsConfig;

/// "TS++" semantic options used during analysis.
#[derive(Debug, Clone, Copy)]
pub struct AnalyzeOptions {
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
    /// Interpret optional property types as written without implicit `undefined`.
    pub exact_optional_property_types: bool,
    /// Add `undefined` to index signature access results.
    pub no_unchecked_indexed_access: bool,
    /// Disallow property access from index signatures without explicit index access.
    pub no_property_access_from_index_signature: bool,
    /// Allow unreachable code without warnings.
    pub allow_unreachable_code: bool,
    /// Allow unused labels.
    pub allow_unused_labels: bool,
    /// Require `override` on class members that override base members.
    pub no_implicit_override: bool,
    /// Report errors for fallthrough cases in switch statements.
    pub no_fallthrough_cases_in_switch: bool,
    /// Forbid `throw` and `try`/`catch` (use Result types instead).
    pub no_exceptions: bool,
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
    /// Require explicit ownership for managed types and values.
    pub no_implicit_managed: bool,
    /// Forbid GC-managed defaults and allocations (explicit ownership still allowed).
    pub no_managed: bool,
    /// Forbid runtime entirely (no managed memory, no Promise, no exceptions, ...).
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
}

impl AnalyzeOptions {
    /// Apply symbol decorator overrides to these options.
    pub fn with_symbol_decorators(mut self, decorators: &SymbolDecorators) -> Self {
        // force no-managed checks for stack-only or no-managed declarations
        if decorators.is_no_managed || decorators.is_stack_only {
            self.no_managed = true;
        }

        self
    }
}

/// Language compatibility options for a module.
#[derive(Debug, Clone, Copy)]
pub struct ModuleCheckOptions {
    /// Allow TypeScript modules.
    pub allow_ts: bool,
    /// Type check TypeScript modules.
    pub check_ts: bool,
    /// Allow JavaScript modules.
    pub allow_js: bool,
    /// Type check JavaScript modules.
    pub check_js: bool,
    /// Skip type checking for declaration files.
    pub skip_lib_check: bool,
}

impl ModuleCheckOptions {
    /// Build module checks from Destack compiler options.
    fn from_dsconfig(options: &DsConfigCompilerOptions) -> Self {
        Self {
            allow_ts: options.allow_ts,
            check_ts: options.check_ts,
            allow_js: options.allow_js,
            check_js: options.check_js,
            skip_lib_check: options.skip_lib_check,
        }
    }

    /// Override module checks with TypeScript compiler options.
    fn apply_tsconfig(&mut self, options: &TsCompilerOptions) {
        self.allow_js = options.allow_js;
        self.check_js = options.check_js;
        self.skip_lib_check = options.skip_lib_check;
    }
}

impl From<&DsConfigCompilerOptions> for AnalyzeOptions {
    /// Build semantic options from Destack compiler options.
    fn from(options: &DsConfigCompilerOptions) -> Self {
        Self {
            strict: options.strict,
            always_strict: options.always_strict,
            no_implicit_any: options.no_implicit_any,
            strict_null_checks: options.strict_null_checks,
            no_implicit_this: options.no_implicit_this,
            strict_function_types: options.strict_function_types,
            strict_bind_call_apply: options.strict_bind_call_apply,
            strict_builtin_iterator_return: options.strict_builtin_iterator_return,
            strict_property_initialization: options.strict_property_initialization,
            use_unknown_in_catch_variables: options.use_unknown_in_catch_variables,
            no_unused_locals: options.no_unused_locals,
            no_unused_parameters: options.no_unused_parameters,
            no_implicit_returns: options.no_implicit_returns,
            exact_optional_property_types: options.exact_optional_property_types,
            no_unchecked_indexed_access: options.no_unchecked_indexed_access,
            no_property_access_from_index_signature: options
                .no_property_access_from_index_signature,
            allow_unreachable_code: options.allow_unreachable_code,
            allow_unused_labels: options.allow_unused_labels,
            no_implicit_override: options.no_implicit_override,
            no_fallthrough_cases_in_switch: options.no_fallthrough_cases_in_switch,
            no_exceptions: options.no_exceptions,
            no_any: options.no_any,
            no_unknown: options.no_unknown,
            no_imprecise_primitives: options.no_imprecise_primitives,
            no_implicit_conversions: options.no_implicit_conversions,
            no_unsafe_type_assertions: options.no_unsafe_type_assertions,
            no_implicit_managed: options.no_implicit_managed,
            no_managed: options.no_managed,
            no_runtime: options.no_runtime,
            no_referential_equality: options.no_referential_equality,
            no_dynamic_evaluation: options.no_dynamic_evaluation,
            no_global_this: options.no_global_this,
            no_dynamic_import: options.no_dynamic_import,
            no_dynamic_shapes: options.no_dynamic_shapes,
            no_computed_property_access: options.no_computed_property_access,
            no_proxy: options.no_proxy,
            no_implicit_dynamic_dispatch: options.no_implicit_dynamic_dispatch,
        }
    }
}

impl From<&TsCompilerOptions> for AnalyzeOptions {
    /// Build semantic options from TypeScript compiler options.
    fn from(options: &TsCompilerOptions) -> Self {
        Self {
            strict: options.strict,
            always_strict: options.always_strict,
            no_implicit_any: options.no_implicit_any,
            strict_null_checks: options.strict_null_checks,
            no_implicit_this: options.no_implicit_this,
            strict_function_types: options.strict_function_types,
            strict_bind_call_apply: options.strict_bind_call_apply,
            strict_builtin_iterator_return: options.strict_builtin_iterator_return,
            strict_property_initialization: options.strict_property_initialization,
            use_unknown_in_catch_variables: options.use_unknown_in_catch_variables,
            no_unused_locals: options.no_unused_locals,
            no_unused_parameters: options.no_unused_parameters,
            no_implicit_returns: options.no_implicit_returns,
            exact_optional_property_types: options.exact_optional_property_types,
            no_unchecked_indexed_access: options.no_unchecked_indexed_access,
            no_property_access_from_index_signature: options
                .no_property_access_from_index_signature,
            allow_unreachable_code: options.allow_unreachable_code,
            allow_unused_labels: options.allow_unused_labels,
            no_implicit_override: options.no_implicit_override,
            no_fallthrough_cases_in_switch: options.no_fallthrough_cases_in_switch,
            no_exceptions: false,
            no_any: false,
            no_unknown: false,
            no_imprecise_primitives: false,
            no_implicit_conversions: false,
            no_unsafe_type_assertions: false,
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
        }
    }
}

impl Compiler {
    /// Get the effective TS-compatible semantic options for a module.
    pub(crate) fn analyze_context_options_for_module(&self, module_id: ModuleId) -> AnalyzeOptions {
        let module = self.program.modules.get(module_id);
        let module = module.read();

        // use tsconfig options for ts/js modules when available
        if module.language_type.is_typescript() || module.language_type.is_javascript() {
            if let Some(options) = self
                .program
                .with_tsconfig_options(&module, |ts| ts.compiler.clone())
            {
                return AnalyzeOptions::from(&options);
            }

            if let Some(options) = self
                .program
                .with_dsconfig_options(&module, |ds| ds.compiler.clone())
            {
                return AnalyzeOptions::from(&options);
            }

            return AnalyzeOptions::from(&DsConfigCompilerOptions::default());
        }

        if let Some(options) = self
            .program
            .with_dsconfig_options(&module, |ds| ds.compiler.clone())
        {
            return AnalyzeOptions::from(&options);
        }

        AnalyzeOptions::from(&DsConfigCompilerOptions::default())
    }

    /// Get the module compatibility options for a module.
    pub(crate) fn module_check_options_for_module(
        &self,
        module_id: ModuleId,
    ) -> ModuleCheckOptions {
        let module = self.program.modules.get(module_id);
        let module = module.read();

        // start from dsconfig defaults
        let mut options = self
            .program
            .with_dsconfig_options(&module, |ds| ds.compiler.clone())
            .map(|options| ModuleCheckOptions::from_dsconfig(&options))
            .or_else(|| self.module_check_options_from_path(&module))
            .unwrap_or_else(|| {
                ModuleCheckOptions::from_dsconfig(&DsConfigCompilerOptions::default())
            });

        // overlay tsconfig options when present
        if let Some(ts_options) = self
            .program
            .with_tsconfig_options(&module, |ts| ts.compiler.clone())
        {
            options.apply_tsconfig(&ts_options);
        }

        options
    }

    /// Load module checks from a dsconfig.json alongside the module path.
    fn module_check_options_from_path(
        &self,
        module: &destack_workspace::Module,
    ) -> Option<ModuleCheckOptions> {
        let path = module.path.as_ref()?;

        // locate dsconfig.json in the module directory
        let directory = path.parent().unwrap_or(path);
        let dsconfig_path = directory.join("dsconfig.json");
        let has_dsconfig = self.program.fs.exists(&dsconfig_path).ok()?;
        if !has_dsconfig {
            return None;
        }

        // parse dsconfig.json content
        let content = self.program.fs.read_to_string(&dsconfig_path).ok()?;
        let name = dsconfig_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let uri = Uri::from_path(&dsconfig_path);
        let file_id = self.program.files.next_id();
        let file = File::from_text_as_jsonc(
            file_id,
            name,
            uri,
            Some(dsconfig_path),
            FileType::Json,
            content,
        )
        .ok()?;
        let file = Arc::new(file);
        let dsconfig = DsConfig::parse(&file).ok()?;

        // cache dsconfig on the package for future lookups
        let package = self.program.packages.get(module.package_id);
        let mut package = package.write();
        package.dsconfig = Some(dsconfig.clone());

        Some(ModuleCheckOptions::from_dsconfig(
            &dsconfig.options.compiler,
        ))
    }

    /// Check whether a module language mode is allowed by configuration.
    pub(crate) fn module_language_allowed(&self, module_id: ModuleId) -> bool {
        let module = self.program.modules.get(module_id);
        let module = module.read();

        // skip enforcement for builtins
        if module.is_builtin() {
            return true;
        }

        // read compatibility options
        let options = self.module_check_options_for_module(module_id);

        // enforce allowTs and allowJs for user modules
        if module.language_type.is_typescript() && !options.allow_ts {
            self.error(AnalyzeError::TypeScriptDisabled { module: module_id });
            return false;
        }
        if module.language_type.is_javascript() && !options.allow_js {
            self.error(AnalyzeError::JavaScriptDisabled { module: module_id });
            return false;
        }

        true
    }
}
