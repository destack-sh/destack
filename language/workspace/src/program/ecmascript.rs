//! ECMAScript ecosystem shared types.
//!
//! New types for ECMAScript concepts that aren't already in tsconfig.rs.
//! Types like `ModuleTarget`, `EsTarget`, `ModuleResolution` remain in tsconfig.rs
//! for backwards compatibility.

use std::path::Path;

use super::tsconfig::ModuleDetection;

// nocheckin TODO: merge ecmascript.rs back into tsconfig.rs

/// Module type that determines how a file is parsed (Script vs Module).
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
pub enum ModuleType {
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

impl ModuleType {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_module_type_from_extension() {
        assert_eq!(
            ModuleType::from_extension(&PathBuf::from("foo.mjs")),
            Some(ModuleType::Module)
        );
        assert_eq!(
            ModuleType::from_extension(&PathBuf::from("foo.mts")),
            Some(ModuleType::Module)
        );
        assert_eq!(
            ModuleType::from_extension(&PathBuf::from("foo.cjs")),
            Some(ModuleType::Script)
        );
        assert_eq!(
            ModuleType::from_extension(&PathBuf::from("foo.cts")),
            Some(ModuleType::Script)
        );
        assert_eq!(ModuleType::from_extension(&PathBuf::from("foo.js")), None);
        assert_eq!(ModuleType::from_extension(&PathBuf::from("foo.ts")), None);
    }

    #[test]
    fn test_module_type_detect_by_extension() {
        let path = PathBuf::from("test.mjs");
        assert_eq!(
            ModuleType::detect(&path, false, ModuleDetection::Auto, None),
            ModuleType::Module
        );

        let path = PathBuf::from("test.cjs");
        assert_eq!(
            ModuleType::detect(&path, true, ModuleDetection::Auto, None),
            ModuleType::Script
        );
    }

    #[test]
    fn test_module_type_detect_by_package_type() {
        let path = PathBuf::from("test.js");
        assert_eq!(
            ModuleType::detect(&path, false, ModuleDetection::Auto, Some("module")),
            ModuleType::Module
        );
        assert_eq!(
            ModuleType::detect(&path, false, ModuleDetection::Auto, Some("commonjs")),
            ModuleType::Script
        );
    }

    #[test]
    fn test_module_type_detect_by_content() {
        let path = PathBuf::from("test.js");
        assert_eq!(
            ModuleType::detect(&path, true, ModuleDetection::Auto, None),
            ModuleType::Module
        );
        assert_eq!(
            ModuleType::detect(&path, false, ModuleDetection::Auto, None),
            ModuleType::Script
        );
    }

    #[test]
    fn test_module_type_force_module() {
        let path = PathBuf::from("test.cjs");
        assert_eq!(
            ModuleType::detect(&path, false, ModuleDetection::Force, Some("commonjs")),
            ModuleType::Module
        );
    }
}
