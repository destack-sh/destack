use destack_artifact::ProfileKey;
use destack_source::ProfileId;

use crate::ProfileEnv;

/// One resolved semantic profile.
#[derive(Debug, Clone)]
pub struct Profile {
    /// The canonical profile key.
    pub key: ProfileKey,
    /// The resolved environment values.
    pub env: ProfileEnv,
}

impl Profile {
    /// Build one resolved semantic profile from one canonical key.
    pub fn from_key(key: ProfileKey) -> Self {
        let env = ProfileEnv::from_snapshot(&key.env, key.debug);

        Self { key, env }
    }

    /// Return the deterministic profile id for this profile.
    pub fn id(&self) -> ProfileId {
        ProfileId::new(self.key.stable_hash())
    }
}
