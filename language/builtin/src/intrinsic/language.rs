use serde::{Deserialize, Serialize};

/// The kind of symbol a lang item expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageSymbolKind {
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

macro_rules! define_language_symbols {
    (
        $(
            $(#[$cat_attr:meta])*
            $category:ident {
                $(
                    $(#[$item_attr:meta])*
                    $name:ident => ($kind:ident, $module:literal, $export:literal),
                )*
            }
        )*
    ) => {
        /// Well-known language items that the compiler references.
        ///
        /// These are symbols from the builtin package that the compiler needs for:
        /// - Operator desugaring (`a + b` => `a.add(b)`)
        /// - Type descriptor generation
        /// - Special syntax handling (`?`, `??`, etc.)
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[allow(clippy::upper_case_acronyms)]
        pub enum LanguageSymbol {
            $($(
                $(#[$item_attr])*
                $name,
            )*)*
        }

        impl LanguageSymbol {
            /// The module path where this item is defined relative to intrinsic/.
            pub fn module(&self) -> &'static str {
                match self {
                    $($(Self::$name => $module,)*)*
                }
            }

            /// The exported name to look up in the module.
            pub fn export_name(&self) -> &'static str {
                match self {
                    $($(Self::$name => $export,)*)*
                }
            }

            /// The expected symbol kind.
            pub fn kind(&self) -> LanguageSymbolKind {
                match self {
                    $($(Self::$name => LanguageSymbolKind::$kind,)*)*
                }
            }

            /// Iterate over all lang items.
            pub fn all() -> impl Iterator<Item = Self> {
                const ALL: &[LanguageSymbol] = &[$($(LanguageSymbol::$name,)*)*];
                ALL.iter().copied()
            }
        }

        impl std::fmt::Display for LanguageSymbol {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.export_name())
            }
        }
    };
}

