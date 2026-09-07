use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// JavaScript output configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct TargetJsOptions {
    /// JavaScript module format.
    pub module: JsModuleFormat,
    /// ECMAScript target version.
    pub target: EsTarget,
    /// JavaScript output topology.
    pub mode: JsOutputMode,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<PathBuf>,
    /// Dependency and resolution options.
    pub dependencies: JsDependencyOptions,
    /// Asset handling options.
    pub assets: JsAssetOptions,
    /// Output configuration for assembled products.
    pub output: JsOutputOptions,
}

/// Module format for emitted JavaScript output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum JsModuleFormat {
    /// ES2015 modules.
    Es2015,
    /// ES2020 modules.
    Es2020,
    /// ES2022 modules.
    Es2022,
    /// ESNext modules.
    #[default]
    EsNext,
}

impl JsModuleFormat {
    /// Parse a JavaScript module format from config text.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "es2015" | "es6" => Some(Self::Es2015),
            "es2020" => Some(Self::Es2020),
            "es2022" => Some(Self::Es2022),
            "esnext" => Some(Self::EsNext),
            _ => None,
        }
    }
}

/// ECMAScript target for emitted JavaScript output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EsTarget {
    /// ES5.
    Es5,
    /// ES2015.
    Es2015,
    /// ES2016.
    Es2016,
    /// ES2017.
    Es2017,
    /// ES2018.
    Es2018,
    /// ES2019.
    Es2019,
    /// ES2020.
    Es2020,
    /// ES2021.
    Es2021,
    /// ES2022.
    Es2022,
    /// ES2023.
    Es2023,
    /// ES2024.
    Es2024,
    /// ESNext.
    #[default]
    EsNext,
}

impl EsTarget {
    /// Parse an ECMAScript target from config text.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "es5" => Some(Self::Es5),
            "es2015" | "es6" => Some(Self::Es2015),
            "es2016" => Some(Self::Es2016),
            "es2017" => Some(Self::Es2017),
            "es2018" => Some(Self::Es2018),
            "es2019" => Some(Self::Es2019),
            "es2020" => Some(Self::Es2020),
            "es2021" => Some(Self::Es2021),
            "es2022" => Some(Self::Es2022),
            "es2023" => Some(Self::Es2023),
            "es2024" => Some(Self::Es2024),
            "esnext" => Some(Self::EsNext),
            _ => None,
        }
    }
}

/// Format for assembled JavaScript outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum JsOutputFormat {
    /// Emit ECMAScript modules.
    #[default]
    Esm,
    /// Emit one self executing bundle.
    Iife,
}

/// Output topology for one JavaScript target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum JsOutputMode {
    /// Assemble one entry rooted bundle.
    #[default]
    SingleFile,
    /// Preserve one emitted module file per reachable module.
    PreserveModules,
}

impl JsOutputMode {
    /// Return whether this mode uses entry output paths.
    pub fn uses_entry_output_layout(self) -> bool {
        self == Self::SingleFile
    }
}

/// Legal comment handling for assembled outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum JsLegalComment {
    /// Keep legal comments where they were printed.
    #[default]
    Inline,
    /// Gather legal comments at the end of each output.
    EndOfFile,
    /// Drop legal comments entirely.
    None,
}

/// Asset handling policy for one JavaScript target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum JsAssetMode {
    /// Emit referenced assets as output files.
    #[default]
    Emit,
    /// Inline referenced assets into the consumer output.
    Inline,
    /// Leave referenced assets external and do not emit them.
    Reference,
}

/// JavaScript dependency options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct JsDependencyOptions {
    /// Module specifiers to leave external.
    pub external: Vec<String>,
    /// Module specifiers that must remain external.
    pub never_bundle: Vec<String>,
    /// Module specifiers that must always be bundled.
    pub always_bundle: Vec<String>,
    /// Module specifiers that are the only allowed bundle inputs.
    pub only_bundle: Vec<String>,
}

/// JavaScript asset handling options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct JsAssetOptions {
    /// Asset handling mode for referenced assets.
    pub mode: JsAssetMode,
    /// Inline asset payloads smaller than this many bytes.
    pub inline_limit: Option<u64>,
}

/// JavaScript output options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct JsOutputOptions {
    /// Format for assembled JavaScript outputs.
    pub format: Option<JsOutputFormat>,
    /// Global name for IIFE bundles.
    pub name: Option<String>,
    /// Output naming template for entry files.
    pub entry_file_names: Option<String>,
    /// Output naming template for assets.
    pub asset_file_names: Option<String>,
    /// Public path prefix for runtime asset resolution.
    pub public_path: Option<String>,
    /// Whether to emit one build manifest.
    pub manifest: bool,
    /// Legal comment handling policy.
    pub legal_comments: JsLegalComment,
    /// Banner text to prepend to each emitted bundle.
    pub banner: Option<String>,
    /// Footer text to append to each emitted bundle.
    pub footer: Option<String>,
    /// Whether to omit source contents from source maps.
    pub source_map_exclude_sources: bool,
}
