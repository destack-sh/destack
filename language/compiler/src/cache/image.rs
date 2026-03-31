use crate::compile::Compiler;
use destack_artifact::{
    ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey, ProfileKey,
};
use destack_source::ProfileId;
use destack_workspace::Revision;

impl Compiler {
    /// Return the live profile id for one stable image profile key.
    fn profile_id_for_artifact_image_key(&self, profile_key: &ProfileKey) -> Option<ProfileId> {
        Some(self.remember_profile_key(profile_key.clone()))
    }

    /// Build the expected persisted image header for one stable image key.
    pub(crate) fn expected_image_header_for_artifact_key(
        &self,
        revision: Revision,
        artifact_key: &ArtifactImageKey,
    ) -> Result<Option<ArtifactImageHeader>, ArtifactImageError> {
        let header = match artifact_key {
            ArtifactImageKey::ModuleGraph { profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                Some(self.module_graph_image_header(revision, profile_id))
            }
            ArtifactImageKey::LanguageEnvironment { profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.language_environment_image_header(revision, profile_id)
            }
            ArtifactImageKey::IntrinsicEnvironment { profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.intrinsic_environment_image_header(revision, profile_id)
            }
            ArtifactImageKey::LibraryEnvironment { profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.library_environment_image_header(revision, profile_id)
            }
            ArtifactImageKey::Ast { module } => self.current_ast_image_header(revision, *module),
            ArtifactImageKey::Data { module } => self.current_data_image_header(revision, *module),
            ArtifactImageKey::DirBase { module } => self.dir_base_image_header(
                revision,
                *module,
                self.artifact_stamp_for_revision(revision, &ArtifactKey::dir_base(*module)),
            ),
            ArtifactImageKey::DirPrepared { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_prepared_image_header(
                    revision,
                    *module,
                    self.artifact_stamp_for_revision(
                        revision,
                        &ArtifactKey::dir_prepared(*module, profile_id),
                    ),
                    profile_id,
                )
            }
            ArtifactImageKey::DirResolved { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_resolved_image_header(
                    revision,
                    *module,
                    self.artifact_stamp_for_revision(
                        revision,
                        &ArtifactKey::dir_resolved(*module, profile_id),
                    ),
                    profile_id,
                )
            }
            ArtifactImageKey::DirDeclared { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_declared_image_header(
                    revision,
                    *module,
                    self.artifact_stamp_for_revision(
                        revision,
                        &ArtifactKey::dir_declared(*module, profile_id),
                    ),
                    profile_id,
                )
            }
            ArtifactImageKey::DirInterface { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_interface_image_header(
                    revision,
                    *module,
                    self.artifact_stamp_for_revision(
                        revision,
                        &ArtifactKey::dir_interface(*module, profile_id),
                    ),
                    profile_id,
                )
            }
            ArtifactImageKey::DirAnalyzed { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_analyzed_image_header(
                    revision,
                    *module,
                    self.artifact_stamp_for_revision(
                        revision,
                        &ArtifactKey::dir_analyzed(*module, profile_id),
                    ),
                    profile_id,
                )
            }
            ArtifactImageKey::DirElaborated { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_elaborated_image_header(
                    revision,
                    *module,
                    self.artifact_stamp_for_revision(
                        revision,
                        &ArtifactKey::dir_elaborated(*module, profile_id),
                    ),
                    profile_id,
                )
            }
            ArtifactImageKey::DirPatched { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_patched_image_header(
                    revision,
                    *module,
                    self.artifact_stamp_for_revision(
                        revision,
                        &ArtifactKey::dir_patched(*module, profile_id),
                    ),
                    profile_id,
                )
            }
            ArtifactImageKey::MirBase {
                module,
                profile,
                target,
            } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.mir_base_image_header(
                    revision,
                    *module,
                    self.artifact_stamp_for_revision(
                        revision,
                        &ArtifactKey::mir_base(*module, profile_id, *target),
                    ),
                    profile_id,
                    target,
                )
            }
            ArtifactImageKey::MirOptimized {
                module,
                profile,
                target,
            } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.mir_optimized_image_header(
                    revision,
                    *module,
                    self.artifact_stamp_for_revision(
                        revision,
                        &ArtifactKey::mir_optimized(*module, profile_id, *target),
                    ),
                    profile_id,
                    target,
                )
            }
            ArtifactImageKey::ModuleOutput { .. } => None,
            ArtifactImageKey::PackageOutput { package, target } => {
                self.package_output_image_header(revision, *package, target)
            }
        };

        Ok(header)
    }

    /// Load the expected persisted content id for one stable image key.
    pub(crate) fn load_expected_artifact_content_id(
        &self,
        revision: Revision,
        artifact_key: &ArtifactImageKey,
    ) -> Result<Option<destack_artifact::ArtifactContentId>, ArtifactImageError> {
        self.load_expected_artifact_content_id_with_active(revision, artifact_key, &mut Vec::new())
    }

    /// Load the expected persisted content id for one stable image key.
    pub(crate) fn load_expected_artifact_content_id_with_active(
        &self,
        revision: Revision,
        artifact_key: &ArtifactImageKey,
        active_keys: &mut Vec<ArtifactImageKey>,
    ) -> Result<Option<destack_artifact::ArtifactContentId>, ArtifactImageError> {
        let Some(expected) = self.expected_image_header_for_artifact_key(revision, artifact_key)?
        else {
            return Ok(None);
        };

        self.load_current_content_id_with_active(revision, &expected, active_keys)
    }
}
