use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use indexmap::IndexMap;
use parking_lot::{Mutex, RwLock};
use serde::Deserialize;

use destack_source::{File, FileContent, FileId, LanguageFeature, LanguageFeatureSet};

use super::tsconfig::{EsTarget, ModuleKind};

/// Unique identifier for DsConfigs.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DsConfigId(pub u32);

impl DsConfigId {
    /// Wrap an id as a DsConfigId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Destack configuration (usually from `dsconfig.json`).
#[derive(Debug, Clone)]
pub struct DsConfig {
    /// The id of the DsConfig.
    pub id: DsConfigId,
    /// The id of the `dsconfig.json` file.
    pub file_id: FileId,
    /// Whether this is the root dsconfig in its context.
    pub is_root: bool,
    /// Path to the `dsconfig.json` file (including the `dsconfig.json`).
    pub path: PathBuf,
    /// The directory containing the `dsconfig.json` file.
    pub directory: PathBuf,
    /// The raw JSON content of the `dsconfig.json` file.
    pub content: DsConfigJson,
    /// The normalized/resolved configuration options.
    pub options: DsConfigOptions,
}

/// DsConfig JSON (usually from `dsconfig.json`)
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DsConfigJson {
    /// Extends other dsconfigs or tsconfigs.
    pub extends: Option<DsConfigExtendsField>,
    /// Specific files to include in the project.
    pub files: Option<Vec<String>>,
    /// Glob patterns for files to include.
    pub include: Option<Vec<String>>,
    /// Glob patterns for files to exclude.
    pub exclude: Option<Vec<String>>,
    /// Compiler options.
    #[serde(default)]
    pub compiler_options: DsConfigCompilerOptionsJson,
    /// Build targets.
    pub targets: Option<IndexMap<String, DsConfigTargetJson>>,
}

impl DsConfig {
    /// Parse a dsconfig from a File with JSON content.
    pub fn parse(
        id: DsConfigId,
        is_root: bool,
        file: &Arc<File>,
    ) -> Result<Self, serde_json::Error> {
        // extract the JSON value from file content
        let FileContent::Json { value, .. } = &file.content else {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file is not JSON",
            )));
        };

        // parse the dsconfig from the JSON value
        let dsconfig_json: DsConfigJson = serde_json::from_value(value.clone())?;

        // extract path from file (prefer file.path, fall back to URI conversion)
        let path = file
            .path
            .clone()
            .or_else(|| file.uri.to_path_buf())
            .expect("dsconfig file must have a valid path");
        let directory = path
            .parent()
            .expect("dsconfig.json must have a parent directory")
            .to_path_buf();

        // create initial options from JSON
        let options = DsConfigOptions::from(&dsconfig_json);

        let dsconfig = Self {
            id,
            file_id: file.id,
            is_root,
            path,
            directory,
            content: dsconfig_json,
            options,
        };
        Ok(dsconfig)
    }

    /// Inherits settings from the given dsconfig into `self`.
    pub fn extend_from(&mut self, dsconfig: &Self) {
        let parent = &dsconfig.options;

        // extend files/include/exclude (child overrides if non-empty)
        if self.options.files.is_empty() {
            self.options.files = parent.files.clone();
        }
        if self.options.include.is_empty() {
            self.options.include = parent.include.clone();
        }
        if self.options.exclude.is_empty() {
            self.options.exclude = parent.exclude.clone();
        }

        // extend compiler options
        let parent_compiler = &parent.compiler;

        // inherit features from parent
        for feature in LanguageFeature::ALL {
            if parent_compiler.features.is_enabled(*feature) {
                self.options.compiler.features.enable(*feature);
            }
        }

        // inherit path resolution (child overrides if set)
        if self.options.compiler.base_url.is_none() {
            self.options.compiler.base_url = parent_compiler.base_url.clone();
        }
        if self.options.compiler.paths.is_none() {
            self.options.compiler.paths = parent_compiler.paths.clone();
        }
        if self.options.compiler.root_dir.is_none() {
            self.options.compiler.root_dir = parent_compiler.root_dir.clone();
        }
        if self.options.compiler.out_dir.is_none() {
            self.options.compiler.out_dir = parent_compiler.out_dir.clone();
        }

        // inherit tsconfig path if not set
        if self.options.compiler.tsconfig.is_none() {
            self.options.compiler.tsconfig = parent_compiler.tsconfig.clone();
        }

        // extend targets (add missing targets from parent)
        for (name, target) in &parent.targets {
            if !self.options.targets.contains_key(name) {
                self.options.targets.insert(name.clone(), target.clone());
            }
        }
    }

    /// "Build" the root dsconfig in place, finalizing options.
    pub fn build(&mut self) {
        // currently no special build steps needed for dsconfig
        // this is here for symmetry with TsConfig::build()
    }
}

