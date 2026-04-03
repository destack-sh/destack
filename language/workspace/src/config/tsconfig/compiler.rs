use std::hash::BuildHasherDefault;
use std::path::PathBuf;

use indexmap::IndexMap;
use rustc_hash::FxHasher;
use serde::Deserialize;

use super::language::{
    EsTarget, ImportsNotUsedAsValues, JsxMode, ModuleDetection, ModuleResolution, ModuleTarget,
};

/// Path alias mapping (resolved from tsconfig paths).
pub type PathAliases = IndexMap<String, Vec<String>, BuildHasherDefault<FxHasher>>;

/// Normalized TypeScript compiler options.
#[derive(Debug, Clone)]
pub struct TsCompilerOptions {
    // module resolution
    /// Base URL for resolving non-relative module names.
    pub base_url: Option<PathBuf>,
    /// Path alias mappings.
    pub paths: Option<PathAliases>,
    /// Module resolution strategy.
    pub module_resolution: ModuleResolution,
    /// Allow arbitrary file extensions in imports.
    pub allow_arbitrary_extensions: bool,
    /// Allow importing TypeScript files directly.
    pub allow_importing_ts_extensions: bool,
    /// Resolve JSON modules.
    pub resolve_json_module: bool,
    /// Use package.json exports field.
    pub resolve_package_json_exports: bool,
    /// Use package.json imports field.
    pub resolve_package_json_imports: bool,
    /// Custom conditions for package exports.
    pub custom_conditions: Vec<String>,

    // module & target
    /// Module format.
    pub module: ModuleTarget,
    /// ECMAScript target.
    pub es_target: EsTarget,
    /// How to detect modules vs scripts.
    pub module_detection: ModuleDetection,

    // jsx
    /// JSX transformation mode (ignored).
    pub jsx: JsxMode,
    /// JSX factory function (e.g., "React.createElement") (ignored).
    pub jsx_factory: Option<String>,
    /// JSX fragment factory (e.g., "React.Fragment") (ignored).
    pub jsx_fragment_factory: Option<String>,
    /// JSX import source (e.g., "react") (ignored).
    pub jsx_import_source: Option<String>,

    // decorators
    /// Enable legacy experimental decorators (ignored).
    pub experimental_decorators: bool,
    /// Emit decorator metadata (ignored).
    pub emit_decorator_metadata: bool,

    // class fields
    /// Use define semantics for class fields (ignored).
    pub use_define_for_class_fields: bool,

    // import handling
    /// Verbatim module syntax (no import elision).
    pub verbatim_module_syntax: bool,
    /// Preserve value imports (deprecated).
    pub preserve_value_imports: bool,
    /// How to handle type-only imports.
    pub imports_not_used_as_values: ImportsNotUsedAsValues,
    /// Rewrite relative import extensions.
    pub rewrite_relative_import_extensions: bool,

    // strict mode flags
    /// Enable all strict type-checking options.
    pub strict: bool,
    /// Parse in strict mode.
    pub always_strict: bool,
    /// Error on expressions and declarations with implied `any` type.
    pub no_implicit_any: bool,
    /// Error on `this` expressions with implied `any` type.
    pub no_implicit_this: bool,
    /// Enable strict null checks.
    pub strict_null_checks: bool,
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

    // checking flags
    /// Allow unreachable code.
    pub allow_unreachable_code: bool,
    /// Allow unused labels.
    pub allow_unused_labels: bool,
    /// Interpret optional property types as written without implicit `undefined`.
    pub exact_optional_property_types: bool,
    /// Report errors for fallthrough cases in switch statements.
    pub no_fallthrough_cases_in_switch: bool,
    /// Require `override` on class members that override base members.
    pub no_implicit_override: bool,
    /// Report errors when not all code paths return a value.
    pub no_implicit_returns: bool,
    /// Add `undefined` to index signature access results.
    pub no_unchecked_indexed_access: bool,
    /// Report errors on unused local variables.
    pub no_unused_locals: bool,
    /// Report errors on unused parameters.
    pub no_unused_parameters: bool,
    /// Disallow property access from index signatures without explicit index access.
    pub no_property_access_from_index_signature: bool,

