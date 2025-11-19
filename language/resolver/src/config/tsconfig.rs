use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::BuildHasherDefault;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;
use rustc_hash::FxHasher;
use serde::Deserialize;

use crate::resolve::TypeScriptOptionsReferences;
use dyst_source::{PathExt, strip_json};

const TEMPLATE_VARIABLE: &str = "${configDir}";

pub type CompilerOptionsPathsMap = IndexMap<String, Vec<String>, BuildHasherDefault<FxHasher>>;

/// TypeScript configuration (usually from `tsconfig.json`)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptOptions {
    /// Whether this is the caller tsconfig.
    /// Used for final template variable substitution when all configs are extended and merged.
    #[serde(skip)]
    pub is_root: bool,

    /// Path to `tsconfig.json`. Contains the `tsconfig.json` filename.
    #[serde(skip)]
    pub path: PathBuf,

    /// Specific files to include in the project.
    #[serde(default)]
    pub files: Option<Vec<String>>,

    /// Files to include in the project.
    #[serde(default)]
    pub include: Option<Vec<String>>,

    /// Files to exclude from the project.
    #[serde(default)]
    pub exclude: Option<Vec<String>>,

    /// Paths to other tsconfigs to extend.
    #[serde(default)]
    pub extends: Option<ExtendsField>,

    /// Compiler options.
    #[serde(default)]
    pub compiler_options: CompilerOptions,

    /// Bubbled up project references with a reference to their tsconfig.
    #[serde(default)]
    pub references: Vec<TypeScriptProjectReference>,
}

impl TypeScriptOptions {
    /// Directory of the `tsconfig.json` file.
    pub fn directory(&self) -> &Path {
        debug_assert!(self.path.file_name().is_some());
        self.path.parent().unwrap()
    }

