use crate::compile::Compiler;

use destack_artifact::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey,
    ModuleGraph,
};
use destack_workspace::{ProfileId, Revision};

use super::CacheHasher;

impl Compiler {
    /// Load one persisted module graph image entry.
    fn load_module_graph_image_entry(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Result<Option<ArtifactImage<ModuleGraph>>, ArtifactImageError> {
        let expected = self.module_graph_image_header(revision, profile_id);
        let Some(image) = self.load_image::<ModuleGraph>(revision, expected.clone())? else {
            return Ok(None);
        };

        Ok(Some(image))
    }

    /// Build one image header for one module graph.
    pub(crate) fn module_graph_image_header(
        &self,
        _revision: Revision,
        profile_id: ProfileId,
    ) -> ArtifactImageHeader {
        let profile = self.profile(profile_id);

        // module graph images are scoped by profile identity
        let mut hasher = CacheHasher::new();
        hasher.hash_value(&profile.key);

        ArtifactImageHeader::new(
            ArtifactImageKey::ModuleGraph {
                profile: profile.key.clone(),
            },
            hasher.finish(),
        )
    }

    /// Load one persisted module graph image.
    pub(crate) fn load_module_graph_image(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Result<Option<ModuleGraph>, ArtifactImageError> {
        let Some(image) = self.load_module_graph_image_entry(revision, profile_id)? else {
            return Ok(None);
        };

        Ok(Some(image.payload))
    }

    /// Persist one module graph image.
    pub(crate) fn store_module_graph_image(
        &self,
        revision: Revision,
        profile_id: ProfileId,
        graph: &ModuleGraph,
    ) -> Result<(), ArtifactImageError> {
        let artifact_key = ArtifactKey::module_graph(profile_id);
        let header = self.module_graph_image_header(revision, profile_id);
        let payload = graph.clone();

        self.store_image(revision, &artifact_key, header, payload)
    }
}
