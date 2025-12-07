use std::path::{Path, PathBuf};

use super::tsconfig::{EsTarget, ModuleKind};

/// Output format for a build target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OutputFormat {
    /// JavaScript (.js).
    #[default]
    Js,
    /// TypeScript (.ts).
    Ts,
    /// WebAssembly (.wasm).
    Wasm,
    /// Native binary.
    Native,
}

impl OutputFormat {
    /// Whether this format produces JavaScript output.
    pub fn is_js(&self) -> bool {
        matches!(self, Self::Js)
    }

    /// Whether this format produces TypeScript output.
    pub fn is_ts(&self) -> bool {
        matches!(self, Self::Ts)
    }

    /// Whether this format produces WebAssembly output.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::Wasm)
    }

    /// Whether this format produces native binary output.
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Native)
    }

    /// Whether this format typically produces a single output file.
    pub fn is_single_file(&self) -> bool {
        matches!(self, Self::Wasm | Self::Native)
    }
}

/// Output mode for build targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OutputMode {
    /// One output file per source file, preserving directory structure.
    /// Uses `out_dir` for the output directory.
    #[default]
    Directory,
    /// Single bundled/compiled output file.
    /// Uses `out_file` for the output path.
    File,
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

impl From<u8> for OptimizeLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => Self::O0,
            1 => Self::O1,
            2 => Self::O2,
            _ => Self::O3,
        }
    }
}

impl From<OptimizeLevel> for u8 {
    fn from(level: OptimizeLevel) -> Self {
        match level {
            OptimizeLevel::O0 => 0,
            OptimizeLevel::O1 => 1,
            OptimizeLevel::O2 => 2,
            OptimizeLevel::O3 => 3,
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

impl From<u8> for ShrinkLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => Self::S0,
            1 => Self::S1,
            2 => Self::S2,
            _ => Self::S3,
        }
    }
}

impl From<ShrinkLevel> for u8 {
    fn from(level: ShrinkLevel) -> Self {
        match level {
            ShrinkLevel::S0 => 0,
            ShrinkLevel::S1 => 1,
            ShrinkLevel::S2 => 2,
            ShrinkLevel::S3 => 3,
        }
    }
}

/// Default output directory for targets.
pub const DEFAULT_OUT_DIR: &str = "dist";

/// A build target configuration.
///
/// Can be constructed from dsconfig.json or programmatically.
/// This is the type used by compiler/codegen - independent of dsconfig parsing.
#[derive(Debug, Clone)]
pub struct Target {
    /// Target name (e.g., "npm", "wasm", "dev").
    pub name: String,

    // output format
    /// Output format (js, ts, wasm, native).
    pub output: OutputFormat,
    /// Emit declaration files (.d.ts) alongside JS output.
    pub declaration: bool,
    /// Emit source maps.
    pub source_map: bool,

    // output paths
    /// Output directory for this target (relative to package, defaults to "dist").
    pub out_dir: PathBuf,
    /// Output file for single-file targets.
    pub out_file: Option<PathBuf>,
    /// Separate directory for declaration files.
    pub declaration_dir: Option<PathBuf>,

    // JS/TS specific
    /// Module format (esnext, commonjs, etc.).
    pub module: ModuleKind,
    /// ECMAScript target version.
    pub es_target: EsTarget,

    // filtering
    /// Glob patterns for files to include in this target.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude from this target.
    pub exclude: Vec<String>,

    // optimization
    /// Whether this is a debug build.
    pub debug: bool,
    /// Whether optimization is enabled.
    pub optimize: bool,
    /// Optimization level.
    pub optimize_level: OptimizeLevel,
    /// Shrink level (code size reduction).
    pub shrink_level: ShrinkLevel,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            name: String::new(),
            output: OutputFormat::default(),
            declaration: false,
            source_map: false,
            out_dir: PathBuf::from(DEFAULT_OUT_DIR),
            out_file: None,
            declaration_dir: None,
            module: ModuleKind::default(),
            es_target: EsTarget::default(),
            include: Vec::new(),
            exclude: Vec::new(),
            debug: true,
            optimize: false,
            optimize_level: OptimizeLevel::O0,
            shrink_level: ShrinkLevel::S0,
        }
    }
}

