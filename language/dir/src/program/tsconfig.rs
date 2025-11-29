use std::fmt::Debug;
use std::hash::BuildHasherDefault;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use indexmap::IndexMap;
use parking_lot::RwLock;
use rustc_hash::FxHasher;
use serde::Deserialize;

use destack_source::{File, FileContent, FileId, PathExt, Uri};

/// Template variable for the config directory path (e.g. `${configDir}`).
/// <https://github.com/microsoft/TypeScript/pull/58042>
const TEMPLATE_VARIABLE: &str = "${configDir}";

/// Unique identifier for TsConfigs.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TsConfigId(pub u32);

impl std::fmt::Debug for TsConfigId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for TsConfigId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl TsConfigId {
    /// Wrap an id as a TsConfigId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// TypeScript configuration (usually from `tsconfig.json`).
#[derive(Debug, Clone)]
pub struct TsConfig {
    /// The id of the TsConfig.
    pub id: TsConfigId,
    /// The id of the `tsconfig.json` file.
    pub file_id: FileId,
    /// Whether this is the root tsconfig in its context.
    pub is_root: bool,
    /// The URI of the `tsconfig.json` file.
    pub uri: Uri,
    /// Path to the `tsconfig.json` file (including the `tsconfig.json`).
    pub path: PathBuf,
    /// The directory containing the `tsconfig.json` file.
    pub directory: PathBuf,
    /// Base directory from which to resolve path aliases.
    pub paths_base: PathBuf,
    /// The content of the `tsconfig.json` file.
    pub content: TsConfigJson,
}

impl TsConfig {
    /// Parse a tsconfig from a File with JSON content.
    pub fn parse(
        id: TsConfigId,
        is_root: bool,
        file: &Arc<File>,
    ) -> Result<Self, serde_json::Error> {
        // extract the JSON value from file content
        let FileContent::Json { value, .. } = &file.content else {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file is not JSON",
            )));
        };

        // parse the tsconfig from the JSON value
        let tsconfig_json: TsConfigJson = serde_json::from_value(value.clone())?;

        // extract path from file (prefer file.path, fall back to URI conversion)
        let path = file
            .path
            .clone()
            .or_else(|| file.uri.to_path_buf())
            .expect("tsconfig file must have a valid path");
        let directory = path
            .parent()
            .expect("tsconfig.json must have a parent directory")
            .to_path_buf();

        let tsconfig = Self {
            id,
            file_id: file.id,
            is_root,
            uri: file.uri.clone(),
            path,
            directory: directory.clone(),
            paths_base: directory,
            content: tsconfig_json,
        };
        Ok(tsconfig)
    }

    /// Returns the base path from which to resolve aliases.
    pub fn base_path(&self) -> &Path {
        self.content
            .compiler_options
            .base_url
            .as_deref()
            .unwrap_or_else(|| &self.directory)
    }

    /// Inherits settings from the given tsconfig into `self`.
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    pub fn extend_from(&mut self, tsconfig: &Self) {
        // files
        if self.content.files.is_none()
            && let Some(files) = &tsconfig.content.files
        {
            self.content.files = Some(files.clone());
        }

        // include
        if self.content.include.is_none()
            && let Some(include) = &tsconfig.content.include
        {
            self.content.include = Some(include.clone());
        }

        // exclude
        if self.content.exclude.is_none()
            && let Some(exclude) = &tsconfig.content.exclude
        {
            self.content.exclude = Some(exclude.clone());
        }

        let compiler_options = &mut self.content.compiler_options;

        // compilerOptions.baseUrl
        if compiler_options.base_url.is_none()
            && let Some(base_url) = &tsconfig.content.compiler_options.base_url
        {
            compiler_options.base_url = Some(
                if base_url.to_string_lossy().starts_with(TEMPLATE_VARIABLE) {
                    base_url.clone()
                } else {
                    tsconfig.directory.join(base_url).normalize()
                },
            );
        }

        // compilerOptions.paths
        if compiler_options.paths.is_none() {
            self.paths_base = compiler_options.base_url.as_ref().map_or_else(
                || tsconfig.directory.to_path_buf(),
                |path| {
                    if path.to_string_lossy().starts_with(TEMPLATE_VARIABLE) {
                        path.clone()
                    } else {
                        tsconfig.directory.join(path).normalize()
                    }
                },
            );
            compiler_options.paths = tsconfig.content.compiler_options.paths.clone();
        }

        // compilerOptions.experimentalDecorators
        if compiler_options.experimental_decorators.is_none()
            && let Some(experimental_decorators) =
                tsconfig.content.compiler_options.experimental_decorators
        {
            compiler_options.experimental_decorators = Some(experimental_decorators);
        }

        // compilerOptions.emitDecoratorMetadata
        if compiler_options.emit_decorator_metadata.is_none()
            && let Some(emit_decorator_metadata) =
                tsconfig.content.compiler_options.emit_decorator_metadata
        {
            compiler_options.emit_decorator_metadata = Some(emit_decorator_metadata);
        }

        // compilerOptions.useDefineForClassFields
        if compiler_options.use_define_for_class_fields.is_none()
            && let Some(use_define_for_class_fields) = tsconfig
                .content
                .compiler_options
                .use_define_for_class_fields
        {
            compiler_options.use_define_for_class_fields = Some(use_define_for_class_fields);
        }

        // compilerOptions.rewriteRelativeImportExtensions
        if compiler_options
            .rewrite_relative_import_extensions
            .is_none()
            && let Some(rewrite_relative_import_extensions) = tsconfig
                .content
                .compiler_options
                .rewrite_relative_import_extensions
        {
            compiler_options.rewrite_relative_import_extensions =
                Some(rewrite_relative_import_extensions);
        }

        // compilerOptions.jsx
        if compiler_options.jsx.is_none()
            && let Some(jsx) = &tsconfig.content.compiler_options.jsx
        {
            compiler_options.jsx = Some(jsx.clone());
        }

        // compilerOptions.jsxFactory
        if compiler_options.jsx_factory.is_none()
            && let Some(jsx_factory) = &tsconfig.content.compiler_options.jsx_factory
        {
            compiler_options.jsx_factory = Some(jsx_factory.clone());
        }

        // compilerOptions.jsxFragmentFactory
        if compiler_options.jsx_fragment_factory.is_none()
            && let Some(jsx_fragment_factory) =
                &tsconfig.content.compiler_options.jsx_fragment_factory
        {
            compiler_options.jsx_fragment_factory = Some(jsx_fragment_factory.clone());
        }

        // compilerOptions.jsxImportSource
        if compiler_options.jsx_import_source.is_none()
            && let Some(jsx_import_source) = &tsconfig.content.compiler_options.jsx_import_source
        {
            compiler_options.jsx_import_source = Some(jsx_import_source.clone());
        }

        // compilerOptions.verbatimModuleSyntax
        if compiler_options.verbatim_module_syntax.is_none()
            && let Some(verbatim_module_syntax) =
                tsconfig.content.compiler_options.verbatim_module_syntax
        {
            compiler_options.verbatim_module_syntax = Some(verbatim_module_syntax);
        }

        // compilerOptions.preserveValueImports
        if compiler_options.preserve_value_imports.is_none()
            && let Some(preserve_value_imports) =
                tsconfig.content.compiler_options.preserve_value_imports
        {
            compiler_options.preserve_value_imports = Some(preserve_value_imports);
        }

        // compilerOptions.importsNotUsedAsValues
        if compiler_options.imports_not_used_as_values.is_none()
            && let Some(imports_not_used_as_values) =
                &tsconfig.content.compiler_options.imports_not_used_as_values
        {
            compiler_options.imports_not_used_as_values = Some(imports_not_used_as_values.clone());
        }

        // compilerOptions.target
        if compiler_options.target.is_none()
            && let Some(target) = &tsconfig.content.compiler_options.target
        {
            compiler_options.target = Some(target.clone());
        }

        // compilerOptions.module
        if compiler_options.module.is_none()
            && let Some(module) = &tsconfig.content.compiler_options.module
        {
            compiler_options.module = Some(module.clone());
        }

        // compilerOptions.allowJs
        if compiler_options.allow_js.is_none()
            && let Some(allow_js) = tsconfig.content.compiler_options.allow_js
        {
            compiler_options.allow_js = Some(allow_js);
        }
    }

    /// "Build" the root tsconfig in place, resolving:
    /// * `{configDir}` template variable
    /// * `paths_base` for resolving paths alias
    /// * `baseUrl` to absolute path
    pub fn build(&mut self) {
        // only the root tsconfig requires path resolution
        if !self.is_root {
            return;
        }

        let config_dir = self.directory.to_path_buf();

        if let Some(base_url) = &self.content.compiler_options.base_url {
            // substitute template variable in `tsconfig.compilerOptions.baseUrl`
            let base_url = base_url
                .to_string_lossy()
                .strip_prefix(TEMPLATE_VARIABLE)
                .map_or_else(
                    || config_dir.normalize_with(base_url),
                    |stripped_path| config_dir.join(stripped_path.trim_start_matches('/')),
                );
            self.content.compiler_options.base_url = Some(base_url);
        }

        if self.content.compiler_options.paths.is_some() {
            // paths_base should use base_url if set, otherwise config dir
            if let Some(base_url) = &self.content.compiler_options.base_url {
                self.paths_base = base_url.clone();
            }

            // default to config dir if paths_base is still empty
            if self.paths_base.as_os_str().is_empty() {
                self.paths_base = config_dir.clone();
            }

            // substitute template variable in `tsconfig.compilerOptions.paths`
            for paths in self
                .content
                .compiler_options
                .paths
                .as_mut()
                .unwrap()
                .values_mut()
            {
                for path in paths {
                    Self::substitute_template_variable(&config_dir, path);
                }
            }
        }
    }

    /// Resolves the given `specifier` within the project configured by this
    /// tsconfig, relative to the given `path`.
    pub fn resolve(
        &self,
        path: &Path,
        specifier: &str,
        registry: &TsConfigRegistry,
    ) -> Vec<PathBuf> {
        let paths = self.content.resolve_path_alias(specifier, &self.paths_base);
        for reference in &self.content.references {
            // compute the full path to the reference tsconfig and look it up
            let reference_path = self.directory.normalize_with(&reference.path);
            let Some(tsconfig_id) = registry.get_id_by_path(&reference_path) else {
                continue;
            };
            let tsconfig_lock = registry.get(tsconfig_id);
            let tsconfig = tsconfig_lock.read();
            if path.starts_with(tsconfig.base_path()) {
                let ref_paths = tsconfig
                    .content
                    .resolve_path_alias(specifier, &tsconfig.paths_base);
                return [ref_paths, paths].concat();
            }
        }
        paths
    }

    /// Template variable `${configDir}` for substitution of config files directory path.
    fn substitute_template_variable(directory: &Path, path: &mut String) {
        if let Some(stripped_path) = path.strip_prefix(TEMPLATE_VARIABLE) {
            *path = directory
                .join(stripped_path.trim_start_matches('/'))
                .to_string_lossy()
                .to_string();
        }
    }
}

