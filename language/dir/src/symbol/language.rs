use serde::{Deserialize, Serialize};

/// The declaration form expected for one language item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageItemForm {
    /// A variable-like value declaration.
    Variable,
    /// A `class` declaration.
    Class,
    /// A `newtype interface` declaration.
    Interface,
    /// A `struct` declaration.
    Struct,
    /// An `enum` declaration.
    Enum,
    /// A `type` alias declaration.
    TypeAlias,
    /// A `newtype` declaration.
    Newtype,
    /// A `function` declaration.
    Function,
}

macro_rules! define_language_items {
    (
        $(
            $(#[$cat_attr:meta])*
            $category:ident {
                $(
                    $(#[$item_attr:meta])*
                    $name:ident => ($form:ident, $module:literal, $export:literal),
                )*
            }
        )*
    ) => {
        /// Well-known language library items that the toolchain references.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[allow(clippy::upper_case_acronyms)]
        pub enum LanguageItem {
            $($(
                $(#[$item_attr])*
                $name,
            )*)*
        }

        impl LanguageItem {
            /// Return the language library module path where this item is defined.
            pub fn module(&self) -> &'static str {
                match self {
                    $($(Self::$name => $module,)*)*
                }
            }

            /// Return the exported name to look up in the module.
            pub fn export_name(&self) -> &'static str {
                match self {
                    $($(Self::$name => $export,)*)*
                }
            }

            /// Return the expected declaration form.
            pub fn form(&self) -> LanguageItemForm {
                match self {
                    $($(Self::$name => LanguageItemForm::$form,)*)*
                }
            }

            /// Iterate over all language items.
            pub fn all() -> impl Iterator<Item = Self> {
                const ALL: &[LanguageItem] = &[$($(LanguageItem::$name,)*)*];
                ALL.iter().copied()
            }
        }

        impl std::fmt::Display for LanguageItem {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.export_name())
            }
        }
    };
}

