use serde::Serialize;
use wasm_bindgen::prelude::*;

use super::error::to_js_value;

/// Capability flags for this wasm build.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmCapabilities {
    /// The selected preset name.
    pub preset: String,
    /// Whether query APIs are enabled.
    pub query: bool,
    /// Whether lint support is enabled.
    pub lint: bool,
    /// Whether optimize support is enabled.
    pub optimize: bool,
    /// Whether format APIs are enabled.
    pub format_api: bool,
    /// Whether transform APIs are enabled.
    pub transform_api: bool,
    /// Whether parallel support is enabled.
    pub parallel: bool,
    /// Whether native code generation is enabled.
    pub native_codegen: bool,
    /// Whether deadlock detection is enabled.
    pub deadlock_detection: bool,
    /// Whether full builtin libraries are enabled.
    pub builtin_full: bool,
    /// Whether core builtin libraries are enabled.
    pub builtin_core: bool,
}

/// Return the capabilities enabled in this wasm build.
#[wasm_bindgen(js_name = capabilities)]
pub fn capabilities() -> Result<JsValue, JsValue> {
    to_js_value(&WasmCapabilities {
        preset: capability_preset_name().to_string(),
        query: cfg!(feature = "query"),
        lint: cfg!(feature = "lint"),
        optimize: cfg!(feature = "optimize"),
        format_api: cfg!(feature = "format-api"),
        transform_api: cfg!(feature = "transform-api"),
        parallel: cfg!(feature = "parallel"),
        native_codegen: cfg!(feature = "native-codegen"),
        deadlock_detection: cfg!(feature = "deadlock-detection"),
        builtin_full: cfg!(feature = "builtin-full"),
        builtin_core: cfg!(feature = "builtin-core"),
    })
}

/// Resolve the active preset name.
fn capability_preset_name() -> &'static str {
    if cfg!(feature = "core") {
        return "core";
    }

    if cfg!(feature = "ide") {
        return "ide";
    }

    if cfg!(feature = "run") {
        return "run";
    }

    if cfg!(feature = "full") {
        return "full";
    }

    "custom"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keep preset naming stable for host capability checks.
    #[test]
    fn test_capability_preset_name_is_supported() {
        let preset = capability_preset_name();

        assert!(matches!(preset, "core" | "ide" | "run" | "full" | "custom"));
    }

    /// Keep capability flags aligned with compile time cfg values.
    #[test]
    fn test_capability_flags_match_cfg() {
        let capabilities = WasmCapabilities {
            preset: capability_preset_name().to_string(),
            query: cfg!(feature = "query"),
            lint: cfg!(feature = "lint"),
            optimize: cfg!(feature = "optimize"),
            format_api: cfg!(feature = "format-api"),
            transform_api: cfg!(feature = "transform-api"),
            parallel: cfg!(feature = "parallel"),
            native_codegen: cfg!(feature = "native-codegen"),
            deadlock_detection: cfg!(feature = "deadlock-detection"),
            builtin_full: cfg!(feature = "builtin-full"),
            builtin_core: cfg!(feature = "builtin-core"),
        };

        assert_eq!(capabilities.query, cfg!(feature = "query"));
        assert_eq!(capabilities.lint, cfg!(feature = "lint"));
        assert_eq!(capabilities.optimize, cfg!(feature = "optimize"));
        assert_eq!(capabilities.format_api, cfg!(feature = "format-api"));
        assert_eq!(capabilities.transform_api, cfg!(feature = "transform-api"));
        assert_eq!(capabilities.parallel, cfg!(feature = "parallel"));
        assert_eq!(
            capabilities.native_codegen,
            cfg!(feature = "native-codegen")
        );
        assert_eq!(
            capabilities.deadlock_detection,
            cfg!(feature = "deadlock-detection")
        );
        assert_eq!(capabilities.builtin_full, cfg!(feature = "builtin-full"));
        assert_eq!(capabilities.builtin_core, cfg!(feature = "builtin-core"));
    }
}
