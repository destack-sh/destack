use std::hash::{Hash, Hasher};

use indexmap::IndexMap;
use serde::Deserialize;

use super::output::SourceMapMode;

/// Bundler format for assembled JavaScript outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BundleFormat {
    /// Emit ECMAScript modules.
    #[default]
    Esm,
    /// Emit CommonJS modules.
    Cjs,
    /// Emit one self executing bundle.
    Iife,
}

/// Assembly mode for one script target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum BundleMode {
    /// Assemble one entry rooted bundle.
    #[default]
    SingleFile,
    /// Preserve one emitted module file per reachable module.
    PreserveModules,
    /// Assemble one chunk graph with multiple linked outputs.
    Chunked,
}

impl BundleMode {
    /// Return whether this mode emits entry or chunk collections instead of module trees.
    pub fn uses_entry_output_layout(self) -> bool {
        matches!(self, Self::SingleFile | Self::Chunked)
    }
}

/// Legal comment handling for assembled outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum BundleLegalComment {
    /// Keep legal comments where they were printed.
    #[default]
    Inline,
    /// Gather legal comments at the end of each output.
    EndOfFile,
    /// Drop legal comments entirely.
    None,
}

/// Export mode for one assembled bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BundleExportsMode {
    /// Infer the best export mode from the bundle shape.
    #[default]
    Auto,
    /// Prefer a default export wrapper.
    Default,
    /// Prefer named export bindings.
    Named,
    /// Omit export bindings entirely.
    None,
}

/// Interop mode for external modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum BundleInteropMode {
    /// Use compatibility interop.
    Compat,
    /// Infer interop from the dependency shape.
    #[default]
    Auto,
    /// Treat externals as ES modules.
    EsModule,
    /// Prefer default interop helpers.
    Default,
    /// Only allow default interop.
    DefaultOnly,
}

/// `esModule` output mode for CommonJS style bundles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum BundleEsModuleMode {
    /// Always emit `__esModule`.
    Always,
    /// Never emit `__esModule`.
    Never,
    /// Emit `__esModule` only for default property cases.
    IfDefaultProp,
}

/// Generated code preset for one output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TargetGeneratedCodePreset {
    /// Favor ES5 compatible output forms.
    Es5,
    /// Favor ES2015 compatible output forms.
    #[default]
    Es2015,
}

/// Side effect policy for tree shaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum BundleSideEffectMode {
    /// Respect package and module side effect metadata.
    #[default]
    Auto,
    /// Treat every reachable module as side effectful.
    Keep,
    /// Treat modules as side effect free unless Destack marks them otherwise.
    Drop,
}

/// Asset handling policy for one script target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum BundleAssetMode {
    /// Emit referenced assets as output files.
    #[default]
    Emit,
    /// Inline referenced assets into the consumer output.
    Inline,
    /// Leave referenced assets external and do not emit them.
    Reference,
}

/// Bundler dependency options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetDependencyOptions {
    /// Module specifiers to leave external.
    pub external: Vec<String>,
    /// Module specifiers that must remain external.
    pub never_bundle: Vec<String>,
    /// Module specifiers that must always be bundled.
    pub always_bundle: Vec<String>,
    /// Module specifiers that are the only allowed bundle inputs.
    pub only_bundle: Vec<String>,
}

impl Hash for TargetDependencyOptions {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.external.hash(state);
        self.never_bundle.hash(state);
        self.always_bundle.hash(state);
        self.only_bundle.hash(state);
    }
}

/// Bundler asset handling options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetAssetOptions {
    /// Asset handling mode for referenced assets.
    pub mode: BundleAssetMode,
    /// Inline asset payloads smaller than this many bytes.
    pub inline_limit: Option<u64>,
}

/// Bundler tree shaking options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetTreeshakeOptions {
    /// Whether to tree shake unused modules and exports.
    pub enabled: bool,
    /// Side effect policy for individual modules.
    pub module_side_effects: BundleSideEffectMode,
    /// Side effect policy for packages and dependency boundaries.
    pub package_side_effects: BundleSideEffectMode,
}

impl TargetTreeshakeOptions {
    /// Return whether tree shaking is active.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Bundler minification options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetMinifyOptions {
    /// Whether to minify final bundled output.
    pub enabled: bool,
    /// Whether to minify syntax forms.
    pub syntax: bool,
    /// Whether to minify whitespace.
    pub whitespace: bool,
    /// Whether to minify identifiers.
    pub identifiers: bool,
    /// Whether to preserve function and class names.
    pub keep_names: bool,
}

impl TargetMinifyOptions {
    /// Return whether any minification pass is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled || self.syntax || self.whitespace || self.identifiers
    }

    /// Return whether output text should be compacted or structurally minified.
    pub fn minifies_output(&self) -> bool {
        self.enabled || self.syntax || self.whitespace
    }
}

