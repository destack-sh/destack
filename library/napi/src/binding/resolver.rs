#![allow(clippy::derivable_impls)]

use std::path::PathBuf;

use napi_derive::napi;

/// How to enforce file extensions.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnforceExtension {
    /// Enforce file extensions (path must include extension).
    Enabled,
    /// Do not enforce file extensions (resolve tries appending extensions from the list).
    #[default]
    Disabled,
}

impl From<EnforceExtension> for destack_resolver::EnforceExtension {
    fn from(enforce: EnforceExtension) -> Self {
        match enforce {
            EnforceExtension::Enabled => destack_resolver::EnforceExtension::Enabled,
            EnforceExtension::Disabled => destack_resolver::EnforceExtension::Disabled,
        }
    }
}

/// Alias value for module resolution.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct AliasValue {
    /// The path to alias to (None means ignore/false).
    pub path: Option<String>,
}

impl From<AliasValue> for destack_resolver::AliasValue {
    fn from(value: AliasValue) -> Self {
        match value.path {
            Some(path) => destack_resolver::AliasValue::Path(path),
            None => destack_resolver::AliasValue::Ignore,
        }
    }
}

/// An alias entry mapping a pattern to target values.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct AliasEntry {
    /// The pattern to match (e.g., "@/*").
    pub pattern: String,
    /// The target values to alias to.
    pub targets: Vec<AliasValue>,
}

/// How to handle TypeScript project references.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeScriptReferences {
    /// Disable references.
    Disabled,
    /// Auto-discover references from tsconfig.json.
    Automatic,
}

impl Default for TypeScriptReferences {
    fn default() -> Self {
        Self::Automatic
    }
}

/// How to discover the TypeScript configuration file.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeScriptDiscovery {
    /// Disable TypeScript configuration discovery.
    Disabled,
    /// Auto-discover the TypeScript configuration file.
    Automatic,
}

impl Default for TypeScriptDiscovery {
    fn default() -> Self {
        Self::Automatic
    }
}

/// TypeScript configuration options.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct TypeScriptOptions {
    /// How to discover the TypeScript configuration file.
    pub discovery: TypeScriptDiscovery,
    /// Path to a specific tsconfig.json file (used when discovery is Disabled).
    pub config_file: Option<String>,
    /// How to handle references.
    pub references: TypeScriptReferences,
}

impl Default for TypeScriptOptions {
    fn default() -> Self {
        Self {
            discovery: TypeScriptDiscovery::Automatic,
            config_file: None,
            references: TypeScriptReferences::Automatic,
        }
    }
}

// -- Resolve Options --

/// Resolution options for module resolution.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// Current working directory to start from.
    pub cwd: Option<String>,

    /// TypeScript configuration options.
    pub tsconfig: TypeScriptOptions,

    /// Aliases to import or require certain modules more easily.
    pub alias: Vec<AliasEntry>,

    /// Condition names for exports field which defines entry points of a package.
    pub conditions: Vec<String>,

    /// Whether and how to enforce file extensions.
    pub enforce_extension: EnforceExtension,

    /// Attempt to resolve these extensions in order.
    pub extensions: Vec<String>,

    /// Request passed to resolve is already fully specified.
    pub is_fully_specified: bool,

    /// Redirect module requests when normal resolving fails.
    pub fallback: Vec<AliasEntry>,

    /// Main files in description files (e.g., ["index"]).
    pub main_files: Vec<String>,

    /// Directories to resolve modules from (e.g., ["node_modules"]).
    pub modules: Vec<String>,

    /// Resolve to a context instead of a file.
    pub resolve_to_context: bool,

    /// Prefer to resolve module requests as relative requests.
    pub prefer_relative: bool,

    /// Prefer to resolve server-relative urls as absolute paths.
    pub prefer_absolute: bool,

    /// A list of directories where requests of server-relative URLs are resolved.
    pub roots: Vec<String>,

    /// Whether to resolve symlinks to their symlinked location.
    pub canonicalize_symlinks: bool,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            tsconfig: TypeScriptOptions::default(),
            alias: vec![],
            conditions: vec![],
            enforce_extension: EnforceExtension::Disabled,
            extensions: vec![
                ".ds".into(),
                ".tsx".into(),
                ".ts".into(),
                ".jsx".into(),
                ".js".into(),
                ".mjs".into(),
                ".cjs".into(),
                ".json".into(),
                ".node".into(),
            ],
            is_fully_specified: false,
            fallback: vec![],
            main_files: vec!["index".into()],
            modules: vec!["node_modules".into()],
            resolve_to_context: false,
            prefer_relative: false,
            prefer_absolute: false,
            roots: vec![],
            canonicalize_symlinks: true,
        }
    }
}

/// Convert alias entries to the internal format.
fn convert_alias(entries: Vec<AliasEntry>) -> destack_resolver::Alias {
    entries
        .into_iter()
        .map(|entry| {
            let targets: Vec<destack_resolver::AliasValue> =
                entry.targets.into_iter().map(|v| v.into()).collect();
            (entry.pattern, targets)
        })
        .collect()
}

impl From<ResolveOptions> for destack_resolver::ResolveOptions {
    fn from(options: ResolveOptions) -> Self {
        // convert tsconfig options
        let tsconfig = match options.tsconfig.discovery {
            TypeScriptDiscovery::Disabled => {
                if let Some(config_file) = options.tsconfig.config_file {
                    let references = match options.tsconfig.references {
                        TypeScriptReferences::Disabled => {
                            destack_resolver::TypeScriptOptionsReferences::Disabled
                        }
                        TypeScriptReferences::Automatic => {
                            destack_resolver::TypeScriptOptionsReferences::Automatic
                        }
                    };
                    Some(destack_resolver::TypeScriptOptionsDiscovery::Manual(
                        destack_resolver::TypeScriptOptionsLocation {
                            config_file: PathBuf::from(config_file),
                            references,
                        },
                    ))
                } else {
                    None
                }
            }
            TypeScriptDiscovery::Automatic => {
                Some(destack_resolver::TypeScriptOptionsDiscovery::Automatic)
            }
        };

        Self {
            cwd: options.cwd.map(PathBuf::from),
            tsconfig,
            alias: convert_alias(options.alias),
            conditions: options.conditions,
            enforce_extension: options.enforce_extension.into(),
            extension_alias: indexmap::IndexMap::new(),
            extensions: options.extensions,
            is_fully_specified: options.is_fully_specified,
            fallback: convert_alias(options.fallback),
            main_files: options.main_files,
            modules: options.modules,
            resolve_to_context: options.resolve_to_context,
            prefer_relative: options.prefer_relative,
            prefer_absolute: options.prefer_absolute,
            restrictions: vec![],
            roots: options.roots.into_iter().map(PathBuf::from).collect(),
            canonicalize_symlinks: options.canonicalize_symlinks,
        }
    }
}

/// Get the default resolve options.
#[napi(js_name = "defaultResolveOptions")]
pub fn default_resolve_options() -> ResolveOptions {
    ResolveOptions::default()
}

// -- Resolution Result --

/// The result of a successful module resolution.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Resolution {
    /// The resolved path.
    pub path: String,
    /// Optional query string (e.g., "?foo").
    pub query: Option<String>,
    /// Optional fragment (e.g., "#bar").
    pub fragment: Option<String>,
}

impl From<destack_resolver::Resolution> for Resolution {
    fn from(resolution: destack_resolver::Resolution) -> Self {
        Self {
            path: resolution.path.to_string_lossy().to_string(),
            query: resolution.query,
            fragment: resolution.fragment,
        }
    }
}
