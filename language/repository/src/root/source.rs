use std::path::{Path, PathBuf};

use tspp_source::FileSystem;

use crate::{DestackFile, RepositoryError};

/// A source root discovered from one filesystem path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceRoot {
    /// A root declared by a `destack.json` manifest.
    Declared(PathBuf),
    /// A root implied by a source path outside a declared package.
    Implicit(PathBuf),
}

impl SourceRoot {
    /// Discover the nearest source root for one filesystem path.
    pub fn discover(file_system: &dyn FileSystem, path: &Path) -> Result<Self, RepositoryError> {
        let metadata = file_system
            .metadata(path)
            .map_err(|error| RepositoryError::FileSystem {
                operation: "metadata",
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;

        // normalize file inputs to their containing directory
        let directory = if metadata.is_file {
            path.parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| path.to_path_buf())
        } else {
            path.to_path_buf()
        };
        let mut current = directory.clone();

        // walk up directories looking for a declared source root
        loop {
            if DestackFile::read(file_system, &current)?.is_some() {
                return Ok(Self::Declared(current));
            }

            let Some(parent) = current.parent() else {
                break;
            };
            current = parent.to_path_buf();
        }

        Ok(Self::Implicit(directory))
    }
}

impl From<SourceRoot> for PathBuf {
    /// Consume one discovered source root.
    fn from(root: SourceRoot) -> Self {
        match root {
            SourceRoot::Declared(path) | SourceRoot::Implicit(path) => path,
        }
    }
}
