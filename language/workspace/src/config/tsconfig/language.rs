use std::path::Path;

/// Module resolution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModuleResolution {
    /// Classic TypeScript resolution (deprecated).
    Classic,
    /// Node.js resolution (CommonJS).
    Node,
    /// Node.js 16+ resolution (ESM).
    Node16,
    /// Node.js Next resolution (ESM).
    NodeNext,
    /// Bundler-style resolution.
    #[default]
    Bundler,
}

impl ModuleResolution {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "classic" => Some(Self::Classic),
            "node" | "node10" => Some(Self::Node),
            "node16" => Some(Self::Node16),
            "nodenext" => Some(Self::NodeNext),
            "bundler" => Some(Self::Bundler),
            _ => None,
        }
    }
}

/// Module format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModuleTarget {
    /// CommonJS modules.
    CommonJs,
    /// ES2015 modules.
    Es2015,
    /// ES2020 modules.
    Es2020,
    /// ES2022 modules.
    Es2022,
    /// ESNext modules.
    #[default]
    EsNext,
    /// Node16 modules.
    Node16,
    /// NodeNext modules.
    NodeNext,
    /// Preserve original module syntax.
    Preserve,
    /// No module system.
    None,
}

impl ModuleTarget {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "commonjs" => Some(Self::CommonJs),
            "es2015" | "es6" => Some(Self::Es2015),
            "es2020" => Some(Self::Es2020),
            "es2022" => Some(Self::Es2022),
            "esnext" => Some(Self::EsNext),
            "node16" => Some(Self::Node16),
            "nodenext" => Some(Self::NodeNext),
            "preserve" => Some(Self::Preserve),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    /// Whether this module kind is ESM-based.
    pub fn is_esm(&self) -> bool {
        matches!(
            self,
            Self::Es2015
                | Self::Es2020
                | Self::Es2022
                | Self::EsNext
                | Self::Node16
                | Self::NodeNext
                | Self::Preserve
        )
    }
}

/// ECMAScript target version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EsTarget {
    /// ES5.
    Es5,
    /// ES2015 (ES6).
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
    /// ESNext (latest).
    #[default]
    EsNext,
}

impl EsTarget {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
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

/// How to detect module vs script files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModuleDetection {
    /// Auto-detect based on imports/exports.
    #[default]
    Auto,
    /// Legacy detection (TypeScript <4.7).
    Legacy,
    /// Force all files to be modules.
    Force,
}

impl ModuleDetection {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "legacy" => Some(Self::Legacy),
            "force" => Some(Self::Force),
            _ => None,
        }
    }
}

/// Source type that determines how a file is parsed (Script vs Module).
///
/// This is the **result** of module detection, not the strategy.
/// See [`ModuleDetection`] for the detection strategy.
///
/// This affects:
/// - Whether HTML comments (`<!--`, `-->`) are allowed (Script only)
/// - Whether top-level `await` is allowed (Module only)
/// - Default strict mode (Module is always strict)
/// - Whether `import`/`export` statements are allowed (Module only)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SourceType {
    /// Script mode: classic `<script>` behavior.
    /// - HTML comments are allowed (legacy web compat)
    /// - `this` at top level is the global object
    /// - No top-level `await`
    #[default]
    Script,
    /// Module mode: ES modules (`<script type="module">`).
    /// - HTML comments are NOT allowed
    /// - `this` at top level is `undefined`
    /// - Top-level `await` is allowed
    /// - Always in strict mode
    Module,
}

impl SourceType {
    /// Whether this is module mode.
    pub fn is_module(&self) -> bool {
        matches!(self, Self::Module)
    }

    /// Whether this is script mode.
    pub fn is_script(&self) -> bool {
        matches!(self, Self::Script)
    }

    /// Detect source type from file path extension:
    /// - `.mjs`, `.mts` → Module (explicit ES module)
    /// - `.cjs`, `.cts` → Script (explicit CommonJS)
    /// - `.ds` → Module (Destack files are always modules)
    /// - Other → None (needs further detection)
    pub fn from_extension(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        match ext {
            // explicit ES module extensions
            "mjs" | "mts" => Some(Self::Module),
            // explicit CommonJS extensions
            "cjs" | "cts" => Some(Self::Script),
            // Destack files are always modules (modern language, no legacy baggage)
            "ds" => Some(Self::Module),
            _ => None,
        }
    }

    /// Detect source type using the full detection strategy.
    ///
    /// # Arguments
    /// - `path`: File path (checked for `.mjs`/`.cjs` extensions)
    /// - `has_import_export`: Whether the file contains `import`/`export` statements
    /// - `detection`: The detection strategy from config
    /// - `package_type`: The `type` field from nearest `package.json` (`"module"` or `"commonjs"`)
    pub fn detect(
        path: &Path,
        has_import_export: bool,
        detection: ModuleDetection,
        package_type: Option<&str>,
    ) -> Self {
        match detection {
            ModuleDetection::Force => Self::Module,
            ModuleDetection::Auto | ModuleDetection::Legacy => {
                // 1. extension-based detection takes priority
                if let Some(module_type) = Self::from_extension(path) {
                    return module_type;
                }

                // 2. package.json "type" field
                match package_type {
                    Some("module") => return Self::Module,
                    Some("commonjs") => return Self::Script,
                    _ => {}
                }

                // 3. auto-detect from content (import/export presence)
                if has_import_export {
                    Self::Module
                } else {
                    Self::Script
                }
            }
        }
    }
}
