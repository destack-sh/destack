#![allow(clippy::derivable_impls)]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use {destack_resolver as resolver, destack_workspace as workspace};

use super::error::{js_error, parse_optional_input, to_js_value};

/// How to enforce file extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EnforceExtension {
    /// Enforce file extensions (path must include extension).
    Enabled,
    /// Do not enforce file extensions (resolve tries appending extensions from the list).
    #[default]
    Disabled,
}

impl From<EnforceExtension> for resolver::EnforceExtension {
    fn from(enforce: EnforceExtension) -> Self {
        match enforce {
            EnforceExtension::Enabled => resolver::EnforceExtension::Enabled,
            EnforceExtension::Disabled => resolver::EnforceExtension::Disabled,
        }
    }
}

/// Alias value for module resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasValue {
    /// The path to alias to (None means ignore/false).
    pub path: Option<String>,
}

impl From<AliasValue> for resolver::AliasValue {
    fn from(value: AliasValue) -> Self {
        match value.path {
            Some(path) => resolver::AliasValue::Path(path),
            None => resolver::AliasValue::Ignore,
        }
    }
}

/// An alias entry mapping a pattern to target values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasEntry {
    /// The pattern to match (e.g., "@/*").
    pub pattern: String,
    /// The target values to alias to.
    pub targets: Vec<AliasValue>,
}

/// How to handle TypeScript project references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TypeScriptReferences {
    /// Disable references.
    Disabled,
    /// Auto-discover references from tsconfig.json.
    Automatic,
    /// Resolve only manually provided references.
    Manual,
}

impl Default for TypeScriptReferences {
    fn default() -> Self {
        Self::Automatic
    }
}

/// How to discover the TypeScript configuration file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptOptions {
    /// How to discover the TypeScript configuration file.
    pub discovery: TypeScriptDiscovery,
    /// Path to a specific tsconfig.json file (used when discovery is Disabled).
    pub config_file: Option<String>,
    /// How to handle references.
    pub references: TypeScriptReferences,
    /// Paths used for manual references.
    pub reference_paths: Vec<String>,
}

impl Default for TypeScriptOptions {
    fn default() -> Self {
        Self {
            discovery: TypeScriptDiscovery::Automatic,
            config_file: None,
            references: TypeScriptReferences::Automatic,
            reference_paths: Vec::new(),
        }
    }
}

/// Extension alias mapping entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionAliasEntry {
    /// Extension key (for example ".js").
    pub extension: String,
    /// Aliases for this extension.
    pub aliases: Vec<String>,
}

/// Resolution options for module resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    /// Extension aliases for resolving import requests.
    pub extension_alias: Vec<ExtensionAliasEntry>,

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

    /// Path restrictions for resolved modules.
    pub restrictions: Vec<String>,

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
            extension_alias: vec![],
            main_files: vec!["index".into()],
            modules: vec!["node_modules".into()],
            resolve_to_context: false,
            prefer_relative: false,
            prefer_absolute: false,
            restrictions: vec![],
            roots: vec![],
            canonicalize_symlinks: true,
        }
    }
}

/// Convert alias entries to the internal format.
fn convert_alias(entries: Vec<AliasEntry>) -> resolver::Alias {
    entries
        .into_iter()
        .map(|entry| {
            let targets: Vec<resolver::AliasValue> =
                entry.targets.into_iter().map(|v| v.into()).collect();
            (entry.pattern, targets)
        })
        .collect()
}

