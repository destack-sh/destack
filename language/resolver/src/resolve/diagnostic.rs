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
    Ignored(PathBuf),

    /// Module not found.
    NotFound(/* specifier */ String),

    /// Matched alias value not found
    MatchedAliasNotFound(/* specifier */ String, /* alias key */ String),

    /// Tsconfig not found
    TsConfigNotFound(PathBuf),

    /// Tsconfig's project reference path points to itself
    TsConfigSelfReference(PathBuf),

    /// Occurs when tsconfig extends configs circularly
    TsConfigCircular(CircularPathBufs),

    IOError(IOError),

    /// Indicates the resulting path won't be able consumable by NodeJS `import` or `require`.
    /// For example, DOS device path with Volume GUID (`\\?\Volume{...}`) is not supported.
    PathNotSupported(PathBuf),

    /// Node.js builtin module when `Options::builtin_modules` is enabled.
    ///
    /// `is_runtime_module` can be used to determine whether the request
    /// was prefixed with `node:` or not.
    ///
    /// `resolved` is always prefixed with "node:" in compliance with the ESM specification.
    Builtin {
        resolved: String,
        is_runtime_module: bool,
    },

    /// All of the aliased extension are not found
    ///
    /// Displays `Cannot resolve 'index.mjs' with extension aliases 'index.mts' in ...`
    ExtensionAlias(
        /* File name */ String,
        /* Tried file names */ String,
        /* Path to dir */ PathBuf,
    ),

    /// The provided path specifier cannot be parsed
    Specifier(SpecifierError),

    /// JSON parse error
    Json(JSONError),

    /// Invalid module specifier.
    InvalidModuleSpecifier(String, PathBuf),

    /// Invalid package target.
    InvalidPackageTarget(String, String, PathBuf),

    /// Package path not exported.
    PackagePathNotExported {
        subpath: String,
        package_path: PathBuf,
        package_json_path: PathBuf,
        conditions: ConditionNames,
    },

    /// Invalid package config.
    InvalidPackageConfig(PathBuf),

    /// Invalid package config default.
    InvalidPackageConfigDefault(PathBuf),

    /// Invalid package config directory.
    InvalidPackageConfigDirectory(PathBuf),

    /// Package import not defined.
    PackageImportNotDefined(String, PathBuf),

    /// Occurs when alias paths reference each other.
    Recursion,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::Ignored(path) => write!(f, "Path is ignored {path:?}"),
            ResolveError::NotFound(spec) => write!(f, "Module '{spec}' not found"),
            ResolveError::MatchedAliasNotFound(spec, key) => write!(
                f,
                "Module '{spec}' not found for matched aliased key '{key}'"
            ),
            ResolveError::TsConfigNotFound(path) => write!(f, "Tsconfig '{path:?}' not found"),
            ResolveError::TsConfigSelfReference(path) => write!(
                f,
                "Tsconfig's project reference path points to this tsconfig {path:?}"
            ),
            ResolveError::TsConfigCircular(paths) => {
                write!(f, "Tsconfig extends configs circularly: {paths:?}")
            }
            ResolveError::IOError(err) => write!(f, "{err}"),
            ResolveError::PathNotSupported(path) => {
                write!(f, "Path {path:?} contains unsupported construct.")
            }
            ResolveError::Builtin {
                resolved,
                is_runtime_module: _,
            } => write!(f, "Builtin module {resolved}"),
            ResolveError::ExtensionAlias(filename, tried, dir) => write!(
                f,
                "Cannot resolve '{filename}' for extension aliases '{tried}' in '{dir:?}'"
            ),
            ResolveError::Specifier(e) => write!(f, "{e}"),
            ResolveError::Json(e) => write!(f, "{e:?}"),
            ResolveError::InvalidModuleSpecifier(spec, pkg) => write!(
                f,
                "Invalid module \"{spec}\" specifier is not a valid subpath for the \"exports\" resolution of {pkg:?}"
            ),
            ResolveError::InvalidPackageTarget(target, name, path) => write!(
                f,
                "Invalid \"exports\" target \"{target}\" defined for '{name}' in the package config {path:?}"
            ),
            ResolveError::PackagePathNotExported {
                subpath,
                package_path,
                package_json_path,
                conditions,
            } => write!(
                f,
                "\"{subpath}\" is not exported under {conditions} from package {package_path:?} (see exports field in {package_json_path:?})"
            ),
            ResolveError::InvalidPackageConfig(path) => write!(
                f,
                "Invalid package config \"{path:?}\", \"exports\" cannot contain some keys starting with '.' and some not. The exports object must either be an object of package subpath keys or an object of main entry condition name keys only."
            ),
            ResolveError::InvalidPackageConfigDefault(path) => {
                write!(f, "Default condition should be last one in \"{path:?}\"")
            }
            ResolveError::InvalidPackageConfigDirectory(path) => write!(
                f,
                "Expecting folder to folder mapping. \"{path:?}\" should end with \"/\""
            ),
            ResolveError::PackageImportNotDefined(spec, path) => write!(
                f,
                "Package import specifier \"{spec}\" is not defined in package {path:?}"
            ),
            ResolveError::Recursion => write!(f, "Recursion in resolving"),
        }
    }
}

impl std::error::Error for ResolveError {}

impl ResolveError {
    pub const fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignored(_))
    }
}

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

/// JSON error from [serde_json::Error]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JSONError {
    pub path: PathBuf,
    pub message: String,
    pub line: usize,
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
        Self::IOError(IOError(Arc::new(err)))
    }
}

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

/// Helper type for formatting condition names in error messages
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
