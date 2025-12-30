use destack_source::ModuleId;
use destack_workspace::{DsConfigCompilerOptions, TsCompilerOptions};

use crate::Compiler;

/// "TS++" semantic options used during analysis.
/// NOTE #Architecture: merge AnalyzeContextOptions with InferContext..?
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
    /// Add `undefined` to indexed access results.
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
        }
    }
}

impl Compiler {
    /// Get the effective TS-compatible semantic options for a module.
    pub(crate) fn analyze_context_options_for_module(&self, module_id: ModuleId) -> AnalyzeOptions {
        let module = self.program.modules.get(module_id);
        let module = module.read();

        if module.language_type.is_typescript() {
            if let Some(options) = self
                .program
                .with_tsconfig_options(&module, |ts| ts.compiler.clone())
            {
                return AnalyzeOptions::from(&options);
            }

            return AnalyzeOptions::from(&TsCompilerOptions::default());
        }

        if let Some(options) = self
            .program
            .with_dsconfig_options(&module, |ds| ds.compiler.clone())
        {
            return AnalyzeOptions::from(&options);
        }

        AnalyzeOptions::from(&DsConfigCompilerOptions::default())
    }
}