/// Registry of TsConfigs. THREAD-SAFE.
#[derive(Debug)]
pub struct TsConfigRegistry {
    /// The tsconfigs by id.
    tsconfigs_by_id: DashMap<TsConfigId, Arc<RwLock<TsConfig>>>,
    /// URI-based index for looking up tsconfigs by their URI.
    tsconfigs_by_uri: DashMap<Uri, TsConfigId>,
    /// Path-based index for looking up tsconfigs by their file path.
    tsconfigs_by_path: DashMap<PathBuf, TsConfigId>,
    /// The next tsconfig id.
    next_tsconfig_id: AtomicU32,
}

impl Default for TsConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TsConfigRegistry {
    /// Create a new TsConfigRegistry.
    pub fn new() -> Self {
        Self {
            tsconfigs_by_id: DashMap::new(),
            tsconfigs_by_uri: DashMap::new(),
            tsconfigs_by_path: DashMap::new(),
            next_tsconfig_id: AtomicU32::new(0),
        }
    }

    /// Get and increment the next tsconfig id.
    pub fn next_id(&self) -> TsConfigId {
        let next_tsconfig_id = self.next_tsconfig_id.fetch_add(1, Ordering::Relaxed);
        TsConfigId::new(next_tsconfig_id)
    }

    /// Insert a tsconfig into the registry.
    pub fn insert(&self, tsconfig: TsConfig) {
        let uri = tsconfig.uri.clone();
        let path = tsconfig.path.clone();
        let id = tsconfig.id;
        self.tsconfigs_by_id
            .insert(id, Arc::new(RwLock::new(tsconfig)));
        self.tsconfigs_by_uri.insert(uri, id);
        self.tsconfigs_by_path.insert(path, id);
    }

