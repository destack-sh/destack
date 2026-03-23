use crate::compile::Compiler;

use destack_artifact::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey,
    DirAnalyzed, DirAnalyzedImage, DirBase, DirDeclared, DirDeclaredImage, DirElaborated,
    DirElaboratedImage, DirInterface, DirInterfaceImage, DirPatched, DirPatchedImage, DirPrepared,
    DirPreparedImage, DirResolved, DirResolvedImage, ProfileKey, hash_bytes,
};
use destack_source::{FileContent, ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{CacheHasher, compiler_version};

/// Persistent image context for one module scoped base DIR artifact.
#[derive(Debug, Clone)]
struct BaseDirImageContext {
    /// The module version used when producing the image.
    module_version: ModuleVersion,
    /// Hash of the effective compiler configuration and source.
    config_hash: u64,
    /// Hash of the workspace string universe used by the DIR payload.
    workspace_strings_hash: u64,
}

impl BaseDirImageContext {
    /// Build one image header for the base DIR.
    fn header(&self, module_id: ModuleId) -> ArtifactImageHeader {
        ArtifactImageHeader::new(
            ArtifactImageKey::DirBase { module: module_id },
            compiler_version(),
            None,
            self.config_hash,
            Some(self.workspace_strings_hash),
            0,
        )
    }
}

/// Persistent image context for one profile scoped DIR artifact.
#[derive(Debug, Clone)]
struct ProfileDirImageContext {
    /// The profile key for the image.
    profile_key: ProfileKey,
    /// The profile version used when producing the image.
    profile_version: ProfileVersion,
    /// The module version used when producing the image.
    module_version: ModuleVersion,
    /// Hash of the effective compiler configuration and source.
    config_hash: u64,
    /// Hash of the workspace string universe used by the DIR payload.
    workspace_strings_hash: u64,
}

impl ProfileDirImageContext {
    /// Build one image header for the chosen profile scoped DIR family.
    fn header(&self, image_key: ArtifactImageKey) -> ArtifactImageHeader {
        ArtifactImageHeader::new(
            image_key,
            compiler_version(),
            Some(self.profile_version),
            self.config_hash,
            Some(self.workspace_strings_hash),
            0,
        )
    }
}

impl Compiler {
    /// Load one direct module scoped base DIR image entry.
    fn load_base_dir_entry(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> Result<Option<ArtifactImage<DirBase>>, ArtifactImageError> {
        let Some(context) = self.base_dir_image_context(module_id, module_version) else {
            return Ok(None);
        };
        let expected = context.header(module_id);
        let Some(image) = self.load_image::<DirBase>(expected)? else {
            return Ok(None);
        };

        if image.payload.version != context.module_version {
            return Ok(None);
        }

        Ok(Some(image))
    }

    /// Load one profile scoped DIR image entry.
    fn load_profile_dir_entry<T, D>(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        image_key: impl FnOnce(ModuleId, ProfileKey) -> ArtifactImageKey,
        into_dir: impl Fn(T, ProfileId) -> D + Copy,
        version: impl FnOnce(&D) -> ModuleVersion,
    ) -> Result<Option<ArtifactImage<T>>, ArtifactImageError>
    where
        T: Clone + DeserializeOwned + Serialize,
    {
        let Some(context) = self.profile_dir_image_context(module_id, module_version, profile_id)
        else {
            return Ok(None);
        };
        let expected = context.header(image_key(module_id, context.profile_key.clone()));
        let Some(image) = self.load_image::<T>(expected)? else {
            return Ok(None);
        };
        let dir = into_dir(image.payload.clone(), profile_id);

        if version(&dir) != context.module_version {
            return Ok(None);
        }

        Ok(Some(image))
    }

    /// Hash the current source content for one module.
    pub(super) fn module_source_hash(&self, module_id: ModuleId) -> Option<u64> {
        let module = self.program.modules.get(module_id);
        let file = self.program.files.get(module.file_id);

        match &file.content {
            FileContent::Text { content } => Some(hash_bytes(content.as_bytes())),
            FileContent::Json { content, .. } => Some(hash_bytes(content.as_bytes())),
            FileContent::Binary { content } => Some(hash_bytes(content)),
            FileContent::Missing | FileContent::Unloaded => None,
        }
    }

    /// Build one persistent image context for one base DIR artifact.
    fn base_dir_image_context(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> Option<BaseDirImageContext> {
        let source_hash = self.module_source_hash(module_id)?;

        // base dir images are scoped by compiler behavior and current source
        let mut hasher = CacheHasher::new();
        hasher.hash_compiler_options(&self.options);
        hasher.hash_value(&source_hash);

        Some(BaseDirImageContext {
            module_version,
            config_hash: hasher.finish(),
            workspace_strings_hash: self.program.strings.stable_hash(),
        })
    }

    /// Build one persistent image context for one profile scoped DIR artifact.
    fn profile_dir_image_context(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Option<ProfileDirImageContext> {
        let profile = self.program.profile(profile_id);
        let source_hash = self.module_source_hash(module_id)?;

        // profile dir images are scoped by compiler behavior, profile identity, and source
        let mut hasher = CacheHasher::new();
        hasher.hash_compiler_options(&self.options);
        hasher.hash_value(&profile.key);
        hasher.hash_value(&source_hash);

        Some(ProfileDirImageContext {
            profile_key: profile.key.clone(),
            profile_version: profile.version,
            module_version,
            config_hash: hasher.finish(),
            workspace_strings_hash: self.program.strings.stable_hash(),
        })
    }

    /// Load one direct module scoped base DIR image.
    fn load_base_dir(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> Result<Option<DirBase>, ArtifactImageError> {
        Ok(self
            .load_base_dir_entry(module_id, module_version)?
            .map(|image| image.payload))
    }

    /// Store one direct module scoped base DIR image.
    fn store_base_dir(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        dir: &DirBase,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.base_dir_image_context(module_id, module_version) else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::dir_base(module_id);
        let header = context.header(module_id);

        self.store_image(&artifact_key, header, dir.clone())
    }

    /// Load one profile scoped DIR image.
    fn load_profile_dir<T, D>(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        image_key: impl FnOnce(ModuleId, ProfileKey) -> ArtifactImageKey,
        into_dir: impl Fn(T, ProfileId) -> D + Copy,
        version: impl FnOnce(&D) -> ModuleVersion,
    ) -> Result<Option<D>, ArtifactImageError>
    where
        T: Clone + DeserializeOwned + Serialize,
    {
        Ok(self
            .load_profile_dir_entry(
                module_id,
                module_version,
                profile_id,
                image_key,
                into_dir,
                version,
            )?
            .map(|image| into_dir(image.payload, profile_id)))
    }

    /// Store one profile scoped DIR image.
    fn store_profile_dir<T, D>(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        artifact_key: ArtifactKey,
        dir: &D,
        image_key: impl FnOnce(ModuleId, ProfileKey) -> ArtifactImageKey,
        from_dir: impl FnOnce(&D) -> T,
    ) -> Result<(), ArtifactImageError>
    where
        T: Serialize,
    {
        let Some(context) = self.profile_dir_image_context(module_id, module_version, profile_id)
        else {
            return Ok(());
        };
        let header = context.header(image_key(module_id, context.profile_key.clone()));
        let payload = from_dir(dir);

        self.store_image(&artifact_key, header, payload)
    }

    /// Build the current expected base DIR image header.
    pub(crate) fn dir_base_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> Option<ArtifactImageHeader> {
        let context = self.base_dir_image_context(module_id, module_version)?;

        Some(context.header(module_id))
    }

    /// Build the current expected prepared DIR image header.
    pub(crate) fn dir_prepared_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(module_id, module_version, profile_id)?;

        Some(context.header(ArtifactImageKey::DirPrepared {
            module: module_id,
            profile: context.profile_key.clone(),
        }))
    }

    /// Build the current expected resolved DIR image header.
    pub(crate) fn dir_resolved_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(module_id, module_version, profile_id)?;

        Some(context.header(ArtifactImageKey::DirResolved {
            module: module_id,
            profile: context.profile_key.clone(),
        }))
    }

    /// Build the current expected declared DIR image header.
    pub(crate) fn dir_declared_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(module_id, module_version, profile_id)?;

        Some(context.header(ArtifactImageKey::DirDeclared {
            module: module_id,
            profile: context.profile_key.clone(),
        }))
    }

    /// Build the current expected interface DIR image header.
    pub(crate) fn dir_interface_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(module_id, module_version, profile_id)?;

        Some(context.header(ArtifactImageKey::DirInterface {
            module: module_id,
            profile: context.profile_key.clone(),
        }))
    }

    /// Build the current expected analyzed DIR image header.
    pub(crate) fn dir_analyzed_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(module_id, module_version, profile_id)?;

        Some(context.header(ArtifactImageKey::DirAnalyzed {
            module: module_id,
            profile: context.profile_key.clone(),
        }))
    }

    /// Build the current expected elaborated DIR image header.
    pub(crate) fn dir_elaborated_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(module_id, module_version, profile_id)?;

        Some(context.header(ArtifactImageKey::DirElaborated {
            module: module_id,
            profile: context.profile_key.clone(),
        }))
    }

    /// Build the current expected patched DIR image header.
    pub(crate) fn dir_patched_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(module_id, module_version, profile_id)?;

        Some(context.header(ArtifactImageKey::DirPatched {
            module: module_id,
            profile: context.profile_key.clone(),
        }))
    }

    /// Load one base DIR image.
    pub(crate) fn load_dir_base_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> Result<Option<DirBase>, ArtifactImageError> {
        self.load_base_dir(module_id, module_version)
    }

    /// Persist one base DIR image.
    pub(crate) fn store_dir_base_image(
        &self,
        module_id: ModuleId,
        dir: &DirBase,
    ) -> Result<(), ArtifactImageError> {
        self.store_base_dir(module_id, dir.version, dir)
    }

    /// Load one prepared DIR image.
    pub(crate) fn load_dir_prepared_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Result<Option<DirPrepared>, ArtifactImageError> {
        self.load_profile_dir(
            module_id,
            module_version,
            profile_id,
            |module, profile| ArtifactImageKey::DirPrepared { module, profile },
            DirPreparedImage::into_dir,
            |dir| dir.version,
        )
    }

    /// Persist one prepared DIR image.
    pub(crate) fn store_dir_prepared_image(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: &DirPrepared,
    ) -> Result<(), ArtifactImageError> {
        self.store_profile_dir(
            module_id,
            profile_id,
            dir.version,
            ArtifactKey::dir_prepared(module_id, profile_id),
            dir,
            |module, profile| ArtifactImageKey::DirPrepared { module, profile },
            DirPreparedImage::from_dir,
        )
    }

    /// Load one resolved DIR image.
    pub(crate) fn load_dir_resolved_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Result<Option<DirResolved>, ArtifactImageError> {
        self.load_profile_dir(
            module_id,
            module_version,
            profile_id,
            |module, profile| ArtifactImageKey::DirResolved { module, profile },
            DirResolvedImage::into_dir,
            |dir| dir.version,
        )
    }

    /// Persist one resolved DIR image.
    pub(crate) fn store_dir_resolved_image(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: &DirResolved,
    ) -> Result<(), ArtifactImageError> {
        self.store_profile_dir(
            module_id,
            profile_id,
            dir.version,
            ArtifactKey::dir_resolved(module_id, profile_id),
            dir,
            |module, profile| ArtifactImageKey::DirResolved { module, profile },
            DirResolvedImage::from_dir,
        )
    }

    /// Load one declared DIR image.
    pub(crate) fn load_dir_declared_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Result<Option<DirDeclared>, ArtifactImageError> {
        self.load_profile_dir(
            module_id,
            module_version,
            profile_id,
            |module, profile| ArtifactImageKey::DirDeclared { module, profile },
            DirDeclaredImage::into_dir,
            |dir| dir.version,
        )
    }

    /// Persist one declared DIR image.
    pub(crate) fn store_dir_declared_image(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: &DirDeclared,
    ) -> Result<(), ArtifactImageError> {
        self.store_profile_dir(
            module_id,
            profile_id,
            dir.version,
            ArtifactKey::dir_declared(module_id, profile_id),
            dir,
            |module, profile| ArtifactImageKey::DirDeclared { module, profile },
            DirDeclaredImage::from_dir,
        )
    }

    /// Load one interface DIR image.
    pub(crate) fn load_dir_interface_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Result<Option<DirInterface>, ArtifactImageError> {
        self.load_profile_dir(
            module_id,
            module_version,
            profile_id,
            |module, profile| ArtifactImageKey::DirInterface { module, profile },
            DirInterfaceImage::into_dir,
            |dir| dir.version,
        )
    }

    /// Persist one interface DIR image.
    pub(crate) fn store_dir_interface_image(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: &DirInterface,
    ) -> Result<(), ArtifactImageError> {
        self.store_profile_dir(
            module_id,
            profile_id,
            dir.version,
            ArtifactKey::dir_interface(module_id, profile_id),
            dir,
            |module, profile| ArtifactImageKey::DirInterface { module, profile },
            DirInterfaceImage::from_dir,
        )
    }

    /// Load one analyzed DIR image.
    pub(crate) fn load_dir_analyzed_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Result<Option<DirAnalyzed>, ArtifactImageError> {
        self.load_profile_dir(
            module_id,
            module_version,
            profile_id,
            |module, profile| ArtifactImageKey::DirAnalyzed { module, profile },
            DirAnalyzedImage::into_dir,
            |dir| dir.version,
        )
    }

    /// Persist one analyzed DIR image.
    pub(crate) fn store_dir_analyzed_image(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: &DirAnalyzed,
    ) -> Result<(), ArtifactImageError> {
        self.store_profile_dir(
            module_id,
            profile_id,
            dir.version,
            ArtifactKey::dir_analyzed(module_id, profile_id),
            dir,
            |module, profile| ArtifactImageKey::DirAnalyzed { module, profile },
            DirAnalyzedImage::from_dir,
        )
    }

    /// Load one elaborated DIR image.
    pub(crate) fn load_dir_elaborated_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Result<Option<DirElaborated>, ArtifactImageError> {
        self.load_profile_dir(
            module_id,
            module_version,
            profile_id,
            |module, profile| ArtifactImageKey::DirElaborated { module, profile },
            DirElaboratedImage::into_dir,
            |dir| dir.version,
        )
    }

    /// Persist one elaborated DIR image.
    pub(crate) fn store_dir_elaborated_image(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: &DirElaborated,
    ) -> Result<(), ArtifactImageError> {
        self.store_profile_dir(
            module_id,
            profile_id,
            dir.version,
            ArtifactKey::dir_elaborated(module_id, profile_id),
            dir,
            |module, profile| ArtifactImageKey::DirElaborated { module, profile },
            DirElaboratedImage::from_dir,
        )
    }

    /// Load one patched DIR image.
    pub(crate) fn load_dir_patched_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
    ) -> Result<Option<DirPatched>, ArtifactImageError> {
        self.load_profile_dir(
            module_id,
            module_version,
            profile_id,
            |module, profile| ArtifactImageKey::DirPatched { module, profile },
            DirPatchedImage::into_dir,
            |dir| dir.version,
        )
    }

    /// Persist one patched DIR image.
    pub(crate) fn store_dir_patched_image(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir: &DirPatched,
    ) -> Result<(), ArtifactImageError> {
        self.store_profile_dir(
            module_id,
            profile_id,
            dir.version,
            ArtifactKey::dir_patched(module_id, profile_id),
            dir,
            |module, profile| ArtifactImageKey::DirPatched { module, profile },
            DirPatchedImage::from_dir,
        )
    }
}
