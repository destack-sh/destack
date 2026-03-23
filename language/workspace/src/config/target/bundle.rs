use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use indexmap::IndexMap;
use serde::Deserialize;

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

/// Chunk planning strategy for one assembled script target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum BundleChunkStrategy {
    /// Use the default graph driven chunk planner.
    #[default]
    Auto,
    /// Prefer coarse entry rooted chunks.
    Entry,
    /// Prefer one shared vendor chunk for external packages.
    Vendor,
    /// Only use manual chunk declarations.
    Manual,
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

/// Bundler chunk planning options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetBundleChunk {
    /// Whether to split output into multiple chunks.
    pub splitting: bool,
    /// Whether to inline dynamic imports into one bundle.
    pub inline_dynamic_imports: bool,
    /// Whether to preserve module boundaries in emitted output.
    pub preserve_modules: bool,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<PathBuf>,
    /// Manual chunk assignments keyed by chunk name.
    pub manual_chunks: IndexMap<String, Vec<String>>,
    /// Chunk planning strategy for automatic chunking.
    pub strategy: BundleChunkStrategy,
}

impl TargetBundleChunk {
    /// Return whether this chunk policy requires assembled output.
    pub fn is_assembled(&self) -> bool {
        self.splitting
            || self.inline_dynamic_imports
            || self.preserve_modules
            || !self.manual_chunks.is_empty()
    }
}

impl Hash for TargetBundleChunk {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.splitting.hash(state);
        self.inline_dynamic_imports.hash(state);
        self.preserve_modules.hash(state);
        self.preserve_modules_root.hash(state);
        self.strategy.hash(state);

        self.manual_chunks.len().hash(state);
        for (chunk, modules) in &self.manual_chunks {
            chunk.hash(state);
            modules.hash(state);
        }
    }
}

/// Bundler dependency and resolution options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetBundleDependency {
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

impl Hash for TargetBundleDependency {
    fn hash<H: Hasher>(&self, state: &mut H) {
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
pub struct TargetBundleAsset {
    /// Asset handling mode for referenced assets.
    pub mode: BundleAssetMode,
    /// Inline asset payloads smaller than this many bytes.
    pub inline_limit: Option<u64>,
}

/// Bundler tree shaking options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetBundleTreeShake {
    /// Whether to tree shake unused modules and exports.
    pub enabled: bool,
    /// Side effect policy for individual modules.
    pub module_side_effects: BundleSideEffectMode,
    /// Side effect policy for packages and dependency boundaries.
    pub package_side_effects: BundleSideEffectMode,
}

impl TargetBundleTreeShake {
    /// Return whether tree shaking is active.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Bundler output shaping options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetBundleOutput {
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
}

/// Target scoped bundling options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetBundle {
    /// Bundle format for assembled JavaScript outputs.
    pub format: Option<BundleFormat>,
    /// Chunk planning options.
    pub chunk: TargetBundleChunk,
    /// Dependency and resolution options.
    pub dependency: TargetBundleDependency,
    /// Asset handling options.
    pub asset: TargetBundleAsset,
    /// Tree shaking options.
    pub tree_shake: TargetBundleTreeShake,
    /// Output shaping options.
    pub output: TargetBundleOutput,
    /// Compile time define replacements.
    pub define: IndexMap<String, String>,
    /// Global names for externals in IIFE and UMD formats.
    pub globals: IndexMap<String, String>,
    /// Minification options.
    pub minify: TargetBundleMinify,
}

impl TargetBundle {
    /// Return whether this bundler policy implies target level assembly.
    pub fn is_assembled(&self) -> bool {
        self != &Self::default()
    }
}

impl Hash for TargetBundle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.format.hash(state);
        self.chunk.hash(state);
        self.dependency.hash(state);
        self.asset.hash(state);
        self.tree_shake.hash(state);
        self.output.hash(state);
        self.minify.hash(state);

        self.define.len().hash(state);
        for (name, value) in &self.define {
            name.hash(state);
            value.hash(state);
        }

        self.globals.len().hash(state);
        for (name, value) in &self.globals {
            name.hash(state);
            value.hash(state);
        }
    }
}

impl From<&TargetBundleJson> for TargetBundle {
    fn from(json: &TargetBundleJson) -> Self {
        Self {
            format: json.format,
            chunk: json
                .chunk
                .as_ref()
                .map(TargetBundleChunk::from)
                .unwrap_or_default(),
            dependency: json
                .dependency
                .as_ref()
                .map(TargetBundleDependency::from)
                .unwrap_or_default(),
            asset: json
                .asset
                .as_ref()
                .map(TargetBundleAsset::from)
                .unwrap_or_default(),
            tree_shake: json
                .tree_shake
                .as_ref()
                .map(TargetBundleTreeShake::from)
                .unwrap_or_default(),
            output: json
                .output
                .as_ref()
                .map(TargetBundleOutput::from)
                .unwrap_or_default(),
            define: json.define.clone().unwrap_or_default(),
            globals: json.globals.clone().unwrap_or_default(),
            minify: json
                .minify
                .as_ref()
                .map(TargetBundleMinify::from)
                .unwrap_or_default(),
        }
    }
}