/// Generated code controls for one output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetGeneratedCodeOptions {
    /// Base preset for generated code features.
    pub preset: Option<TargetGeneratedCodePreset>,
    /// Whether to emit arrow functions where possible.
    pub arrow_functions: Option<bool>,
    /// Whether to emit `const` bindings where possible.
    pub const_bindings: Option<bool>,
    /// Whether to emit object shorthand properties.
    pub object_shorthand: Option<bool>,
    /// Whether to preserve reserved names as properties.
    pub reserved_names_as_props: Option<bool>,
    /// Whether to emit symbol based helpers.
    pub symbols: Option<bool>,
}

/// Bundler output options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetOutputPolicy {
    /// Bundle format for assembled JavaScript outputs.
    pub format: Option<BundleFormat>,
    /// Global name for IIFE bundles.
    pub name: Option<String>,
    /// Output naming template for entry chunks.
    pub entry_file_names: Option<String>,
    /// Output naming template for shared chunks.
    pub chunk_file_names: Option<String>,
    /// Output naming template for assets.
    pub asset_file_names: Option<String>,
    /// Public path prefix for runtime asset resolution.
    pub public_path: Option<String>,
    /// Whether to emit one build manifest.
    pub manifest: bool,
    /// Legal comment handling policy.
    pub legal_comments: BundleLegalComment,
    /// Banner text to prepend to each emitted bundle.
    pub banner: Option<String>,
    /// Footer text to append to each emitted bundle.
    pub footer: Option<String>,
    /// Export mode for the assembled output.
    pub exports: Option<BundleExportsMode>,
    /// Interop mode for external modules.
    pub interop: Option<BundleInteropMode>,
    /// Generated code controls for final output rendering.
    pub generated_code: Option<TargetGeneratedCodeOptions>,
    /// Whether to freeze namespace imports and export objects.
    pub freeze: Option<bool>,
    /// Whether to emit `__esModule` markers for CommonJS output.
    pub es_module: Option<BundleEsModuleMode>,
    /// Whether to preserve external live bindings in output wrappers.
    pub external_live_bindings: bool,
    /// Whether to hoist transitive imports on entry facades.
    pub hoist_transitive_imports: bool,
    /// Whether to minify internal export names.
    pub minify_internal_exports: bool,
    /// Source map emission mode for bundled script output.
    pub sourcemap: Option<SourceMapMode>,
    /// Whether to omit source contents from source maps.
    pub sourcemap_exclude_sources: bool,
    /// Whether to include debug ids in source maps.
    pub sourcemap_debug_ids: bool,
    /// Global names for externals in IIFE format.
    pub globals: IndexMap<String, String>,
}

impl Hash for TargetOutputPolicy {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.format.hash(state);
        self.name.hash(state);
        self.entry_file_names.hash(state);
        self.chunk_file_names.hash(state);
        self.asset_file_names.hash(state);
        self.public_path.hash(state);
        self.manifest.hash(state);
        self.legal_comments.hash(state);
        self.banner.hash(state);
        self.footer.hash(state);
        self.exports.hash(state);
        self.interop.hash(state);
        self.generated_code.hash(state);
        self.freeze.hash(state);
        self.es_module.hash(state);
        self.external_live_bindings.hash(state);
        self.hoist_transitive_imports.hash(state);
        self.minify_internal_exports.hash(state);
        self.sourcemap.hash(state);
        self.sourcemap_exclude_sources.hash(state);
        self.sourcemap_debug_ids.hash(state);

        self.globals.len().hash(state);
        for (name, value) in &self.globals {
            name.hash(state);
            value.hash(state);
        }
    }
}

/// Bundler dependency options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetDependencyOptionsJson {
    /// Module specifiers to leave external.
    pub external: Option<Vec<String>>,
    /// Module specifiers that must remain external.
    pub never_bundle: Option<Vec<String>>,
    /// Module specifiers that must always be bundled.
    pub always_bundle: Option<Vec<String>>,
    /// Module specifiers that are the only allowed bundle inputs.
    pub only_bundle: Option<Vec<String>>,
}

impl From<&TargetDependencyOptionsJson> for TargetDependencyOptions {
    fn from(json: &TargetDependencyOptionsJson) -> Self {
        Self {
            external: json.external.clone().unwrap_or_default(),
            never_bundle: json.never_bundle.clone().unwrap_or_default(),
            always_bundle: json.always_bundle.clone().unwrap_or_default(),
            only_bundle: json.only_bundle.clone().unwrap_or_default(),
        }
    }
}

/// Bundler asset handling options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetAssetOptionsJson {
    /// Asset handling mode for referenced assets.
    pub mode: Option<BundleAssetMode>,
    /// Inline asset payloads smaller than this many bytes.
    pub inline_limit: Option<u64>,
}