/// Value for the "extends" field of a dsconfig.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum DsConfigExtendsField {
    /// Extend a single dsconfig.
    Single(String),
    /// Extend multiple dsconfigs.
    Multiple(Vec<String>),
}

/// Destack configuration compiler options (JSON representation).
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DsConfigCompilerOptionsJson {
    // features
    /// Allow precise numeric types, raw strings, byte literals.
    pub allow_primitives: Option<bool>,
    /// Allow range literals (`1..10`, `1..=10`).
    pub allow_ranges: Option<bool>,
    /// Allow tuple types and literals.
    pub allow_tuples: Option<bool>,
    /// Allow tree literals (`<Node>...</Node>`).
    pub allow_trees: Option<bool>,
    /// Allow nominal (distinct) types with `newtype`.
    pub allow_newtypes: Option<bool>,
    /// Allow struct declarations for value-oriented data types.
    pub allow_structs: Option<bool>,
    /// Allow explicit ownership and reference semantics (`&T`, `&mut T`, `^T`).
    pub allow_ownership: Option<bool>,
    /// Allow constraint guards with `where` clauses.
    pub allow_constraints: Option<bool>,
    /// Allow type extensions for organizing implementations.
    pub allow_extensions: Option<bool>,
    /// Allow function and operator overloading.
    pub allow_overloading: Option<bool>,
    /// Allow pattern matching with `match` expressions.
    pub allow_patterns: Option<bool>,
    /// Allow effect declarations with `with` clauses.
    pub allow_effects: Option<bool>,
    /// Allow defer statements for cleanup (`defer file.close()`).
    pub allow_defer: Option<bool>,

    // path resolution
    /// Base URL for resolving non-relative module names.
    pub base_url: Option<String>,
    /// Path alias mappings (like tsconfig paths).
    pub paths: Option<IndexMap<String, Vec<String>>>,
    /// Root directory of input files.
    pub root_dir: Option<String>,
    /// Output directory for compiled files.
    pub out_dir: Option<String>,

    // module & target
    /// Module format for output (e.g., "esnext", "commonjs").
    pub module: Option<String>,
    /// ECMAScript target version (e.g., "es2022", "esnext").
    pub target: Option<String>,

    // typescript/javascript interop
    /// Path to tsconfig.json to inherit settings from.
    pub tsconfig: Option<String>,
    /// Allow TypeScript files (.ts, .tsx) in the project.
    pub allow_ts: Option<bool>,
    /// Type-check TypeScript files.
    pub check_ts: Option<bool>,
    /// Allow JavaScript files (.js, .jsx) in the project.
    pub allow_js: Option<bool>,
    /// Type-check JavaScript files.
    pub check_js: Option<bool>,
}

