use serde::{Deserialize, Serialize};

/// Vendored dependency resolution options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VendorOptions {
    /// Vendored dependency resolution mode.
    pub mode: VendorMode,
}

impl Default for VendorOptions {
    fn default() -> Self {
        Self {
            mode: VendorMode::Auto,
        }
    }
}

impl VendorOptions {
    /// Convert from JSON vendor options.
    pub fn from_json(json: Option<&VendorOptionsJson>) -> Self {
        let Some(json) = json else {
            return Self::default();
        };

        Self {
            mode: json.mode.unwrap_or_default(),
        }
    }
}

/// Vendored dependency resolution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum VendorMode {
    /// Use the vendor directory when it is complete.
    #[default]
    Auto,
    /// Use vendored packages when present and fall back to normal resolution.
    Prefer,
    /// Require every resolved package to be vendored.
    Require,
    /// Ignore vendored packages.
    Ignore,
}

/// Vendor options accepted by `destack.json`.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct VendorOptionsJson {
    /// Vendored dependency resolution mode.
    pub mode: Option<VendorMode>,
}

impl VendorOptionsJson {
    /// Apply explicit vendor options to one normalized value.
    pub fn apply_to(&self, options: &mut VendorOptions) {
        if let Some(mode) = self.mode {
            options.mode = mode;
        }
    }
}

/// Resolve one vendor option value over one parent value.
pub fn vendor_options_with_base(
    parent: &VendorOptions,
    json: Option<&VendorOptionsJson>,
) -> VendorOptions {
    let mut options = *parent;

    if let Some(json) = json {
        json.apply_to(&mut options);
    }

    options
}