    /// Get a tsconfig by id.
    ///
    /// # Panics
    /// Panics if the tsconfig is not found.
    pub fn get(&self, id: TsConfigId) -> Arc<RwLock<TsConfig>> {
        self.tsconfigs_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("tsconfig not found for id: {id:?}"))
            .clone()
    }

    /// Get a tsconfig id by its URI.
    pub fn get_id_by_uri(&self, uri: &Uri) -> Option<TsConfigId> {
        self.tsconfigs_by_uri.get(uri).map(|r| *r.value())
    }

    /// Get a tsconfig by its URI.
    pub fn get_by_uri(&self, uri: &Uri) -> Option<Arc<RwLock<TsConfig>>> {
        let id = self.get_id_by_uri(uri)?;
        Some(self.get(id))
    }

    /// Check if a tsconfig exists with the given URI.
    pub fn contains_uri(&self, uri: &Uri) -> bool {
        self.tsconfigs_by_uri.contains_key(uri)
    }

    /// Get a tsconfig id by its file path.
    pub fn get_id_by_path(&self, path: &Path) -> Option<TsConfigId> {
        self.tsconfigs_by_path.get(path).map(|r| *r.value())
    }

    /// Get a tsconfig by its file path.
    pub fn get_by_path(&self, path: &Path) -> Option<Arc<RwLock<TsConfig>>> {
        let id = self.get_id_by_path(path)?;
        Some(self.get(id))
    }

    /// Check if a tsconfig exists at the given file path.
    pub fn contains_path(&self, path: &Path) -> bool {
        self.tsconfigs_by_path.contains_key(path)
    }

    /// Iterate over the tsconfigs in the registry.
    pub fn iter(&self) -> impl Iterator<Item = Arc<RwLock<TsConfig>>> {
        let snapshot: Vec<_> = self
            .tsconfigs_by_id
            .iter()
            .map(|r| r.value().clone())
            .collect();
        snapshot.into_iter()
    }

    /// Get the number of tsconfigs in the registry.
    pub fn len(&self) -> usize {
        self.tsconfigs_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tsconfigs_by_id.is_empty()
    }
}

