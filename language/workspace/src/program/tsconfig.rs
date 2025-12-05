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
    /// The normalized/resolved configuration options.
    pub options: TsConfigOptions,
    /// The raw JSON content of the `tsconfig.json` file.
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

        // create initial options from JSON
        let options = TsConfigOptions::from(&tsconfig_json);

        let tsconfig = Self {
            id,
            file_id: file.id,
            is_root,
            uri: file.uri.clone(),
            path,
            directory: directory.clone(),
            paths_base: directory,
            content: tsconfig_json,
            options,
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

/// Project Reference
///
/// <https://www.typescriptlang.org/docs/handbook/project-references.html>
#[derive(Debug, Deserialize, Clone)]
pub struct TsConfigProjectReferences {
    /// Path to the tsconfig.json file (relative to containing tsconfig).
    pub path: PathBuf,
}

/// JSX transformation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum JsxMode {
    /// Preserve JSX as-is in output.
    #[default]
    Preserve,
    /// Transform to React.createElement calls.
    React,
    /// Transform to React 17+ JSX runtime (automatic import).
    ReactJsx,
    /// Transform to React 17+ JSX runtime (development mode).
    ReactJsxDev,
    /// Transform to h() calls (Preact, etc.).
    ReactNative,
}

impl JsxMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "preserve" => Some(Self::Preserve),
            "react" => Some(Self::React),
            "react-jsx" => Some(Self::ReactJsx),
            "react-jsxdev" => Some(Self::ReactJsxDev),
            "react-native" => Some(Self::ReactNative),
            _ => None,
        }
    }
}

/// Module resolution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModuleResolution {
    /// Classic TypeScript resolution (deprecated).
    Classic,
    /// Node.js resolution (CommonJS).
    Node,
    /// Node.js 16+ resolution (ESM).
    Node16,
    /// Node.js Next resolution (ESM).
    NodeNext,
    /// Bundler-style resolution.
    #[default]
    Bundler,
}

impl ModuleResolution {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "classic" => Some(Self::Classic),
            "node" | "node10" => Some(Self::Node),
            "node16" => Some(Self::Node16),
            "nodenext" => Some(Self::NodeNext),
            "bundler" => Some(Self::Bundler),
            _ => None,
        }
    }
}

/// Module format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModuleKind {
    /// CommonJS modules.
    CommonJs,
    /// AMD modules.
    Amd,
    /// UMD modules.
    Umd,
    /// SystemJS modules.
    System,
    /// ES2015 modules.
    Es2015,
    /// ES2020 modules.
    Es2020,
    /// ES2022 modules.
    Es2022,
    /// ESNext modules.
    #[default]
    EsNext,
    /// Node16 modules.
    Node16,
    /// NodeNext modules.
    NodeNext,
    /// Preserve original module syntax.
    Preserve,
    /// No module system.
    None,
}

impl ModuleKind {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "commonjs" => Some(Self::CommonJs),
            "amd" => Some(Self::Amd),
            "umd" => Some(Self::Umd),
            "system" => Some(Self::System),
            "es2015" | "es6" => Some(Self::Es2015),
            "es2020" => Some(Self::Es2020),
            "es2022" => Some(Self::Es2022),
            "esnext" => Some(Self::EsNext),
            "node16" => Some(Self::Node16),
            "nodenext" => Some(Self::NodeNext),
            "preserve" => Some(Self::Preserve),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    /// Whether this module kind is ESM-based.
    pub fn is_esm(&self) -> bool {
        matches!(
            self,
            Self::Es2015
                | Self::Es2020
                | Self::Es2022
                | Self::EsNext
                | Self::Node16
                | Self::NodeNext
                | Self::Preserve
        )
    }
}

/// ECMAScript target version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EsTarget {
    /// ES3 (legacy).
    Es3,
    /// ES5.
    Es5,
    /// ES2015 (ES6).
    Es2015,
    /// ES2016.
    Es2016,
    /// ES2017.
    Es2017,
    /// ES2018.
    Es2018,
    /// ES2019.
    Es2019,
    /// ES2020.
    Es2020,
    /// ES2021.
    Es2021,
    /// ES2022.
    Es2022,
    /// ES2023.
    Es2023,
    /// ES2024.
    Es2024,
    /// ESNext (latest).
    #[default]
    EsNext,
}

impl EsTarget {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "es3" => Some(Self::Es3),
            "es5" => Some(Self::Es5),
            "es2015" | "es6" => Some(Self::Es2015),
            "es2016" => Some(Self::Es2016),
            "es2017" => Some(Self::Es2017),
            "es2018" => Some(Self::Es2018),
            "es2019" => Some(Self::Es2019),
            "es2020" => Some(Self::Es2020),
            "es2021" => Some(Self::Es2021),
            "es2022" => Some(Self::Es2022),
            "es2023" => Some(Self::Es2023),
            "es2024" => Some(Self::Es2024),
            "esnext" => Some(Self::EsNext),
            _ => None,
        }
    }
}

/// How to detect module vs script files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModuleDetection {
    /// Auto-detect based on imports/exports.
    #[default]
    Auto,
    /// Legacy detection (TypeScript <4.7).
    Legacy,
    /// Force all files to be modules.
    Force,
}

impl ModuleDetection {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "legacy" => Some(Self::Legacy),
            "force" => Some(Self::Force),
            _ => None,
        }
    }
}