impl DsConfigCompilerOptionsJson {
    /// Apply feature flags from options to a feature set.
    pub fn apply_features(&self, features: &mut LanguageFeatureSet) {
        if let Some(enabled) = self.allow_overloading {
            features.set(LanguageFeature::Overloading, enabled);
        }
        if let Some(enabled) = self.allow_newtypes {
            features.set(LanguageFeature::Newtypes, enabled);
        }
        if let Some(enabled) = self.allow_primitives {
            features.set(LanguageFeature::Primitives, enabled);
        }
        if let Some(enabled) = self.allow_patterns {
            features.set(LanguageFeature::Patterns, enabled);
        }
        if let Some(enabled) = self.allow_effects {
            features.set(LanguageFeature::Effects, enabled);
        }
        if let Some(enabled) = self.allow_extensions {
            features.set(LanguageFeature::Extensions, enabled);
        }
        if let Some(enabled) = self.allow_ownership {
            features.set(LanguageFeature::Ownership, enabled);
        }
        if let Some(enabled) = self.allow_structs {
            features.set(LanguageFeature::Structs, enabled);
        }
        if let Some(enabled) = self.allow_constraints {
            features.set(LanguageFeature::Constraints, enabled);
        }
        if let Some(enabled) = self.allow_ranges {
            features.set(LanguageFeature::Ranges, enabled);
        }
        if let Some(enabled) = self.allow_tuples {
            features.set(LanguageFeature::Tuples, enabled);
        }
        if let Some(enabled) = self.allow_defer {
            features.set(LanguageFeature::Defer, enabled);
        }
        if let Some(enabled) = self.allow_trees {
            features.set(LanguageFeature::Trees, enabled);
        }
    }
}

/// Destack target.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DsConfigTargetJson {
    /// whether this is a debug build.
    pub debug: bool,
    /// whether this is an optimized build.
    pub optimize: bool,
    /// Optimization level (0-3).
    pub optimize_level: Option<u8>,
    /// Shrink levels (0-3).
    pub shrink_level: Option<u8>,
}

/// Destack configuration registry.
#[derive(Debug)]
pub struct DsConfigRegistry {
    /// The dsconfigs by id.
    dsconfigs_by_id: Mutex<HashMap<DsConfigId, Arc<RwLock<DsConfig>>>>,
    /// Path-based index for looking up dsconfigs by their file path.
    dsconfigs_by_path: Mutex<HashMap<PathBuf, DsConfigId>>,
    /// The next dsconfig id.
    next_dsconfig_id: AtomicU32,
}

impl Default for DsConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DsConfigRegistry {
    /// Create a new DsConfigRegistry.
    pub fn new() -> Self {
        Self {
            dsconfigs_by_id: Mutex::new(HashMap::new()),
            dsconfigs_by_path: Mutex::new(HashMap::new()),
            next_dsconfig_id: AtomicU32::new(0),
        }
    }

    /// Get and increment the next dsconfig id.
    pub fn next_id(&self) -> DsConfigId {
        let next_dsconfig_id = self.next_dsconfig_id.fetch_add(1, Ordering::Relaxed);
        DsConfigId::new(next_dsconfig_id)
    }

    /// Insert a dsconfig into the registry.
    pub fn insert(&self, dsconfig: DsConfig) {
        let path = dsconfig.path.clone();
        let id = dsconfig.id;
        let mut dsconfigs_by_id = self.dsconfigs_by_id.lock();
        dsconfigs_by_id.insert(id, Arc::new(RwLock::new(dsconfig)));

        // maintain path index
        let mut dsconfigs_by_path = self.dsconfigs_by_path.lock();
        dsconfigs_by_path.insert(path, id);
    }

