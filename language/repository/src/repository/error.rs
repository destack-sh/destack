use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use tspp_artifact::{ArtifactCacheError, ArtifactKey, ArtifactVersion};
use tspp_source::{FileId, ModuleId, PackageId, ProfileId, TargetId};

use crate::repository::Revision;

/// One error raised by repository operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    /// One narrowed module id collided across two distinct modules.
    ModuleIdCollision {
        /// The colliding module id.
        id: ModuleId,
        /// The module already registered under the id.
        left: String,
        /// The module that collided with it.
        right: String,
    },
    /// One mount name was registered with two different bases.
    MountConflict {
        /// The conflicting mount name.
        name: String,
        /// The base already registered.
        existing: PathBuf,
        /// The base that failed to register.
        base: PathBuf,
    },

    /// The requested revision does not exist.
    MissingRevision { revision: Revision },
    /// The repository BlobStore failed.
    Blob { message: String },
    /// One loaded File is invalid.
    InvalidFile {
        /// The invalid File.
        file: FileId,
        /// The validation failure.
        message: String,
    },
    /// The requested module does not exist in the given revision.
    MissingModule { module: ModuleId },
    /// The requested package does not exist in the given revision.
    MissingPackage { package: PackageId },
    /// One package names a dependency the revision holds no package for.
    UnresolvedDependency {
        /// The depending package.
        package: PackageId,
        /// The dependency name as declared.
        name: String,
    },
    /// The requested package has no file system path.
    MissingPackagePath { package: PackageId },
    /// One package root has no `destack.json` declaration.
    MissingPackageConfig { path: PathBuf },
    /// One package declaration has no name.
    MissingPackageName { path: PathBuf },
    /// Multiple packages declare the same package name.
    DuplicatePackageName { name: String },
    /// One builtin module lies outside the canonical source directory.
    BuiltinModuleOutsideSourceDirectory {
        /// The module path.
        path: PathBuf,
        /// The canonical source directory.
        source_directory: PathBuf,
    },
    /// The requested target does not exist in the given revision.
    MissingTarget { target: TargetId },
    /// The requested target belongs to a different package than the module.
    TargetPackageMismatch {
        /// The requested module.
        module: ModuleId,
        /// The requested target.
        target: TargetId,
    },
    /// The requested module path does not resolve to a package module.
    MissingModulePath { path: PathBuf },
    /// The requested product does not exist in the given revision.
    MissingProduct { product: String },
    /// The requested product does not contain the selected target.
    MissingProductTarget { product: String, target: String },
    /// The requested product target matches multiple product roles.
    AmbiguousProductTarget { product: String, target: String },
    /// The requested profile does not exist in the repository.
    MissingProfile { profile: ProfileId },
    /// The requested artifact result is not live.
    MissingArtifact { version: ArtifactVersion },
    /// Artifact dependencies form a cycle.
    CircularArtifactDependency { key: ArtifactKey },
    /// Artifact state violates an internal invariant.
    InvalidArtifact { message: String },
    /// One persistent cache operation failed.
    Cache {
        /// The underlying cache failure.
        source: Box<ArtifactCacheError>,
    },
    /// The requested file does not exist in the base revision.
    MissingFile { path: String },
    /// The requested file already exists in the base revision.
    FileAlreadyExists { path: String },
    /// The requested edit path is not writable through generic repository edits.
    InvalidEditPath { path: String, message: String },
    /// Repository config is invalid.
    InvalidConfig {
        /// The config file id.
        file: FileId,
        /// The parse error message.
        message: String,
    },
    /// One physical `destack.json` file is invalid.
    InvalidConfigFile {
        /// The invalid config path.
        path: PathBuf,
        /// The parse error message.
        message: String,
    },
    /// A `destack.json` inheritance chain is cyclic.
    ConfigCycle {
        /// The config path that repeated.
        path: PathBuf,
    },
    /// A `destack.json` inheritance specifier is invalid.
    InvalidConfigExtends {
        /// The config file id.
        file: FileId,
        /// The invalid specifier.
        specifier: String,
    },
    /// One attached file system operation failed.
    FileSystem {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
    /// One physical path lies outside its required root.
    PathOutsideRoot {
        /// The path outside the root.
        path: PathBuf,
        /// The required containing root.
        root: PathBuf,
    },
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModuleIdCollision { id, left, right } => {
                write!(
                    formatter,
                    "module id {id:?} collides between '{left}' and '{right}'"
                )
            }
            Self::MountConflict {
                name,
                existing,
                base,
            } => {
                write!(
                    formatter,
                    "dependency mount '{name}' maps to {existing:?} and {base:?}"
                )
            }
            Self::MissingRevision { revision } => {
                write!(formatter, "missing repository revision '{revision}'")
            }
            Self::Blob { message } => {
                write!(formatter, "repository BlobStore failed: {message}")
            }
            Self::InvalidFile { file, message } => {
                write!(formatter, "invalid repository File '{file}': {message}")
            }
            Self::MissingModule { module } => {
                write!(formatter, "missing repository module '{module}'")
            }
            Self::MissingPackage { package } => {
                write!(formatter, "missing repository package '{package}'")
            }
            Self::UnresolvedDependency { package, name } => {
                write!(
                    formatter,
                    "package '{package}' names a dependency '{name}' the revision holds no package for"
                )
            }
            Self::MissingPackagePath { package } => {
                write!(formatter, "missing repository package path for '{package}'")
            }
            Self::MissingPackageConfig { path } => {
                write!(formatter, "missing package config '{}'", path.display())
            }
            Self::MissingPackageName { path } => {
                write!(
                    formatter,
                    "package config '{}' has no package name",
                    path.display()
                )
            }
            Self::DuplicatePackageName { name } => {
                write!(formatter, "duplicate package name '{name}'")
            }
            Self::BuiltinModuleOutsideSourceDirectory {
                path,
                source_directory,
            } => {
                write!(
                    formatter,
                    "builtin module '{}' lies outside source directory '{}'",
                    path.display(),
                    source_directory.display()
                )
            }
            Self::MissingTarget { target } => {
                write!(formatter, "missing repository target '{target}'")
            }
            Self::TargetPackageMismatch { module, target } => {
                write!(
                    formatter,
                    "repository target '{target}' does not apply to module '{module}'"
                )
            }
            Self::MissingModulePath { path } => {
                write!(
                    formatter,
                    "missing repository module path '{}'",
                    path.display()
                )
            }
            Self::MissingProduct { product } => {
                write!(formatter, "missing repository product '{product}'")
            }
            Self::MissingProductTarget { product, target } => {
                write!(
                    formatter,
                    "product '{product}' does not contain target '{target}'"
                )
            }
            Self::AmbiguousProductTarget { product, target } => {
                write!(
                    formatter,
                    "product '{product}' matches target '{target}' through multiple roles"
                )
            }
            Self::MissingProfile { profile } => {
                write!(formatter, "missing repository profile '{profile}'")
            }
            Self::MissingArtifact { version } => {
                write!(formatter, "missing repository artifact '{version:?}'")
            }
            Self::CircularArtifactDependency { key } => {
                write!(
                    formatter,
                    "circular repository artifact dependency at '{key:?}'"
                )
            }
            Self::InvalidArtifact { message } => {
                write!(formatter, "invalid repository artifact state: {message}")
            }
            Self::Cache { source } => {
                write!(formatter, "repository cache failed: {source}")
            }
            Self::MissingFile { path } => {
                write!(formatter, "missing file '{path}'")
            }
            Self::FileAlreadyExists { path } => {
                write!(formatter, "file already exists '{path}'")
            }
            Self::InvalidEditPath { path, message } => {
                write!(
                    formatter,
                    "invalid repository edit path '{path}': {message}"
                )
            }
            Self::InvalidConfig { file, message } => {
                write!(
                    formatter,
                    "invalid repository config for '{file}': {message}"
                )
            }
            Self::InvalidConfigFile { path, message } => {
                write!(
                    formatter,
                    "invalid repository config file '{}': {message}",
                    path.display()
                )
            }
            Self::ConfigCycle { path } => {
                write!(
                    formatter,
                    "circular destack config inheritance for '{}'",
                    path.display()
                )
            }
            Self::InvalidConfigExtends { file, specifier } => {
                write!(
                    formatter,
                    "invalid destack config inheritance specifier '{specifier}' for '{file}'"
                )
            }
            Self::FileSystem {
                operation,
                path,
                message,
            } => {
                write!(
                    formatter,
                    "repository file system operation failed during {operation} for '{}': {message}",
                    path.display()
                )
            }
            Self::PathOutsideRoot { path, root } => {
                write!(
                    formatter,
                    "repository path '{}' lies outside root '{}'",
                    path.display(),
                    root.display()
                )
            }
        }
    }
}

impl Error for RepositoryError {
    /// Return the underlying cache error when present.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Cache { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}

impl From<ArtifactCacheError> for RepositoryError {
    /// Preserve one persistent cache failure.
    fn from(error: ArtifactCacheError) -> Self {
        Self::Cache {
            source: Box::new(error),
        }
    }
}