/// TypeScript JSON (usually from `tsconfig.json`)
/// <https://www.typescriptlang.org/tsconfig>
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TsConfigJson {
    /// Specific files to include in the project.
    /// <https://www.typescriptlang.org/tsconfig/#files>
    #[serde(default)]
    pub files: Option<Vec<String>>,
    /// Files to include in the project.
    /// <https://www.typescriptlang.org/tsconfig/#include>
    #[serde(default)]
    pub include: Option<Vec<String>>,
    /// Files to exclude from the project.
    /// <https://www.typescriptlang.org/tsconfig/#exclude>
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
    /// Paths to other tsconfigs to extend.
    /// <https://www.typescriptlang.org/tsconfig/#extends>
    #[serde(default)]
    pub extends: Option<TsConfigExtendsField>,
    /// Compiler options.
    /// <https://www.typescriptlang.org/tsconfig/#compilerOptions>
    #[serde(default)]
    pub compiler_options: TsConfigCompilerOptionsJson,
    /// Bubbled up project references with a reference to their tsconfig.
    /// <https://www.typescriptlang.org/tsconfig/#references>
    #[serde(default)]
    pub references: Vec<TsConfigProjectReferences>,
}

impl TsConfigJson {
    /// Directory of the `tsconfig.json` file.
    /// Returns any paths to tsconfigs that should be extended by this tsconfig.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
        let specifiers = match &self.extends {
            Some(TsConfigExtendsField::Single(specifier)) => {
                vec![specifier.as_str()]
            }
            Some(TsConfigExtendsField::Multiple(specifiers)) => {
                specifiers.iter().map(String::as_str).collect()
            }
            None => Vec::new(),
        };
        specifiers.into_iter()
    }

    /// Resolves the given `specifier` within the project configured by this tsconfig.
    // <https://github.com/parcel-bundler/parcel/blob/b6224fd519f95e68d8b93ba90376fd94c8b76e69/packages/utils/node-resolver-rs/src/tsconfig.rs#L93>
    pub(super) fn resolve_path_alias(&self, specifier: &str, paths_base: &Path) -> Vec<PathBuf> {
        if specifier.starts_with('.') {
            return Vec::new();
        }

        let compiler_options = &self.compiler_options;
        let base_url_iter = compiler_options
            .base_url
            .as_ref()
            .map_or_else(Vec::new, |base_url| {
                vec![base_url.normalize_with(specifier)]
            });

        let Some(paths_map) = &compiler_options.paths else {
            return base_url_iter;
        };

        let paths = paths_map.get(specifier).map_or_else(
            || {
                let mut longest_prefix_length = 0;
                let mut longest_suffix_length = 0;
                let mut best_key: Option<&String> = None;

                for key in paths_map.keys() {
                    if let Some((prefix, suffix)) = key.split_once('*')
                        && (best_key.is_none() || prefix.len() > longest_prefix_length)
                        && specifier.starts_with(prefix)
                        && specifier.ends_with(suffix)
                    {
                        longest_prefix_length = prefix.len();
                        longest_suffix_length = suffix.len();
                        best_key.replace(key);
                    }
                }

                best_key
                    .and_then(|key| paths_map.get(key))
                    .map_or_else(Vec::new, |paths| {
                        paths
                            .iter()
                            .map(|path| {
                                path.replace(
                                    '*',
                                    &specifier[longest_prefix_length
                                        ..specifier.len() - longest_suffix_length],
                                )
                            })
                            .collect::<Vec<_>>()
                    })
            },
            Clone::clone,
        );

        paths
            .into_iter()
            .map(|p| paths_base.normalize_with(p))
            .chain(base_url_iter)
            .collect()
    }
}

