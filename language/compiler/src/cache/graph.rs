use crate::compile::Compiler;

use destack_artifact::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey,
    ModuleGraph, ModuleGraphImage,
};
use destack_source::{FileKey, ModuleId, ModuleVersion};
use destack_workspace::ProfileId;
use indexmap::IndexMap;

use super::{CacheHasher, compiler_version};

impl Compiler {
    /// Return the stable file key for one live module id.
    fn module_graph_file_key(&self, module_id: ModuleId) -> Option<FileKey> {
        let module = self.program.modules.get(module_id);
        let file = self.program.files.get(module.file_id);
        Some(file.key)
    }

    /// Resolve one stable file key back into a live module id.
    fn module_id_for_graph_file_key(&self, file_key: FileKey) -> Option<ModuleId> {
        if let Some(file_id) = self.program.files.get_id_by_key(file_key) {
            if let Some(module_id) = self.program.modules.get_id_by_file_id(file_id) {
                return Some(module_id);
            }

            let file = self.program.files.get(file_id);
            if let Some(path) = file.path.as_ref() {
                return self.resolve_path_to_module(path).ok();
            }
        }

        let path = self.session.workspace_index_path_for_file_key(file_key)?;
        self.resolve_path_to_module(&path).ok()
    }

    /// Build one stable module graph version snapshot for header validation.
    fn module_graph_header_context(
        module_versions: &IndexMap<ModuleId, ModuleVersion>,
        file_key_for_module_id: impl Fn(ModuleId) -> Option<FileKey>,
    ) -> Result<Vec<u8>, ArtifactImageError> {
        let mut snapshot: Vec<_> = module_versions
            .iter()
            .map(|(id, version)| file_key_for_module_id(*id).map(|file_key| (file_key, *version)))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                ArtifactImageError::Io(std::io::Error::other(
                    "missing stable file key for module graph image",
                ))
            })?;
        snapshot.sort_unstable_by_key(|(file_key, _)| *file_key);

        postcard::to_allocvec(&snapshot).map_err(ArtifactImageError::Serialize)
    }

    /// Return whether one module graph header context still matches current module versions.
    fn module_graph_header_matches_current_versions(
        &self,
        header: &ArtifactImageHeader,
    ) -> Result<bool, ArtifactImageError> {
        let snapshot: Vec<(FileKey, ModuleVersion)> =
            postcard::from_bytes(&header.context_bytes).map_err(ArtifactImageError::Deserialize)?;

        for (file_key, module_version) in snapshot {
            let Some(module_id) = self.module_id_for_graph_file_key(file_key) else {
                return Ok(false);
            };

            if self.module_version(module_id) != module_version {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Load one persisted module graph image entry.
    fn load_module_graph_image_entry(
        &self,
        profile_id: ProfileId,
    ) -> Result<Option<ArtifactImage<ModuleGraphImage>>, ArtifactImageError> {
        let expected = self.module_graph_image_header(profile_id);
        let Some(artifact_store) = self.artifact_store() else {
            return Ok(None);
        };
        let Some(image) = artifact_store.load::<ModuleGraphImage>(&expected.artifact_image_key)?
        else {
            return Ok(None);
        };

        if !expected.matches_without_context_bytes(&image.header) {
            return Ok(None);
        }

        if !self.persisted_image_requirements_are_satisfied(&image.header.requirements)? {
            return Ok(None);
        }

        if !self.module_graph_header_matches_current_versions(&image.header)? {
            return Ok(None);
        }

        Ok(Some(image))
    }

    /// Build one image header for one module graph.
    pub(crate) fn module_graph_image_header(&self, profile_id: ProfileId) -> ArtifactImageHeader {
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
        let Some(image) = self.load_module_graph_image_entry(profile_id)? else {
            return Ok(None);
        };
        let Some(graph) = image.payload.into_graph(profile_id, |file_key| {
            self.module_id_for_graph_file_key(file_key)
        }) else {
            return Ok(None);
        };

        Ok(Some(graph))
    }

    /// Load the expected reusable module graph image validation hash when available.
    pub(crate) fn load_expected_module_graph_image_validation_hash(
        &self,
        profile_id: ProfileId,
    ) -> Result<Option<u64>, ArtifactImageError> {
        let expected = self.module_graph_image_header(profile_id);
        let Some(artifact_store) = self.artifact_store() else {
            return Ok(None);
        };
        let Some(header) = artifact_store.load_header(&expected.artifact_image_key)? else {
            return Ok(None);
        };

        if !expected.matches_without_context_bytes(&header) {
            return Ok(None);
        }

        if !self.persisted_image_requirements_are_satisfied(&header.requirements)? {
            return Ok(None);
        }

        if !self.module_graph_header_matches_current_versions(&header)? {
            return Ok(None);
        }

        Ok(Some(header.validation_hash))
    }

    /// Persist one module graph image.
    pub(crate) fn store_module_graph_image(
        &self,
        profile_id: ProfileId,
        graph: &ModuleGraph,
    ) -> Result<(), ArtifactImageError> {
        let artifact_key = ArtifactKey::module_graph(profile_id);
        let header = self
            .module_graph_image_header(profile_id)
            .with_context_bytes(Self::module_graph_header_context(
                &graph.module_versions,
                |module_id| self.module_graph_file_key(module_id),
            )?);
        let payload =
            ModuleGraphImage::from_graph(graph, |module_id| self.module_graph_file_key(module_id))
                .ok_or_else(|| {
                    ArtifactImageError::Io(std::io::Error::other(
                        "missing stable file key for module graph image",
                    ))
                })?;

        self.store_image(&artifact_key, header, payload)
    }
}