/// How to handle type-only imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ImportsNotUsedAsValues {
    /// Remove unused imports.
    #[default]
    Remove,
    /// Preserve all imports.
    Preserve,
    /// Error on unused imports.
    Error,
}

impl ImportsNotUsedAsValues {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "remove" => Some(Self::Remove),
            "preserve" => Some(Self::Preserve),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// Path alias mapping (resolved from tsconfig paths).
pub type PathAliases = IndexMap<String, Vec<String>, BuildHasherDefault<FxHasher>>;

/// Normalized TypeScript configuration options (from `tsconfig.json`).
#[derive(Debug, Clone, Default)]
pub struct TsConfigOptions {
    /// Specific files to include in the project.
    pub files: Vec<String>,
    /// Glob patterns for files to include.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,
    /// Compiler options.
    pub compiler: TsConfigCompilerOptions,
}

impl From<&TsConfigJson> for TsConfigOptions {
    fn from(json: &TsConfigJson) -> Self {
        Self {
            files: json.files.clone().unwrap_or_default(),
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            compiler: TsConfigCompilerOptions::from(&json.compiler_options),
        }
    }
}

/// Normalized TypeScript compiler options.
#[derive(Debug, Clone)]
pub struct TsConfigCompilerOptions {
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
    pub module: ModuleKind,
    /// ECMAScript target.
    pub target: EsTarget,
    /// How to detect modules vs scripts.
    pub module_detection: ModuleDetection,

    // jsx
    /// JSX transformation mode.
    pub jsx: JsxMode,
    /// JSX factory function (e.g., "React.createElement").
    pub jsx_factory: Option<String>,
    /// JSX fragment factory (e.g., "React.Fragment").
    pub jsx_fragment_factory: Option<String>,
    /// JSX import source (e.g., "react").
    pub jsx_import_source: Option<String>,

    // decorators
    /// Enable legacy experimental decorators.
    pub experimental_decorators: bool,
    /// Emit decorator metadata.
    pub emit_decorator_metadata: bool,

    // class fields
    /// Use define semantics for class fields.
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
    /// Error on implicit any.
    pub no_implicit_any: bool,
    /// Error on implicit this.
    pub no_implicit_this: bool,
    /// Strict null checks.
    pub strict_null_checks: bool,
    /// Strict function types.
    pub strict_function_types: bool,
    /// Strict bind/call/apply.
    pub strict_bind_call_apply: bool,
    /// Strict property initialization.
    pub strict_property_initialization: bool,
    /// Use unknown in catch variables.
    pub use_unknown_in_catch_variables: bool,

    // checking flags
    /// Allow unreachable code.
    pub allow_unreachable_code: bool,
    /// Allow unused labels.
    pub allow_unused_labels: bool,
    /// Exact optional property types.
    pub exact_optional_property_types: bool,
    /// No fallthrough in switch.
    pub no_fallthrough_cases_in_switch: bool,
    /// Require override keyword.
    pub no_implicit_override: bool,
    /// Require explicit returns.
    pub no_implicit_returns: bool,
    /// No unchecked indexed access.
    pub no_unchecked_indexed_access: bool,
    /// No unused locals.
    pub no_unused_locals: bool,
    /// No unused parameters.
    pub no_unused_parameters: bool,
    /// No property access from index signature.
    pub no_property_access_from_index_signature: bool,

    // library
    /// Built-in library types to include.
    pub lib: Vec<String>,
    /// Type roots.
    pub type_roots: Vec<String>,
    /// Types to include.
    pub types: Vec<String>,
    /// Skip lib check.
    pub skip_lib_check: bool,
    /// No lib.
    pub no_lib: bool,

    // javascript
    /// Allow JavaScript files.
    pub allow_js: bool,
    /// Check JavaScript files.
    pub check_js: bool,
}

impl Default for TsConfigCompilerOptions {
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

            module: ModuleKind::default(),
            target: EsTarget::default(),
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
            type_roots: Vec::new(),
            types: Vec::new(),
            skip_lib_check: false,
            no_lib: false,

            allow_js: false,
            check_js: false,
        }
    }
}

impl From<&TsConfigCompilerOptionsJson> for TsConfigCompilerOptions {
    fn from(json: &TsConfigCompilerOptionsJson) -> Self {
        // determine if strict mode is enabled
        let strict = json.strict.unwrap_or(false);

        Self {
            base_url: json.base_url.clone(),
            paths: json.paths.clone(),
            module_resolution: json
                .module_resolution
                .as_deref()
                .and_then(ModuleResolution::parse)
                .unwrap_or_default(),
            allow_arbitrary_extensions: json.allow_arbitrary_extensions.unwrap_or(false),
            allow_importing_ts_extensions: json.allow_importing_ts_extensions.unwrap_or(false),
            resolve_json_module: json.resolve_json_module.unwrap_or(false),
            resolve_package_json_exports: json.resolve_package_json_exports.unwrap_or(true),
            resolve_package_json_imports: json.resolve_package_json_imports.unwrap_or(true),
            custom_conditions: json.custom_conditions.clone().unwrap_or_default(),

            module: json
                .module
                .as_deref()
                .and_then(ModuleKind::parse)
                .unwrap_or_default(),
            target: json
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
            type_roots: json.type_roots.clone().unwrap_or_default(),
            types: json.types.clone().unwrap_or_default(),
            skip_lib_check: json.skip_lib_check.unwrap_or(false),
            no_lib: json.no_lib.unwrap_or(false),

            allow_js: json.allow_js.unwrap_or(false),
            check_js: json.check_js.unwrap_or(false),
        }
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
