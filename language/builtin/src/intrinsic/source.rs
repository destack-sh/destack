/// A builtin source file.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinSource {
    /// Module path relative to intrinsic/.
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

    /// Return the full virtual path.
    pub fn virtual_path(&self) -> String {
        if self.path.is_empty() {
            format!("builtin://{}", self.name)
        } else {
            format!("builtin://{}/{}", self.path, self.name)
        }
    }

    /// Return the relative module path.
    pub fn module_path(&self) -> String {
        if self.path.is_empty() {
            self.name.to_string()
        } else {
            format!("{}/{}", self.path, self.name)
        }
    }
}

/// Define a builtin source file embedded from intrinsic/.
macro_rules! builtin_source {
    // files in subdirectories
    ($name:ident, $dir:literal, $file:literal) => {
        const $name: BuiltinSource = BuiltinSource::new(
            $dir,
            $file,
            include_str!(concat!("../../intrinsic/", $dir, "/", $file)),
        );
    };
    // root level files
    ($name:ident, $file:literal) => {
        const $name: BuiltinSource =
            BuiltinSource::new("", $file, include_str!(concat!("../../intrinsic/", $file)));
    };
}

// intrinsic root
builtin_source!(INTRINSIC_ROOT_INDEX, "index.ds");
builtin_source!(INTRINSIC_PRELUDE, "prelude.ds");

// memory
builtin_source!(MEMORY_INDEX, "memory", "index.ds");
builtin_source!(MEMORY_BYTES, "memory", "bytes.ds");
builtin_source!(MEMORY_CAPABILITY, "memory", "capability.ds");
builtin_source!(MEMORY_DISPOSE, "memory", "dispose.ds");
builtin_source!(MEMORY_OWNERSHIP, "memory", "ownership.ds");

// operator
builtin_source!(OPERATOR_ARITHMETIC, "operator", "arithmetic.ds");
builtin_source!(OPERATOR_COMPARISON, "operator", "comparison.ds");
builtin_source!(OPERATOR_INDEX, "operator", "index.ds");
builtin_source!(OPERATOR_FORMAT, "operator", "format.ds");
builtin_source!(OPERATOR_SUBSCRIPT, "operator", "subscript.ds");

// control
builtin_source!(CONTROL_INDEX, "control", "index.ds");
builtin_source!(CONTROL_TRY, "control", "try.ds");
builtin_source!(CONTROL_ERROR, "control", "error.ds");
builtin_source!(CONTROL_RESULT, "control", "result.ds");
builtin_source!(CONTROL_ITERABLE, "control", "iterable.ds");

// reflect
builtin_source!(REFLECT_INDEX, "reflect", "index.ds");
builtin_source!(REFLECT_TYPE, "reflect", "type.ds");
builtin_source!(REFLECT_PROPERTY, "reflect", "property.ds");
builtin_source!(REFLECT_ANNOTATION, "reflect", "annotation.ds");

// primitive
builtin_source!(PRIMITIVE_INDEX, "primitive", "index.ds");
builtin_source!(PRIMITIVE_IMPORT_META, "primitive", "import-meta.ds");
builtin_source!(PRIMITIVE_BINDING, "primitive", "binding.ds");
builtin_source!(PRIMITIVE_DECORATOR, "primitive", "decorator.ds");
builtin_source!(PRIMITIVE_BIT, "primitive", "bit.ds");
builtin_source!(PRIMITIVE_ARITHMETIC, "primitive", "arithmetic.ds");
builtin_source!(PRIMITIVE_CAST, "primitive", "cast.ds");
builtin_source!(PRIMITIVE_MATH, "primitive", "math.ds");
builtin_source!(PRIMITIVE_MEMORY, "primitive", "memory.ds");
builtin_source!(PRIMITIVE_ATOMIC, "primitive", "atomic.ds");
builtin_source!(PRIMITIVE_CONTROL, "primitive", "control.ds");
builtin_source!(PRIMITIVE_VECTOR, "primitive", "vector.ds");