    /// Get a dsconfig by id.
    ///
    /// # Panics
    /// Panics if the dsconfig is not found.
    pub fn get(&self, id: DsConfigId) -> Arc<RwLock<DsConfig>> {
        let dsconfigs_by_id = self.dsconfigs_by_id.lock();
        dsconfigs_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("dsconfig not found: {id:?}"))
            .clone()
    }

    /// Get a dsconfig id by its file path.
    pub fn get_id_by_path(&self, path: &Path) -> Option<DsConfigId> {
        let dsconfigs_by_path = self.dsconfigs_by_path.lock();
        dsconfigs_by_path.get(path).copied()
    }

    /// Get a dsconfig by its file path.
    pub fn get_by_path(&self, path: &Path) -> Option<Arc<RwLock<DsConfig>>> {
        let id = self.get_id_by_path(path)?;
        Some(self.get(id))
    }

    /// Check if a dsconfig exists at the given file path.
    pub fn contains_path(&self, path: &Path) -> bool {
        let dsconfigs_by_path = self.dsconfigs_by_path.lock();
        dsconfigs_by_path.contains_key(path)
    }

    /// Iterate over the dsconfigs in the registry.
    pub fn iter(&self) -> impl Iterator<Item = Arc<RwLock<DsConfig>>> {
        let dsconfigs_by_id = self.dsconfigs_by_id.lock();
        let snapshot: Vec<_> = dsconfigs_by_id.values().cloned().collect();
        snapshot.into_iter()
    }

    /// Get the number of dsconfigs in the registry.
    pub fn len(&self) -> usize {
        let dsconfigs_by_id = self.dsconfigs_by_id.lock();
        dsconfigs_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        let dsconfigs_by_id = self.dsconfigs_by_id.lock();
        dsconfigs_by_id.is_empty()
    }
}

/// Normalized Destack configuration options (from `dsconfig.json`).
#[derive(Debug, Clone, Default)]
pub struct DsConfigOptions {
    /// Specific files to include in the project.
    pub files: Vec<String>,
    /// Glob patterns for files to include.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,
    /// Compiler options.
    pub compiler: DsConfigCompilerOptions,
    /// Build targets.
    pub targets: IndexMap<String, DsConfigTargetOptions>,
}

