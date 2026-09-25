use std::fmt;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tspp_core::StableHasher;
use tspp_source::PackageId;

use crate::{ArtifactPack, ArtifactVersion, BuildId};

/// Deterministic identity of the artifact versions stored in one pack.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactPackVersion(u128);

impl ArtifactPackVersion {
    /// Build one pack version from its exact artifact versions.
    pub fn new(versions: impl IntoIterator<Item = ArtifactVersion>) -> Self {
        let mut versions = versions.into_iter().collect::<Vec<_>>();
        versions.sort_unstable();

        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"tspp.artifact.pack.version.v2");
        ArtifactPack::FORMAT.hash(&mut hasher);
        versions.hash(&mut hasher);

        Self(hasher.finish_u128())
    }
}

impl fmt::Debug for ArtifactPackVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "v{:032x}", self.0)
    }
}

impl fmt::Display for ArtifactPackVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:032x}", self.0)
    }
}

/// One artifact pack selected by an artifact cache manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPackReference {
    /// The common package owner, or the repository-wide artifact group.
    pub package: Option<PackageId>,
    /// The exact artifact version set stored in the pack.
    pub version: ArtifactPackVersion,
    /// The pack checksum verified before decoding.
    pub checksum: u32,
}

impl ArtifactPackReference {
    /// Build one artifact pack reference.
    pub const fn new(
        package: Option<PackageId>,
        version: ArtifactPackVersion,
        checksum: u32,
    ) -> Self {
        Self {
            package,
            version,
            checksum,
        }
    }
}

/// One persisted artifact selection for a repository revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactCacheManifest<R> {
    /// The persistent cache format.
    pub format: u32,
    /// The toolchain build that produced the selected artifacts.
    pub build_id: BuildId,
    /// The canonical repository path receiving this manifest.
    repository: PathBuf,
    /// The repository revision that selected the artifacts.
    pub revision: R,
    /// The selected artifact packs in package and version order.
    pub packs: Box<[ArtifactPackReference]>,
}

impl<R> ArtifactCacheManifest<R> {
    /// The current persistent manifest format.
    pub const FORMAT: u32 = 6;

    /// Build one manifest from an exact revision selection.
    pub fn new(
        build_id: BuildId,
        repository: &Path,
        revision: R,
        packs: impl Into<Box<[ArtifactPackReference]>>,
    ) -> Self {
        Self {
            format: Self::FORMAT,
            build_id,
            repository: repository.to_path_buf(),
            revision,
            packs: packs.into(),
        }
    }

    /// Validate one decoded manifest.
    pub(crate) fn validate(&self, build_id: BuildId) -> Result<(), crate::ArtifactCacheError> {
        if self.format != Self::FORMAT {
            return Err(crate::ArtifactCacheError::Invalid(format!(
                "manifest format {} is unsupported",
                self.format
            )));
        }
        if self.build_id != build_id {
            return Err(crate::ArtifactCacheError::Invalid(format!(
                "manifest build {} does not match host build {build_id}",
                self.build_id
            )));
        }
        if !self.packs.windows(2).all(|packs| {
            let left = (packs[0].package, packs[0].version);
            let right = (packs[1].package, packs[1].version);

            left < right
        }) {
            return Err(crate::ArtifactCacheError::Invalid(
                "manifest artifact packs are not strictly ordered".to_string(),
            ));
        }

        Ok(())
    }

    /// Return whether this manifest belongs to one canonical repository path.
    pub(crate) fn belongs_to(&self, repository: &Path) -> bool {
        self.repository == repository
    }

    /// Return the canonical repository path owning this manifest.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub(crate) fn repository(&self) -> &Path {
        &self.repository
    }
}