    // library
    /// Built-in library types to include.
    pub lib: Vec<String>,
    /// Whether to perform lib replacement (ignored).
    pub lib_replacement: bool,
    /// Do not include the default library declarations.
    pub no_lib: bool,
    /// Type roots.
    pub type_roots: Vec<String>,
    /// Types to include.
    pub types: Vec<String>,
    /// Skip type checking of default library declaration files (ignored).
    pub skip_default_lib_check: bool,
    /// Skip type checking of declaration files.
    pub skip_lib_check: bool,

    // javascript
    /// Allow JavaScript files.
    pub allow_js: bool,
    /// Check JavaScript files.
    pub check_js: bool,

    // emit
    /// Root directory of source files.
    pub root_dir: Option<PathBuf>,
    /// Output directory for compiled files.
    pub out_dir: Option<PathBuf>,
    /// Output file for bundled output.
    pub out_file: Option<PathBuf>,
    /// Output directory for declaration files.
    pub declaration_dir: Option<PathBuf>,
    /// Generate declaration files.
    pub declaration: bool,
    /// Generate source maps.
    pub source_map: bool,
    /// Generate inline source maps.
    pub inline_source_map: bool,
    /// Include source content in source maps.
    pub inline_sources: bool,
    /// Generate declaration maps.
    pub declaration_map: bool,
    /// Emit output files.
    pub no_emit: bool,
}

impl Default for TsCompilerOptions {
    fn default() -> Self {
        Self {
            base_url: None,
            paths: None,
            module_resolution: ModuleResolution::default(),
            allow_arbitrary_extensions: false,
            allow_importing_ts_extensions: false,
            resolve_json_module: false,
            resolve_package_json_exports: true,
            resolve_package_json_imports: true,
            custom_conditions: Vec::new(),

            module: ModuleTarget::default(),
            es_target: EsTarget::default(),
            module_detection: ModuleDetection::default(),

            jsx: JsxMode::default(),
            jsx_factory: None,
            jsx_fragment_factory: None,
            jsx_import_source: None,

            experimental_decorators: false,
            emit_decorator_metadata: false,

            use_define_for_class_fields: true,

            verbatim_module_syntax: false,
            preserve_value_imports: false,
            imports_not_used_as_values: ImportsNotUsedAsValues::default(),
            rewrite_relative_import_extensions: false,

            strict: false,
            always_strict: false,
            no_implicit_any: false,
            no_implicit_this: false,
            strict_null_checks: false,
            strict_function_types: false,
            strict_bind_call_apply: false,
            strict_builtin_iterator_return: false,
            strict_property_initialization: false,
            use_unknown_in_catch_variables: false,

            allow_unreachable_code: false,
            allow_unused_labels: false,
            exact_optional_property_types: false,
            no_fallthrough_cases_in_switch: false,
            no_implicit_override: false,
            no_implicit_returns: false,
            no_unchecked_indexed_access: false,
            no_unused_locals: false,
            no_unused_parameters: false,
            no_property_access_from_index_signature: false,

            lib: Vec::new(),
            lib_replacement: false,
            no_lib: false,
            type_roots: Vec::new(),
            types: Vec::new(),
            skip_default_lib_check: false,
            skip_lib_check: false,

            allow_js: false,
            check_js: false,

            root_dir: None,
            out_dir: None,
            out_file: None,
            declaration_dir: None,
            declaration: false,
            source_map: false,
            inline_source_map: false,
            inline_sources: false,
            declaration_map: false,
            no_emit: false,
        }
    }
}

