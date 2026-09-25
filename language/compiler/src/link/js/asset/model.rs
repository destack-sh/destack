use std::sync::Arc;

use tspp_source::File;

use crate::link::OutputLocation;

/// One linker-local Asset derived from one source module.
#[derive(Debug)]
pub(crate) struct Asset {
    /// The loaded source File.
    file: Arc<File>,
    /// The stable content hash for `[hash]`.
    hash: String,
    /// The emitted extension used for `[ext]`.
    output_extension: String,
    /// The emitted media type used for output files and inline data URLs.
    media_type: String,
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
    /// Create one linker-local Asset.
    pub(crate) fn new(
        file: Arc<File>,
        hash: String,
        output_extension: String,
        media_type: String,
    ) -> Self {
        Self {
            file,
            hash,
            output_extension,
            media_type,
        }
    }

    /// Return the loaded source File.
    pub(crate) fn file(&self) -> &File {
        self.file.as_ref()
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

    /// Return the source byte length.
    pub(crate) fn byte_len(&self) -> u64 {
        self.file.blob().byte_len
    }
}