    /// Returns any paths to tsconfigs that should be extended by this tsconfig.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
        let specifiers = match &self.extends {
            Some(ExtendsField::Single(specifier)) => {
                vec![specifier.as_str()]
            }
            Some(ExtendsField::Multiple(specifiers)) => {
                specifiers.iter().map(String::as_str).collect()
            }
            None => Vec::new(),
        };
        specifiers.into_iter()
    }

    /// Loads the given references into this tsconfig.
    ///
    /// Returns whether any references are defined in the tsconfig.
    pub(crate) fn load_references(&mut self, references: &TypeScriptOptionsReferences) -> bool {
        match references {
            TypeScriptOptionsReferences::Disabled => {
                self.references.drain(..);
            }
            TypeScriptOptionsReferences::Auto => {}
            TypeScriptOptionsReferences::Paths(paths) => {
                self.references = paths
                    .iter()
                    .map(|path| TypeScriptProjectReference {
                        path: path.clone(),
                        tsconfig: None,
                    })
                    .collect();
            }
        }

        !self.references.is_empty()
    }

    /// Returns the base path from which to resolve aliases.
    ///
    /// The base path can be configured by the user as part of the
    /// [CompilerOptions]. If not configured, it returns the directory in which
    /// the tsconfig itself is found.
    pub(crate) fn base_path(&self) -> &Path {
        self.compiler_options
            .base_url
            .as_deref()
            .unwrap_or_else(|| self.directory())
    }

    /// Inherits settings from the given tsconfig into `self`.
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    pub(crate) fn extend_tsconfig(&mut self, tsconfig: &Self) {
        if self.files.is_none()
            && let Some(files) = &tsconfig.files
        {
            self.files = Some(files.clone());
        }

        if self.include.is_none()
            && let Some(include) = &tsconfig.include
        {
            self.include = Some(include.clone());
        }

        if self.exclude.is_none()
            && let Some(exclude) = &tsconfig.exclude
        {
            self.exclude = Some(exclude.clone());
        }

        let tsconfig_dir = tsconfig.directory();
        let compiler_options = &mut self.compiler_options;

        if compiler_options.base_url.is_none()
            && let Some(base_url) = &tsconfig.compiler_options.base_url
        {
            compiler_options.base_url = Some(
                if base_url.to_string_lossy().starts_with(TEMPLATE_VARIABLE) {
                    base_url.clone()
                } else {
                    tsconfig_dir.join(base_url).normalize()
                },
            );
        }

        if compiler_options.paths.is_none() {
            let paths_base = compiler_options.base_url.as_ref().map_or_else(
                || tsconfig_dir.to_path_buf(),
                |path| {
                    if path.to_string_lossy().starts_with(TEMPLATE_VARIABLE) {
                        path.clone()
                    } else {
                        tsconfig_dir.join(path).normalize()
                    }
                },
            );
            compiler_options.paths_base = paths_base;
            compiler_options.paths = tsconfig.compiler_options.paths.clone();
        }

        if compiler_options.experimental_decorators.is_none()
            && let Some(experimental_decorators) = tsconfig.compiler_options.experimental_decorators
        {
            compiler_options.experimental_decorators = Some(experimental_decorators);
        }

        if compiler_options.emit_decorator_metadata.is_none()
            && let Some(emit_decorator_metadata) = tsconfig.compiler_options.emit_decorator_metadata
        {
            compiler_options.emit_decorator_metadata = Some(emit_decorator_metadata);
        }

        if compiler_options.use_define_for_class_fields.is_none()
            && let Some(use_define_for_class_fields) =
                tsconfig.compiler_options.use_define_for_class_fields
        {
            compiler_options.use_define_for_class_fields = Some(use_define_for_class_fields);
        }

        if compiler_options
            .rewrite_relative_import_extensions
            .is_none()
            && let Some(rewrite_relative_import_extensions) =
                tsconfig.compiler_options.rewrite_relative_import_extensions
        {
            compiler_options.rewrite_relative_import_extensions =
                Some(rewrite_relative_import_extensions);
        }

        if compiler_options.jsx.is_none()
            && let Some(jsx) = &tsconfig.compiler_options.jsx
        {
            compiler_options.jsx = Some(jsx.clone());
        }

        if compiler_options.jsx_factory.is_none()
            && let Some(jsx_factory) = &tsconfig.compiler_options.jsx_factory
        {
            compiler_options.jsx_factory = Some(jsx_factory.clone());
        }

        if compiler_options.jsx_fragment_factory.is_none()
            && let Some(jsx_fragment_factory) = &tsconfig.compiler_options.jsx_fragment_factory
        {
            compiler_options.jsx_fragment_factory = Some(jsx_fragment_factory.clone());
        }

        if compiler_options.jsx_import_source.is_none()
            && let Some(jsx_import_source) = &tsconfig.compiler_options.jsx_import_source
        {
            compiler_options.jsx_import_source = Some(jsx_import_source.clone());
        }

        if compiler_options.verbatim_module_syntax.is_none()
            && let Some(verbatim_module_syntax) = tsconfig.compiler_options.verbatim_module_syntax
        {
            compiler_options.verbatim_module_syntax = Some(verbatim_module_syntax);
        }

        if compiler_options.preserve_value_imports.is_none()
            && let Some(preserve_value_imports) = tsconfig.compiler_options.preserve_value_imports
        {
            compiler_options.preserve_value_imports = Some(preserve_value_imports);
        }

        if compiler_options.imports_not_used_as_values.is_none()
            && let Some(imports_not_used_as_values) =
                &tsconfig.compiler_options.imports_not_used_as_values
        {
            compiler_options.imports_not_used_as_values = Some(imports_not_used_as_values.clone());
        }

        if compiler_options.target.is_none()
            && let Some(target) = &tsconfig.compiler_options.target
        {
            compiler_options.target = Some(target.clone());
        }

        if compiler_options.module.is_none()
            && let Some(module) = &tsconfig.compiler_options.module
        {
            compiler_options.module = Some(module.clone());
        }

        if compiler_options.allow_js.is_none()
            && let Some(allow_js) = tsconfig.compiler_options.allow_js
        {
            compiler_options.allow_js = Some(allow_js);
        }
    }

    /// "Build" the root tsconfig, resolve:
    ///
    /// * `{configDir}` template variable
    /// * `paths_base` for resolving paths alias
    /// * `baseUrl` to absolute path
    pub(crate) fn build(mut self) -> Self {
        // Only the root tsconfig requires paths resolution.
        if !self.is_root {
            return self;
        }

        let config_dir = self.directory().to_path_buf();

        if let Some(base_url) = &self.compiler_options.base_url {
            // Substitute template variable in `tsconfig.compilerOptions.baseUrl`.
            let base_url = base_url
                .to_string_lossy()
                .strip_prefix(TEMPLATE_VARIABLE)
                .map_or_else(
                    || config_dir.normalize_with(base_url),
                    |stripped_path| config_dir.join(stripped_path.trim_start_matches('/')),
                );
            self.compiler_options.base_url = Some(base_url);
        }

        if self.compiler_options.paths.is_some() {
            // `paths_base` should use config dir if it is not resolved with base url nor extended
            // with another tsconfig.
            if let Some(base_url) = &self.compiler_options.base_url {
                self.compiler_options.paths_base = base_url.clone();
            }

            if self.compiler_options.paths_base.as_os_str().is_empty() {
                self.compiler_options.paths_base = config_dir.clone();
            }

            // Substitute template variable in `tsconfig.compilerOptions.paths`.
            for paths in self.compiler_options.paths.as_mut().unwrap().values_mut() {
                for path in paths {
                    Self::substitute_template_variable(&config_dir, path);
                }
            }
        }

        self
    }

    /// Template variable `${configDir}` for substitution of config files
    /// directory path.
    ///
    /// NOTE: All tests cases are just a head replacement of `${configDir}`, so
    ///       we are constrained as such.
    ///
    /// See <https://github.com/microsoft/TypeScript/pull/58042>.
    pub(crate) fn substitute_template_variable(directory: &Path, path: &mut String) {
        if let Some(stripped_path) = path.strip_prefix(TEMPLATE_VARIABLE) {
            *path = directory
                .join(stripped_path.trim_start_matches('/'))
                .to_string_lossy()
                .to_string();
        }
    }

    /// Resolves the given `specifier` within the project configured by this
    /// tsconfig, relative to the given `path`.
    ///
    /// `specifier` can be either a real path or an alias.
    pub(crate) fn resolve(&self, path: &Path, specifier: &str) -> Vec<PathBuf> {
        let paths = self.resolve_path_alias(specifier);
        for tsconfig in self
            .references
            .iter()
            .filter_map(TypeScriptProjectReference::tsconfig)
        {
            if path.starts_with(tsconfig.base_path()) {
                return [tsconfig.resolve_path_alias(specifier), paths].concat();
            }
        }
        paths
    }

    /// Resolves the given `specifier` within the project configured by this tsconfig.
    // <https://github.com/parcel-bundler/parcel/blob/b6224fd519f95e68d8b93ba90376fd94c8b76e69/packages/utils/node-resolver-rs/src/tsconfig.rs#L93>
    pub(crate) fn resolve_path_alias(&self, specifier: &str) -> Vec<PathBuf> {
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
            .map(|p| compiler_options.paths_base.normalize_with(p))
            .chain(base_url_iter)
            .collect()
    }
}

