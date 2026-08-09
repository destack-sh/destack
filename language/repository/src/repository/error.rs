use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use destack_artifact::{ArtifactBindingId, ArtifactKey, ArtifactVersion};
use destack_source::{ContentId, File, FileId, ModuleId, PackageId, ProfileId, TargetId};

use crate::repository::{Ref, Revision};

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

    /// The requested ref does not exist.
    MissingRef { reference: Ref },
    /// One ref moved away from the revision expected by its writer.
    RefChanged {
        /// The ref that moved.
        reference: Ref,
        /// The revision expected by the writer.
        expected: Revision,
        /// The current ref revision.
        current: Revision,
    },
    /// The requested revision does not exist.
    MissingRevision { revision: Revision },
    /// The requested content payload does not exist.
    MissingContent { content: ContentId },
    /// One content payload exceeds the source coordinate range.
    ContentTooLarge { length: usize },
    /// The repository content store failed.
    ContentStore { message: String },
    /// The repository artifact store failed.
    ArtifactStore { message: String },
    /// The requested module does not exist in the given revision.
    MissingModule { module: ModuleId },
    /// The requested package does not exist in the given revision.
    MissingPackage { package: PackageId },
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
    /// The requested artifact entry does not exist in the repository store.
    MissingArtifact { version: ArtifactVersion },
    /// The requested artifact binding does not exist in the repository table.
    MissingArtifactBindingId { binding: ArtifactBindingId },
    /// The artifact table does not contain one required dense artifact id.
    MissingArtifactId { key: ArtifactKey },
    /// An artifact binding does not contain one recorded dependency ordinal.
    MissingArtifactDependency {
        /// The artifact binding.
        key: ArtifactKey,
        /// The missing dependency ordinal.
        dependency: usize,
    },
    /// Artifact bindings form a dependency cycle.
    CircularArtifactBinding { key: ArtifactKey },
    /// Artifact state violates an internal invariant.
    InvalidArtifact { message: String },
    /// Artifact resolution exhausted its commit attempts under contention.
    ContendedResolution { detail: String },
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
            Self::MissingRef { reference } => {
                write!(formatter, "missing repository ref '{reference}'")
            }
            Self::RefChanged {
                reference,
                expected,
                current,
            } => {
                write!(
                    formatter,
                    "repository ref '{reference}' changed from {expected} to {current}"
                )
            }
            Self::MissingRevision { revision } => {
                write!(formatter, "missing repository revision '{revision}'")
            }
            Self::MissingContent { content } => {
                write!(formatter, "missing repository content '{content}'")
            }
            Self::ContentTooLarge { length } => write!(
                formatter,
                "content is {length} bytes, maximum is {}",
                File::MAX_BYTES
            ),
            Self::ContentStore { message } => {
                write!(formatter, "repository content store failed: {message}")
            }
            Self::ArtifactStore { message } => {
                write!(formatter, "repository artifact store failed: {message}")
            }
            Self::MissingModule { module } => {
                write!(formatter, "missing repository module '{module}'")
            }
            Self::MissingPackage { package } => {
                write!(formatter, "missing repository package '{package}'")
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
            Self::MissingArtifactBindingId { binding } => {
                write!(
                    formatter,
                    "missing repository artifact binding '{binding:?}'"
                )
            }
            Self::MissingArtifactId { key } => {
                write!(formatter, "missing repository artifact id for '{key:?}'")
            }
            Self::MissingArtifactDependency { key, dependency } => {
                write!(
                    formatter,
                    "missing dependency {dependency} in repository artifact binding '{key:?}'"
                )
            }
            Self::CircularArtifactBinding { key } => {
                write!(
                    formatter,
                    "circular repository artifact binding dependency at '{key:?}'"
                )
            }
            Self::InvalidArtifact { message } => {
                write!(formatter, "invalid repository artifact state: {message}")
            }
            Self::ContendedResolution { detail } => {
                write!(formatter, "contended artifact resolution: {detail}")
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

impl Error for RepositoryError {}
