use std::fmt::{self, Debug, Display};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

/// Resolution error.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ResolveError {
    /// Ignored path.
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
    TypeScriptOptionsNotFound { path: PathBuf },

    /// TypeScriptOptions's project reference path points to itself.
    TypeScriptOptionsSelfReference { path: PathBuf },

    /// TypeScriptOptions extends configs circularly.
    TypeScriptOptionsCircular { paths: CircularPathBufs },

    /// IO error.
    IOError { error: IOError },

    /// Path won't be able consumable by NodeJS `import` or `require`.
    PathNotSupported { path: PathBuf },

    /// Builtin module (that therefore cannot be resolved to a file on the file system).
    Builtin {
        /// Resolved path including the prefix (e.g. "node:").
        resolved: String,
        is_runtime_module: bool,
    },

    /// None of the aliased extensions were found.
    ExtensionAlias {
        filename: String,
        tried: String,
        dir: PathBuf,
    },

    /// Path specifier cannot be parsed.
    Specifier { error: SpecifierError },

    /// JSON parse error.
    Json { error: JSONError },

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
        conditions: ConditionNames,
    },

    /// Invalid package config.
    PackageJsonInvalid { path: PathBuf },

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
            Self::TypeScriptOptionsNotFound { .. } => 4,
            Self::TypeScriptOptionsSelfReference { .. } => 5,
            Self::TypeScriptOptionsCircular { .. } => 6,
            Self::IOError { .. } => 7,
            Self::PathNotSupported { .. } => 8,
            Self::Builtin { .. } => 9,
            Self::ExtensionAlias { .. } => 10,
            Self::Specifier { .. } => 11,
            Self::Json { .. } => 12,
            Self::InvalidModuleSpecifier { .. } => 13,
            Self::InvalidPackageTarget { .. } => 14,
            Self::PackagePathNotExported { .. } => 15,
            Self::PackageJsonInvalid { .. } => 16,
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
            Self::TypeScriptOptionsNotFound { path } => format!("tsconfig '{path:?}' not found"),
            Self::TypeScriptOptionsSelfReference { path } => {
                format!("tsconfig's project reference path points to this tsconfig {path:?}")
            }
            Self::TypeScriptOptionsCircular { paths } => {
                format!("tsconfig extends configs circularly: {paths:?}")
            }
            Self::IOError { error } => format!("{error}"),
            Self::PathNotSupported { path } => {
                format!("path {path:?} contains unsupported construct.")
            }
            Self::Builtin {
                resolved,
                is_runtime_module: _,
            } => format!("builtin module '{resolved}' does not exist as a file"),
            Self::ExtensionAlias {
                filename,
                tried,
                dir,
            } => {
                format!("cannot resolve '{filename}' for extension aliases '{tried}' in '{dir:?}'")
            }
            Self::Specifier { error } => format!("{error}"),
            Self::Json { error } => format!("{error:?}"),
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
            } => format!(
                "'{subpath}' is not exported under {conditions} from package {package_path:?} (see exports field in {package_json_path:?})"
            ),
            Self::PackageJsonInvalid { path } => format!(
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

/// Error for [ResolveError::Specifier]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SpecifierError {
    Empty(String),
}

impl std::fmt::Display for SpecifierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecifierError::Empty(spec) => write!(
                f,
                "The specifiers must be a non-empty string. Received \"{spec}\""
            ),
        }
    }
}

/// JSON parse error.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JSONError {
    /// Path to the JSON file.
    pub path: PathBuf,
    /// Error message.
    pub message: String,
    /// Line number.
    pub line: usize,
    /// Column number.
    pub column: usize,
}

impl std::fmt::Display for JSONError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JSON parse error: {:?}:{}:{}: {}",
            self.path, self.line, self.column, self.message
        )
    }
}

/// IO error.
#[derive(Debug, Clone)]
pub struct IOError(Arc<io::Error>);

impl std::fmt::Display for IOError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IO error: {:?}", self.0)
    }
}

impl PartialEq for IOError {
    fn eq(&self, other: &Self) -> bool {
        self.0.kind() == other.0.kind()
    }
}

impl From<IOError> for io::Error {
    #[cold]
    fn from(error: IOError) -> Self {
        let io_error = error.0.as_ref();
        Self::new(io_error.kind(), io_error.to_string())
    }
}

impl From<io::Error> for ResolveError {
    #[cold]
    fn from(err: io::Error) -> Self {
        Self::IOError {
            error: IOError(Arc::new(err)),
        }
    }
}

/// Circular path buffers (for displaying tsconfig circular references).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircularPathBufs(Vec<PathBuf>);

impl Display for CircularPathBufs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, path) in self.0.iter().enumerate() {
            if i != 0 {
                write!(f, " -> ")?;
            }
            path.fmt(f)?;
        }
        Ok(())
    }
}

impl From<Vec<PathBuf>> for CircularPathBufs {
    #[cold]
    fn from(value: Vec<PathBuf>) -> Self {
        Self(value)
    }
}

/// Condition names (for formatting condition names in error messages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionNames(Vec<String>);

impl From<Vec<String>> for ConditionNames {
    fn from(conditions: Vec<String>) -> Self {
        Self(conditions)
    }
}

impl Display for ConditionNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.len() {
            0 => write!(f, "no conditions"),
            1 => write!(f, "the condition \"{}\"", self.0[0]),
            _ => {
                write!(f, "the conditions ")?;
                let conditions_str = self
                    .0
                    .iter()
                    .map(|s| format!("\"{s}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "[{conditions_str}]")
            }
        }
    }
}
