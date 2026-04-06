use std::collections::HashMap;

use crate::compile::Compiler;

use super::CacheHasher;
use destack_artifact::{
    ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey, ArtifactStamp,
    DehydrationContext, DirAnalyzed, DirBase, DirDeclared, DirElaborated, DirInterface, DirPatched,
    DirPrepared, DirResolved, HydrationContext, Image, ProfileKey,
};
use destack_core::{ImmutableStringPool, StringId, StringPool};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Revision};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Persisted artifact wrapper with one self-contained string table.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedArtifact<T> {
    /// The image-local string table.
    strings: ImmutableStringPool,
    /// The persisted payload using image-local string ids.
    payload: T,
}

/// String-id mapping state while dehydrating one live artifact.
struct ArtifactDehydrationContext<'a> {
    /// The live repository string pool.
    repository_strings: &'a StringPool,
    /// The image-local string pool under construction.
    image_strings: StringPool,
    /// The image-local id for each live string id already seen.
    image_id_by_live_id: HashMap<StringId, StringId>,
}

impl<'a> ArtifactDehydrationContext<'a> {
    /// Create one dehydration context for one repository string pool.
    fn new(repository_strings: &'a StringPool) -> Self {
        Self {
            repository_strings,
            image_strings: StringPool::new(),
            image_id_by_live_id: HashMap::new(),
        }
    }

    /// Dehydrate one live payload into one persisted artifact wrapper.
    fn dehydrate_payload<I>(mut self, payload: &I) -> PersistedArtifact<I>
    where
        I: Image<Live = I>,
    {
        let payload = I::dehydrate(payload, &mut self);

        PersistedArtifact {
            strings: self.image_strings.into_immutable(),
            payload,
        }
    }

    /// Map one live repository string id into one image-local string id.
    fn dehydrate_string_id(&mut self, string_id: StringId) -> StringId {
        *self
            .image_id_by_live_id
            .entry(string_id)
            .or_insert_with(|| {
                let string = self.repository_strings.get(string_id);
                self.image_strings.intern(&string)
            })
    }
}

impl DehydrationContext for ArtifactDehydrationContext<'_> {
    fn dehydrate_string_id(&mut self, string_id: StringId) -> StringId {
        ArtifactDehydrationContext::dehydrate_string_id(self, string_id)
    }
}

/// String-id mapping state while hydrating one persisted artifact.
struct ArtifactHydrationContext<'a> {
    /// The live repository string pool.
    repository_strings: &'a StringPool,
    /// The persisted image-local string table.
    persisted_strings: &'a ImmutableStringPool,
    /// The live repository id for each persisted string id already seen.
    live_id_by_persisted_id: HashMap<StringId, StringId>,
}

impl<'a> ArtifactHydrationContext<'a> {
    /// Create one hydration context for one persisted string table.
    fn new(repository_strings: &'a StringPool, persisted_strings: &'a ImmutableStringPool) -> Self {
        Self {
            repository_strings,
            persisted_strings,
            live_id_by_persisted_id: HashMap::new(),
        }
    }

    /// Rehydrate one persisted payload back into one live payload.
    fn rehydrate_payload<I>(mut self, payload: I) -> I
    where
        I: Image<Live = I>,
    {
        I::rehydrate(payload, &mut self)
    }

    /// Map one persisted image-local string id into one live repository string id.
    fn hydrate_string_id(&mut self, string_id: StringId) -> StringId {
        *self
            .live_id_by_persisted_id
            .entry(string_id)
            .or_insert_with(|| {
                let string = self.persisted_strings.get(string_id);
                self.repository_strings.intern(string)
            })
    }
}

impl HydrationContext for ArtifactHydrationContext<'_> {
    fn rehydrate_string_id(&mut self, string_id: StringId) -> StringId {
        ArtifactHydrationContext::hydrate_string_id(self, string_id)
    }
}

/// Persistent image context for one module scoped base DIR artifact.
#[derive(Debug, Clone)]
struct BaseDirImageContext {
    /// Hash of the effective compiler configuration and source.
    header_hash: u64,
}

impl BaseDirImageContext {
    /// Build one image header for the base DIR.
    fn header(&self, module_id: ModuleId) -> ArtifactImageHeader {
        ArtifactImageHeader::new(
            ArtifactImageKey::DirBase { module: module_id },
            self.header_hash,
        )
    }
}

/// Persistent image context for one profile scoped DIR artifact.
#[derive(Debug, Clone)]
struct ProfileDirImageContext {
    /// The profile key for the image.
    profile_key: ProfileKey,
    /// Hash of the effective compiler configuration and source.
    header_hash: u64,
}

impl ProfileDirImageContext {
    /// Build one image header for the chosen profile scoped DIR family.
    fn header(&self, image_key: ArtifactImageKey) -> ArtifactImageHeader {
        ArtifactImageHeader::new(image_key, self.header_hash)
    }
}

