use destack_artifact::{OutputContent, OutputFile};
use destack_source::{ModuleId, Uri};

use crate::link::OutputLocation;

/// One linker-local asset payload derived from one source module.
#[derive(Debug, Clone)]
pub(crate) struct Asset {
    /// The source module identity for this asset payload.
    module_id: ModuleId,
    /// The emitted payload content for this asset.
    content: OutputContent,
    /// The stable content hash for `[hash]`.
    hash: String,
    /// The emitted extension used for `[ext]`.
    output_extension: String,
    /// The emitted media type used for output files and inline data URLs.
    media_type: String,
    /// The source URI carried into emitted output metadata.
    source: Uri,
}

/// One final asset reference policy.
#[derive(Debug, Clone)]
pub(crate) enum AssetReference {
    /// Keep the authored reference untouched.
    Original,
    /// Replace the reference with one inline data URL.
    Inline { url: String },
    /// Replace the reference with one emitted output path.
    Emitted { output_location: OutputLocation },
}

impl Asset {
    /// Create one linker-local asset payload.
    pub(crate) fn new(
        module_id: ModuleId,
        content: OutputContent,
        hash: String,
        output_extension: String,
        media_type: String,
        source: Uri,
    ) -> Self {
        Self {
            module_id,
            content,
            hash,
            output_extension,
            media_type,
            source,
        }
    }

    /// Return the source module identity for this asset payload.
    pub(crate) fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Return the emitted payload content for this asset.
    pub(crate) fn content(&self) -> &OutputContent {
        &self.content
    }

    /// Return the stable content hash for `[hash]`.
    pub(crate) fn hash(&self) -> &str {
        &self.hash
    }

    /// Return the emitted extension for `[ext]`.
    pub(crate) fn output_extension(&self) -> &str {
        &self.output_extension
    }

    /// Return the emitted media type for this asset.
    pub(crate) fn media_type(&self) -> &str {
        &self.media_type
    }

    /// Return the output payload size in bytes.
    pub(crate) fn byte_len(&self) -> usize {
        match &self.content {
            OutputContent::Text { code, .. } => code.len(),
            OutputContent::Json { content, .. } => content.len(),
            OutputContent::Binary { bytes, .. } => bytes.len(),
        }
    }

    /// Build one emitted output file for this asset at one concrete location.
    pub(crate) fn output_file(&self, output_location: &OutputLocation) -> OutputFile {
        OutputFile {
            uri: Uri::from_path(output_location.path()),
            content: self.content.clone(),
            source: Some(self.source.clone()),
        }
    }
}