/// TypeScript compiler options.
/// <https://www.typescriptlang.org/tsconfig#compilerOptions>
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TsConfigCompilerOptionsJson {
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

    /// Enforce that indexed accesses are properly checked.
    /// <https://www.typescriptlang.org/tsconfig/#noUncheckedIndexedAccess>
    pub no_unchecked_indexed_access: Option<bool>,

    /// Report errors on unused locals.
    /// <https://www.typescriptlang.org/tsconfig/#noUnusedLocals>
    pub no_unused_locals: Option<bool>,

    /// Report errors on unused parameters.
    /// <https://www.typescriptlang.org/tsconfig/#noUnusedParameters>
    pub no_unused_parameters: Option<bool>,

    /// Disallow property access from index signatures without explicit checks.
    /// <https://www.typescriptlang.org/tsconfig/#noPropertyAccessFromIndexSignature>
    pub no_property_access_from_index_signature: Option<bool>,

    /// Enable all strict type-checking options.
    /// <https://www.typescriptlang.org/tsconfig/#strict>
    pub strict: Option<bool>,

    /// Enable strict checking of `bind`, `call`, and `apply`.
    /// <https://www.typescriptlang.org/tsconfig/#strictBindCallApply>
    pub strict_bind_call_apply: Option<bool>,

    /// Enable strict checking for built-in iterators.
    /// <https://www.typescriptlang.org/tsconfig/#strictBuiltinIteratorReturn>
    pub strict_builtin_iterator_return: Option<bool>,

    /// Enable strict checking of function types.
    /// <https://www.typescriptlang.org/tsconfig/#strictFunctionTypes>
    pub strict_function_types: Option<bool>,

    /// Enable strict null checks.
    /// <https://www.typescriptlang.org/tsconfig/#strictNullChecks>
    pub strict_null_checks: Option<bool>,

    /// Enable strict checking of property initialization in classes.
    /// <https://www.typescriptlang.org/tsconfig/#strictPropertyInitialization>
    pub strict_property_initialization: Option<bool>,

    /// Use `unknown` instead of `any` for `catch` clause variables.
    /// <https://www.typescriptlang.org/tsconfig/#useUnknownInCatchVariables>
    pub use_unknown_in_catch_variables: Option<bool>,

    /// Experimental decorators (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#experimentalDecorators>
    pub experimental_decorators: Option<bool>,

    /// Emit decorator metadata (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#emitDecoratorMetadata>
    pub emit_decorator_metadata: Option<bool>,

    /// Use define semantics for class fields (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#useDefineForClassFields>
    pub use_define_for_class_fields: Option<bool>,

    /// Rewrite relative import extensions (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#rewriteRelativeImportExtensions>
    pub rewrite_relative_import_extensions: Option<bool>,

    /// JSX (e.g. `"react-jsx"`)
    /// <https://www.typescriptlang.org/tsconfig/#jsx>
    pub jsx: Option<String>,

    /// JSX factory (e.g. `"React.createElement"`)
    /// <https://www.typescriptlang.org/tsconfig/#jsxFactory>
    pub jsx_factory: Option<String>,

    /// JSX fragment factory (e.g. `"React.Fragment"`)
    /// <https://www.typescriptlang.org/tsconfig/#jsxFragmentFactory>
    pub jsx_fragment_factory: Option<String>,

    /// JSX import source (e.g. `"react"`)
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

    /// Built-in library types to include (e.g. `["ES2020", "DOM"]`)
    /// <https://www.typescriptlang.org/tsconfig/#lib>
    pub lib: Option<Vec<String>>,

    /// Whether to perform lib replacement (TS 5.7+).
    /// <https://www.typescriptlang.org/tsconfig/#libReplacement>
    pub lib_replacement: Option<bool>,

    /// Do not include the default library declarations.
    /// <https://www.typescriptlang.org/tsconfig/#noLib>
    pub no_lib: Option<bool>,

    /// Allow JavaScript files (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#allowJs>
    pub allow_js: Option<bool>,

    /// Enable type-checking of JavaScript files.
    /// <https://www.typescriptlang.org/tsconfig/#checkJs>
    pub check_js: Option<bool>,

    /// Type roots (e.g. `["src/types"]`)
    /// <https://www.typescriptlang.org/tsconfig/#typeRoots>
    pub type_roots: Option<Vec<String>>,

    /// Types (e.g. `["node"]`)
    /// <https://www.typescriptlang.org/tsconfig/#types>
    pub types: Option<Vec<String>>,

    /// [Deprecated] Skip type checking of default library declaration files.
    /// <https://www.typescriptlang.org/tsconfig/#skipDefaultLibCheck>
    pub skip_default_lib_check: Option<bool>,

    /// Skip type checking of declaration files.
    /// <https://www.typescriptlang.org/tsconfig/#skipLibCheck>
    pub skip_lib_check: Option<bool>,
}