impl From<ResolveOptions> for resolver::ResolveOptions {
    fn from(options: ResolveOptions) -> Self {
        // convert tsconfig options
        let tsconfig = match options.tsconfig.discovery {
            TypeScriptDiscovery::Disabled => {
                if let Some(config_file) = options.tsconfig.config_file {
                    let references = match options.tsconfig.references {
                        TypeScriptReferences::Disabled => {
                            resolver::TypeScriptOptionsReferences::Disabled
                        }
                        TypeScriptReferences::Automatic => {
                            resolver::TypeScriptOptionsReferences::Automatic
                        }
                        TypeScriptReferences::Manual => {
                            resolver::TypeScriptOptionsReferences::Paths(
                                options
                                    .tsconfig
                                    .reference_paths
                                    .into_iter()
                                    .map(PathBuf::from)
                                    .collect(),
                            )
                        }
                    };
                    Some(resolver::TypeScriptOptionsDiscovery::Manual(
                        resolver::TypeScriptOptionsLocation {
                            config_file: PathBuf::from(config_file),
                            references,
                        },
                    ))
                } else {
                    None
                }
            }
            TypeScriptDiscovery::Automatic => Some(resolver::TypeScriptOptionsDiscovery::Automatic),
        };

        Self {
            cwd: options.cwd.map(PathBuf::from),
            tsconfig,
            alias: convert_alias(options.alias),
            conditions: options.conditions,
            enforce_extension: options.enforce_extension.into(),
            extension_alias: options
                .extension_alias
                .into_iter()
                .map(|entry| (entry.extension, entry.aliases))
                .collect(),
            extensions: options.extensions,
            is_fully_specified: options.is_fully_specified,
            fallback: convert_alias(options.fallback),
            main_files: options.main_files,
            modules: options.modules,
            resolve_to_context: options.resolve_to_context,
            prefer_relative: options.prefer_relative,
            prefer_absolute: options.prefer_absolute,
            restrictions: options
                .restrictions
                .into_iter()
                .map(PathBuf::from)
                .map(resolver::Restriction::Path)
                .collect(),
            roots: options.roots.into_iter().map(PathBuf::from).collect(),
            canonicalize_symlinks: options.canonicalize_symlinks,
        }
    }
}

impl From<resolver::ResolveOptions> for ResolveOptions {
    fn from(options: resolver::ResolveOptions) -> Self {
        let tsconfig = match options.tsconfig {
            None => TypeScriptOptions::default(),
            Some(resolver::TypeScriptOptionsDiscovery::Automatic) => TypeScriptOptions {
                discovery: TypeScriptDiscovery::Automatic,
                config_file: None,
                references: TypeScriptReferences::Automatic,
                reference_paths: Vec::new(),
            },
            Some(resolver::TypeScriptOptionsDiscovery::Manual(location)) => {
                let (references, reference_paths) = match location.references {
                    resolver::TypeScriptOptionsReferences::Disabled => {
                        (TypeScriptReferences::Disabled, Vec::new())
                    }
                    resolver::TypeScriptOptionsReferences::Automatic => {
                        (TypeScriptReferences::Automatic, Vec::new())
                    }
                    resolver::TypeScriptOptionsReferences::Paths(paths) => (
                        TypeScriptReferences::Manual,
                        paths
                            .into_iter()
                            .map(|path| path.to_string_lossy().to_string())
                            .collect(),
                    ),
                };

                TypeScriptOptions {
                    discovery: TypeScriptDiscovery::Disabled,
                    config_file: Some(location.config_file.to_string_lossy().to_string()),
                    references,
                    reference_paths,
                }
            }
        };

        let alias = options
            .alias
            .into_iter()
            .map(|(pattern, targets)| AliasEntry {
                pattern,
                targets: targets
                    .into_iter()
                    .map(|target| match target {
                        resolver::AliasValue::Path(path) => AliasValue { path: Some(path) },
                        resolver::AliasValue::Ignore => AliasValue { path: None },
                    })
                    .collect(),
            })
            .collect();

        let fallback = options
            .fallback
            .into_iter()
            .map(|(pattern, targets)| AliasEntry {
                pattern,
                targets: targets
                    .into_iter()
                    .map(|target| match target {
                        resolver::AliasValue::Path(path) => AliasValue { path: Some(path) },
                        resolver::AliasValue::Ignore => AliasValue { path: None },
                    })
                    .collect(),
            })
            .collect();

        let restrictions = options
            .restrictions
            .into_iter()
            .filter_map(|restriction| match restriction {
                resolver::Restriction::Path(path) => Some(path.to_string_lossy().to_string()),
                resolver::Restriction::Function(_) => None,
            })
            .collect();

        ResolveOptions {
            cwd: options.cwd.map(|path| path.to_string_lossy().to_string()),
            tsconfig,
            alias,
            conditions: options.conditions,
            enforce_extension: match options.enforce_extension {
                resolver::EnforceExtension::Enabled => EnforceExtension::Enabled,
                resolver::EnforceExtension::Disabled => EnforceExtension::Disabled,
            },
            extension_alias: options
                .extension_alias
                .into_iter()
                .map(|(extension, aliases)| ExtensionAliasEntry { extension, aliases })
                .collect(),
            extensions: options.extensions,
            is_fully_specified: options.is_fully_specified,
            fallback,
            main_files: options.main_files,
            modules: options.modules,
            resolve_to_context: options.resolve_to_context,
            prefer_relative: options.prefer_relative,
            prefer_absolute: options.prefer_absolute,
            restrictions,
            roots: options
                .roots
                .into_iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect(),
            canonicalize_symlinks: options.canonicalize_symlinks,
        }
    }
}

