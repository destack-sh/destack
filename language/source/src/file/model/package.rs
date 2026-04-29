use std::path::Path;

use serde::{Deserialize, Serialize};

use super::hash::{stable_source_id, stable_source_path};
use crate::Uri;

const PACKAGE_ID_DOMAIN: &[u8] = b"destack.source.package.v1";
const PACKAGE_KIND_PHYSICAL: &[u8] = b"physical";
const PACKAGE_KIND_SYNTHETIC: &[u8] = b"synthetic";
const PACKAGE_KIND_URI: &[u8] = b"uri";

/// Unique identifier for one source package.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PackageId(pub u128);

impl std::fmt::Debug for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:032x}", self.0)
    }
}

impl std::fmt::Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:032x}", self.0)
    }
}

impl PackageId {
    /// Well-known ID for ephemeral packages (e.g., REPL, root module).
    pub const EPHEMERAL: Self = Self(0);

    /// Create a PackageId from a raw hash value.
    pub const fn new(id: u128) -> Self {
        Self(id)
    }

    /// Create a PackageId from a URI (deterministic).
    pub fn from_uri(uri: &Uri) -> Self {
        Self(stable_source_id(
            PACKAGE_ID_DOMAIN,
            &[PACKAGE_KIND_URI, uri.as_ref().as_bytes()],
        ))
    }

    /// Create a PackageId from a directory path (for physical packages).
    pub fn from_path(path: &Path) -> Self {
        let path = stable_source_path(path);
        Self(stable_source_id(
            PACKAGE_ID_DOMAIN,
            &[PACKAGE_KIND_PHYSICAL, path.as_bytes()],
        ))
    }

    /// Create a PackageId for a synthetic package (loose files in a directory).
    pub fn from_synthetic_path(path: &Path) -> Self {
        let path = stable_source_path(path);
        Self(stable_source_id(
            PACKAGE_ID_DOMAIN,
            &[PACKAGE_KIND_SYNTHETIC, path.as_bytes()],
        ))
    }

    /// Get the raw id value.
    pub fn raw(&self) -> u128 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::PackageId;

    #[test]
    fn test_distinguish_physical_and_synthetic_packages() {
        let physical = PackageId::from_path(Path::new("workspace/app"));
        let synthetic = PackageId::from_synthetic_path(Path::new("workspace/app"));

        assert_ne!(physical, synthetic);
    }
}
