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
        if self.path.is_empty() {
            format!("builtin://core/{}", self.name)
        } else {
            format!("builtin://core/{}/{}", self.path, self.name)
        }
    }
}

/// Define a builtin source file embedded from `core/`.
macro_rules! builtin_source {
    // Files in subdirectories: builtin_source!(NAME, "subdir", "file.ds")
    ($name:ident, $dir:literal, $file:literal) => {
        const $name: BuiltinSource = BuiltinSource::new(
            $dir,
            $file,
            include_str!(concat!("../../core/", $dir, "/", $file)),
        );
    };
    // Root-level files: builtin_source!(NAME, "file.ds")
    ($name:ident, $file:literal) => {
        const $name: BuiltinSource =
            BuiltinSource::new("", $file, include_str!(concat!("../../core/", $file)));
    };
}

// core (root level)
builtin_source!(CORE_INDEX, "index.ds");
builtin_source!(CORE_PRELUDE, "prelude.ds");

// operator
builtin_source!(OPERATOR_INDEX, "operator", "index.ds");
builtin_source!(OPERATOR_ARITHMETIC, "operator", "arithmetic.ds");
builtin_source!(OPERATOR_COMPARISON, "operator", "comparison.ds");
builtin_source!(OPERATOR_FORMAT, "operator", "format.ds");
builtin_source!(OPERATOR_SUBSCRIPT, "operator", "subscript.ds");

// control
builtin_source!(CONTROL_INDEX, "control", "index.ds");
builtin_source!(CONTROL_TRY, "control", "try.ds");
builtin_source!(CONTROL_RESULT, "control", "result.ds");
builtin_source!(CONTROL_RANGE, "control", "range.ds");

// reflect
builtin_source!(REFLECT_INDEX, "reflect", "index.ds");
builtin_source!(REFLECT_TYPE, "reflect", "type.ds");
builtin_source!(REFLECT_PROPERTY, "reflect", "property.ds");
builtin_source!(REFLECT_REFINEMENT, "reflect", "refinement.ds");
builtin_source!(REFLECT_DECORATOR, "reflect", "decorator.ds");

// intrinsic
builtin_source!(INTRINSIC_INDEX, "intrinsic", "index.ds");
builtin_source!(INTRINSIC_IMPORT_META, "intrinsic", "import-meta.ds");
builtin_source!(INTRINSIC_DECORATOR, "intrinsic", "decorator.ds");

/// All core source files in load order.
///
/// Dependencies should be loaded before dependents.
pub const CORE_SOURCES: &[BuiltinSource] = &[
    // operator (no deps)
    OPERATOR_ARITHMETIC,
    OPERATOR_COMPARISON,
    OPERATOR_SUBSCRIPT,
    OPERATOR_FORMAT,
    OPERATOR_INDEX,
    // control (Try has no deps, Result depends on Try, Range has no deps)
    CONTROL_TRY,
    CONTROL_RANGE,
    CONTROL_RESULT,
    CONTROL_INDEX,
    // intrinsic (decorator newtypes have no deps, needed by reflect/type.ds)
    INTRINSIC_DECORATOR,
    INTRINSIC_IMPORT_META,
    INTRINSIC_INDEX,
    // reflect (depends on operator, control, and intrinsic/decorator)
    REFLECT_PROPERTY,
    REFLECT_REFINEMENT,
    REFLECT_DECORATOR,
    REFLECT_TYPE,
    REFLECT_INDEX,
    // top level
    CORE_INDEX,
    // prelude
    CORE_PRELUDE,
];

/// The prelude module source.
pub const PRELUDE_SOURCE: &BuiltinSource = &CORE_PRELUDE;

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
