use serde::{Deserialize, Serialize};

use destack_source::{FileContentId, FileId, ModuleId, PackageId, ProfileId, TargetId};

use crate::{ArtifactVersion, ProfileKey};

/// Exact source path state observed by one artifact computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactPathState {
    /// The path did not exist.
    Missing,
    /// The path was a regular file.
    File,
    /// The path was a directory.
    Directory,
    /// The path was a symbolic link.
    Symlink,
    /// The path existed with another host-specific kind.
    Other,
}

/// One exact directory entry observed by one artifact computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactDirectoryEntry {
    /// The entry path identity.
    pub path: FileId,
    /// The exact entry path state.
    pub state: ArtifactPathState,
}

impl ArtifactDirectoryEntry {
    /// Build one exact directory entry dependency fact.
    pub const fn new(path: FileId, state: ArtifactPathState) -> Self {
        Self { path, state }
    }
}

/// Exact target configuration identity.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub struct TargetKey(pub u128);

impl TargetKey {
    /// Build one target key from a stable hash.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }
}

/// One exact dependency read while building an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactDependency {
    /// Another exact artifact version.
    Artifact(ArtifactVersion),
    /// The exact state observed for one source path.
    Path {
        /// The source path identity.
        path: FileId,
        /// The exact path state.
        state: ArtifactPathState,
    },
    /// The exact direct entries observed for one directory.
    DirectoryEntries {
        /// The source directory path identity.
        directory: FileId,
        /// The direct entries in deterministic order.
        entries: Vec<ArtifactDirectoryEntry>,
    },
    /// The exact source content read for one file.
    FileContent {
        /// The source file id.
        file: FileId,
        /// The exact source content id.
        content: FileContentId,
    },
    /// The exact packages in one repository revision.
    RepositoryPackages {
        /// The package ids in deterministic order.
        packages: Vec<PackageId>,
    },
    /// The exact module membership read for one package.
    PackageModules {
        /// The package id.
        package: PackageId,
        /// The selected modules in deterministic order.
        modules: Vec<ModuleId>,
    },
    /// The exact target configuration read for one target.
    TargetConfiguration {
        /// The target id.
        target: TargetId,
        /// The exact target key.
        key: TargetKey,
    },
    /// The exact modules selected by one target.
    TargetModules {
        /// The target id.
        target: TargetId,
        /// The selected modules in deterministic order.
        modules: Vec<ModuleId>,
    },
    /// The exact profile selected for one module.
    ModuleProfile {
        /// The module id.
        module: ModuleId,
        /// The selected profile id.
        profile: ProfileId,
    },
    /// The exact profile selected for one target.
    TargetProfile {
        /// The target id.
        target: TargetId,
        /// The selected profile id.
        profile: ProfileId,
    },
    /// The exact profile configuration read for one profile.
    ProfileConfiguration {
        /// The profile id.
        profile: ProfileId,
        /// The exact profile key.
        key: ProfileKey,
    },
}

impl ArtifactDependency {
    /// Build one artifact dependency.
    pub fn artifact(version: ArtifactVersion) -> Self {
        Self::Artifact(version)
    }

    /// Build one source path state dependency.
    pub fn path(path: FileId, state: ArtifactPathState) -> Self {
        Self::Path { path, state }
    }

    /// Build one directory entries dependency.
    pub fn directory_entries(
        directory: FileId,
        entries: impl IntoIterator<Item = ArtifactDirectoryEntry>,
    ) -> Self {
        let mut entries = entries.into_iter().collect::<Vec<_>>();
        entries.sort_unstable();
        entries.dedup();

        Self::DirectoryEntries { directory, entries }
    }

    /// Build one file content dependency.
    pub fn file_content(file: FileId, content: FileContentId) -> Self {
        Self::FileContent { file, content }
    }

    /// Build one repository package dependency.
    pub fn repository_packages(packages: impl IntoIterator<Item = PackageId>) -> Self {
        let mut packages = packages.into_iter().collect::<Vec<_>>();
        packages.sort_unstable();
        packages.dedup();

        Self::RepositoryPackages { packages }
    }

    /// Build one package module membership dependency.
    pub fn package_modules(
        package: PackageId,
        modules: impl IntoIterator<Item = ModuleId>,
    ) -> Self {
        let mut modules = modules.into_iter().collect::<Vec<_>>();
        modules.sort_unstable();
        modules.dedup();

        Self::PackageModules { package, modules }
    }

    /// Build one target configuration dependency.
    pub fn target_configuration(target: TargetId, key: TargetKey) -> Self {
        Self::TargetConfiguration { target, key }
    }

    /// Build one target module selection dependency.
    pub fn target_modules(target: TargetId, modules: impl IntoIterator<Item = ModuleId>) -> Self {
        let mut modules = modules.into_iter().collect::<Vec<_>>();
        modules.sort_unstable();
        modules.dedup();

        Self::TargetModules { target, modules }
    }

    /// Build one module profile selection dependency.
    pub fn module_profile(module: ModuleId, profile: ProfileId) -> Self {
        Self::ModuleProfile { module, profile }
    }

    /// Build one target profile selection dependency.
    pub fn target_profile(target: TargetId, profile: ProfileId) -> Self {
        Self::TargetProfile { target, profile }
    }

    /// Build one profile configuration dependency.
    pub fn profile_configuration(profile: ProfileId, key: ProfileKey) -> Self {
        Self::ProfileConfiguration { profile, key }
    }
}
