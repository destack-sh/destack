use serde::{Deserialize, Serialize};

use destack_core::StableHasher;

use crate::{ArtifactDependency, ArtifactKey};

/// Deterministic identity of one artifact's complete semantic dependencies.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct ArtifactFingerprint(u128);

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
    /// Create one artifact fingerprint from an artifact key and exact dependencies.
    pub(crate) fn new(
        key: ArtifactKey,
        dependencies: impl IntoIterator<Item = ArtifactDependency>,
    ) -> Self {
        // artifact dependencies are a set
        let mut dependencies = dependencies.into_iter().collect::<Vec<_>>();
        dependencies.sort_unstable();
        dependencies.dedup();

        // stable fingerprint stream
        let mut hasher = StableHasher::new();

        hasher.update_len_prefixed(b"destack.artifact.fingerprint.v1");
        update_stable_value(&mut hasher, &key);
        update_stable_values(&mut hasher, &dependencies);

        Self(hasher.finish_u128())
    }
}

/// Add one stable serialized value to a fingerprint hash.
fn update_stable_value<T: Serialize>(hasher: &mut StableHasher, value: &T) {
    let bytes = stable_fingerprint_bytes(value);
    hasher.update_len_prefixed(&bytes);
}

/// Add one stable serialized sequence to a fingerprint hash.
fn update_stable_values<T: Serialize>(hasher: &mut StableHasher, values: &[T]) {
    hasher.update(&(values.len() as u64).to_le_bytes());

    for value in values {
        update_stable_value(hasher, value);
    }
}

/// Encode one fingerprint component into stable bytes.
fn stable_fingerprint_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    postcard::to_allocvec(value)
        .unwrap_or_else(|error| panic!("failed to serialize artifact fingerprint value: {error}"))
}