define_language_items! {
    /// Operator interfaces.
    operator {
        /// `+` operator: `a + b` => `a.add(b)`
        Add => (Interface, "operator/plus", "Add"),

        /// `-` operator: `a - b` => `a.subtract(b)`
        Subtract => (Interface, "operator/minus", "Subtract"),

        /// Unary `-`: `-a` => `a.negate()`
        Negate => (Interface, "operator/negate", "Negate"),

        /// `*` operator: `a * b` => `a.multiply(b)`
        Multiply => (Interface, "operator/multiply", "Multiply"),

        /// `/` operator: `a / b` => `a.divide(b)`
        Divide => (Interface, "operator/divide", "Divide"),

        /// `%` operator: `a % b` => `a.remainder(b)`
        Remainder => (Interface, "operator/remainder", "Remainder"),

        /// `**` operator: `a ** b` => `a.power(b)`
        Power => (Interface, "operator/power", "Power"),

        /// Unary `+`: `+a` => `a.plus()`
        Plus => (Interface, "operator/plus", "Plus"),

        /// `&` operator: `a & b` => `a.and(b)`
        And => (Interface, "operator/bitwise", "And"),

        /// `|` operator: `a | b` => `a.or(b)`
        Or => (Interface, "operator/bitwise", "Or"),

        /// `^` operator: `a ^ b` => `a.xor(b)`
        Xor => (Interface, "operator/bitwise", "Xor"),

        /// `~` operator: `~a` => `a.not()`
        Not => (Interface, "operator/bitwise", "Not"),

        /// `<<` operator
        ShiftLeft => (Interface, "operator/shift", "ShiftLeft"),

        /// `>>` operator
        ShiftRight => (Interface, "operator/shift", "ShiftRight"),

        /// `>>>` operator
        ShiftRightUnsigned => (Interface, "operator/shift", "ShiftRightUnsigned"),
    }

    /// Equality operator interfaces.
    equality {
        /// `==` and `!=` operators
        Equal => (Interface, "operator/equality", "Equal"),

        /// Partial equality for types like float
        PartialEqual => (Interface, "operator/equality", "PartialEqual"),
    }

    /// Comparison operator interfaces.
    comparison {
        /// `<`, `<=`, `>`, `>=` operators
        Compare => (Interface, "operator/comparison", "Compare"),

        /// Comparison result enum (Less, Equal, Greater)
        Ordering => (Enum, "operator/comparison", "Ordering"),

        /// Partial comparison for types like float
        PartialCompare => (Interface, "operator/comparison", "PartialCompare"),
    }

    /// Formatting operator interfaces.
    format {
        /// Interface for the `Display` trait.
        Display => (Interface, "operator/format", "Display"),

        /// Interface for the `Debug` trait.
        Debug => (Interface, "operator/format", "Debug"),
    }

    /// Subscript operators.
    subscript {
        /// `a[i]` access
        Index => (Interface, "operator/subscript", "Index"),

        /// `a[i] = v` assignment
        IndexSet => (Interface, "operator/subscript", "IndexSet"),
    }

    /// Dereference operators.
    dereference {
        /// `*a` readonly dereference
        ReadonlyDereference => (Interface, "operator/dereference", "ReadonlyDereference"),

        /// `*a = v` mutable dereference
        Dereference => (Interface, "operator/dereference", "Dereference"),
    }

    /// Result and error types.
    result {
        /// `?` operator for early return
        Try => (Interface, "operator/try", "Try"),

        /// Try branch shape for ? and ??
        TryBranch => (TypeAlias, "operator/try", "TryBranch"),

        /// Error interface for conventional error shapes
        Error => (Interface, "error/error", "Error"),

        /// Result type for ? operator
        Result => (Newtype, "error/result", "Result"),

        /// Ok variant
        Ok => (Struct, "error/result", "Ok"),

        /// Err variant
        Err => (Struct, "error/result", "Err"),

        /// Async result type
        AsyncResult => (Newtype, "error/result", "AsyncResult"),
    }

    /// Reflection types and layout queries.
    reflect {
        /// Reflected semantic type.
        Type => (TypeAlias, "reflect/type", "Type"),

        /// Stable type identifier.
        TypeId => (Newtype, "reflect/type", "TypeId"),

        /// Target-specific layout.
        Layout => (Struct, "reflect/type", "Layout"),

        /// Target-specific field layout.
        LayoutField => (Struct, "reflect/type", "LayoutField"),

        /// Runtime type query intrinsic.
        TypeOf => (Function, "reflect/type", "typeOf"),

        /// Size query intrinsic.
        SizeOf => (Function, "reflect/type", "sizeOf"),

        /// Alignment query intrinsic.
        AlignOf => (Function, "reflect/type", "alignOf"),

        /// Stride query intrinsic.
        StrideOf => (Function, "reflect/type", "strideOf"),

        /// Layout query intrinsic.
        LayoutOf => (Function, "reflect/type", "layoutOf"),
    }

    /// Intrinsic interfaces for compiler-known metadata.
    intrinsic {
        /// The `import.meta` interface.
        ImportMeta => (Interface, "module/meta", "ImportMeta"),

        /// The `import.meta.env` interface.
        ImportMetaEnv => (Interface, "module/meta", "ImportMetaEnv"),
    }

    /// Well-known decorator markers.
    decorator_markers {
        /// `@binding` marker
        Binding => (Newtype, "platform/core/binding", "binding"),

        /// `@extern` marker
        Extern => (Newtype, "decorator/metadata", "extern"),

        /// `@deprecated` marker
        Deprecated => (Newtype, "decorator/metadata", "deprecated"),

        /// `@experimental` marker
        Experimental => (Newtype, "decorator/metadata", "experimental"),

        /// `@allow` marker
        Allow => (Newtype, "decorator/diagnostic", "allow"),

        /// `@warn` marker
        Warn => (Newtype, "decorator/diagnostic", "warn"),

        /// `@deny` marker
        Deny => (Newtype, "decorator/diagnostic", "deny"),

        /// `@forbid` marker
        Forbid => (Newtype, "decorator/diagnostic", "forbid"),

        /// `@expect` marker
        Expect => (Newtype, "decorator/diagnostic", "expect"),

        /// `@intrinsic` marker
        Intrinsic => (Newtype, "decorator/intrinsic", "intrinsic"),

        /// `@languageItem` marker
        LanguageItem => (Newtype, "decorator/intrinsic", "languageItem"),

        /// `@noManaged` marker
        NoManaged => (Newtype, "decorator/memory", "noManaged"),

        /// `@noHeap` marker
        NoHeap => (Newtype, "decorator/memory", "noHeap"),

        /// `@capture` marker
        Capture => (Newtype, "decorator/capture", "capture"),
    }

    /// Patch protocol items.
    patch {
        /// Expansion context.
        ExpandContext => (Struct, "module/patch", "ExpandContext"),

        /// Materialization context.
        MaterializeContext => (Struct, "module/patch", "MaterializeContext"),

        /// Expansion add patch variant.
        ExpandPatchAdd => (Newtype, "module/patch", "ExpandPatchAdd"),

        /// Expansion replace patch variant.
        ExpandPatchReplace => (Newtype, "module/patch", "ExpandPatchReplace"),

        /// Expansion rename patch variant.
        ExpandPatchRename => (Newtype, "module/patch", "ExpandPatchRename"),

        /// Expansion remove patch variant.
        ExpandPatchRemove => (Newtype, "module/patch", "ExpandPatchRemove"),

        /// Expansion patch union.
        ExpandPatch => (Newtype, "module/patch", "ExpandPatch"),

        /// Materialization body patch variant.
        MaterializePatchBody => (Newtype, "module/patch", "MaterializePatchBody"),

        /// Materialization expression patch variant.
        MaterializePatchExpression => (Newtype, "module/patch", "MaterializePatchExpression"),

        /// Materialization remove patch variant.
        MaterializePatchRemove => (Newtype, "module/patch", "MaterializePatchRemove"),

        /// Materialization patch union.
        MaterializePatch => (Newtype, "module/patch", "MaterializePatch"),

        /// Patcher protocol.
        Patcher => (Interface, "module/patch", "Patcher"),
    }

    /// Derive provider items.
    derive {
        /// The `Tagged` derive provider.
        Tagged => (Newtype, "decorator/derive", "Tagged"),

        /// The `Clone` derive provider.
        CloneDerive => (Newtype, "decorator/derive", "Clone"),

        /// The `Debug` derive provider.
        DebugDerive => (Newtype, "decorator/derive", "Debug"),
    }

    /// System decorator markers.
    system_decorator_markers {
        /// `@inline` hint
        Inline => (Newtype, "decorator/system", "inline"),

        /// `@noinline` hint
        Noinline => (Newtype, "decorator/system", "noinline"),

        /// `@unroll` hint
        Unroll => (Newtype, "decorator/system", "unroll"),

        /// `@hot` hint
        Hot => (Newtype, "decorator/system", "hot"),

        /// `@cold` hint
        Cold => (Newtype, "decorator/system", "cold"),

        /// `@likely` hint
        Likely => (Newtype, "decorator/system", "likely"),

        /// `@unlikely` hint
        Unlikely => (Newtype, "decorator/system", "unlikely"),

        /// `@mustUse` marker
        MustUse => (Newtype, "decorator/system", "mustUse"),

        /// `@pure` marker
        Pure => (Newtype, "decorator/system", "pure"),

        /// `@tailcall` hint
        Tailcall => (Newtype, "decorator/system", "tailcall"),
    }

    /// Security decorator markers.
    security_decorator_markers {
        /// `@unsafe` marker
        Unsafe => (Newtype, "decorator/security", "unsafe"),

        /// `@transmute` marker
        Transmute => (Newtype, "decorator/security", "transmute"),

        /// `@taint` marker
        Taint => (Newtype, "decorator/security", "taint"),

        /// `@sink` marker
        Sink => (Newtype, "decorator/security", "sink"),

        /// `@sanitizer` marker
        Sanitizer => (Newtype, "decorator/security", "sanitizer"),
    }

    /// Metadata decorator markers.
    metadata_decorator_markers {
        /// `@tag` marker
        Tag => (Newtype, "decorator/metadata", "tag"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_properties() {
        assert_eq!(LanguageItem::Add.module(), "operator/plus");
        assert_eq!(LanguageItem::Add.export_name(), "Add");
        assert_eq!(LanguageItem::Add.form(), LanguageItemForm::Interface);
    }
}