/// Value for the "extends" field.
///
/// <https://www.typescriptlang.org/tsconfig/#extends>
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum TsConfigExtendsField {
    /// Extend a single tsconfig.
    Single(String),
    /// Extend multiple tsconfigs.
    Multiple(Vec<String>),
}

/// Project Reference
///
/// <https://www.typescriptlang.org/docs/handbook/project-references.html>
#[derive(Debug, Deserialize, Clone)]
pub struct TsConfigProjectReferences {
    /// Path to the tsconfig.json file (relative to containing tsconfig).
    pub path: PathBuf,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use destack_source::{File, FileId, FileType, Uri};

    use crate::{TsConfig, TsConfigId};

    #[test]
    fn test_extend_tsconfig_no_override_existing() {
        // verify extend_tsconfig doesn't override existing values
        let parent_config = serde_json::json!({
            "compilerOptions": {
                "baseUrl": "./src",
                "jsx": "react-jsx",
                "target": "ES2020"
            }
        })
        .to_string();

        let child_config = serde_json::json!({
            "compilerOptions": {
                "jsx": "preserve"
            }
        })
        .to_string();

        // create parent file and parse
        let parent_file = Arc::new(
            File::from_text_as_jsonc(
                FileId::new(0),
                "tsconfig.json".to_string(),
                Uri::from_path("/parent/tsconfig.json"),
                Some(PathBuf::from("/parent/tsconfig.json")),
                FileType::Json,
                parent_config,
            )
            .unwrap(),
        );
        let mut parent_tsconfig = TsConfig::parse(TsConfigId::new(0), true, &parent_file).unwrap();
        parent_tsconfig.build();

        // create child file and parse
        let child_file = Arc::new(
            File::from_text_as_jsonc(
                FileId::new(1),
                "tsconfig.json".to_string(),
                Uri::from_path("/child/tsconfig.json"),
                Some(PathBuf::from("/child/tsconfig.json")),
                FileType::Json,
                child_config,
            )
            .unwrap(),
        );
        let mut child_tsconfig = TsConfig::parse(TsConfigId::new(1), true, &child_file).unwrap();

        child_tsconfig.extend_from(&parent_tsconfig);
        child_tsconfig.build();

        let compiler_options = &child_tsconfig.content.compiler_options;

        // child's jsx should be preserved
        assert_eq!(compiler_options.jsx, Some("preserve".to_string()));
        // parent's target should be inherited
        assert_eq!(compiler_options.target, Some("ES2020".to_string()));
        // parent's baseUrl should be inherited (with proper path resolution)
        assert!(compiler_options.base_url.is_some());
    }
}
