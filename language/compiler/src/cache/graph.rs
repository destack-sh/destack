use crate::compile::Compiler;

use destack_workspace::{
    ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ModuleGraph, ModuleGraphImage,
    ProfileId,
};

use super::{CacheHasher, compiler_version};

impl Compiler {
    /// Build one image header for one module graph.
    fn module_graph_image_header(&self, profile_id: ProfileId) -> ArtifactImageHeader {
        let profile = self.program.profile(profile_id);

        // module graph images are scoped by profile identity
        let mut hasher = CacheHasher::new();
        hasher.hash_value(&profile.key);

        ArtifactImageHeader::new(
            ArtifactImageKey::ModuleGraph {
                profile: profile.key.clone(),
            },
            compiler_version(),
            Some(profile.version),
            hasher.finish(),
            None,
            0,
        )
    }

    /// Load one persisted module graph image.
    pub(crate) fn load_module_graph_image(
        &self,
        profile_id: ProfileId,
    ) -> Result<Option<ModuleGraph>, ArtifactImageError> {
        let expected = self.module_graph_image_header(profile_id);
        let Some(image) = self.load_image::<ModuleGraphImage>(expected)? else {
            return Ok(None);
        };
        let graph = image.payload.into_graph(profile_id);

        // reject graphs whose tracked module versions no longer match
        for (module_id, module_version) in &graph.module_versions {
            if self.module_version(*module_id) != *module_version {
                return Ok(None);
            }
        }

        Ok(Some(graph))
    }

    /// Persist one module graph image.
    pub(crate) fn store_module_graph_image(
        &self,
        profile_id: ProfileId,
        graph: &ModuleGraph,
    ) -> Result<(), ArtifactImageError> {
        let header = self.module_graph_image_header(profile_id);
        let payload = ModuleGraphImage::from_graph(graph);

        self.store_image(header, payload)
    }
}
