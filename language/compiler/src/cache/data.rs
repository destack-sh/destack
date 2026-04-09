use crate::compile::Compiler;

use destack_artifact::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey,
    ArtifactStamp, Data, Loader, hash_bytes,
};
use destack_source::{File, FileContent, FileType, ModuleId};
use destack_workspace::Revision;

use super::CacheHasher;

/// Persistent image context for one module scoped data artifact.
#[derive(Debug, Clone)]
pub(crate) struct DataImageContext {
    /// Hash of the source file content.
    source_hash: u64,
    /// The file type shaping the parse result.
    file_type: FileType,
    /// The loader shaping the parse result.
    loader: Loader,
}

impl DataImageContext {
    /// Build one image header for a stable data image key.
    fn header(&self, module_id: ModuleId) -> ArtifactImageHeader {
        let mut hasher = CacheHasher::new();
        hasher.hash_value(&self.source_hash);
        hasher.hash_value(&self.file_type);
        hasher.hash_value(&self.loader);

        ArtifactImageHeader::new(
            ArtifactImageKey::Data { module: module_id },
            hasher.finish(),
        )
    }
}

impl Compiler {
    /// Build the current expected data image header.
    pub(crate) fn data_image_header(
        &self,
        module_id: ModuleId,
        file: &File,
        loader: Loader,
    ) -> Option<ArtifactImageHeader> {
        let context = self.data_image_context(file, loader)?;

        Some(context.header(module_id))
    }

    /// Build the current expected data image header for one module.
    pub(crate) fn current_data_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Option<ArtifactImageHeader> {
        let module = self.cache_module_snapshot(revision, module_id).ok()?;
        let file = self.cache_file_snapshot(revision, module.file_id).ok()?;

        self.data_image_header(module_id, &file, module.loader)
    }

    /// Load one persisted data image entry when disk mode is enabled.
    fn load_data_image_entry(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        file: &File,
        loader: Loader,
    ) -> Result<Option<ArtifactImage<Data>>, ArtifactImageError> {
        let Some(context) = self.data_image_context(file, loader) else {
            return Ok(None);
        };

        let expected = context.header(module_id);
        let Some(image) = self.load_image::<Data>(revision, expected)? else {
            return Ok(None);
        };

        Ok(Some(image))
    }

    /// Build one persistent image context for one parsed data module.
    fn data_image_context(&self, file: &File, loader: Loader) -> Option<DataImageContext> {
        let source_hash = match file.content.payload() {
            FileContent::Text { content } => hash_bytes(content.as_bytes()),
            FileContent::Binary { content } => hash_bytes(content),
        };

        Some(DataImageContext {
            source_hash,
            file_type: file.ty,
            loader,
        })
    }

    /// Load one persisted data image when disk mode is enabled.
    pub(crate) fn load_data_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        file: &File,
        loader: Loader,
    ) -> Result<Option<Data>, ArtifactImageError> {
        let Some(image) =
            self.load_data_image_entry(revision, module_id, artifact_stamp, file, loader)?
        else {
            return Ok(None);
        };

        Ok(Some(image.payload))
    }

    /// Persist one data image when disk mode is enabled.
    pub(crate) fn store_data_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        file: &File,
        loader: Loader,
        data: &Data,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.data_image_context(file, loader) else {
            return Ok(());
        };

        let artifact_key = ArtifactKey::data(module_id);
        let header = context.header(module_id);
        let payload = data.clone();

        self.store_image(revision, &artifact_key, header, payload)
    }
}
