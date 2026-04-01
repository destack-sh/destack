use std::path::{Path, PathBuf};

use serde::Deserialize;

use destack_source::PathExt;

use super::compiler::{TsCompilerOptions, TsCompilerOptionsJson};

/// Project Reference
///
/// <https://www.typescriptlang.org/docs/handbook/project-references.html>
#[derive(Debug, Deserialize, Clone)]
pub struct TsConfigProjectReferences {
    /// Path to the tsconfig.json file (relative to containing tsconfig).
    pub path: PathBuf,
}

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
    pub compiler: TsCompilerOptions,
}

impl From<&TsConfigJson> for TsConfigOptions {
    fn from(json: &TsConfigJson) -> Self {
        Self {
            files: json.files.clone().unwrap_or_default(),
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            compiler: TsCompilerOptions::from(&json.compiler_options),
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
    pub compiler_options: TsCompilerOptionsJson,
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
    pub(crate) fn resolve_path_alias(&self, specifier: &str, paths_base: &Path) -> Vec<PathBuf> {
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
