// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::PackageId;

/// External module id crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ModuleId {
    package: PackageId,
    key: String,
}

#[wasm_bindgen]
impl ModuleId {
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

    /// Canonical lowercase hex module key within the package.
    #[wasm_bindgen(getter, js_name = "key")]
    pub fn key(&self) -> String {
        self.key.clone()
    }
}

impl ModuleId {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ModuleId {
        bridge::ModuleId {
            package: self.package.into_bridge(),
            key: self.key,
        }
    }
}

impl ModuleId {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ModuleId) -> Self {
        Self {
            package: PackageId::from_bridge(value.package),
            key: value.key,
        }
    }
}
