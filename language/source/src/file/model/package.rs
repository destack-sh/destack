use std::path::Path;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};

use super::hash::{stable_source_id, stable_source_path};
use crate::Uri;

const PACKAGE_DOMAIN: &[u8] = b"tspp.source.package.v1";
const PACKAGE_KIND_PHYSICAL: &[u8] = b"physical";
const PACKAGE_KIND_URI: &[u8] = b"uri";

/// Unique identifier for one source package.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
pub struct PackageId(pub u64);

impl std::fmt::Debug for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:016x}", self.0)
    }
}

impl std::fmt::Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:016x}", self.0)
    }
}

impl PackageId {
    /// Create a PackageId from a raw hash value.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Create a PackageId from a URI (deterministic).
    pub fn from_uri(uri: &Uri) -> Self {
        Self(stable_source_id(
            PACKAGE_DOMAIN,
            &[PACKAGE_KIND_URI, uri.as_ref().as_bytes()],
        ))
    }

    /// Create a PackageId from a directory path (for physical packages).
    pub fn from_path(path: &Path) -> Self {
        debug_assert!(
            !path.is_absolute(),
            "package identities derive from logical paths, got {path:?}"
        );
        let path = stable_source_path(path);
        Self(stable_source_id(
            PACKAGE_DOMAIN,
            &[PACKAGE_KIND_PHYSICAL, path.as_bytes()],
        ))
    }

    /// Get the raw id value.
    pub fn raw(&self) -> u64 {
        self.0
    }
}
