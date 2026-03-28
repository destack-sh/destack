use std::path::PathBuf;

/// One host generator artifact.
pub(crate) struct HostArtifact {
    /// The output path for this artifact.
    pub path: PathBuf,
    /// The rendered artifact contents.
    pub contents: String,
}

impl HostArtifact {
    /// Build one host generator artifact.
    pub(crate) fn new(path: PathBuf, contents: String) -> Self {
        Self { path, contents }
    }
}