impl From<&TsCompilerOptionsJson> for TsCompilerOptions {
    fn from(json: &TsCompilerOptionsJson) -> Self {
        // determine if strict mode is enabled
        let strict = json.strict.unwrap_or(false);

        // resolve module resolution first because package json defaults depend on it
        let module_resolution = json
            .module_resolution
            .as_deref()
            .and_then(ModuleResolution::parse)
            .unwrap_or_default();
        let resolve_package_json_default = matches!(
            module_resolution,
            ModuleResolution::Node16 | ModuleResolution::NodeNext | ModuleResolution::Bundler
        );

        Self {
            base_url: json.base_url.clone(),
            paths: json.paths.clone(),
            module_resolution,
            allow_arbitrary_extensions: json.allow_arbitrary_extensions.unwrap_or(false),
            allow_importing_ts_extensions: json.allow_importing_ts_extensions.unwrap_or(false),
            resolve_json_module: json.resolve_json_module.unwrap_or(false),
            resolve_package_json_exports: json
                .resolve_package_json_exports
                .unwrap_or(resolve_package_json_default),
            resolve_package_json_imports: json
                .resolve_package_json_imports
                .unwrap_or(resolve_package_json_default),
            custom_conditions: json.custom_conditions.clone().unwrap_or_default(),

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

            jsx: json
                .jsx
                .as_deref()
                .and_then(JsxMode::parse)
                .unwrap_or_default(),
            jsx_factory: json.jsx_factory.clone(),
            jsx_fragment_factory: json.jsx_fragment_factory.clone(),
            jsx_import_source: json.jsx_import_source.clone(),

            experimental_decorators: json.experimental_decorators.unwrap_or(false),
            emit_decorator_metadata: json.emit_decorator_metadata.unwrap_or(false),

            use_define_for_class_fields: json.use_define_for_class_fields.unwrap_or(true),

            verbatim_module_syntax: json.verbatim_module_syntax.unwrap_or(false),
            preserve_value_imports: json.preserve_value_imports.unwrap_or(false),
            imports_not_used_as_values: json
                .imports_not_used_as_values
                .as_deref()
                .and_then(ImportsNotUsedAsValues::parse)
                .unwrap_or_default(),
            rewrite_relative_import_extensions: json
                .rewrite_relative_import_extensions
                .unwrap_or(false),

            // strict mode implies several sub-flags
            strict,
            always_strict: json.always_strict.unwrap_or(strict),
            no_implicit_any: json.no_implicit_any.unwrap_or(strict),
            no_implicit_this: json.no_implicit_this.unwrap_or(strict),
            strict_null_checks: json.strict_null_checks.unwrap_or(strict),
            strict_function_types: json.strict_function_types.unwrap_or(strict),
            strict_bind_call_apply: json.strict_bind_call_apply.unwrap_or(strict),
            strict_builtin_iterator_return: json.strict_builtin_iterator_return.unwrap_or(strict),
            strict_property_initialization: json.strict_property_initialization.unwrap_or(strict),
            use_unknown_in_catch_variables: json.use_unknown_in_catch_variables.unwrap_or(strict),

            allow_unreachable_code: json.allow_unreachable_code.unwrap_or(false),
            allow_unused_labels: json.allow_unused_labels.unwrap_or(false),
            exact_optional_property_types: json.exact_optional_property_types.unwrap_or(false),
            no_fallthrough_cases_in_switch: json.no_fallthrough_cases_in_switch.unwrap_or(false),
            no_implicit_override: json.no_implicit_override.unwrap_or(false),
            no_implicit_returns: json.no_implicit_returns.unwrap_or(false),
            no_unchecked_indexed_access: json.no_unchecked_indexed_access.unwrap_or(false),
            no_unused_locals: json.no_unused_locals.unwrap_or(false),
            no_unused_parameters: json.no_unused_parameters.unwrap_or(false),
            no_property_access_from_index_signature: json
                .no_property_access_from_index_signature
                .unwrap_or(false),

            lib: json.lib.clone().unwrap_or_default(),
            lib_replacement: json.lib_replacement.unwrap_or(false),
            no_lib: json.no_lib.unwrap_or(false),
            type_roots: json.type_roots.clone().unwrap_or_default(),
            types: json.types.clone().unwrap_or_default(),
            skip_default_lib_check: json.skip_default_lib_check.unwrap_or(false),
            skip_lib_check: json.skip_lib_check.unwrap_or(false),

            allow_js: json.allow_js.unwrap_or(false),
            check_js: json.check_js.unwrap_or(false),

            root_dir: json.root_dir.clone(),
            out_dir: json.out_dir.clone(),
            out_file: json.out_file.clone(),
            declaration_dir: json.declaration_dir.clone(),
            declaration: json.declaration.unwrap_or(false),
            source_map: json.source_map.unwrap_or(false),
            inline_source_map: json.inline_source_map.unwrap_or(false),
            inline_sources: json.inline_sources.unwrap_or(false),
            declaration_map: json.declaration_map.unwrap_or(false),
            no_emit: json.no_emit.unwrap_or(false),
        }
    }
}