/// Compiler Options
/// <https://www.typescriptlang.org/tsconfig#compilerOptions>
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptions {
    /// Base URL (e.g. `./src`)
    /// <https://www.typescriptlang.org/tsconfig/#baseUrl>
    pub base_url: Option<PathBuf>,

    /// Path aliases (e.g. `{ "src/*": ["src/*"] }`)
    /// <https://www.typescriptlang.org/tsconfig/#paths>
    pub paths: Option<CompilerOptionsPathsMap>,

    /// The actual base from where path aliases are resolved.
    #[serde(skip)]
    pub paths_base: PathBuf,

    /// Experimental decorators (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#experimentalDecorators>
    pub experimental_decorators: Option<bool>,

    /// Emit decorator metadata (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#emitDecoratorMetadata>
    pub emit_decorator_metadata: Option<bool>,

    /// Use define for class fields (e.g. `true`)
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

    /// Module (e.g. `"ESNext"`)
    /// <https://www.typescriptlang.org/tsconfig/#module>
    pub module: Option<String>,

    /// Allow JavaScript files (e.g. `true`)
    /// <https://www.typescriptlang.org/tsconfig/#allowJs>
    pub allow_js: Option<bool>,

    /// Type roots (e.g. `["src/types"]`)
    /// <https://www.typescriptlang.org/tsconfig/#typeRoots>
    pub type_roots: Option<Vec<String>>,

    /// Types (e.g. `["node"]`)
    /// <https://www.typescriptlang.org/tsconfig/#types>
    pub types: Option<Vec<String>>,
}

/// Value for the "extends" field.
///
/// <https://www.typescriptlang.org/tsconfig/#extends>
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum ExtendsField {
    Single(String),
    Multiple(Vec<String>),
}

/// Project Reference
///
/// <https://www.typescriptlang.org/docs/handbook/project-references.html>
#[derive(Debug, Deserialize)]
pub struct TypeScriptProjectReference {
    /// Path to the tsconfig.json file.
    pub path: PathBuf,

    /// Resolved tsconfig.
    #[serde(skip)]
    pub tsconfig: Option<Arc<TypeScriptOptions>>,
}

impl TypeScriptProjectReference {
    /// Returns the path to the tsconfig.json file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the resolved tsconfig.
    pub fn tsconfig(&self) -> Option<Arc<TypeScriptOptions>> {
        self.tsconfig.clone()
    }

    /// Sets the resolved tsconfig.
    pub(crate) fn set_tsconfig(&mut self, tsconfig: Arc<TypeScriptOptions>) {
        self.tsconfig.replace(tsconfig);
    }
}

impl TypeScriptOptions {
    /// Parses the tsconfig from a JSON string.
    pub fn parse(root: bool, path: &Path, json: &mut str) -> Result<Self, serde_json::Error> {
        let json = trim_start_matches_mut(json, '\u{feff}'); // strip bom
        let stripped = strip_json(json).map_err(serde_json::Error::io)?;
        let json = if stripped.trim().is_empty() {
            Cow::Borrowed("{}")
        } else {
            Cow::Owned(stripped)
        };
        let mut tsconfig: Self = serde_json::from_str(json.as_ref())?;
        tsconfig.is_root = root;
        tsconfig.path = path.to_path_buf();
        Ok(tsconfig)
    }
}

/// Trims the start of a string if it starts with the given character.
fn trim_start_matches_mut(string: &mut str, pattern: char) -> &mut str {
    if string.starts_with(pattern) {
        // trim the prefix
        &mut string[pattern.len_utf8()..]
    } else {
        string
    }
}
