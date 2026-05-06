use serde::Deserialize;

use super::target::{
    BundleLegalComment, DebugInfoLevel, DebugInfoLevelJson, LtoMode, LtoModeJson, OptimizeLevel,
    ShrinkLevel, SourceMapMode, StripLevel, StripLevelJson, Target, TargetMinifyOptions,
    TargetMinifyOptionsJson, TargetOptions, TargetTreeshakeOptions, TargetTreeshakeOptionsJson,
};

/// Development mode name.
pub const MODE_DEV: &str = "dev";

/// Production mode name.
pub const MODE_PROD: &str = "prod";

/// Test mode name.
pub const MODE_TEST: &str = "test";

/// Benchmark mode name.
pub const MODE_BENCH: &str = "bench";

/// Lint mode name.
pub const MODE_LINT: &str = "lint";

/// Well-known mode names with built-in command and file suffix behavior.
pub const BUILTIN_MODES: &[&str] = &[MODE_DEV, MODE_PROD, MODE_TEST, MODE_BENCH, MODE_LINT];

/// Return whether one mode name is well-known to the toolchain.
pub fn is_builtin_mode(mode: &str) -> bool {
    BUILTIN_MODES.contains(&mode)
}

/// Named build mode options.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ModeOptions {
    /// Source map emission policy for this mode.
    pub source_map_mode: Option<SourceMapMode>,
    /// Whether to omit source contents from emitted source maps.
    pub source_map_exclude_sources: Option<bool>,
    /// Whether to include debug ids in emitted source maps.
    pub source_map_debug_ids: Option<bool>,
    /// Legal comment handling policy for bundled output.
    pub legal_comments: Option<BundleLegalComment>,
    /// Bundled output tree shaking policy for this mode.
    pub treeshake: Option<TargetTreeshakeOptions>,
    /// Bundled output minification policy for this mode.
    pub minify: Option<TargetMinifyOptions>,
    /// Whether this mode enables optimized code generation.
    pub optimize: Option<bool>,
    /// Optimization level override for this mode.
    pub optimize_level: Option<OptimizeLevel>,
    /// Link time optimization override for this mode.
    pub lto_mode: Option<LtoMode>,
    /// Size-oriented shrinking level override for this mode.
    pub shrink_level: Option<ShrinkLevel>,
    /// Debug info emission policy for this mode.
    pub debug_info: Option<DebugInfoLevel>,
    /// Symbol stripping policy for this mode.
    pub strip: Option<StripLevel>,
}

impl ModeOptions {
    /// Convert from one JSON mode.
    pub fn from_json(json: &ModeJson) -> Self {
        Self {
            source_map_mode: json.source_map_mode,
            source_map_exclude_sources: json.source_map_exclude_sources,
            source_map_debug_ids: json.source_map_debug_ids,
            legal_comments: json.legal_comments,
            treeshake: json.treeshake.as_ref().map(TargetTreeshakeOptions::from),
            minify: json.minify.as_ref().map(TargetMinifyOptions::from),
            optimize: json.optimize,
            optimize_level: json.optimize_level.map(OptimizeLevel::from),
            lto_mode: json.lto_mode.map(LtoMode::from),
            shrink_level: json.shrink_level.map(ShrinkLevel::from),
            debug_info: json.debug_info.map(DebugInfoLevel::from),
            strip: json.strip.map(StripLevel::from),
        }
    }