impl Compiler {
    /// Build one persistent image context for one base DIR artifact.
    fn base_dir_image_context(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Option<BaseDirImageContext> {
        let source_hash = self.module_source_hash(revision, module_id)?;

        // base dir images are scoped by compiler behavior and current source
        let mut hasher = CacheHasher::new();
        hasher.hash_compiler_options(&self.options);
        hasher.hash_value(&source_hash);

        Some(BaseDirImageContext {
            header_hash: hasher.finish(),
        })
    }

    /// Build one persistent image context for one profile scoped DIR artifact.
    fn profile_dir_image_context(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<ProfileDirImageContext> {
        let profile = self.profile(profile_id);
        let source_hash = self.module_source_hash(revision, module_id)?;

        // profile dir images are scoped by compiler behavior, profile identity, and source
        let mut hasher = CacheHasher::new();
        hasher.hash_compiler_options(&self.options);
        hasher.hash_value(&profile.key);
        hasher.hash_value(&source_hash);

        Some(ProfileDirImageContext {
            profile_key: profile.key.clone(),
            header_hash: hasher.finish(),
        })
    }

    /// Load one hydrated DIR image payload.
    fn load_persisted_dir<I>(
        &self,
        revision: Revision,
        expected: ArtifactImageHeader,
    ) -> Result<Option<I>, ArtifactImageError>
    where
        I: DeserializeOwned + Image<Live = I>,
    {
        let Some(image) = self.load_image::<PersistedArtifact<I>>(revision, expected)? else {
            return Ok(None);
        };
        let PersistedArtifact { strings, payload } = image.payload;

        let payload = ArtifactHydrationContext::new(self.repository.strings.as_ref(), &strings)
            .rehydrate_payload(payload);

        Ok(Some(payload))
    }

    /// Persist one dehydrated DIR image payload.
    fn store_persisted_dir<I>(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        header: ArtifactImageHeader,
        payload: &I,
    ) -> Result<(), ArtifactImageError>
    where
        I: Image<Live = I> + Serialize,
    {
        let payload = ArtifactDehydrationContext::new(self.repository.strings.as_ref())
            .dehydrate_payload::<I>(payload);

        self.store_image(revision, artifact_key, header, payload)
    }

    /// Build the current expected base DIR image header.
    pub(crate) fn dir_base_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
    ) -> Option<ArtifactImageHeader> {
        let context = self.base_dir_image_context(revision, module_id)?;

        Some(context.header(module_id))
    }

    /// Build the current expected prepared DIR image header.
    pub(crate) fn dir_prepared_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(revision, module_id, profile_id)?;
        let profile_key = context.profile_key.clone();

        Some(context.header(ArtifactImageKey::DirPrepared {
            module: module_id,
            profile: profile_key,
        }))
    }

    /// Build the current expected resolved DIR image header.
    pub(crate) fn dir_resolved_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(revision, module_id, profile_id)?;
        let profile_key = context.profile_key.clone();