/// All intrinsic source files in load order.
///
/// Dependencies should be loaded before dependents.
pub const INTRINSIC_SOURCES: &[BuiltinSource] = &[
    // memory
    MEMORY_OWNERSHIP,
    MEMORY_CAPABILITY,
    MEMORY_DISPOSE,
    MEMORY_BYTES,
    MEMORY_INDEX,
    // operator
    OPERATOR_ARITHMETIC,
    OPERATOR_COMPARISON,
    OPERATOR_SUBSCRIPT,
    OPERATOR_FORMAT,
    OPERATOR_INDEX,
    // control
    CONTROL_TRY,
    CONTROL_ERROR,
    CONTROL_ITERABLE,
    CONTROL_RESULT,
    CONTROL_INDEX,
    // intrinsic
    PRIMITIVE_BINDING,
    PRIMITIVE_DECORATOR,
    PRIMITIVE_IMPORT_META,
    PRIMITIVE_BIT,
    PRIMITIVE_ARITHMETIC,
    PRIMITIVE_CAST,
    PRIMITIVE_MATH,
    PRIMITIVE_MEMORY,
    PRIMITIVE_ATOMIC,
    PRIMITIVE_CONTROL,
    PRIMITIVE_VECTOR,
    PRIMITIVE_INDEX,
    // reflect
    REFLECT_PROPERTY,
    REFLECT_ANNOTATION,
    REFLECT_TYPE,
    REFLECT_INDEX,
    INTRINSIC_ROOT_INDEX,
    INTRINSIC_PRELUDE,
];

/// The prelude module source.
pub const PRELUDE_BUILTIN_SOURCE: &BuiltinSource = &INTRINSIC_PRELUDE;

#[cfg(test)]
mod tests {
    use super::*;

    fn category_index(path: &str) -> Option<&'static BuiltinSource> {
        match path {
            "control" => Some(&CONTROL_INDEX),
            "primitive" => Some(&PRIMITIVE_INDEX),
            "memory" => Some(&MEMORY_INDEX),
            "operator" => Some(&OPERATOR_INDEX),
            "reflect" => Some(&REFLECT_INDEX),
            _ => None,
        }
    }

    #[test]
    fn test_sources_not_empty() {
        assert!(!INTRINSIC_SOURCES.is_empty());
    }

    #[test]
    fn test_source_content_not_empty() {
        for source in INTRINSIC_SOURCES {
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
            "builtin://operator/arithmetic.ds"
        );
    }

    #[test]
    fn test_category_indexes_reexport_all_registered_leaf_sources() {
        for source in INTRINSIC_SOURCES {
            if source.name == "index.ds" || source.name == "prelude.ds" {
                continue;
            }

            let index = if source.path.is_empty() {
                &INTRINSIC_ROOT_INDEX
            } else {
                category_index(source.path).unwrap_or_else(|| {
                    panic!("missing category index for {}", source.virtual_path())
                })
            };
            let expected_reexport = format!("\"./{}\"", source.name);

            assert!(
                index.content.contains(&expected_reexport),
                "missing category index reexport for {} in {}",
                source.virtual_path(),
                index.virtual_path(),
            );
        }
    }

    #[test]
    fn test_core_index_reexports_all_registered_category_indexes() {
        let category_indexes = [
            &CONTROL_INDEX,
            &PRIMITIVE_INDEX,
            &MEMORY_INDEX,
            &OPERATOR_INDEX,
            &REFLECT_INDEX,
        ];

        for index in category_indexes {
            let expected_reexport = format!("\"./{}/{}\"", index.path, index.name);

            assert!(
                INTRINSIC_ROOT_INDEX.content.contains(&expected_reexport),
                "missing intrinsic index reexport for {} in {}",
                index.virtual_path(),
                INTRINSIC_ROOT_INDEX.virtual_path(),
            );
        }
    }
}