impl Target {
    /// Create a new target with the given name and default JS output.
    pub fn js(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Js,
            declaration: true, // default to emitting declarations for JS
            ..Default::default()
        }
    }

    /// Create a new target with the given name and TypeScript output.
    pub fn ts(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Ts,
            ..Default::default()
        }
    }

    /// Create a new target with the given name and WASM output.
    pub fn wasm(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Wasm,
            optimize: true,
            ..Default::default()
        }
    }

    /// Create a new target with the given name and native output.
    pub fn native(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: OutputFormat::Native,
            optimize: true,
            ..Default::default()
        }
    }

    /// Derive the output mode from the target configuration.
    pub fn output_mode(&self) -> OutputMode {
        if self.out_file.is_some() || self.output.is_single_file() {
            OutputMode::File
        } else {
            OutputMode::Directory
        }
    }

    /// Whether this target produces single-file output.
    pub fn is_single_file(&self) -> bool {
        self.output_mode() == OutputMode::File
    }

    /// Whether this target produces directory output (one file per source file).
    pub fn is_directory(&self) -> bool {
        self.output_mode() == OutputMode::Directory
    }

    /// Set the output directory.
    pub fn with_out_dir(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.out_dir = out_dir.into();
        self
    }

    /// Set the output file.
    pub fn with_out_file(mut self, out_file: impl Into<PathBuf>) -> Self {
        self.out_file = Some(out_file.into());
        self
    }

    /// Set whether to emit declarations.
    pub fn with_declaration(mut self, declaration: bool) -> Self {
        self.declaration = declaration;
        self
    }

    /// Set whether to emit source maps.
    pub fn with_source_map(mut self, source_map: bool) -> Self {
        self.source_map = source_map;
        self
    }

    /// Set the module format.
    pub fn with_module(mut self, module: ModuleKind) -> Self {
        self.module = module;
        self
    }

    /// Set the ES target.
    pub fn with_es_target(mut self, es_target: EsTarget) -> Self {
        self.es_target = es_target;
        self
    }

    /// Set whether optimization is enabled.
    pub fn with_optimize(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self
    }

    /// Set the optimization level.
    pub fn with_optimize_level(mut self, level: OptimizeLevel) -> Self {
        self.optimize_level = level;
        self
    }

    /// Set the shrink level.
    pub fn with_shrink_level(mut self, level: ShrinkLevel) -> Self {
        self.shrink_level = level;
        self
    }

    /// Set include patterns.
    pub fn with_include(mut self, include: Vec<String>) -> Self {
        self.include = include;
        self
    }

    /// Set exclude patterns.
    pub fn with_exclude(mut self, exclude: Vec<String>) -> Self {
        self.exclude = exclude;
        self
    }

    /// Resolve the absolute output directory for this target.
    ///
    /// If out_dir is relative, it's resolved relative to the package directory.
    /// If out_dir is absolute, it's returned as-is.
    pub fn resolve_out_dir(&self, package_dir: &Path) -> PathBuf {
        if self.out_dir.is_absolute() {
            self.out_dir.clone()
        } else {
            package_dir.join(&self.out_dir)
        }
    }

    /// Compute the output path for a module artifact.
    ///
    /// Takes the module's path, computes its relative position within the package,
    /// and returns the corresponding output path with the given extension.
    pub fn resolve_out_file(
        &self,
        package_dir: &Path,
        module_path: &Path,
        extension: &str,
    ) -> PathBuf {
        let out_dir = self.resolve_out_dir(package_dir);

        // compute module's relative path within the package
        let relative = module_path.strip_prefix(package_dir).unwrap_or(module_path);

        // change extension and join with output directory
        out_dir.join(relative.with_extension(extension))
    }
}
