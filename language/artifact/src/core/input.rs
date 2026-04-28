use serde::{Deserialize, Serialize};

use destack_source::{FileContentId, FileId, ModuleId, PackageId, ProfileId, TargetId};

use crate::ProfileKey;

/// One exact non-artifact fact read while building an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactInput {
    /// The exact source content read for one file.
    FileContent {
        /// The source file id.
        file: FileId,
        /// The exact source content id.
        content: FileContentId,
    },
    /// The exact profile configuration read for one profile.
    Profile {
        /// The profile id.
        profile: ProfileId,
        /// The exact profile key.
        key: ProfileKey,
    },
    /// The exact module membership read for one package.
    PackageModules {
        /// The package id.
        package: PackageId,
        /// The selected modules in deterministic order.
        modules: Vec<ModuleId>,
    },
    /// The exact target configuration read for one target.
    Target {
        /// The target id.
        target: TargetId,
        /// The stable hash of the effective target configuration.
        configuration_hash: u128,
    },
    /// The exact profile selected for one target.
    TargetProfile {
        /// The target id.
        target: TargetId,
        /// The selected profile id.
        profile: ProfileId,
    },
}

impl ArtifactInput {
    /// Build one file content input.
    pub fn file_content(file: FileId, content: FileContentId) -> Self {
        Self::FileContent { file, content }
    }

    /// Build one profile input.
    pub fn profile(profile: ProfileId, key: ProfileKey) -> Self {
        Self::Profile { profile, key }
    }

    /// Build one package module membership input.
    pub fn package_modules(
        package: PackageId,
        modules: impl IntoIterator<Item = ModuleId>,
    ) -> Self {
        let mut modules = modules.into_iter().collect::<Vec<_>>();
        modules.sort_unstable();
        modules.dedup();

        Self::PackageModules { package, modules }
    }

    /// Build one target configuration input.
    pub fn target(target: TargetId, configuration_hash: u128) -> Self {
        Self::Target {
            target,
            configuration_hash,
        }
    }

    /// Build one target profile input.
    pub fn target_profile(target: TargetId, profile: ProfileId) -> Self {
        Self::TargetProfile { target, profile }
    }
}