/// Bundler chunk planning options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetBundleChunkJson {
    /// Whether to split output into multiple chunks.
    #[serde(default)]
    pub splitting: bool,
    /// Whether to inline dynamic imports into one bundle.
    #[serde(default)]
    pub inline_dynamic_imports: bool,
    /// Whether to preserve module boundaries in emitted output.
    #[serde(default)]
    pub preserve_modules: bool,
    /// Root directory for preserved module paths.
    pub preserve_modules_root: Option<String>,
    /// Manual chunk assignments keyed by chunk name.
    pub manual_chunks: Option<IndexMap<String, Vec<String>>>,
    /// Chunk planning strategy for automatic chunking.
    pub strategy: Option<BundleChunkStrategy>,
}

impl From<&TargetBundleChunkJson> for TargetBundleChunk {
    fn from(json: &TargetBundleChunkJson) -> Self {
        Self {
            splitting: json.splitting,
            inline_dynamic_imports: json.inline_dynamic_imports,
            preserve_modules: json.preserve_modules,
            preserve_modules_root: json.preserve_modules_root.as_ref().map(PathBuf::from),
            manual_chunks: json.manual_chunks.clone().unwrap_or_default(),
            strategy: json.strategy.unwrap_or_default(),
        }
    }
}

/// Bundler dependency and resolution options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetBundleDependencyJson {
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

impl From<&TargetBundleDependencyJson> for TargetBundleDependency {
    fn from(json: &TargetBundleDependencyJson) -> Self {
        Self {
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
pub struct TargetBundleAssetJson {
    /// Asset handling mode for referenced assets.
    pub mode: Option<BundleAssetMode>,
    /// Inline asset payloads smaller than this many bytes.
    pub inline_limit: Option<u64>,
}

impl From<&TargetBundleAssetJson> for TargetBundleAsset {
    fn from(json: &TargetBundleAssetJson) -> Self {
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
pub struct TargetBundleTreeShakeJson {
    /// Whether to tree shake unused modules and exports.
    #[serde(default)]
    pub enabled: bool,
    /// Side effect policy for individual modules.
    pub module_side_effects: Option<BundleSideEffectMode>,
    /// Side effect policy for packages and dependency boundaries.
    pub package_side_effects: Option<BundleSideEffectMode>,
}

impl From<&TargetBundleTreeShakeJson> for TargetBundleTreeShake {
    fn from(json: &TargetBundleTreeShakeJson) -> Self {
        Self {
            enabled: json.enabled,
            module_side_effects: json.module_side_effects.unwrap_or_default(),
            package_side_effects: json.package_side_effects.unwrap_or_default(),
        }
    }
}

/// Bundler output shaping options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetBundleOutputJson {
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
}

impl From<&TargetBundleOutputJson> for TargetBundleOutput {
    fn from(json: &TargetBundleOutputJson) -> Self {
        Self {
            entry_file_names: json.entry_file_names.clone(),
            chunk_file_names: json.chunk_file_names.clone(),
            asset_file_names: json.asset_file_names.clone(),
            public_path: json.public_path.clone(),
            manifest: json.manifest,
            legal_comments: json.legal_comments.unwrap_or_default(),
            banner: json.banner.clone(),
            footer: json.footer.clone(),
        }
    }
}

/// Bundler minification options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetBundleMinifyJson {
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

impl From<&TargetBundleMinifyJson> for TargetBundleMinify {
    fn from(json: &TargetBundleMinifyJson) -> Self {
        Self {
            enabled: json.enabled,
            syntax: json.syntax,
            whitespace: json.whitespace,
            identifiers: json.identifiers,
            keep_names: json.keep_names,
        }
    }
}

/// Target scoped bundling options in `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetBundleJson {
    /// Bundle format for assembled JavaScript outputs.
    pub format: Option<BundleFormat>,
    /// Chunk planning options.
    pub chunk: Option<TargetBundleChunkJson>,
    /// Dependency and resolution options.
    pub dependency: Option<TargetBundleDependencyJson>,
    /// Asset handling options.
    pub asset: Option<TargetBundleAssetJson>,
    /// Tree shaking options.
    pub tree_shake: Option<TargetBundleTreeShakeJson>,
    /// Output shaping options.
    pub output: Option<TargetBundleOutputJson>,
    /// Compile time define replacements.
    pub define: Option<IndexMap<String, String>>,
    /// Global names for externals in IIFE and UMD formats.
    pub globals: Option<IndexMap<String, String>>,
    /// Minification options.
    pub minify: Option<TargetBundleMinifyJson>,
}
