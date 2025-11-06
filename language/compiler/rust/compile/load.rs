use dyst_source::Uri;

/// Request to load a file.
#[derive(Debug, Clone)]
pub enum LoadRequest {
    /// Load a file.
    LoadFile { path: Uri },
}
