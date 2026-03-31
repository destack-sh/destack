use std::hash::{Hash, Hasher};
use std::path::PathBuf;

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
    /// Emit one UMD bundle.
    Umd,
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
pub enum BundleGeneratedCodePreset {
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

/// Package handling policy for node_modules style imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum BundlePackageMode {
    /// Use the tool default package policy.
    #[default]
    Auto,
    /// Prefer bundling package dependencies.
    Bundle,
    /// Prefer leaving package dependencies external.
    External,
}

/// Bundler dependency and resolution options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetBundleDeps {
    /// Package policy for node_modules style imports.
    pub packages: BundlePackageMode,
    /// Whether to skip resolving and bundling node_modules entries entirely.
    pub skip_node_modules_bundle: bool,
    /// Module specifiers to leave external.
    pub external: Vec<String>,
    /// Module specifiers that must remain external.
    pub never_bundle: Vec<String>,
    /// Module specifiers that must always be bundled.
    pub always_bundle: Vec<String>,
    /// Module specifiers that are the only allowed bundle inputs.
    pub only_bundle: Vec<String>,
    /// Module specifier aliases.
    pub alias: IndexMap<String, String>,
    /// Resolution conditions to prefer.
    pub conditions: Vec<String>,
    /// Package.json fields to prefer during resolution.
    pub main_fields: Vec<String>,
}

impl Hash for TargetBundleDeps {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.packages.hash(state);
        self.skip_node_modules_bundle.hash(state);
        self.external.hash(state);
        self.never_bundle.hash(state);
        self.always_bundle.hash(state);
        self.only_bundle.hash(state);
        self.conditions.hash(state);
        self.main_fields.hash(state);

        self.alias.len().hash(state);
        for (from, to) in &self.alias {
            from.hash(state);
            to.hash(state);
        }
    }
}

/// Bundler asset handling options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetBundleAssets {
    /// Asset handling mode for referenced assets.
    pub mode: BundleAssetMode,
    /// Inline asset payloads smaller than this many bytes.
    pub inline_limit: Option<u64>,
}

/// Bundler tree shaking options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetBundleTreeshake {
    /// Whether to tree shake unused modules and exports.
    pub enabled: bool,
    /// Side effect policy for individual modules.
    pub module_side_effects: BundleSideEffectMode,
    /// Side effect policy for packages and dependency boundaries.
    pub package_side_effects: BundleSideEffectMode,
}

impl TargetBundleTreeshake {
    /// Return whether tree shaking is active.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Bundler minification options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetBundleMinify {
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

impl TargetBundleMinify {
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
pub struct BundleGeneratedCode {
    /// Base preset for generated code features.
    pub preset: Option<BundleGeneratedCodePreset>,
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
pub struct TargetBundleOutput {
    /// Bundle format for assembled JavaScript outputs.
    pub format: Option<BundleFormat>,
    /// Global name for IIFE and UMD bundles.
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
    pub generated_code: Option<BundleGeneratedCode>,
    /// Whether to freeze namespace imports and export objects.
    pub freeze: Option<bool>,
    /// Whether to emit `__esModule` markers for CommonJS output.
    pub es_module: Option<BundleEsModuleMode>,
    /// Whether to rewrite dynamic imports for CommonJS output.
    pub dynamic_import_in_cjs: Option<bool>,
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
    /// Global names for externals in IIFE and UMD formats.
    pub globals: IndexMap<String, String>,
}

impl Hash for TargetBundleOutput {
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
        self.dynamic_import_in_cjs.hash(state);
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

/// Target scoped bundling options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetBundle {
    /// Assembly topology for this script target.
    pub assembly: BundleMode,
    /// Whether to preserve one emitted module per reachable source module.
    pub preserve_modules: bool,
    /// Whether to inline dynamic imports into the current output.
    pub inline_dynamic_imports: bool,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<PathBuf>,
    /// Manual chunk assignments keyed by chunk name.
    pub manual_chunks: IndexMap<String, Vec<String>>,
    /// Whether to only honor explicit manual chunk declarations.
    pub only_explicit_manual_chunks: bool,
    /// Dependency and resolution options.
    pub dependencies: TargetBundleDeps,
    /// Asset handling options.
    pub assets: TargetBundleAssets,
    /// Tree shaking options.
    pub treeshake: TargetBundleTreeshake,
    /// Output configuration for bundled products.
    pub output: TargetBundleOutput,
    /// Compile time define replacements.
    pub define: IndexMap<String, String>,
    /// Minification options.
    pub minify: TargetBundleMinify,
}

impl TargetBundle {
    /// Return whether this bundler policy emits entry or chunk collections.
    pub fn uses_entry_output_layout(&self) -> bool {
        self.assembly.uses_entry_output_layout()
    }
}

impl Hash for TargetBundle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.assembly.hash(state);
        self.preserve_modules.hash(state);
        self.inline_dynamic_imports.hash(state);
        self.preserve_modules_root.hash(state);
        self.only_explicit_manual_chunks.hash(state);
        self.dependencies.hash(state);
        self.assets.hash(state);
        self.treeshake.hash(state);
        self.output.hash(state);
        self.minify.hash(state);

