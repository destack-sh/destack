use destack_source::ModuleId;
use destack_workspace::{DsConfigCompilerOptions, TsCompilerOptions};

use crate::Compiler;

/// "TS++" semantic options used during analysis.
/// NOTE #Architecture: merge AnalyzeContextOptions with InferContext..?
#[derive(Debug, Clone, Copy)]
pub struct AnalyzeOptions {
    /// Enable strict checking of function types.
    pub strict_function_types: bool,
    /// Interpret optional property types as written without implicit `undefined`.
    pub exact_optional_property_types: bool,
    /// Add `undefined` to indexed access results.
    pub no_unchecked_indexed_access: bool,
    /// Disallow property access from index signatures without explicit index access.
    pub no_property_access_from_index_signature: bool,
}

impl From<&DsConfigCompilerOptions> for AnalyzeOptions {
    /// Build semantic options from Destack compiler options.
    fn from(options: &DsConfigCompilerOptions) -> Self {
        Self {
            strict_function_types: options.strict_function_types,
            exact_optional_property_types: options.exact_optional_property_types,
            no_unchecked_indexed_access: options.no_unchecked_indexed_access,
            no_property_access_from_index_signature: options
                .no_property_access_from_index_signature,
        }
    }
}

impl From<&TsCompilerOptions> for AnalyzeOptions {
    /// Build semantic options from TypeScript compiler options.
    fn from(options: &TsCompilerOptions) -> Self {
        Self {
            strict_function_types: options.strict_function_types,
            exact_optional_property_types: options.exact_optional_property_types,
            no_unchecked_indexed_access: options.no_unchecked_indexed_access,
            no_property_access_from_index_signature: options
                .no_property_access_from_index_signature,
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