/// Get the default resolve options.
#[wasm_bindgen(js_name = defaultResolveOptions)]
pub fn default_resolve_options() -> Result<JsValue, JsValue> {
    to_js_value(&ResolveOptions::default())
}

/// The result of a successful module resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    /// The resolved path.
    pub path: String,
    /// Optional query string (e.g., "?foo").
    pub query: Option<String>,
    /// Optional fragment (e.g., "#bar").
    pub fragment: Option<String>,
}

impl From<resolver::Resolution> for Resolution {
    fn from(resolution: resolver::Resolution) -> Self {
        Self {
            path: resolution.path.to_string_lossy().to_string(),
            query: resolution.query,
            fragment: resolution.fragment,
        }
    }
}

/// Resolve a module specifier synchronously.
#[wasm_bindgen(js_name = resolveSync)]
pub fn resolve_sync(
    specifier: String,
    from: String,
    options: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    // decode options and resolve the base directory
    let options: ResolveOptions = parse_optional_input(options)?;
    let cwd = resolve_cwd_from_options(&options)?;
    let from_path = resolve_from_path(&from, &cwd);
    let from_directory = resolve_from_directory(&from_path);

    // create a resolver session with mapped options
    let core_options: resolver::ResolveOptions = options.into();
    let session = workspace::Session::new(cwd);
    let resolver = resolver::Resolver::from_session(&session, core_options);

    // resolve and encode the result payload
    let resolution = resolver
        .resolve(&from_directory, &specifier)
        .map_err(|error| resolve_error(error, &specifier, &from_directory))?;

    to_js_value(&Resolution::from(resolution))
}

/// Resolve cwd from options or current process directory.
fn resolve_cwd_from_options(options: &ResolveOptions) -> Result<PathBuf, JsValue> {
    if let Some(cwd) = options.cwd.as_ref() {
        return Ok(PathBuf::from(cwd));
    }

    std::env::current_dir()
        .map_err(|error| js_error(format!("failed to read current working directory: {error}")))
}

/// Resolve the `from` value into an absolute path.
fn resolve_from_path(from: &str, cwd: &Path) -> PathBuf {
    let from_path = PathBuf::from(from);
    if from_path.is_absolute() {
        from_path
    } else {
        cwd.join(from_path)
    }
}

/// Resolve a containing directory for module resolution.
fn resolve_from_directory(from_path: &Path) -> PathBuf {
    // treat existing files as file paths
    if from_path
        .metadata()
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
    {
        return from_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| from_path.to_path_buf());
    }

    // treat extension-like values as file paths when metadata is unavailable
    if from_path.extension().is_some() {
        return from_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| from_path.to_path_buf());
    }

    from_path.to_path_buf()
}

/// Build a js error payload for resolve failures.
fn resolve_error(error: resolver::ResolveError, specifier: &str, from: &Path) -> JsValue {
    js_error(format!(
        "failed to resolve '{specifier}' from '{}': {error}",
        from.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keep the resolve options mapping stable between wasm and core options.
    #[test]
    fn test_resolve_options_roundtrip() {
        let core_options = resolver::ResolveOptions::default();
        let wasm_options: ResolveOptions = core_options.clone().into();
        let mapped_core: resolver::ResolveOptions = wasm_options.into();

        assert_eq!(core_options.conditions, mapped_core.conditions);
        assert_eq!(core_options.extensions, mapped_core.extensions);
        assert_eq!(core_options.main_files, mapped_core.main_files);
        assert_eq!(core_options.modules, mapped_core.modules);
        assert_eq!(
            core_options.canonicalize_symlinks,
            mapped_core.canonicalize_symlinks
        );
    }
}