impl From<&DsConfigJson> for DsConfigOptions {
    fn from(json: &DsConfigJson) -> Self {
        Self {
            files: json.files.clone().unwrap_or_default(),
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            compiler: DsConfigCompilerOptions::from(&json.compiler_options),
            targets: json
                .targets
                .as_ref()
                .map(|t| {
                    t.iter()
                        .map(|(k, v)| (k.clone(), DsConfigTargetOptions::from(v)))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// Path alias mapping (resolved from dsconfig paths).
pub type DsPathAliases = IndexMap<String, Vec<String>>;

/// Normalized Destack compiler options.
///
/// **By default, all language features are enabled.** Set `allow*: false` in
/// `dsconfig.json` to disable specific features.
#[derive(Debug, Clone)]
pub struct DsConfigCompilerOptions {
    // language features
    /// Enabled language features. All features are enabled by default.
    pub features: LanguageFeatureSet,

    // path resolution
    /// Base URL for resolving non-relative module names.
    pub base_url: Option<PathBuf>,
    /// Path alias mappings.
    pub paths: Option<DsPathAliases>,
    /// Root directory of input files.
    pub root_dir: Option<PathBuf>,
    /// Output directory for compiled files.
    pub out_dir: Option<PathBuf>,

    // module & target
    /// Module format for output.
    pub module: ModuleKind,
    /// ECMAScript target version.
    pub target: EsTarget,

    // typescript/javascript interop
    /// Path to tsconfig.json to inherit settings from.
    pub tsconfig: Option<PathBuf>,
    /// Allow TypeScript files (.ts, .tsx) in the project.
    pub allow_ts: bool,
    /// Type-check TypeScript files.
    pub check_ts: bool,
    /// Allow JavaScript files (.js, .jsx) in the project.
    pub allow_js: bool,
    /// Type-check JavaScript files.
    pub check_js: bool,
}

impl Default for DsConfigCompilerOptions {
    fn default() -> Self {
        Self {
            features: LanguageFeatureSet::all(),
            base_url: None,
            paths: None,
            root_dir: None,
            out_dir: None,
            module: ModuleKind::default(),
            target: EsTarget::default(),
            tsconfig: None,
            allow_ts: true,
            check_ts: false,
            allow_js: true,
            check_js: false,
        }
    }
}

impl From<&DsConfigCompilerOptionsJson> for DsConfigCompilerOptions {
    fn from(json: &DsConfigCompilerOptionsJson) -> Self {
        // start with all features enabled
        let mut features = LanguageFeatureSet::all();

        // apply feature flags from JSON (only explicit false disables)
        json.apply_features(&mut features);

        Self {
            features,
            base_url: json.base_url.as_ref().map(PathBuf::from),
            paths: json.paths.clone(),
            root_dir: json.root_dir.as_ref().map(PathBuf::from),
            out_dir: json.out_dir.as_ref().map(PathBuf::from),
            module: json
                .module
                .as_deref()
                .and_then(ModuleKind::parse)
                .unwrap_or_default(),
            target: json
                .target
                .as_deref()
                .and_then(EsTarget::parse)
                .unwrap_or_default(),
            tsconfig: json.tsconfig.as_ref().map(PathBuf::from),
            allow_ts: json.allow_ts.unwrap_or(true),
            check_ts: json.check_ts.unwrap_or(false),
            allow_js: json.allow_js.unwrap_or(true),
            check_js: json.check_js.unwrap_or(false),
        }
    }
}

/// Optimization level for builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptimizeLevel {
    /// No optimization (O0).
    #[default]
    O0,
    /// Basic optimization (O1).
    O1,
    /// Standard optimization (O2).
    O2,
    /// Aggressive optimization (O3).
    O3,
}

impl OptimizeLevel {
    /// Create from a numeric level (0-3).
    pub fn from_level(level: u8) -> Self {
        match level {
            0 => Self::O0,
            1 => Self::O1,
            2 => Self::O2,
            _ => Self::O3,
        }
    }

    /// Get the numeric level.
    pub fn as_level(&self) -> u8 {
        match self {
            Self::O0 => 0,
            Self::O1 => 1,
            Self::O2 => 2,
            Self::O3 => 3,
        }
    }
}

/// Shrink level for builds (code size reduction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShrinkLevel {
    /// No shrinking (S0).
    #[default]
    S0,
    /// Basic shrinking (S1).
    S1,
    /// Standard shrinking (S2).
    S2,
    /// Aggressive shrinking (S3).
    S3,
}

impl ShrinkLevel {
    /// Create from a numeric level (0-3).
    pub fn from_level(level: u8) -> Self {
        match level {
            0 => Self::S0,
            1 => Self::S1,
            2 => Self::S2,
            _ => Self::S3,
        }
    }

    /// Get the numeric level.
    pub fn as_level(&self) -> u8 {
        match self {
            Self::S0 => 0,
            Self::S1 => 1,
            Self::S2 => 2,
            Self::S3 => 3,
        }
    }
}

/// Normalized Destack build target options.
#[derive(Debug, Clone)]
pub struct DsConfigTargetOptions {
    /// Whether this is a debug build.
    pub debug: bool,
    /// Whether optimization is enabled.
    pub optimize: bool,
    /// Optimization level.
    pub optimize_level: OptimizeLevel,
    /// Shrink level (code size reduction).
    pub shrink_level: ShrinkLevel,
}

impl Default for DsConfigTargetOptions {
    fn default() -> Self {
        Self {
            debug: true,
            optimize: false,
            optimize_level: OptimizeLevel::O0,
            shrink_level: ShrinkLevel::S0,
        }
    }
}

impl From<&DsConfigTargetJson> for DsConfigTargetOptions {
    fn from(json: &DsConfigTargetJson) -> Self {
        Self {
            debug: json.debug,
            optimize: json.optimize,
            optimize_level: json
                .optimize_level
                .map(OptimizeLevel::from_level)
                .unwrap_or_default(),
            shrink_level: json
                .shrink_level
                .map(ShrinkLevel::from_level)
                .unwrap_or_default(),
        }
    }
}