        self.manual_chunks.len().hash(state);
        for (name, modules) in &self.manual_chunks {
            name.hash(state);
            modules.hash(state);
        }

        self.define.len().hash(state);
        for (name, value) in &self.define {
            name.hash(state);
            value.hash(state);
        }
    }
}

impl From<&TargetBundleJson> for TargetBundle {
    fn from(json: &TargetBundleJson) -> Self {
        Self {
            assembly: json.assembly.unwrap_or_default(),
            preserve_modules: json.preserve_modules,
            inline_dynamic_imports: json.inline_dynamic_imports,
            preserve_modules_root: json.preserve_modules_root.as_ref().map(PathBuf::from),
            manual_chunks: json.manual_chunks.clone().unwrap_or_default(),
            only_explicit_manual_chunks: json.only_explicit_manual_chunks,
            dependencies: json
                .dependencies
                .as_ref()
                .map(TargetBundleDeps::from)
                .unwrap_or_default(),
            assets: json
                .assets
                .as_ref()
                .map(TargetBundleAssets::from)
                .unwrap_or_default(),
            treeshake: json
                .treeshake
                .as_ref()
                .map(TargetBundleTreeshake::from)
                .unwrap_or_default(),
            output: json
                .output
                .as_ref()
                .map(TargetBundleOutput::from)
                .unwrap_or_default(),
            define: json.define.clone().unwrap_or_default(),
            minify: json
                .minify
                .as_ref()
                .map(TargetBundleMinify::from)
                .unwrap_or_default(),
        }
    }
}

/// Bundler dependency and resolution options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetBundleDepsJson {
    /// Package policy for node_modules style imports.
    pub packages: Option<BundlePackageMode>,
    /// Whether to skip resolving and bundling node_modules entries entirely.
    #[serde(default)]
    pub skip_node_modules_bundle: bool,
    /// Module specifiers to leave external.
    pub external: Option<Vec<String>>,
    /// Module specifiers that must remain external.
    pub never_bundle: Option<Vec<String>>,
    /// Module specifiers that must always be bundled.
    pub always_bundle: Option<Vec<String>>,
    /// Module specifiers that are the only allowed bundle inputs.
    pub only_bundle: Option<Vec<String>>,
    /// Module specifier aliases.
    pub alias: Option<IndexMap<String, String>>,
    /// Resolution conditions to prefer.
    pub conditions: Option<Vec<String>>,
    /// Package.json fields to prefer during resolution.
    pub main_fields: Option<Vec<String>>,
}

impl From<&TargetBundleDepsJson> for TargetBundleDeps {
    fn from(json: &TargetBundleDepsJson) -> Self {
        Self {
            packages: json.packages.unwrap_or_default(),
            skip_node_modules_bundle: json.skip_node_modules_bundle,
            external: json.external.clone().unwrap_or_default(),
            never_bundle: json.never_bundle.clone().unwrap_or_default(),
            always_bundle: json.always_bundle.clone().unwrap_or_default(),
            only_bundle: json.only_bundle.clone().unwrap_or_default(),
            alias: json.alias.clone().unwrap_or_default(),
            conditions: json.conditions.clone().unwrap_or_default(),
            main_fields: json.main_fields.clone().unwrap_or_default(),
        }
    }
}

/// Bundler asset handling options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetBundleAssetsJson {
    /// Asset handling mode for referenced assets.
    pub mode: Option<BundleAssetMode>,
    /// Inline asset payloads smaller than this many bytes.
    pub inline_limit: Option<u64>,
}

