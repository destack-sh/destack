use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::TargetId;

use crate::{ArtifactKey, Host, Platform, Runtime};

/// One linked product assembled from one or more target artifacts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct Product {
    /// The configured product name.
    pub name: String,
    /// The linked targets keyed by configured product target name.
    pub targets: IndexMap<String, ProductTarget>,
}

impl Product {
    /// Create one product.
    pub fn new(name: String, targets: IndexMap<String, ProductTarget>) -> Self {
        Self { name, targets }
    }
}

/// One linked target assembled into a product.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ProductTarget {
    /// The configured product target name.
    pub name: String,
    /// The repository target assembled into this product.
    pub target: TargetId,
    /// The runtime selected by this target.
    pub runtime: Runtime,
    /// The host environment this target expects.
    pub host: Host,
    /// The platform this target expects.
    pub platform: Platform,
    /// Whether this product target includes its toolchain build payload.
    pub includes_build: bool,
    /// Whether this product target includes its linked bundle.
    pub includes_bundle: bool,
    /// Whether this product target includes its linked Program.
    pub includes_program: bool,
}

impl ProductTarget {
    /// Create one product target.
    pub fn new(
        name: String,
        target: TargetId,
        runtime: Runtime,
        host: Host,
        platform: Platform,
        includes_build: bool,
        includes_bundle: bool,
        includes_program: bool,
    ) -> Self {
        Self {
            name,
            target,
            runtime,
            host,
            platform,
            includes_build,
            includes_bundle,
            includes_program,
        }
    }

    /// Return the artifacts included by this product target.
    pub fn artifact_keys(&self) -> Vec<ArtifactKey> {
        let mut keys = Vec::new();

        // include build payload when the product bundles toolchain support
        if self.includes_build {
            keys.push(ArtifactKey::build(self.target));
        }

        // include linked web or resource bundles
        if self.includes_bundle {
            keys.push(ArtifactKey::bundle(self.target.package_id(), self.target));
        }

        // include the linked Program
        if self.includes_program {
            keys.push(ArtifactKey::program(self.target.package_id(), self.target));
        }

        keys
    }
}