define_language_symbols! {
    /// Arithmetic operator interfaces.
    arithmetic {
        /// `+` operator: `a + b` => `a.add(b)`
        Add => (Interface, "operator/arithmetic", "Add"),

        /// `-` operator: `a - b` => `a.subtract(b)`
        Subtract => (Interface, "operator/arithmetic", "Subtract"),

        /// `*` operator: `a * b` => `a.multiply(b)`
        Multiply => (Interface, "operator/arithmetic", "Multiply"),

        /// `/` operator: `a / b` => `a.divide(b)`
        Divide => (Interface, "operator/arithmetic", "Divide"),

        /// `%` operator: `a % b` => `a.remainder(b)`
        Remainder => (Interface, "operator/arithmetic", "Remainder"),

        /// `**` operator: `a ** b` => `a.power(b)`
        Power => (Interface, "operator/arithmetic", "Power"),

        /// Unary `-`: `-a` => `a.negate()`
        Negate => (Interface, "operator/arithmetic", "Negate"),

        /// Unary `+`: `+a` => `a.plus()`
        Plus => (Interface, "operator/arithmetic", "Plus"),

        /// `&` operator: `a & b` => `a.and(b)`
        And => (Interface, "operator/arithmetic", "And"),

        /// `|` operator: `a | b` => `a.or(b)`
        Or => (Interface, "operator/arithmetic", "Or"),

        /// `^` operator: `a ^ b` => `a.xor(b)`
        Xor => (Interface, "operator/arithmetic", "Xor"),

        /// `~` operator: `~a` => `a.not()`
        Not => (Interface, "operator/arithmetic", "Not"),

        /// `<<` operator
        ShiftLeft => (Interface, "operator/arithmetic", "ShiftLeft"),

        /// `>>` operator
        ShiftRight => (Interface, "operator/arithmetic", "ShiftRight"),

        /// `>>>` operator
        ShiftRightUnsigned => (Interface, "operator/arithmetic", "ShiftRightUnsigned"),

        /// `++` or `+` for sequences
        Concatenate => (Interface, "operator/arithmetic", "Concatenate"),
    }

    /// Comparison operator interfaces.
    comparison {
        /// `==` and `!=` operators
        Equal => (Interface, "operator/comparison", "Equal"),

        /// `<`, `<=`, `>`, `>=` operators
        Compare => (Interface, "operator/comparison", "Compare"),

        /// Comparison result enum (Less, Equal, Greater)
        Ordering => (Enum, "operator/comparison", "Ordering"),

        /// Partial equality for types like float
        PartialEqual => (Interface, "operator/comparison", "PartialEqual"),

        /// Partial comparison for types like float
        PartialCompare => (Interface, "operator/comparison", "PartialCompare"),
    }

    /// Formatting operator interfaces.
    format {
        /// Interface for the `Display` trait
        Display => (Interface, "operator/format", "Display"),

        /// Interface for the `Debug` trait
        Debug => (Interface, "operator/format", "Debug"),
    }

    /// Subscript and dereference operators.
    subscript {
        /// `a[i]` access
        Index => (Interface, "operator/subscript", "Index"),

        /// `a[i] = v` assignment
        IndexSet => (Interface, "operator/subscript", "IndexSet"),

        /// `*a` dereference
        Deref => (Interface, "operator/subscript", "Deref"),

        /// `*a = v` dereference assignment
        DerefSet => (Interface, "operator/subscript", "DerefSet"),
    }

    /// Control flow types and interfaces.
    control {
        /// `?` operator for early return
        Try => (Interface, "control/try", "Try"),

        /// Try branch shape for ? and ??
        TryBranch => (TypeAlias, "control/try", "TryBranch"),

        /// Error interface for conventional error shapes
        Error => (Interface, "control/error", "Error"),

        /// Result type for ? operator
        Result => (Newtype, "control/result", "Result"),

        /// Ok variant
        Ok => (Struct, "control/result", "Ok"),

        /// Err variant
        Err => (Struct, "control/result", "Err"),

        /// Async result type
        AsyncResult => (Newtype, "control/result", "AsyncResult"),
    }

    /// Reflection types for type descriptors.
    reflect {
        /// The `Type<T>` union
        Type => (Newtype, "reflect/type", "Type"),

        /// `TypeBase<T>` interface
        TypeBase => (Interface, "reflect/type", "TypeBase"),

        /// `TypeId` newtype
        TypeId => (Newtype, "reflect/type", "TypeId"),

        /// `typeOf` intrinsic function
        TypeOf => (Function, "reflect/type", "typeOf"),

        /// Primitive type descriptor
        PrimitiveType => (Struct, "reflect/type", "PrimitiveType"),

        /// Struct type descriptor
        StructType => (Struct, "reflect/type", "StructType"),

        /// Class type descriptor
        ClassType => (Struct, "reflect/type", "ClassType"),

        /// Enum type descriptor
        EnumType => (Struct, "reflect/type", "EnumType"),

        /// Interface type descriptor
        InterfaceType => (Struct, "reflect/type", "InterfaceType"),

        /// Newtype type descriptor
        NewtypeType => (Struct, "reflect/type", "NewtypeType"),

        /// Array type descriptor
        ArrayType => (Struct, "reflect/type", "ArrayType"),

        /// Tuple type descriptor
        TupleType => (Struct, "reflect/type", "TupleType"),

        /// Union type descriptor
        UnionType => (Struct, "reflect/type", "UnionType"),

        /// Intersection type descriptor
        IntersectionType => (Struct, "reflect/type", "IntersectionType"),

        /// Function type descriptor
        FunctionType => (Struct, "reflect/type", "FunctionType"),

        /// Object type descriptor
        ObjectType => (Struct, "reflect/type", "ObjectType"),

        /// Property descriptor
        Property => (Struct, "reflect/property", "Property"),

        /// Enum variant descriptor
        Variant => (Struct, "reflect/property", "Variant"),
    }

    /// Decorator metadata (in reflect/).
    decorator_metadata {
        /// Decorator metadata
        DecoratorInfo => (Struct, "reflect/decorator", "DecoratorInfo"),
    }

    /// Intrinsic interfaces for compiler-known metadata.
    intrinsic {
        /// The `import.meta` interface.
        ImportMeta => (Interface, "primitive/import-meta", "ImportMeta"),

        /// The `import.meta.env` interface.
        ImportMetaEnv => (Interface, "primitive/import-meta", "ImportMetaEnv"),
    }

    /// Well-known decorator markers (in intrinsic/).
    decorator_markers {
        /// `@binding` marker
        Binding => (Newtype, "primitive/binding", "binding"),

        /// `@extern` marker
        Extern => (Newtype, "primitive/decorator", "extern"),

        /// `@deprecated` marker
        Deprecated => (Newtype, "primitive/decorator", "deprecated"),

        /// `@experimental` marker
        Experimental => (Newtype, "primitive/decorator", "experimental"),

        /// `@allow` marker
        Allow => (Newtype, "primitive/decorator", "allow"),

        /// `@warn` marker
        Warn => (Newtype, "primitive/decorator", "warn"),

        /// `@deny` marker
        Deny => (Newtype, "primitive/decorator", "deny"),

        /// `@forbid` marker
        Forbid => (Newtype, "primitive/decorator", "forbid"),

        /// `@expect` marker
        Expect => (Newtype, "primitive/decorator", "expect"),

        /// `@intrinsic` marker
        Intrinsic => (Newtype, "primitive/decorator", "intrinsic"),

        /// `@languageItem` marker
        LanguageItem => (Newtype, "primitive/decorator", "languageItem"),

        /// `@noManaged` marker
        NoManaged => (Newtype, "primitive/decorator", "noManaged"),

        /// `@stackOnly` marker
        StackOnly => (Newtype, "primitive/decorator", "stackOnly"),

        /// `@capture` marker
        Capture => (Newtype, "primitive/decorator", "capture"),

        /// `@inline` hint
        Inline => (Newtype, "primitive/decorator", "inline"),

        /// `@noinline` hint
        Noinline => (Newtype, "primitive/decorator", "noinline"),

        /// `@unroll` hint
        Unroll => (Newtype, "primitive/decorator", "unroll"),

        /// `@hot` hint
        Hot => (Newtype, "primitive/decorator", "hot"),

        /// `@cold` hint
        Cold => (Newtype, "primitive/decorator", "cold"),

        /// `@likely` hint
        Likely => (Newtype, "primitive/decorator", "likely"),

        /// `@unlikely` hint
        Unlikely => (Newtype, "primitive/decorator", "unlikely"),

        /// `@mustUse` marker
        MustUse => (Newtype, "primitive/decorator", "mustUse"),

        /// `@pure` marker
        Pure => (Newtype, "primitive/decorator", "pure"),

        /// `@tailcall` hint
        Tailcall => (Newtype, "primitive/decorator", "tailcall"),

        /// `@unsafe` marker
        Unsafe => (Newtype, "primitive/decorator", "unsafe"),

        /// `@transmute` marker
        Transmute => (Newtype, "primitive/decorator", "transmute"),

        /// `@taint` marker
        Taint => (Newtype, "primitive/decorator", "taint"),

        /// `@sink` marker
        Sink => (Newtype, "primitive/decorator", "sink"),

        /// `@sanitizer` marker
        Sanitizer => (Newtype, "primitive/decorator", "sanitizer"),

        /// `@tag` marker
        Tag => (Newtype, "primitive/decorator", "tag"),

        /// `@lifetime` marker
        Lifetime => (Newtype, "primitive/decorator", "lifetime"),

        /// `@space` marker
        Space => (Newtype, "primitive/decorator", "space"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_properties() {
        assert_eq!(LanguageSymbol::Add.module(), "operator/arithmetic");
        assert_eq!(LanguageSymbol::Add.export_name(), "Add");
        assert_eq!(LanguageSymbol::Add.kind(), LanguageSymbolKind::Interface);
    }
}