    /// Apply this mode to normalized target options.
    pub fn apply_to_target_options(&self, target: &mut TargetOptions) {
        // source maps
        if let Some(source_map_mode) = self.source_map_mode {
            target.source_map_mode = Some(source_map_mode);
            target.bundle_output.sourcemap = Some(source_map_mode);
        }
        if let Some(source_map_exclude_sources) = self.source_map_exclude_sources {
            target.bundle_output.sourcemap_exclude_sources = source_map_exclude_sources;
        }
        if let Some(source_map_debug_ids) = self.source_map_debug_ids {
            target.bundle_output.sourcemap_debug_ids = source_map_debug_ids;
        }

        // bundled output policy
        if let Some(legal_comments) = self.legal_comments {
            target.bundle_output.legal_comments = legal_comments;
        }
        if let Some(treeshake) = &self.treeshake {
            target.treeshake = treeshake.clone();
        }
        if let Some(minify) = &self.minify {
            target.minify = minify.clone();
        }

        // optimization and debuggability
        if let Some(optimize) = self.optimize {
            target.optimize = optimize;
        }
        if let Some(optimize_level) = self.optimize_level {
            target.optimize_level = optimize_level;
        }
        if let Some(lto_mode) = self.lto_mode {
            target.lto_mode = lto_mode;
        }
        if let Some(shrink_level) = self.shrink_level {
            target.shrink_level = shrink_level;
        }
        if let Some(debug_info) = self.debug_info {
            target.debug_info = debug_info;
        }
        if let Some(strip) = self.strip {
            target.strip = strip;
        }
    }

    /// Apply this mode to one concrete target.
    pub fn apply_to_target(&self, target: &mut Target) {
        // source maps
        if let Some(source_map_mode) = self.source_map_mode {
            target.source_map_mode = Some(source_map_mode);
            target.bundle_output.sourcemap = Some(source_map_mode);
        }
        if let Some(source_map_exclude_sources) = self.source_map_exclude_sources {
            target.bundle_output.sourcemap_exclude_sources = source_map_exclude_sources;
        }
        if let Some(source_map_debug_ids) = self.source_map_debug_ids {
            target.bundle_output.sourcemap_debug_ids = source_map_debug_ids;
        }

        // bundled output policy
        if let Some(legal_comments) = self.legal_comments {
            target.bundle_output.legal_comments = legal_comments;
        }
        if let Some(treeshake) = &self.treeshake {
            target.treeshake = treeshake.clone();
        }
        if let Some(minify) = &self.minify {
            target.minify = minify.clone();
        }

        // optimization and debuggability
        if let Some(optimize) = self.optimize {
            target.optimize = optimize;
        }
        if let Some(optimize_level) = self.optimize_level {
            target.optimize_level = optimize_level;
        }
        if let Some(lto_mode) = self.lto_mode {
            target.lto_mode = lto_mode;
        }
        if let Some(shrink_level) = self.shrink_level {
            target.shrink_level = shrink_level;
        }
        if let Some(debug_info) = self.debug_info {
            target.debug_info = debug_info;
        }
        if let Some(strip) = self.strip {
            target.strip = strip;
        }
    }
}

/// Build mode JSON (from destack.json).
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ModeJson {
    /// Source map emission mode for this build mode.
    #[serde(alias = "sourcemap")]
    pub source_map_mode: Option<SourceMapMode>,
    /// Whether to omit source contents from emitted source maps.
    pub source_map_exclude_sources: Option<bool>,
    /// Whether to include debug ids in emitted source maps.
    pub source_map_debug_ids: Option<bool>,
    /// Legal comment handling policy for bundled output.
    pub legal_comments: Option<BundleLegalComment>,
    /// Bundled output tree shaking policy for this build mode.
    pub treeshake: Option<TargetTreeshakeOptionsJson>,
    /// Bundled output minification policy for this build mode.
    pub minify: Option<TargetMinifyOptionsJson>,
    /// Whether this mode enables optimized code generation.
    pub optimize: Option<bool>,
    /// Optimization level (0-4).
    #[cfg_attr(feature = "schema", schemars(range(min = 0, max = 4)))]
    pub optimize_level: Option<u8>,
    /// Link time optimization mode.
    pub lto_mode: Option<LtoModeJson>,
    /// Shrink level (0-3).
    #[cfg_attr(feature = "schema", schemars(range(min = 0, max = 3)))]
    pub shrink_level: Option<u8>,
    /// Debug info emission policy for this mode.
    pub debug_info: Option<DebugInfoLevelJson>,
    /// Symbol stripping policy for this mode.
    pub strip: Option<StripLevelJson>,
}
