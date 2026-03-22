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

/// Target scoped bundling options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetBundle {
    /// Bundle format for assembled JavaScript outputs.
    pub format: Option<BundleFormat>,
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
    /// Module specifiers to leave external.
    pub external: Vec<String>,
    /// Module specifier aliases.
    pub alias: IndexMap<String, String>,
    /// Resolution conditions to prefer.
    pub conditions: Vec<String>,
    /// Package.json fields to prefer during resolution.
    pub main_fields: Vec<String>,
    /// Output naming template for entry chunks.
    pub entry_file_names: Option<String>,
    /// Output naming template for shared chunks.
    pub chunk_file_names: Option<String>,
    /// Output naming template for assets.
    pub asset_file_names: Option<String>,
    /// Public path prefix for runtime asset resolution.
    pub public_path: Option<String>,
    /// Whether to emit a build manifest.
    pub manifest: bool,
    /// Whether to tree shake unused modules and exports.
    pub treeshake: bool,
    /// Compile time define replacements.
    pub define: IndexMap<String, String>,
    /// Whether to minify final bundled output.
    pub minify: bool,
    /// Whether to minify syntax forms.
    pub minify_syntax: bool,
    /// Whether to minify whitespace.
    pub minify_whitespace: bool,
    /// Whether to minify identifiers.
    pub minify_identifiers: bool,
    /// Whether to preserve function and class names.
    pub keep_names: bool,
    /// Global names for externals in IIFE and UMD formats.
    pub globals: IndexMap<String, String>,
    /// Banner text to prepend to each emitted bundle.
    pub banner: Option<String>,
    /// Footer text to append to each emitted bundle.
    pub footer: Option<String>,
}

impl Hash for TargetBundle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // simple scalars
        self.format.hash(state);
        self.splitting.hash(state);
        self.inline_dynamic_imports.hash(state);
        self.preserve_modules.hash(state);
        self.preserve_modules_root.hash(state);
        self.entry_file_names.hash(state);
        self.chunk_file_names.hash(state);
        self.asset_file_names.hash(state);
        self.public_path.hash(state);
        self.manifest.hash(state);
        self.treeshake.hash(state);
        self.minify.hash(state);
        self.minify_syntax.hash(state);
        self.minify_whitespace.hash(state);
        self.minify_identifiers.hash(state);
        self.keep_names.hash(state);
        self.banner.hash(state);
        self.footer.hash(state);

        // indexed sets and maps
        self.manual_chunks.len().hash(state);
        for (chunk, modules) in &self.manual_chunks {
            chunk.hash(state);
            modules.hash(state);
        }

        self.external.hash(state);

        self.alias.len().hash(state);
        for (from, to) in &self.alias {
            from.hash(state);
            to.hash(state);
        }

        self.conditions.hash(state);
        self.main_fields.hash(state);

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
            splitting: json.splitting,
            inline_dynamic_imports: json.inline_dynamic_imports,
            preserve_modules: json.preserve_modules,
            preserve_modules_root: json.preserve_modules_root.as_ref().map(PathBuf::from),
            manual_chunks: json.manual_chunks.clone().unwrap_or_default(),
            external: json.external.clone().unwrap_or_default(),
            alias: json.alias.clone().unwrap_or_default(),
            conditions: json.conditions.clone().unwrap_or_default(),
            main_fields: json.main_fields.clone().unwrap_or_default(),
            entry_file_names: json.entry_file_names.clone(),
            chunk_file_names: json.chunk_file_names.clone(),
            asset_file_names: json.asset_file_names.clone(),
            public_path: json.public_path.clone(),
            manifest: json.manifest,
            treeshake: json.treeshake,
            define: json.define.clone().unwrap_or_default(),
            minify: json.minify,
            minify_syntax: json.minify_syntax,
            minify_whitespace: json.minify_whitespace,
            minify_identifiers: json.minify_identifiers,
            keep_names: json.keep_names,
            globals: json.globals.clone().unwrap_or_default(),
            banner: json.banner.clone(),
            footer: json.footer.clone(),
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
    /// Module specifiers to leave external.
    pub external: Option<Vec<String>>,
    /// Module specifier aliases.
    pub alias: Option<IndexMap<String, String>>,
    /// Resolution conditions to prefer.
    pub conditions: Option<Vec<String>>,
    /// Package.json fields to prefer during resolution.
    pub main_fields: Option<Vec<String>>,
    /// Output naming template for entry chunks.
    pub entry_file_names: Option<String>,
    /// Output naming template for shared chunks.
    pub chunk_file_names: Option<String>,
    /// Output naming template for assets.
    pub asset_file_names: Option<String>,
    /// Public path prefix for runtime asset resolution.
    pub public_path: Option<String>,
    /// Whether to emit a build manifest.
    #[serde(default)]
    pub manifest: bool,
    /// Whether to tree shake unused modules and exports.
    #[serde(default)]
    pub treeshake: bool,
    /// Compile time define replacements.
    pub define: Option<IndexMap<String, String>>,
    /// Whether to minify final bundled output.
    #[serde(default)]
    pub minify: bool,
    /// Whether to minify syntax forms.
    #[serde(default)]
    pub minify_syntax: bool,
    /// Whether to minify whitespace.
    #[serde(default)]
    pub minify_whitespace: bool,
    /// Whether to minify identifiers.
    #[serde(default)]
    pub minify_identifiers: bool,
    /// Whether to preserve function and class names.
    #[serde(default)]
    pub keep_names: bool,
    /// Global names for externals in IIFE and UMD formats.
    pub globals: Option<IndexMap<String, String>>,
    /// Banner text to prepend to each emitted bundle.
    pub banner: Option<String>,
    /// Footer text to append to each emitted bundle.
    pub footer: Option<String>,
}