impl From<&TargetAssetOptionsJson> for TargetAssetOptions {
    fn from(json: &TargetAssetOptionsJson) -> Self {
        Self {
            mode: json.mode.unwrap_or_default(),
            inline_limit: json.inline_limit,
        }
    }
}

/// Bundler tree shaking options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetTreeshakeConfigJson {
    /// Whether to tree shake unused modules and exports.
    #[serde(default)]
    pub enabled: bool,
    /// Side effect policy for individual modules.
    pub module_side_effects: Option<BundleSideEffectMode>,
    /// Side effect policy for packages and dependency boundaries.
    pub package_side_effects: Option<BundleSideEffectMode>,
}

/// Bundler tree shaking options in `destack.json`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum TargetTreeshakeOptionsJson {
    /// Enable or disable tree shaking with one boolean.
    Enabled(bool),
    /// Configure tree shaking with one explicit options object.
    Options(TargetTreeshakeConfigJson),
}

impl From<&TargetTreeshakeOptionsJson> for TargetTreeshakeOptions {
    fn from(json: &TargetTreeshakeOptionsJson) -> Self {
        match json {
            TargetTreeshakeOptionsJson::Enabled(enabled) => Self {
                enabled: *enabled,
                ..Self::default()
            },
            TargetTreeshakeOptionsJson::Options(options) => Self {
                enabled: options.enabled,
                module_side_effects: options.module_side_effects.unwrap_or_default(),
                package_side_effects: options.package_side_effects.unwrap_or_default(),
            },
        }
    }
}

/// Bundler source map policy in `destack.json`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum TargetSourceMapPolicyJson {
    /// Enable or disable external source maps with one boolean.
    Enabled(bool),
    /// Select one explicit source map mode.
    Mode(SourceMapMode),
}

impl TargetSourceMapPolicyJson {
    /// Convert this JSON surface into one normalized source map mode.
    pub fn mode(&self) -> Option<SourceMapMode> {
        match self {
            Self::Enabled(true) => Some(SourceMapMode::External),
            Self::Enabled(false) => None,
            Self::Mode(mode) => Some(*mode),
        }
    }
}

/// `esModule` output mode in `destack.json`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum TargetEsModulePolicyJson {
    /// Enable or disable `__esModule` emission with one boolean.
    Enabled(bool),
    /// Select one explicit `esModule` policy.
    Mode(BundleEsModuleMode),
}

impl TargetEsModulePolicyJson {
    /// Convert this JSON surface into one normalized `esModule` policy.
    pub fn mode(&self) -> BundleEsModuleMode {
        match self {
            Self::Enabled(true) => BundleEsModuleMode::Always,
            Self::Enabled(false) => BundleEsModuleMode::Never,
            Self::Mode(mode) => *mode,
        }
    }
}

/// Generated code options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetGeneratedCodeConfigJson {
    /// Base preset for generated code features.
    pub preset: Option<TargetGeneratedCodePreset>,
    /// Whether to emit arrow functions where possible.
    pub arrow_functions: Option<bool>,
    /// Whether to emit `const` bindings where possible.
    pub const_bindings: Option<bool>,
    /// Whether to emit object shorthand properties.
    pub object_shorthand: Option<bool>,
    /// Whether to preserve reserved names as properties.
    pub reserved_names_as_props: Option<bool>,
    /// Whether to emit symbol based helpers.
    pub symbols: Option<bool>,
}

/// Generated code controls in `destack.json`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum TargetGeneratedCodeOptionsJson {
    /// Select one named generated code preset.
    Preset(TargetGeneratedCodePreset),
    /// Configure generated code with one explicit options object.
    Options(TargetGeneratedCodeConfigJson),
}

impl From<&TargetGeneratedCodeOptionsJson> for TargetGeneratedCodeOptions {
    fn from(json: &TargetGeneratedCodeOptionsJson) -> Self {
        match json {
            TargetGeneratedCodeOptionsJson::Preset(preset) => Self {
                preset: Some(*preset),
                ..Self::default()
            },
            TargetGeneratedCodeOptionsJson::Options(options) => Self {
                preset: options.preset,
                arrow_functions: options.arrow_functions,
                const_bindings: options.const_bindings,
                object_shorthand: options.object_shorthand,
                reserved_names_as_props: options.reserved_names_as_props,
                symbols: options.symbols,
            },
        }
    }
}

