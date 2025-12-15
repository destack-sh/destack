/// A builtin source file.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinSource {
    /// Module path (relative to core/, e.g., "operator/arithmetic").
    pub path: &'static str,
    /// File name (e.g., "arithmetic.ds").
    pub name: &'static str,
    /// Source content.
    pub content: &'static str,
}

impl BuiltinSource {
    const fn new(path: &'static str, name: &'static str, content: &'static str) -> Self {
        Self {
            path,
            name,
            content,
        }
    }

    /// Full virtual path (e.g., "builtin://core/operator/arithmetic.ds").
    pub fn virtual_path(&self) -> String {
        format!("builtin://core/{}/{}", self.path, self.name)
    }
}

/// Define a builtin source file embedded from `core/`.
macro_rules! builtin_source {
    // Files in subdirectories: builtin_source!(NAME, "subdir", "file.ds")
    ($name:ident, $dir:literal, $file:literal) => {
        const $name: BuiltinSource = BuiltinSource::new(
            $dir,
            $file,
            include_str!(concat!("../core/", $dir, "/", $file)),
        );
    };
    // Root-level files: builtin_source!(NAME, "file.ds")
    ($name:ident, $file:literal) => {
        const $name: BuiltinSource = BuiltinSource::new(
            "",
            $file,
            include_str!(concat!("../core/", $file)),
        );
    };
}

// core
builtin_source!(CORE_INDEX, "index.ds");

// operator
builtin_source!(OPERATOR_INDEX, "operator", "index.ds");
builtin_source!(OPERATOR_ARITHMETIC, "operator", "arithmetic.ds");
builtin_source!(OPERATOR_COMPARISON, "operator", "comparison.ds");
builtin_source!(OPERATOR_SUBSCRIPT, "operator", "subscript.ds");
builtin_source!(OPERATOR_CONTROL, "operator", "control.ds");

// reflection
builtin_source!(REFLECTION_INDEX, "reflection", "index.ds");
builtin_source!(REFLECTION_TYPE, "reflection", "type.ds");
builtin_source!(REFLECTION_PROPERTY, "reflection", "property.ds");
builtin_source!(REFLECTION_REFINEMENT, "reflection", "refinement.ds");
builtin_source!(REFLECTION_DECORATOR, "reflection", "decorator.ds");
builtin_source!(REFLECTION_VALIDATION, "reflection", "validation.ds");

// intrinsic
builtin_source!(INTRINSIC_INDEX, "intrinsic", "index.ds");
builtin_source!(INTRINSIC_IMPORT_META, "intrinsic", "import-meta.ds");

/// All core source files in load order.
///
/// Dependencies should be loaded before dependents.
pub const CORE_SOURCES: &[BuiltinSource] = &[
    // operator (no deps)
    OPERATOR_ARITHMETIC,
    OPERATOR_COMPARISON,
    OPERATOR_SUBSCRIPT,
    OPERATOR_CONTROL,
    OPERATOR_INDEX,
    // reflection (depends on operator for some types)
    REFLECTION_PROPERTY,
    REFLECTION_REFINEMENT,
    REFLECTION_DECORATOR,
    REFLECTION_VALIDATION,
    REFLECTION_TYPE,
    REFLECTION_INDEX,
    // intrinsic
    INTRINSIC_IMPORT_META,
    INTRINSIC_INDEX,
    // top level
    CORE_INDEX,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sources_not_empty() {
        assert!(!CORE_SOURCES.is_empty());
    }

    #[test]
    fn test_source_content_not_empty() {
        for source in CORE_SOURCES {
            assert!(
                !source.content.is_empty(),
                "source {} is empty",
                source.name
            );
        }
    }

    #[test]
    fn test_virtual_path() {
        assert_eq!(
            OPERATOR_ARITHMETIC.virtual_path(),
            "builtin://core/operator/arithmetic.ds"
        );
    }
}
