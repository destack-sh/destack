use std::fmt::Debug;
use std::io;
use std::path::PathBuf;

/// Resolution error.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ResolveError {
    /// Path explicitly ignored.
    /// <https://github.com/defunctzombie/package-browser-field-spec#ignore-a-module>
    Ignored { path: PathBuf },

    /// Module not found.
    NotFound { specifier: String },

    /// Matched alias value not found.
    MatchedAliasNotFound {
        specifier: String,
        alias_key: String,
    },

    /// TypeScriptOptions not found.
    TsConfigNotFound { path: PathBuf },

    /// Invalid tsconfig.
    TsConfigInvalid { path: PathBuf },

    /// TypeScriptOptions's project reference path points to itself.
    TsConfigSelfReference { path: PathBuf },

    /// Circular tsconfig extends.
    TsConfigCircular { paths: Vec<PathBuf> },

    /// DsConfig not found.
    DsConfigNotFound { path: PathBuf },

    /// Invalid dsconfig.
    DsConfigInvalid { path: PathBuf },

    /// Circular dsconfig extends.
    DsConfigCircular { paths: Vec<PathBuf> },

    /// IO error.
    IoError { path: PathBuf, kind: io::ErrorKind },

    /// Path won't be able consumable by NodeJS `import` or `require`.
    UnsupportedPath { path: PathBuf },

    /// None of the aliased extensions were found.
    ExtensionAliasNotFound {
        filename: String,
        tried: String,
        dir: PathBuf,
    },

    /// Path specifier cannot be parsed.
    InvalidSpecifier {
        specifier: String,
        message: Option<String>,
    },

    /// Invalid module specifier (e.g. `#/`).
    InvalidModuleSpecifier {
        specifier: String,
        package_path: PathBuf,
    },

    /// Invalid package target (e.g. `../`).
    InvalidPackageTarget {
        target: String,
        name: String,
        package_path: PathBuf,
    },

    /// Package path not exported.
    PackagePathNotExported {
        subpath: String,
        package_path: PathBuf,
        package_json_path: PathBuf,
        conditions: Vec<String>,
    },

    /// Invalid package config.
    InvalidPackageJson { path: PathBuf },

    /// Invalid package config default.
    InvalidPackageConfigDefault { path: PathBuf },

    /// Invalid package config directory.
    InvalidPackageConfigDirectory { path: PathBuf },

    /// Package import not defined.
    PackageImportNotDefined {
        specifier: String,
        package_path: PathBuf,
    },

    /// Recursive or too deep dependency.
    RecursiveDependency { depth: u8 },
}

impl ResolveError {
    /// Check if the error is the ignored path error.
    pub const fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignored { .. })
    }

    /// Get the numeric sub-code of the error.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Ignored { .. } => 1,
            Self::NotFound { .. } => 2,
            Self::MatchedAliasNotFound { .. } => 3,
            Self::TsConfigNotFound { .. } => 4,
            Self::TsConfigInvalid { .. } => 5,
            Self::TsConfigSelfReference { .. } => 5,
            Self::TsConfigCircular { .. } => 6,
            Self::DsConfigNotFound { .. } => 21,
            Self::DsConfigInvalid { .. } => 22,
            Self::DsConfigCircular { .. } => 23,
            Self::IoError { .. } => 7,
            Self::UnsupportedPath { .. } => 8,
            Self::ExtensionAliasNotFound { .. } => 10,
            Self::InvalidSpecifier { .. } => 11,
            Self::InvalidModuleSpecifier { .. } => 13,
            Self::InvalidPackageTarget { .. } => 14,
            Self::PackagePathNotExported { .. } => 15,
            Self::InvalidPackageJson { .. } => 16,
            Self::InvalidPackageConfigDefault { .. } => 17,
            Self::InvalidPackageConfigDirectory { .. } => 18,
            Self::PackageImportNotDefined { .. } => 19,
            Self::RecursiveDependency { .. } => 20,
        }
    }

    /// Get the message string for this error.
    pub fn message(&self) -> String {
        match self {
            Self::Ignored { path } => format!("path is ignored {path:?}"),
            Self::NotFound { specifier } => format!("module '{specifier}' not found"),
            Self::MatchedAliasNotFound {
                specifier,
                alias_key,
            } => format!("module '{specifier}' not found for matched aliased key '{alias_key}'"),
            Self::TsConfigNotFound { path } => format!("tsconfig '{path:?}' not found"),
            Self::TsConfigInvalid { path } => format!("invalid tsconfig '{path:?}'"),
            Self::TsConfigSelfReference { path } => {
                format!("tsconfig's project reference path points to this tsconfig {path:?}")
            }
            Self::TsConfigCircular { paths } => {
                format!("tsconfig extends configs circularly: {paths:?}")
            }
            Self::DsConfigNotFound { path } => format!("dsconfig '{path:?}' not found"),
            Self::DsConfigInvalid { path } => format!("invalid dsconfig '{path:?}'"),
            Self::DsConfigCircular { paths } => {
                format!("dsconfig extends configs circularly: {paths:?}")
            }
            Self::IoError { path, kind } => format!("IO error at {path:?}: {kind}"),
            Self::UnsupportedPath { path } => {
                format!("path {path:?} contains unsupported construct.")
            }
            Self::ExtensionAliasNotFound {
                filename,
                tried,
                dir,
            } => {
                format!("cannot resolve '{filename}' for extension aliases '{tried}' in '{dir:?}'")
            }
            Self::InvalidSpecifier { specifier, message } => {
                if let Some(message) = message {
                    format!("invalid specifier '{specifier:?}': {message}")
                } else {
                    format!("invalid specifier '{specifier:?}'")
                }
            }
            Self::InvalidModuleSpecifier {
                specifier,
                package_path,
            } => format!(
                "module '{specifier}' specifier is not a valid subpath for the 'exports' resolution of {package_path:?}"
            ),
            Self::InvalidPackageTarget {
                target,
                name,
                package_path,
            } => format!(
                "invalid 'exports' target '{target}' defined for '{name}' in the package config {package_path:?}"
            ),
            Self::PackagePathNotExported {
                subpath,
                package_path,
                package_json_path,
                conditions,
            } => {
                let conditions_str = if conditions.is_empty() {
                    "<no conditions>".to_string()
                } else {
                    conditions
                        .iter()
                        .map(|s| format!("\"{s}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                format!(
                    "'{subpath}' is not exported under {conditions_str} from package {package_path:?} (see 'exports' field in {package_json_path:?})"
                )
            }
            Self::InvalidPackageJson { path } => format!(
                "invalid package config '{path:?}', 'exports' cannot contain some keys starting with '.' and some not. The exports object must either be an object of package subpath keys or an object of main entry condition name keys only."
            ),
            Self::InvalidPackageConfigDefault { path } => {
                format!("default condition should be last one in '{path:?}'")
            }
            Self::InvalidPackageConfigDirectory { path } => {
                format!("expecting folder to folder mapping. '{path:?}' should end with '/'")
            }
            Self::PackageImportNotDefined {
                specifier,
                package_path,
            } => format!(
                "package import specifier '{specifier}' is not defined in package {package_path:?}"
            ),
            Self::RecursiveDependency { depth } => {
                format!("recursion while resolving at depth {depth}.")
            }
        }
    }
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for ResolveError {}