/// Bundler output options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetOutputPolicyJson {
    /// Bundle format for assembled JavaScript outputs.
    pub format: Option<BundleFormat>,
    /// Global name for IIFE bundles.
    pub name: Option<String>,
    /// Output naming template for entry chunks.
    pub entry_file_names: Option<String>,
    /// Output naming template for shared chunks.
    pub chunk_file_names: Option<String>,
    /// Output naming template for assets.
    pub asset_file_names: Option<String>,
    /// Public path prefix for runtime asset resolution.
    pub public_path: Option<String>,
    /// Whether to emit one build manifest.
    #[serde(default)]
    pub manifest: bool,
    /// Legal comment handling policy.
    pub legal_comments: Option<BundleLegalComment>,
    /// Banner text to prepend to each emitted bundle.
    pub banner: Option<String>,
    /// Footer text to append to each emitted bundle.
    pub footer: Option<String>,
    /// Export mode for the assembled output.
    pub exports: Option<BundleExportsMode>,
    /// Interop mode for external modules.
    pub interop: Option<BundleInteropMode>,
    /// Generated code controls for final output rendering.
    pub generated_code: Option<TargetGeneratedCodeOptionsJson>,
    /// Whether to freeze namespace imports and export objects.
    pub freeze: Option<bool>,
    /// Whether to emit `__esModule` markers for CommonJS output.
    pub es_module: Option<TargetEsModulePolicyJson>,
    /// Whether to preserve external live bindings in output wrappers.
    #[serde(default)]
    pub external_live_bindings: bool,
    /// Whether to hoist transitive imports on entry facades.
    #[serde(default)]
    pub hoist_transitive_imports: bool,
    /// Whether to minify internal export names.
    #[serde(default)]
    pub minify_internal_exports: bool,
    /// Source map emission policy.
    pub sourcemap: Option<TargetSourceMapPolicyJson>,
    /// Whether to omit source contents from source maps.
    #[serde(default)]
    pub sourcemap_exclude_sources: bool,
    /// Whether to include debug ids in source maps.
    #[serde(default)]
    pub sourcemap_debug_ids: bool,
    /// Global names for externals in IIFE format.
    pub globals: Option<IndexMap<String, String>>,
}

impl From<&TargetOutputPolicyJson> for TargetOutputPolicy {
    fn from(json: &TargetOutputPolicyJson) -> Self {
        Self {
            format: json.format,
            name: json.name.clone(),
            entry_file_names: json.entry_file_names.clone(),
            chunk_file_names: json.chunk_file_names.clone(),
            asset_file_names: json.asset_file_names.clone(),
            public_path: json.public_path.clone(),
            manifest: json.manifest,
            legal_comments: json.legal_comments.unwrap_or_default(),
            banner: json.banner.clone(),
            footer: json.footer.clone(),
            exports: json.exports,
            interop: json.interop,
            generated_code: json
                .generated_code
                .as_ref()
                .map(TargetGeneratedCodeOptions::from),
            freeze: json.freeze,
            es_module: json.es_module.as_ref().map(TargetEsModulePolicyJson::mode),
            external_live_bindings: json.external_live_bindings,
            hoist_transitive_imports: json.hoist_transitive_imports,
            minify_internal_exports: json.minify_internal_exports,
            sourcemap: json
                .sourcemap
                .as_ref()
                .and_then(TargetSourceMapPolicyJson::mode),
            sourcemap_exclude_sources: json.sourcemap_exclude_sources,
            sourcemap_debug_ids: json.sourcemap_debug_ids,
            globals: json.globals.clone().unwrap_or_default(),
        }
    }
}

/// Bundler minification options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetMinifyConfigJson {
    /// Whether to minify final bundled output.
    #[serde(default)]
    pub enabled: bool,
    /// Whether to minify syntax forms.
    #[serde(default)]
    pub syntax: bool,
    /// Whether to minify whitespace.
    #[serde(default)]
    pub whitespace: bool,
    /// Whether to minify identifiers.
    #[serde(default)]
    pub identifiers: bool,
    /// Whether to preserve function and class names.
    #[serde(default)]
    pub keep_names: bool,
}

/// Bundler minification options in `destack.json`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum TargetMinifyOptionsJson {
    /// Enable or disable full minification with one boolean.
    Enabled(bool),
    /// Configure minification with one explicit options object.
    Options(TargetMinifyConfigJson),
}

impl From<&TargetMinifyOptionsJson> for TargetMinifyOptions {
    fn from(json: &TargetMinifyOptionsJson) -> Self {
        match json {
            TargetMinifyOptionsJson::Enabled(enabled) => {
                if !enabled {
                    return Self::default();
                }

                Self {
                    enabled: true,
                    syntax: true,
                    whitespace: true,
                    identifiers: false,
                    keep_names: false,
                }
            }
            TargetMinifyOptionsJson::Options(options) => Self {
                enabled: options.enabled,
                syntax: options.syntax,
                whitespace: options.whitespace,
                identifiers: options.identifiers,
                keep_names: options.keep_names,
            },
        }
    }
}
