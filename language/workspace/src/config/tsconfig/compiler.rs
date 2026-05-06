use std::hash::BuildHasherDefault;
use std::path::PathBuf;

use indexmap::IndexMap;
use rustc_hash::FxHasher;
use serde::Deserialize;

use super::language::{EsTarget, ModuleDetection, ModuleResolution, ModuleTarget};

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

    // declarations
    /// Skip type checking of declaration files.
    pub skip_lib_check: bool,
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

            skip_lib_check: false,
        }
    }
}

impl From<&TsCompilerOptionsJson> for TsCompilerOptions {
    fn from(json: &TsCompilerOptionsJson) -> Self {
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

            skip_lib_check: json.skip_lib_check.unwrap_or(false),
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

    /// Target (e.g. `"ES2020"`)
    /// <https://www.typescriptlang.org/tsconfig/#target>
    pub target: Option<String>,

    /// Module (e.g. `"ESNext"`, `"NodeNext"`, `"Preserve"`)
    /// <https://www.typescriptlang.org/tsconfig/#module>
    pub module: Option<String>,

    /// Skip type checking of declaration files.
    /// <https://www.typescriptlang.org/tsconfig/#skipLibCheck>
    pub skip_lib_check: Option<bool>,
}