        Some(context.header(ArtifactImageKey::DirResolved {
            module: module_id,
            profile: profile_key,
        }))
    }

    /// Build the current expected declared DIR image header.
    pub(crate) fn dir_declared_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(revision, module_id, profile_id)?;
        let profile_key = context.profile_key.clone();

        Some(context.header(ArtifactImageKey::DirDeclared {
            module: module_id,
            profile: profile_key,
        }))
    }

    /// Build the current expected interface DIR image header.
    pub(crate) fn dir_interface_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(revision, module_id, profile_id)?;
        let profile_key = context.profile_key.clone();

        Some(context.header(ArtifactImageKey::DirInterface {
            module: module_id,
            profile: profile_key,
        }))
    }

    /// Build the current expected analyzed DIR image header.
    pub(crate) fn dir_analyzed_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(revision, module_id, profile_id)?;
        let profile_key = context.profile_key.clone();

        Some(context.header(ArtifactImageKey::DirAnalyzed {
            module: module_id,
            profile: profile_key,
        }))
    }

    /// Build the current expected elaborated DIR image header.
    pub(crate) fn dir_elaborated_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(revision, module_id, profile_id)?;
        let profile_key = context.profile_key.clone();

        Some(context.header(ArtifactImageKey::DirElaborated {
            module: module_id,
            profile: profile_key,
        }))
    }

    /// Build the current expected patched DIR image header.
    pub(crate) fn dir_patched_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.profile_dir_image_context(revision, module_id, profile_id)?;
        let profile_key = context.profile_key.clone();

        Some(context.header(ArtifactImageKey::DirPatched {
            module: module_id,
            profile: profile_key,
        }))
    }

    /// Load one base DIR image.
    pub(crate) fn load_dir_base_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
    ) -> Result<Option<DirBase>, ArtifactImageError> {
        let Some(expected) = self.dir_base_image_header(revision, module_id, artifact_stamp) else {
            return Ok(None);
        };

        self.load_persisted_dir::<DirBase>(revision, expected)
    }

    /// Persist one base DIR image.
    pub(crate) fn store_dir_base_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        dir: &DirBase,
    ) -> Result<(), ArtifactImageError> {
        let Some(header) = self.dir_base_image_header(revision, module_id, artifact_stamp) else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::dir_base(module_id);

        self.store_persisted_dir::<DirBase>(revision, &artifact_key, header, dir)
    }

    /// Load one prepared DIR image.
    pub(crate) fn load_dir_prepared_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Result<Option<DirPrepared>, ArtifactImageError> {
        let Some(expected) =
            self.dir_prepared_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(None);
        };

        self.load_persisted_dir::<DirPrepared>(revision, expected)
    }

    /// Persist one prepared DIR image.
    pub(crate) fn store_dir_prepared_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        artifact_stamp: ArtifactStamp,
        dir: &DirPrepared,
    ) -> Result<(), ArtifactImageError> {
        let Some(header) =
            self.dir_prepared_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::dir_prepared(module_id, profile_id);

        self.store_persisted_dir::<DirPrepared>(revision, &artifact_key, header, dir)
    }

    /// Load one resolved DIR image.
    pub(crate) fn load_dir_resolved_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Result<Option<DirResolved>, ArtifactImageError> {
        let Some(expected) =
            self.dir_resolved_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(None);
        };

        self.load_persisted_dir::<DirResolved>(revision, expected)
    }

    /// Persist one resolved DIR image.
    pub(crate) fn store_dir_resolved_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        artifact_stamp: ArtifactStamp,
        dir: &DirResolved,
    ) -> Result<(), ArtifactImageError> {
        let Some(header) =
            self.dir_resolved_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::dir_resolved(module_id, profile_id);

        self.store_persisted_dir::<DirResolved>(revision, &artifact_key, header, dir)
    }

    /// Load one declared DIR image.
    pub(crate) fn load_dir_declared_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Result<Option<DirDeclared>, ArtifactImageError> {
        let Some(expected) =
            self.dir_declared_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(None);
        };

        self.load_persisted_dir::<DirDeclared>(revision, expected)
    }

    /// Persist one declared DIR image.
    pub(crate) fn store_dir_declared_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        artifact_stamp: ArtifactStamp,
        dir: &DirDeclared,
    ) -> Result<(), ArtifactImageError> {
        let Some(header) =
            self.dir_declared_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::dir_declared(module_id, profile_id);

        self.store_persisted_dir::<DirDeclared>(revision, &artifact_key, header, dir)
    }

    /// Load one interface DIR image.
    pub(crate) fn load_dir_interface_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Result<Option<DirInterface>, ArtifactImageError> {
        let Some(expected) =
            self.dir_interface_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(None);
        };

        self.load_persisted_dir::<DirInterface>(revision, expected)
    }

    /// Persist one interface DIR image.
    pub(crate) fn store_dir_interface_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        artifact_stamp: ArtifactStamp,
        dir: &DirInterface,
    ) -> Result<(), ArtifactImageError> {
        let Some(header) =
            self.dir_interface_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::dir_interface(module_id, profile_id);

        self.store_persisted_dir::<DirInterface>(revision, &artifact_key, header, dir)
    }

    /// Load one analyzed DIR image.
    pub(crate) fn load_dir_analyzed_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Result<Option<DirAnalyzed>, ArtifactImageError> {
        let Some(expected) =
            self.dir_analyzed_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(None);
        };

        self.load_persisted_dir::<DirAnalyzed>(revision, expected)
    }

    /// Persist one analyzed DIR image.
    pub(crate) fn store_dir_analyzed_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        artifact_stamp: ArtifactStamp,
        dir: &DirAnalyzed,
    ) -> Result<(), ArtifactImageError> {
        let Some(header) =
            self.dir_analyzed_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::dir_analyzed(module_id, profile_id);

        self.store_persisted_dir::<DirAnalyzed>(revision, &artifact_key, header, dir)
    }

    /// Load one elaborated DIR image.
    pub(crate) fn load_dir_elaborated_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Result<Option<DirElaborated>, ArtifactImageError> {
        let Some(expected) =
            self.dir_elaborated_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(None);
        };

        self.load_persisted_dir::<DirElaborated>(revision, expected)
    }

    /// Persist one elaborated DIR image.
    pub(crate) fn store_dir_elaborated_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        artifact_stamp: ArtifactStamp,
        dir: &DirElaborated,
    ) -> Result<(), ArtifactImageError> {
        let Some(header) =
            self.dir_elaborated_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::dir_elaborated(module_id, profile_id);

        self.store_persisted_dir::<DirElaborated>(revision, &artifact_key, header, dir)
    }

    /// Load one patched DIR image.
    pub(crate) fn load_dir_patched_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
    ) -> Result<Option<DirPatched>, ArtifactImageError> {
        let Some(expected) =
            self.dir_patched_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(None);
        };

        self.load_persisted_dir::<DirPatched>(revision, expected)
    }

    /// Persist one patched DIR image.
    pub(crate) fn store_dir_patched_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        artifact_stamp: ArtifactStamp,
        dir: &DirPatched,
    ) -> Result<(), ArtifactImageError> {
        let Some(header) =
            self.dir_patched_image_header(revision, module_id, artifact_stamp, profile_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::dir_patched(module_id, profile_id);

        self.store_persisted_dir::<DirPatched>(revision, &artifact_key, header, dir)
    }
}
