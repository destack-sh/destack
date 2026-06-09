// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::PackageId;

/// External target id crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct TargetId {
    package: PackageId,
    key: String,
}

#[wasm_bindgen]
impl TargetId {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(package: PackageId, key: String) -> Self {
        Self { package, key }
    }

    /// Owning package.
    #[wasm_bindgen(getter, js_name = "package")]
    pub fn package(&self) -> PackageId {
        self.package.clone()
    }

    /// Canonical lowercase hex target key within the package.
    #[wasm_bindgen(getter, js_name = "key")]
    pub fn key(&self) -> String {
        self.key.clone()
    }
}

impl TargetId {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::TargetId {
        bridge::TargetId {
            package: self.package.into_bridge(),
            key: self.key,
        }
    }
}

impl TargetId {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::TargetId) -> Self {
        Self {
            package: PackageId::from_bridge(value.package),
            key: value.key,
        }
    }
}
