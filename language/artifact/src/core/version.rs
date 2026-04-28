use serde::{Deserialize, Serialize};

use destack_core::StableHasher;
use destack_source::{ModuleId, PackageId, ProfileId};

use crate::{ArtifactInput, ArtifactKey};

/// Deterministic identity of one artifact's complete semantic inputs.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct ArtifactFingerprint(pub u128);

impl std::fmt::Debug for ArtifactFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "f{:032x}", self.0)
    }
}

impl std::fmt::Display for ArtifactFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "f{:032x}", self.0)
    }
}

impl ArtifactFingerprint {
    /// Create a new artifact fingerprint.
    pub fn new(value: u128) -> Self {
        Self(value)
    }

    /// Derive one artifact fingerprint from exact inputs and dependencies.
    pub fn from_record(
        key: ArtifactKey,
        inputs: impl IntoIterator<Item = ArtifactInput>,
        dependencies: impl IntoIterator<Item = ArtifactVersion>,
    ) -> Self {
        let inputs = canonical_items(inputs);
        let dependencies = canonical_items(dependencies);
        let mut hasher = StableHasher::new();

        hasher.update_len_prefixed(b"destack.artifact.fingerprint.v1");
        update_canonical_item(&mut hasher, &key);
        update_canonical_items(&mut hasher, &inputs);
        update_canonical_items(&mut hasher, &dependencies);

        Self(hasher.finish_u128())
    }
}

/// One exact live artifact version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactVersion {
    /// The semantic artifact slot.
    pub key: ArtifactKey,
    /// The exact semantic fingerprint.
    pub fingerprint: ArtifactFingerprint,
}

fn canonical_items<T: Serialize>(items: impl IntoIterator<Item = T>) -> Vec<Vec<u8>> {
    let mut items = items
        .into_iter()
        .map(|item| {
            postcard::to_allocvec(&item).unwrap_or_else(|error| {
                panic!("failed to serialize artifact fingerprint item: {error}")
            })
        })
        .collect::<Vec<_>>();
    items.sort();
    items.dedup();

    items
}

fn update_canonical_item<T: Serialize>(hasher: &mut StableHasher, item: &T) {
    let bytes = postcard::to_allocvec(item)
        .unwrap_or_else(|error| panic!("failed to serialize artifact fingerprint item: {error}"));
    hasher.update_len_prefixed(&bytes);
}

fn update_canonical_items(hasher: &mut StableHasher, items: &[Vec<u8>]) {
    hasher.update(&(items.len() as u64).to_le_bytes());

    for item in items {
        hasher.update_len_prefixed(item);
    }
}

impl ArtifactVersion {
    /// Create one artifact version from one key and fingerprint.
    pub fn new(key: ArtifactKey, fingerprint: ArtifactFingerprint) -> Self {
        Self { key, fingerprint }
    }

    /// Return the package referenced by this artifact version when one exists.
    pub fn package_id(&self) -> Option<PackageId> {
        self.key.package_id()
    }

    /// Return the module referenced by this artifact version when one exists.
    pub fn module_id(&self) -> Option<ModuleId> {
        self.key.module_id()
    }

    /// Return the profile referenced by this artifact version when one exists.
    pub fn profile_id(&self) -> Option<ProfileId> {
        self.key.profile_id()
    }
}
