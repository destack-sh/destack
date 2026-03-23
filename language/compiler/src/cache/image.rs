use crate::compile::Compiler;
use destack_artifact::{ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ProfileKey};
use destack_workspace::ProfileId;

impl Compiler {
    /// Return the live profile id for one stable image profile key.
    fn profile_id_for_artifact_image_key(&self, profile_key: &ProfileKey) -> Option<ProfileId> {
        self.program.profiles.id_for_key(profile_key)
    }

    /// Build the expected persisted image header for one stable image key.
    pub(crate) fn expected_image_header_for_artifact_key(
        &self,
        artifact_key: &ArtifactImageKey,
    ) -> Result<Option<ArtifactImageHeader>, ArtifactImageError> {
        let header = match artifact_key {
            ArtifactImageKey::ModuleGraph { profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                Some(self.module_graph_image_header(profile_id))
            }
            ArtifactImageKey::LanguageEnvironment { profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.language_environment_image_header(profile_id)
            }
            ArtifactImageKey::IntrinsicEnvironment { profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.intrinsic_environment_image_header(profile_id)
            }
            ArtifactImageKey::LibraryEnvironment { profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.library_environment_image_header(profile_id)
            }
            ArtifactImageKey::Ast { module } => self.current_ast_image_header(*module),
            ArtifactImageKey::DirBase { module } => {
                self.dir_base_image_header(*module, self.module_version(*module))
            }
            ArtifactImageKey::DirPrepared { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_prepared_image_header(*module, self.module_version(*module), profile_id)
            }
            ArtifactImageKey::DirResolved { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_resolved_image_header(*module, self.module_version(*module), profile_id)
            }
            ArtifactImageKey::DirDeclared { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_declared_image_header(*module, self.module_version(*module), profile_id)
            }
            ArtifactImageKey::DirInterface { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_interface_image_header(*module, self.module_version(*module), profile_id)
            }
            ArtifactImageKey::DirAnalyzed { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_analyzed_image_header(*module, self.module_version(*module), profile_id)
            }
            ArtifactImageKey::DirElaborated { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_elaborated_image_header(*module, self.module_version(*module), profile_id)
            }
            ArtifactImageKey::DirPatched { module, profile } => {
                let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                    return Ok(None);
                };

                self.dir_patched_image_header(*module, self.module_version(*module), profile_id)
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
                    *module,
                    self.module_version(*module),
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
                    *module,
                    self.module_version(*module),
                    profile_id,
                    target,
                )
            }
            ArtifactImageKey::ModuleOutput { module, target } => {
                let Some(profile_id) = self.program.profile_id_for_target(*module, target) else {
                    return Ok(None);
                };

                self.module_output_image_header(
                    *module,
                    self.module_version(*module),
                    profile_id,
                    target,
                )
            }
            ArtifactImageKey::PackageOutput { package, target } => {
                self.package_output_image_header(*package, target)
            }
        };

        Ok(header)
    }

    /// Load the expected persisted validation hash for one stable image key.
    pub(crate) fn load_expected_artifact_image_validation_hash(
        &self,
        artifact_key: &ArtifactImageKey,
    ) -> Result<Option<u64>, ArtifactImageError> {
        // module graph still validates freshness through its header context
        if let ArtifactImageKey::ModuleGraph { profile } = artifact_key {
            let Some(profile_id) = self.profile_id_for_artifact_image_key(profile) else {
                return Ok(None);
            };

            return self.load_expected_module_graph_image_validation_hash(profile_id);
        }

        let Some(expected) = self.expected_image_header_for_artifact_key(artifact_key)? else {
            return Ok(None);
        };

        self.load_image_validation_hash(&expected)
    }
}