/// TypeScript compiler options.
/// <https://www.typescriptlang.org/tsconfig#compilerOptions>
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TsCompilerOptionsJson {
    /// Base URL (e.g. `./src`)
    /// <https://www.typescriptlang.org/tsconfig/#baseUrl>
    pub base_url: Option<PathBuf>,

    /// Path aliases (e.g. `{ "src/*": ["src/*"] }`)
    /// <https://www.typescriptlang.org/tsconfig/#paths>
    pub paths: Option<IndexMap<String, Vec<String>, BuildHasherDefault<FxHasher>>>,

    /// Allow arbitrary non-standard file extensions to be imported.
    /// <https://www.typescriptlang.org/tsconfig/#allowArbitraryExtensions>
    pub allow_arbitrary_extensions: Option<bool>,

    /// Allow importing `.ts`, `.tsx`, `.mts`, `.cts` files directly.
    /// <https://www.typescriptlang.org/tsconfig/#allowImportingTsExtensions>
    pub allow_importing_ts_extensions: Option<bool>,

    /// Module resolution strategy (e.g. `"node"`, `"classic"`, `"bundler"`, `"node16"`, `"nodenext"`).
    /// <https://www.typescriptlang.org/tsconfig/#moduleResolution>
    pub module_resolution: Option<String>,

    /// Resolve `import ... from "./foo.json"` as modules.
    /// <https://www.typescriptlang.org/tsconfig/#resolveJsonModule>
    pub resolve_json_module: Option<bool>,

    /// Use the `exports` field in package.json when resolving modules.
    /// <https://www.typescriptlang.org/tsconfig/#resolvePackageJsonExports>
    pub resolve_package_json_exports: Option<bool>,

    /// Use the `imports` field in package.json when resolving modules.
    /// <https://www.typescriptlang.org/tsconfig/#resolvePackageJsonImports>
    pub resolve_package_json_imports: Option<bool>,

    /// Extra condition strings to consider when resolving packages.
    /// <https://www.typescriptlang.org/tsconfig/#customConditions>
    pub custom_conditions: Option<Vec<String>>,

    /// How to detect whether a file is a module (`"auto"`, `"legacy"`, `"force"`).
    /// <https://www.typescriptlang.org/tsconfig/#moduleDetection>
    pub module_detection: Option<String>,

    /// Do not report errors on unreachable code.
    /// <https://www.typescriptlang.org/tsconfig/#allowUnreachableCode>
    pub allow_unreachable_code: Option<bool>,

    /// Do not report errors on unused labels.
    /// <https://www.typescriptlang.org/tsconfig/#allowUnusedLabels>
    pub allow_unused_labels: Option<bool>,

    /// Parse in strict mode and emit `"use strict"` for each source file.
    /// <https://www.typescriptlang.org/tsconfig/#alwaysStrict>
    pub always_strict: Option<bool>,

    /// Interpret optional property types as written, not adding `undefined`.
    /// <https://www.typescriptlang.org/tsconfig/#exactOptionalPropertyTypes>
    pub exact_optional_property_types: Option<bool>,

    /// Check for fallthrough cases in switch statements.
    /// <https://www.typescriptlang.org/tsconfig/#noFallthroughCasesInSwitch>
    pub no_fallthrough_cases_in_switch: Option<bool>,

    /// Raise error on expressions and declarations with an implied `any` type.
    /// <https://www.typescriptlang.org/tsconfig/#noImplicitAny>
    pub no_implicit_any: Option<bool>,

    /// Require `override` keyword when overriding class members.
    /// <https://www.typescriptlang.org/tsconfig/#noImplicitOverride>
    pub no_implicit_override: Option<bool>,

    /// Report error when not all code paths in function return a value.
    /// <https://www.typescriptlang.org/tsconfig/#noImplicitReturns>
    pub no_implicit_returns: Option<bool>,

    /// Raise error on `this` expressions with implied `any` type.
    /// <https://www.typescriptlang.org/tsconfig/#noImplicitThis>
    pub no_implicit_this: Option<bool>,

    /// Add `undefined` to index signature access results.
    /// <https://www.typescriptlang.org/tsconfig/#noUncheckedIndexedAccess>
    pub no_unchecked_indexed_access: Option<bool>,

    /// Report errors on unused locals.
    /// <https://www.typescriptlang.org/tsconfig/#noUnusedLocals>
    pub no_unused_locals: Option<bool>,

    /// Report errors on unused parameters.
    /// <https://www.typescriptlang.org/tsconfig/#noUnusedParameters>
    pub no_unused_parameters: Option<bool>,

    /// Disallow property access from index signatures without explicit index access.
    /// <https://www.typescriptlang.org/tsconfig/#noPropertyAccessFromIndexSignature>
    pub no_property_access_from_index_signature: Option<bool>,

    /// Enable all strict type checking options.
    /// <https://www.typescriptlang.org/tsconfig/#strict>
    pub strict: Option<bool>,

    /// Enable strict checking of `bind`, `call`, and `apply`.
    /// <https://www.typescriptlang.org/tsconfig/#strictBindCallApply>
    pub strict_bind_call_apply: Option<bool>,

    /// Enable strict checking for built in iterator return types.
    /// <https://www.typescriptlang.org/tsconfig/#strictBuiltinIteratorReturn>
    pub strict_builtin_iterator_return: Option<bool>,

    /// Enable strict checking of function types.
    /// <https://www.typescriptlang.org/tsconfig/#strictFunctionTypes>
    pub strict_function_types: Option<bool>,

    /// Enable strict null checks.
    /// `null` and `undefined` are distinct types.
    /// <https://www.typescriptlang.org/tsconfig/#strictNullChecks>
    pub strict_null_checks: Option<bool>,

    /// Enable strict checking of property initialization in classes.
    /// <https://www.typescriptlang.org/tsconfig/#strictPropertyInitialization>
    pub strict_property_initialization: Option<bool>,

    /// Use `unknown` instead of `any` for `catch` clause variables.
    /// <https://www.typescriptlang.org/tsconfig/#useUnknownInCatchVariables>
    pub use_unknown_in_catch_variables: Option<bool>,

    /// Experimental decorators (e.g. `true`) (ignored).
    /// <https://www.typescriptlang.org/tsconfig/#experimentalDecorators>
    pub experimental_decorators: Option<bool>,

    /// Emit decorator metadata (e.g. `true`) (ignored).
    /// <https://www.typescriptlang.org/tsconfig/#emitDecoratorMetadata>
    pub emit_decorator_metadata: Option<bool>,

    /// Use define semantics for class fields (e.g. `true`) (ignored).
    /// <https://www.typescriptlang.org/tsconfig/#useDefineForClassFields>
    pub use_define_for_class_fields: Option<bool>,

    /// Rewrite relative import extensions (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#rewriteRelativeImportExtensions>
    pub rewrite_relative_import_extensions: Option<bool>,

    /// JSX (e.g. `"react-jsx"`) (ignored).
    /// <https://www.typescriptlang.org/tsconfig/#jsx>
    pub jsx: Option<String>,

    /// JSX factory (e.g. `"React.createElement"`) (ignored).
    /// <https://www.typescriptlang.org/tsconfig/#jsxFactory>
    pub jsx_factory: Option<String>,

    /// JSX fragment factory (e.g. `"React.Fragment"`) (ignored).
    /// <https://www.typescriptlang.org/tsconfig/#jsxFragmentFactory>
    pub jsx_fragment_factory: Option<String>,

    /// JSX import source (e.g. `"react"`) (ignored).
    /// <https://www.typescriptlang.org/tsconfig/#jsxImportSource>
    pub jsx_import_source: Option<String>,

    /// Verbatim module syntax (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#verbatimModuleSyntax>
    pub verbatim_module_syntax: Option<bool>,

    /// Preserve value imports (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#preserveValueImports>
    pub preserve_value_imports: Option<bool>,

    /// Imports not used as values (e.g. `"error"`)
    /// <https://www.typescriptlang.org/tsconfig/#importsNotUsedAsValues>
    pub imports_not_used_as_values: Option<String>,

    /// Target (e.g. `"ES2020"`)
    /// <https://www.typescriptlang.org/tsconfig/#target>
    pub target: Option<String>,

    /// Module (e.g. `"ESNext"`, `"NodeNext"`, `"Preserve"`)
    /// <https://www.typescriptlang.org/tsconfig/#module>
    pub module: Option<String>,

    /// Built-in library types to include (e.g. `["ES2020", "DOM"]`).
    /// <https://www.typescriptlang.org/tsconfig/#lib>
    pub lib: Option<Vec<String>>,

    /// Whether to perform lib replacement (ignored).
    /// <https://www.typescriptlang.org/tsconfig/#libReplacement>
    pub lib_replacement: Option<bool>,

    /// Do not include the default library declarations.
    /// <https://www.typescriptlang.org/tsconfig/#noLib>
    pub no_lib: Option<bool>,

    /// Type roots (e.g. `["src/types"]`)
    /// <https://www.typescriptlang.org/tsconfig/#typeRoots>
    pub type_roots: Option<Vec<String>>,

    /// Types (e.g. `["node"]`)
    /// <https://www.typescriptlang.org/tsconfig/#types>
    pub types: Option<Vec<String>>,

    /// [Deprecated] Skip type checking of default library declaration files (ignored).
    /// <https://www.typescriptlang.org/tsconfig/#skipDefaultLibCheck>
    pub skip_default_lib_check: Option<bool>,

    /// Skip type checking of declaration files.
    /// <https://www.typescriptlang.org/tsconfig/#skipLibCheck>
    pub skip_lib_check: Option<bool>,

    // javascript
    /// Allow JavaScript files (e.g. `true`).
    /// <https://www.typescriptlang.org/tsconfig/#allowJs>
    pub allow_js: Option<bool>,

    /// Enable type-checking of JavaScript files.
    /// <https://www.typescriptlang.org/tsconfig/#checkJs>
    pub check_js: Option<bool>,

    // emit
    /// Root directory of source files.
    /// <https://www.typescriptlang.org/tsconfig/#rootDir>
    pub root_dir: Option<PathBuf>,
    /// Output directory for compiled files.
    /// <https://www.typescriptlang.org/tsconfig/#outDir>
    pub out_dir: Option<PathBuf>,
    /// Output file for bundled output (rarely used).
    /// <https://www.typescriptlang.org/tsconfig/#outFile>
    pub out_file: Option<PathBuf>,
    /// Output directory for declaration files.
    /// <https://www.typescriptlang.org/tsconfig/#declarationDir>
    pub declaration_dir: Option<PathBuf>,
    /// Generate declaration files (.d.ts).
    /// <https://www.typescriptlang.org/tsconfig/#declaration>
    pub declaration: Option<bool>,
    /// Generate source maps (.js.map).
    /// <https://www.typescriptlang.org/tsconfig/#sourceMap>
    pub source_map: Option<bool>,
    /// Embed source maps inline in output files.
    /// <https://www.typescriptlang.org/tsconfig/#inlineSourceMap>
    pub inline_source_map: Option<bool>,
    /// Include source content in source maps.
    /// <https://www.typescriptlang.org/tsconfig/#inlineSources>
    pub inline_sources: Option<bool>,
    /// Generate declaration maps (.d.ts.map).
    /// <https://www.typescriptlang.org/tsconfig/#declarationMap>
    pub declaration_map: Option<bool>,
    /// Do not emit output files.
    /// <https://www.typescriptlang.org/tsconfig/#noEmit>
    pub no_emit: Option<bool>,
}
