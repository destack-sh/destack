use destack_dir::SymbolDecorators;
use destack_source::{File, FileType, ModuleId, TargetId, Uri};
use destack_workspace::{CompilerOptions, DiagnosticPolicy, Module, Program, TsCompilerOptions};

use std::sync::Arc;

use crate::{AnalyzeError, Compiler};
use destack_workspace::Destack;

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
    /// Allow unreachable code without diagnostics.
    pub allow_unreachable_code: bool,
    /// Allow unused labels without diagnostics.
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
    /// Forbid must assertions (`expr!`).
    pub no_must_assertions: bool,
    /// Forbid definite assignment assertions (`x!: T`).
    pub no_definite_assignment_assertions: bool,
    /// Forbid custom type guard predicates (`x is T`).
    pub no_custom_type_guards: bool,
    /// Forbid unsound variance rules.
    pub no_unsound_variance: bool,
    /// Forbid unsound narrowing for `instanceof` and `in`.
    pub no_unsound_narrowing: bool,
    /// Require deep readonly semantics.
    pub deep_readonly: bool,
    /// Forbid untrusted declaration files.
    pub no_untrusted_declarations: bool,
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

    /// Return a stable cache key for these options.
    pub fn cache_key(&self) -> u64 {
        // pack boolean flags into a stable cache key
        let mut key = 0u64;

        // strictness and typing flags
        key |= self.strict as u64;
        key |= (self.always_strict as u64) << 1;
        key |= (self.no_implicit_any as u64) << 2;
        key |= (self.strict_null_checks as u64) << 3;
        key |= (self.no_implicit_this as u64) << 4;
        key |= (self.strict_function_types as u64) << 5;
        key |= (self.strict_bind_call_apply as u64) << 6;
        key |= (self.strict_builtin_iterator_return as u64) << 7;

        // property and optionality flags
        key |= (self.strict_property_initialization as u64) << 8;
        key |= (self.use_unknown_in_catch_variables as u64) << 9;
        key |= (self.no_unused_locals as u64) << 10;
        key |= (self.no_unused_parameters as u64) << 11;
        key |= (self.no_implicit_returns as u64) << 12;
        key |= (self.exact_optional_property_types as u64) << 13;
        key |= (self.no_unchecked_indexed_access as u64) << 14;
        key |= (self.no_property_access_from_index_signature as u64) << 15;

        // diagnostics and control flow flags
        key |= (self.allow_unreachable_code as u64) << 16;
        key |= (self.allow_unused_labels as u64) << 17;
        key |= (self.no_implicit_override as u64) << 18;
        key |= (self.no_fallthrough_cases_in_switch as u64) << 19;
        key |= (self.no_exceptions as u64) << 20;
        key |= (self.no_any as u64) << 21;
        key |= (self.no_unknown as u64) << 22;
        key |= (self.no_imprecise_primitives as u64) << 23;

        // type system restriction flags
        key |= (self.no_implicit_conversions as u64) << 24;
        key |= (self.no_unsafe_type_assertions as u64) << 25;
        key |= (self.no_implicit_managed as u64) << 26;
        key |= (self.no_managed as u64) << 27;
        key |= (self.no_runtime as u64) << 28;
        key |= (self.no_referential_equality as u64) << 29;
        key |= (self.no_dynamic_evaluation as u64) << 30;
        key |= (self.no_global_this as u64) << 31;

        // runtime and dispatch flags
        key |= (self.no_dynamic_import as u64) << 32;
        key |= (self.no_dynamic_shapes as u64) << 33;
        key |= (self.no_computed_property_access as u64) << 34;
        key |= (self.no_proxy as u64) << 35;
        key |= (self.no_implicit_dynamic_dispatch as u64) << 36;
        key |= (self.no_must_assertions as u64) << 37;
        key |= (self.no_definite_assignment_assertions as u64) << 38;
        key |= (self.no_custom_type_guards as u64) << 39;
        key |= (self.no_unsound_variance as u64) << 40;
        key |= (self.no_unsound_narrowing as u64) << 41;
        key |= (self.deep_readonly as u64) << 42;
        key |= (self.no_untrusted_declarations as u64) << 43;
        key
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
    fn from_destack_config(options: &CompilerOptions) -> Self {
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

impl From<&CompilerOptions> for AnalyzeOptions {
    /// Build semantic options from Destack compiler options.
    fn from(options: &CompilerOptions) -> Self {
        Self {
            strict: options.strict,
            always_strict: options.always_strict,
            no_implicit_any: !options.no_implicit_any.is_allow(),
            strict_null_checks: options.strict_null_checks,
            no_implicit_this: !options.no_implicit_this.is_allow(),
            strict_function_types: options.strict_function_types,
            strict_bind_call_apply: options.strict_bind_call_apply,
            strict_builtin_iterator_return: options.strict_builtin_iterator_return,
            strict_property_initialization: options.strict_property_initialization,
            use_unknown_in_catch_variables: options.use_unknown_in_catch_variables,
            no_unused_locals: !options.no_unused_locals.is_allow(),
            no_unused_parameters: !options.no_unused_parameters.is_allow(),
            no_implicit_returns: !options.no_implicit_returns.is_allow(),
            exact_optional_property_types: options.exact_optional_property_types,
            no_unchecked_indexed_access: !options.no_unchecked_indexed_access.is_allow(),
            no_property_access_from_index_signature: !options
                .no_property_access_from_index_signature
                .is_allow(),
            allow_unreachable_code: options.allow_unreachable_code.is_allow(),
            allow_unused_labels: options.allow_unused_labels.is_allow(),
            no_implicit_override: !options.no_implicit_override.is_allow(),
            no_fallthrough_cases_in_switch: !options.no_fallthrough_cases_in_switch.is_allow(),
            no_exceptions: !options.no_exceptions.is_allow(),
            no_any: !options.no_any.is_allow(),
            no_unknown: !options.no_unknown.is_allow(),
            no_imprecise_primitives: !options.no_imprecise_primitives.is_allow(),
            no_implicit_conversions: !options.no_implicit_conversions.is_allow(),
            no_unsafe_type_assertions: !options.no_unsafe_type_assertions.is_allow(),
            no_implicit_managed: !options.no_implicit_managed.is_allow(),
            no_managed: !options.no_managed.is_allow(),
            no_runtime: !options.no_runtime.is_allow(),
            no_referential_equality: !options.no_referential_equality.is_allow(),
            no_dynamic_evaluation: !options.no_dynamic_evaluation.is_allow(),
            no_global_this: !options.no_global_this.is_allow(),
            no_dynamic_import: !options.no_dynamic_import.is_allow(),
            no_dynamic_shapes: !options.no_dynamic_shapes.is_allow(),
            no_computed_property_access: !options.no_computed_property_access.is_allow(),
            no_proxy: !options.no_proxy.is_allow(),
            no_implicit_dynamic_dispatch: !options.no_implicit_dynamic_dispatch.is_allow(),
            no_must_assertions: !options.no_must_assertions.is_allow(),
            no_definite_assignment_assertions: !options
                .no_definite_assignment_assertions
                .is_allow(),
            no_custom_type_guards: !options.no_custom_type_guards.is_allow(),
            no_unsound_variance: !options.no_unsound_variance.is_allow(),
            no_unsound_narrowing: !options.no_unsound_narrowing.is_allow(),
            deep_readonly: !options.deep_readonly.is_allow(),
            no_untrusted_declarations: !options.no_untrusted_declarations.is_allow(),
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
            no_must_assertions: false,
            no_definite_assignment_assertions: false,
            no_custom_type_guards: false,
            no_unsound_variance: false,
            no_unsound_narrowing: false,
            deep_readonly: false,
            no_untrusted_declarations: false,
        }
    }
}

impl Compiler {
    /// Apply target-derived restrictions to compiler options when available.
    fn compiler_options_for_module_target(
        &self,
        module: &Module,
        options: CompilerOptions,
    ) -> (CompilerOptions, bool) {
        // use target-specific options when a config target is present
        let package = self.program.packages.get(module.package_id);
        let package = package.read();
        let Some(config) = package.config.as_ref() else {
            return (options, false);
        };

        // select a target when we have an explicit default (or a single target)
        let target = if let Some(name) = config.options.default_target.as_ref() {
            let target_id = TargetId::new(package.id, name);
            package.targets.get(&target_id).cloned()
        } else if package.targets.len() == 1 {
            package.targets.values().next().cloned()
        } else {
            None
        };

        // skip target-derived restrictions when no default target is selected
        let Some(target) = target else {
            return (options, false);
        };

        let is_native_output = target.output.is_wasm() || target.output.is_native();
        let options = Program::compiler_options_for_target(&target, &options);

        (options, is_native_output)
    }

    /// Apply native target-specific overrides to analyze options.
    fn apply_native_overrides(
        &self,
        analyze_options: &mut AnalyzeOptions,
        native_options: &AnalyzeOptions,
        is_native_output: bool,
    ) {
        if !is_native_output {
            return;
        }

        // enforce strict + soundness options for native targets
        analyze_options.strict = native_options.strict;
        analyze_options.always_strict = native_options.always_strict;
        analyze_options.no_implicit_any = native_options.no_implicit_any;
        analyze_options.strict_null_checks = native_options.strict_null_checks;
        analyze_options.no_implicit_this = native_options.no_implicit_this;
        analyze_options.strict_function_types = native_options.strict_function_types;
        analyze_options.strict_bind_call_apply = native_options.strict_bind_call_apply;
        analyze_options.strict_builtin_iterator_return =
            native_options.strict_builtin_iterator_return;
        analyze_options.strict_property_initialization =
            native_options.strict_property_initialization;
        analyze_options.use_unknown_in_catch_variables =
            native_options.use_unknown_in_catch_variables;
        analyze_options.no_implicit_returns = native_options.no_implicit_returns;
        analyze_options.no_implicit_override = native_options.no_implicit_override;
        analyze_options.exact_optional_property_types =
            native_options.exact_optional_property_types;
        analyze_options.no_unchecked_indexed_access = native_options.no_unchecked_indexed_access;
        analyze_options.no_property_access_from_index_signature =
            native_options.no_property_access_from_index_signature;
        analyze_options.no_any = native_options.no_any;
        analyze_options.no_unsafe_type_assertions = native_options.no_unsafe_type_assertions;
        analyze_options.no_must_assertions = native_options.no_must_assertions;
        analyze_options.no_definite_assignment_assertions =
            native_options.no_definite_assignment_assertions;
        analyze_options.no_custom_type_guards = native_options.no_custom_type_guards;
        analyze_options.no_unsound_variance = native_options.no_unsound_variance;
        analyze_options.no_unsound_narrowing = native_options.no_unsound_narrowing;
        analyze_options.deep_readonly = native_options.deep_readonly;
        analyze_options.no_untrusted_declarations = native_options.no_untrusted_declarations;
        analyze_options.no_implicit_managed = native_options.no_implicit_managed;
        analyze_options.no_dynamic_evaluation = native_options.no_dynamic_evaluation;
        analyze_options.no_dynamic_import = native_options.no_dynamic_import;
        analyze_options.no_dynamic_shapes = native_options.no_dynamic_shapes;
        analyze_options.no_proxy = native_options.no_proxy;
        analyze_options.no_exceptions = native_options.no_exceptions;
        analyze_options.no_global_this = native_options.no_global_this;
        analyze_options.no_computed_property_access = native_options.no_computed_property_access;
    }

    /// Get the effective TS-compatible semantic options for a module.
    pub(crate) fn analyze_context_options_for_module(&self, module_id: ModuleId) -> AnalyzeOptions {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let apply_language_defaults = |mut options: CompilerOptions| {
            // default destack modules to deep readonly unless explicitly configured
            if module.language_type.is_destack() && !options.deep_readonly_explicit {
                options.deep_readonly = DiagnosticPolicy::Deny;
            }

            // keep TS/JS semantics unless explicitly overridden
            if (module.language_type.is_typescript() || module.language_type.is_javascript())
                && !options.deep_readonly_explicit
            {
                options.deep_readonly = DiagnosticPolicy::Allow;
            }

            options
        };
        let apply_target_restrictions =
            |options: CompilerOptions| self.compiler_options_for_module_target(&module, options);

        // use tsconfig options for ts/js modules when available
        if module.language_type.is_typescript() || module.language_type.is_javascript() {
            if let Some(options) = self
                .program
                .with_tsconfig_options(&module, |ts| ts.compiler.clone())
            {
                let mut analyze_options = AnalyzeOptions::from(&options);

                if let Some(ds_options) = self
                    .program
                    .with_config_options(&module, |ds| ds.compiler.clone())
                {
                    let ds_options = apply_language_defaults(ds_options);
                    let (ds_options, is_native_output) = apply_target_restrictions(ds_options);
                    let native_options = AnalyzeOptions::from(&ds_options);
                    self.apply_native_overrides(
                        &mut analyze_options,
                        &native_options,
                        is_native_output,
                    );
                }

                return analyze_options;
            }

            if let Some(options) = self
                .program
                .with_config_options(&module, |ds| ds.compiler.clone())
            {
                let options = apply_language_defaults(options);
                let (options, _is_native_output) = apply_target_restrictions(options);
                return AnalyzeOptions::from(&options);
            }

            return AnalyzeOptions::from(&CompilerOptions::default());
        }

        if let Some(options) = self
            .program
            .with_config_options(&module, |ds| ds.compiler.clone())
        {
            let options = apply_language_defaults(options);
            let (options, _is_native_output) = apply_target_restrictions(options);
            return AnalyzeOptions::from(&options);
        }

        let options = apply_language_defaults(CompilerOptions::default());
        AnalyzeOptions::from(&options)
    }

    /// Get the module compatibility options for a module.
    pub(crate) fn module_check_options_for_module(
        &self,
        module_id: ModuleId,
    ) -> ModuleCheckOptions {
        let module = self.program.modules.get(module_id);
        let module = module.read();

        // start from config defaults
        let mut options = self
            .program
            .with_config_options(&module, |ds| ds.compiler.clone())
            .map(|options| ModuleCheckOptions::from_destack_config(&options))
            .or_else(|| self.module_check_options_from_path(&module))
            .unwrap_or_else(
                || ModuleCheckOptions::from_destack_config(&CompilerOptions::default()),
            );

        // overlay tsconfig options when present
        if let Some(ts_options) = self
            .program
            .with_tsconfig_options(&module, |ts| ts.compiler.clone())
        {
            options.apply_tsconfig(&ts_options);
        }

        options
    }

    /// Load module checks from a destack.json alongside the module path.
    fn module_check_options_from_path(&self, module: &Module) -> Option<ModuleCheckOptions> {
        let path = module.path.as_ref()?;

        // locate destack.json in the module directory
        let directory = path.parent().unwrap_or(path);
        let destack_config_path = directory.join("destack.json");
        let has_destack_config = self.program.fs.exists(&destack_config_path).ok()?;
        if !has_destack_config {
            return None;
        }

        // parse destack.json content
        let content = self.program.fs.read_to_string(&destack_config_path).ok()?;
        let name = destack_config_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let uri = Uri::from_path(&destack_config_path);
        let file_id = self.program.files.next_id();
        let file = File::from_text_as_jsonc(
            file_id,
            name,
            uri,
            Some(destack_config_path),
            FileType::Json,
            content,
        )
        .ok()?;
        let file = Arc::new(file);
        let config = Destack::parse(&file).ok()?;

        // cache the config on the package for future lookups
        let package = self.program.packages.get(module.package_id);
        let mut package = package.write();
        package.config = Some(config.clone());

        Some(ModuleCheckOptions::from_destack_config(
            &config.options.compiler,
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
