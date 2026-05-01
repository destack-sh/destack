use std::fmt::Debug;
use std::io;
use std::path::PathBuf;

/// Resolution error.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ResolverError {
    /// Path explicitly ignored.
    /// <https://github.com/defunctzombie/package-browser-field-spec#ignore-a-module>
    Ignored {
        /// The ignored path.
        path: PathBuf,
    },

    /// Module not found.
    NotFound {
        /// The original import specifier.
        specifier: String,
    },

    /// Matched alias value not found.
    MatchedAliasNotFound {
        /// The original import specifier.
        specifier: String,
        /// The alias key that matched.
        alias_key: String,
    },

    /// TypeScript configuration not found.
    TsConfigNotFound {
        /// The missing config path.
        path: PathBuf,
    },

    /// Invalid TypeScript configuration.
    TsConfigInvalid {
        /// The invalid config path.
        path: PathBuf,
    },

    /// TypeScript project reference path points to itself.
    TsConfigSelfReference {
        /// The offending config path.
        path: PathBuf,
    },

    /// Circular tsconfig extends.
    TsConfigCircular {
        /// The cycle of config paths.
        paths: Vec<PathBuf>,
    },

    /// Destack not found.
    DestackNotFound {
        /// The missing config path.
        path: PathBuf,
    },

    /// Invalid Destack config.
    DestackInvalid {
        /// The invalid config path.
        path: PathBuf,
    },

    /// Circular Destack config extends.
    DestackCircular {
        /// The cycle of config paths.
        paths: Vec<PathBuf>,
    },

    /// Yarn PnP manifest file was not found from the configured cwd.
    #[cfg(not(target_arch = "wasm32"))]
    FailedToFindYarnPnpManifest {
        /// The working directory used for the search.
        cwd: PathBuf,
    },

    /// Yarn PnP returned one resolver error.
    #[cfg(not(target_arch = "wasm32"))]
    YarnPnpError {
        /// The underlying Yarn PnP error.
        error: pnp::Error,
    },

    /// IO error.
    IoError {
        /// The path involved in the IO failure.
        path: PathBuf,
        /// The underlying IO error kind.
        kind: io::ErrorKind,
    },

    /// Repository backed resolution failed.
    RepositoryError {
        /// The repository operation context path.
        path: PathBuf,
        /// The underlying repository error message.
        message: String,
    },

    /// File base resolution received a non file path.
    ExpectedFilePath {
        /// The path that was expected to be a file.
        path: PathBuf,
    },

    /// Directory base resolution received a non directory path.
    ExpectedDirectoryPath {
        /// The path that was expected to be a directory.
        path: PathBuf,
    },

    /// Path won't be able consumable by NodeJS `import` or `require`.
    UnsupportedPath {
        /// The unsupported path.
        path: PathBuf,
    },

    /// None of the aliased extensions were found.
    ExtensionAliasNotFound {
        /// The original requested filename.
        filename: String,
        /// The alias extensions that were tried.
        tried: String,
        /// The directory in which probing happened.
        dir: PathBuf,
    },

    /// Path specifier cannot be parsed.
    InvalidSpecifier {
        /// The original specifier text.
        specifier: String,
        /// The optional parse error detail.
        message: Option<String>,
    },

    /// Invalid module specifier (e.g. `#/`).
    InvalidModuleSpecifier {
        /// The invalid specifier.
        specifier: String,
        /// The package config path involved in validation.
        package_path: PathBuf,
    },

    /// Invalid package target (e.g. `../`).
    InvalidPackageTarget {
        /// The invalid target value.
        target: String,
        /// The matching exports or imports key.
        name: String,
        /// The package config path involved in validation.
        package_path: PathBuf,
    },

    /// Package path not exported.
    PackagePathNotExported {
        /// The requested package subpath.
        subpath: String,
        /// The owning package directory.
        package_path: PathBuf,
        /// The package config path.
        package_json_path: PathBuf,
        /// The active condition names.
        conditions: Vec<String>,
    },

    /// Invalid package config.
    InvalidPackageJson {
        /// The invalid package config path.
        path: PathBuf,
    },

    /// Invalid package config default.
    InvalidPackageConfigDefault {
        /// The invalid package config path.
        path: PathBuf,
    },

    /// Invalid package config directory.
    InvalidPackageConfigDirectory {
        /// The invalid package config path.
        path: PathBuf,
    },

    /// Package import not defined.
    PackageImportNotDefined {
        /// The missing package import specifier.
        specifier: String,
        /// The package config path.
        package_path: PathBuf,
    },

    /// Recursive or too deep dependency.
    RecursiveDependency {
        /// The recursion depth at failure.
        depth: u8,
    },
}

/// Resolver result.
pub type ResolverResult<T> = Result<T, ResolverError>;

impl ResolverError {
    /// Check if the error is the ignored path error.
    pub const fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignored { .. })
    }

    /// Get the numeric subcode of the error.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Ignored { .. } => 1,
            Self::NotFound { .. } => 2,
            Self::MatchedAliasNotFound { .. } => 3,
            Self::TsConfigNotFound { .. } => 4,
            Self::TsConfigInvalid { .. } => 5,
            Self::TsConfigSelfReference { .. } => 5,
            Self::TsConfigCircular { .. } => 6,
            Self::DestackNotFound { .. } => 21,
            Self::DestackInvalid { .. } => 22,
            Self::DestackCircular { .. } => 23,
            #[cfg(not(target_arch = "wasm32"))]
            Self::FailedToFindYarnPnpManifest { .. } => 24,
            #[cfg(not(target_arch = "wasm32"))]
            Self::YarnPnpError { .. } => 25,
            Self::IoError { .. } => 7,
            Self::ExpectedFilePath { .. } => 9,
            Self::ExpectedDirectoryPath { .. } => 12,
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
            Self::RepositoryError { .. } => 26,
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
            Self::DestackNotFound { path } => format!("destack config '{path:?}' not found"),
            Self::DestackInvalid { path } => format!("invalid destack config '{path:?}'"),
            Self::DestackCircular { paths } => {
                format!("destack config extends configs circularly: {paths:?}")
            }
            #[cfg(not(target_arch = "wasm32"))]
            Self::FailedToFindYarnPnpManifest { cwd } => {
                format!("failed to find yarn pnp manifest in {cwd:?}")
            }
            #[cfg(not(target_arch = "wasm32"))]
            Self::YarnPnpError { error } => format!("yarn pnp error: {error}"),
            Self::IoError { path, kind } => format!("IO error at {path:?}: {kind}"),
            Self::RepositoryError { path, message } => {
                format!("repository resolution error at {path:?}: {message}")
            }
            Self::ExpectedFilePath { path } => {
                format!("expected a file path for file base resolution, got {path:?}")
            }
            Self::ExpectedDirectoryPath { path } => {
                format!("expected a directory path for directory base resolution, got {path:?}")
            }
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

    /// Return true when this error means one candidate missed and resolution can try the next.
    pub const fn is_alternative_candidate_miss(&self) -> bool {
        matches!(
            self,
            Self::InvalidPackageTarget { .. }
                | Self::NotFound { .. }
                | Self::MatchedAliasNotFound { .. }
                | Self::PackagePathNotExported { .. }
                | Self::PackageImportNotDefined { .. }
        )
    }
}

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for ResolverError {}