impl From<&TargetBundleAssetsJson> for TargetBundleAssets {
    fn from(json: &TargetBundleAssetsJson) -> Self {
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
pub struct TargetBundleTreeshakeOptionsJson {
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
pub enum TargetBundleTreeshakeJson {
    /// Enable or disable tree shaking with one boolean.
    Enabled(bool),
    /// Configure tree shaking with one explicit options object.
    Options(TargetBundleTreeshakeOptionsJson),
}

impl From<&TargetBundleTreeshakeJson> for TargetBundleTreeshake {
    fn from(json: &TargetBundleTreeshakeJson) -> Self {
        match json {
            TargetBundleTreeshakeJson::Enabled(enabled) => Self {
                enabled: *enabled,
                ..Self::default()
            },
            TargetBundleTreeshakeJson::Options(options) => Self {
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
pub enum TargetBundleSourceMapJson {
    /// Enable or disable external source maps with one boolean.
    Enabled(bool),
    /// Select one explicit source map mode.
    Mode(SourceMapMode),
}

impl TargetBundleSourceMapJson {
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
pub enum TargetBundleEsModuleJson {
    /// Enable or disable `__esModule` emission with one boolean.
    Enabled(bool),
    /// Select one explicit `esModule` policy.
    Mode(BundleEsModuleMode),
}

impl TargetBundleEsModuleJson {
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
pub struct BundleGeneratedCodeOptionsJson {
    /// Base preset for generated code features.
    pub preset: Option<BundleGeneratedCodePreset>,
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
pub enum TargetBundleGeneratedCodeJson {
    /// Select one named generated code preset.
    Preset(BundleGeneratedCodePreset),
    /// Configure generated code with one explicit options object.
    Options(BundleGeneratedCodeOptionsJson),
}

impl From<&TargetBundleGeneratedCodeJson> for BundleGeneratedCode {
    fn from(json: &TargetBundleGeneratedCodeJson) -> Self {
        match json {
            TargetBundleGeneratedCodeJson::Preset(preset) => Self {
                preset: Some(*preset),
                ..Self::default()
            },
            TargetBundleGeneratedCodeJson::Options(options) => Self {
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
pub struct TargetBundleOutputJson {
    /// Bundle format for assembled JavaScript outputs.
    pub format: Option<BundleFormat>,
    /// Global name for IIFE and UMD bundles.
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
    pub generated_code: Option<TargetBundleGeneratedCodeJson>,
    /// Whether to freeze namespace imports and export objects.
    pub freeze: Option<bool>,
    /// Whether to emit `__esModule` markers for CommonJS output.
    pub es_module: Option<TargetBundleEsModuleJson>,
    /// Whether to rewrite dynamic imports for CommonJS output.
    pub dynamic_import_in_cjs: Option<bool>,
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
    pub sourcemap: Option<TargetBundleSourceMapJson>,
    /// Whether to omit source contents from source maps.
    #[serde(default)]
    pub sourcemap_exclude_sources: bool,
    /// Whether to include debug ids in source maps.
    #[serde(default)]
    pub sourcemap_debug_ids: bool,
    /// Global names for externals in IIFE and UMD formats.
    pub globals: Option<IndexMap<String, String>>,
}

impl From<&TargetBundleOutputJson> for TargetBundleOutput {
    fn from(json: &TargetBundleOutputJson) -> Self {
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
            generated_code: json.generated_code.as_ref().map(BundleGeneratedCode::from),
            freeze: json.freeze,
            es_module: json.es_module.as_ref().map(TargetBundleEsModuleJson::mode),
            dynamic_import_in_cjs: json.dynamic_import_in_cjs,
            external_live_bindings: json.external_live_bindings,
            hoist_transitive_imports: json.hoist_transitive_imports,
            minify_internal_exports: json.minify_internal_exports,
            sourcemap: json
                .sourcemap
                .as_ref()
                .and_then(TargetBundleSourceMapJson::mode),
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
pub struct TargetBundleMinifyOptionsJson {
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
pub enum TargetBundleMinifyJson {
    /// Enable or disable full minification with one boolean.
    Enabled(bool),
    /// Configure minification with one explicit options object.
    Options(TargetBundleMinifyOptionsJson),
}

impl From<&TargetBundleMinifyJson> for TargetBundleMinify {
    fn from(json: &TargetBundleMinifyJson) -> Self {
        match json {
            TargetBundleMinifyJson::Enabled(enabled) => {
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
            TargetBundleMinifyJson::Options(options) => Self {
                enabled: options.enabled,
                syntax: options.syntax,
                whitespace: options.whitespace,
                identifiers: options.identifiers,
                keep_names: options.keep_names,
            },
        }
    }
}

/// Target scoped bundling options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetBundleJson {
    /// Assembly topology for this script target.
    #[serde(alias = "mode")]
    pub assembly: Option<BundleMode>,
    /// Whether to preserve one emitted module file per reachable module.
    #[serde(default)]
    pub preserve_modules: bool,
    /// Whether to inline dynamic imports into the current output.
    #[serde(default)]
    pub inline_dynamic_imports: bool,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<String>,
    /// Manual chunk assignments keyed by chunk name.
    pub manual_chunks: Option<IndexMap<String, Vec<String>>>,
    /// Whether to only honor explicit manual chunk declarations.
    #[serde(default)]
    pub only_explicit_manual_chunks: bool,
    /// Dependency and resolution options.
    #[serde(alias = "deps")]
    pub dependencies: Option<TargetBundleDepsJson>,
    /// Asset handling options.
    pub assets: Option<TargetBundleAssetsJson>,
    /// Tree shaking options.
    pub treeshake: Option<TargetBundleTreeshakeJson>,
    /// Output options for assembled bundle files.
    pub output: Option<TargetBundleOutputJson>,
    /// Compile time define replacements.
    pub define: Option<IndexMap<String, String>>,
    /// Minification options.
    pub minify: Option<TargetBundleMinifyJson>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Accept both the canonical assembly key and the legacy mode alias.
    #[test]
    fn test_deserializes_bundle_assembly_and_mode_alias() {
        let assembly_json = r#"{ "assembly": "chunked" }"#;
        let assembly = serde_json::from_str::<TargetBundleJson>(assembly_json)
            .unwrap_or_else(|error| panic!("expected valid assembly json: {error}"));
        assert_eq!(assembly.assembly, Some(BundleMode::Chunked));

        let alias_json = r#"{ "mode": "preserveModules" }"#;
        let alias = serde_json::from_str::<TargetBundleJson>(alias_json)
            .unwrap_or_else(|error| panic!("expected valid mode alias json: {error}"));
        assert_eq!(alias.assembly, Some(BundleMode::PreserveModules));
    }
}
