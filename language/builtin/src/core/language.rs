/// The kind of symbol a lang item expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageItemKind {
    /// A `newtype interface` declaration.
    Interface,
    /// A `struct` declaration.
    Struct,
    /// An `enum` declaration.
    Enum,
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
                    $name:ident => ($kind:ident, $module:literal, $export:literal, $req:ident),
                )*
            }
        )*
    ) => {
        /// Well-known language items that the compiler references.
        ///
        /// These are symbols from the builtin package that the compiler needs for:
        /// - Operator desugaring (`a + b` => `a.add(b)`)
        /// - Type descriptor generation
        /// - Special syntax handling (`?`, `..`, etc.)
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[allow(clippy::upper_case_acronyms)]
        pub enum LanguageItem {
            $($(
                $(#[$item_attr])*
                $name,
            )*)*
        }

        impl LanguageItem {
            /// The module path where this item is defined (relative to core/).
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
            pub fn kind(&self) -> LanguageItemKind {
                match self {
                    $($(Self::$name => LanguageItemKind::$kind,)*)*
                }
            }

            /// Whether this item is required (vs optional).
            pub fn is_required(&self) -> bool {
                match self {
                    $($(Self::$name => define_language_items!(@is_req $req),)*)*
                }
            }

            /// Iterate over all lang items.
            pub fn all() -> impl Iterator<Item = Self> {
                const ALL: &[LanguageItem] = &[$($(LanguageItem::$name,)*)*];
                ALL.iter().copied()
            }

            /// Iterate over all required lang items.
            pub fn required() -> impl Iterator<Item = Self> {
                Self::all().filter(|item| item.is_required())
            }
        }

        impl std::fmt::Display for LanguageItem {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.export_name())
            }
        }
    };

    (@is_req required) => { true };
    (@is_req optional) => { false };
}

define_language_items! {
    /// Arithmetic operator interfaces.
    arithmetic {
        /// `+` operator: `a + b` => `a.add(b)`
        Add => (Interface, "operator/arithmetic", "Add", required),

        /// `-` operator: `a - b` => `a.subtract(b)`
        Subtract => (Interface, "operator/arithmetic", "Subtract", required),

        /// `*` operator: `a * b` => `a.multiply(b)`
        Multiply => (Interface, "operator/arithmetic", "Multiply", required),

        /// `/` operator: `a / b` => `a.divide(b)`
        Divide => (Interface, "operator/arithmetic", "Divide", required),

        /// `%` operator: `a % b` => `a.remainder(b)`
        Remainder => (Interface, "operator/arithmetic", "Remainder", required),

        /// `**` operator: `a ** b` => `a.power(b)`
        Power => (Interface, "operator/arithmetic", "Power", required),

        /// Unary `-`: `-a` => `a.negate()`
        Negate => (Interface, "operator/arithmetic", "Negate", required),

        /// Unary `+`: `+a` => `a.plus()`
        Plus => (Interface, "operator/arithmetic", "Plus", optional),

        /// `&` operator: `a & b` => `a.and(b)`
        And => (Interface, "operator/arithmetic", "And", required),

        /// `|` operator: `a | b` => `a.or(b)`
        Or => (Interface, "operator/arithmetic", "Or", required),

        /// `^` operator: `a ^ b` => `a.xor(b)`
        Xor => (Interface, "operator/arithmetic", "Xor", required),

        /// `~` operator: `~a` => `a.not()`
        Not => (Interface, "operator/arithmetic", "Not", required),

        /// `<<` operator
        ShiftLeft => (Interface, "operator/arithmetic", "ShiftLeft", required),

        /// `>>` operator
        ShiftRight => (Interface, "operator/arithmetic", "ShiftRight", required),

        /// `>>>` operator
        ShiftRightUnsigned => (Interface, "operator/arithmetic", "ShiftRightUnsigned", required),

        /// `++` or `+` for sequences
        Concatenate => (Interface, "operator/arithmetic", "Concatenate", required),
    }

    /// Comparison operator interfaces.
    comparison {
        /// `==` and `!=` operators
        Equal => (Interface, "operator/comparison", "Equal", required),

        /// `<`, `<=`, `>`, `>=` operators
        Compare => (Interface, "operator/comparison", "Compare", required),

        /// Comparison result enum (Less, Equal, Greater)
        Ordering => (Enum, "operator/comparison", "Ordering", required),

        /// Partial equality for types like float
        PartialEqual => (Interface, "operator/comparison", "PartialEqual", optional),

        /// Partial comparison for types like float
        PartialCompare => (Interface, "operator/comparison", "PartialCompare", optional),
    }

    /// String operator interfaces.
    string {
        /// Interface for the `Display` trait
        Display => (Interface, "operator/string", "Display", required),

        /// Interface for the `Debug` trait
        Debug => (Interface, "operator/string", "Debug", required),
    }

    /// Subscript and dereference operators.
    subscript {
        /// `a[i]` access
        Index => (Interface, "operator/subscript", "Index", required),

        /// `a[i] = v` assignment
        IndexSet => (Interface, "operator/subscript", "IndexSet", required),

        /// `*a` dereference
        Deref => (Interface, "operator/subscript", "Deref", optional),

        /// `*a = v` dereference assignment
        DerefSet => (Interface, "operator/subscript", "DerefSet", optional),
    }

    /// Control flow operators.
    control {
        /// `?` operator for early return
        Try => (Interface, "operator/control", "Try", required),

        /// Control flow enum (Continue, Break)
        ControlFlow => (Enum, "operator/control", "ControlFlow", required),

        /// Range bound enum (Included, Excluded, Unbounded)
        Bound => (Enum, "operator/control", "Bound", required),

        /// Range bounds interface
        RangeBounds => (Interface, "operator/control", "RangeBounds", required),

        /// `start..end` exclusive range
        Range => (Struct, "operator/control", "Range", required),

        /// `start..=end` inclusive range
        RangeInclusive => (Struct, "operator/control", "RangeInclusive", required),

        /// `start..` range from
        RangeFrom => (Struct, "operator/control", "RangeFrom", required),

        /// `..end` range to (exclusive)
        RangeTo => (Struct, "operator/control", "RangeTo", required),

        /// `..=end` range to (inclusive)
        RangeToInclusive => (Struct, "operator/control", "RangeToInclusive", required),

        /// `..` full range
        RangeFull => (Struct, "operator/control", "RangeFull", required),
    }

    /// Reflection types for type descriptors.
    reflection {
        /// The `Type<T>` union
        Type => (Newtype, "reflection/type", "Type", required),

        /// `TypeBase<T>` interface
        TypeBase => (Interface, "reflection/type", "TypeBase", required),

        /// `TypeId` newtype
        TypeId => (Newtype, "reflection/type", "TypeId", required),

        /// `typeOf` intrinsic function
        TypeOf => (Function, "reflection/type", "typeOf", required),

        /// Primitive type descriptor
        PrimitiveType => (Struct, "reflection/type", "PrimitiveType", required),

        /// Struct type descriptor
        StructType => (Struct, "reflection/type", "StructType", required),

        /// Class type descriptor
        ClassType => (Struct, "reflection/type", "ClassType", required),

        /// Enum type descriptor
        EnumType => (Struct, "reflection/type", "EnumType", required),

        /// Interface type descriptor
        InterfaceType => (Struct, "reflection/type", "InterfaceType", required),

        /// Newtype type descriptor
        NewtypeType => (Struct, "reflection/type", "NewtypeType", required),

        /// Array type descriptor
        ArrayType => (Struct, "reflection/type", "ArrayType", required),

        /// Tuple type descriptor
        TupleType => (Struct, "reflection/type", "TupleType", required),

        /// Union type descriptor
        UnionType => (Struct, "reflection/type", "UnionType", required),

        /// Intersection type descriptor
        IntersectionType => (Struct, "reflection/type", "IntersectionType", required),

        /// Function type descriptor
        FunctionType => (Struct, "reflection/type", "FunctionType", required),

        /// Object type descriptor
        ObjectType => (Struct, "reflection/type", "ObjectType", required),

        /// Refined type descriptor
        RefinedType => (Struct, "reflection/type", "RefinedType", required),

        /// Property descriptor
        Property => (Struct, "reflection/property", "Property", required),

        /// Enum variant descriptor
        Variant => (Struct, "reflection/property", "Variant", required),

        /// Refinement descriptor
        Refinement => (Struct, "reflection/refinement", "Refinement", required),

        /// Decorator info
        DecoratorInfo => (Struct, "reflection/decorator", "DecoratorInfo", required),
    }

    /// Well-known decorators.
    decorators {
        /// `@intrinsic` marker
        Intrinsic => (Newtype, "reflection/decorator", "intrinsic", required),

        /// `@deprecated` marker
        Deprecated => (Newtype, "reflection/decorator", "deprecated", optional),

        /// `@inline` hint
        Inline => (Newtype, "reflection/decorator", "inline", optional),

        /// `@noinline` hint
        Noinline => (Newtype, "reflection/decorator", "noinline", optional),

        /// `@experimental` marker
        Experimental => (Newtype, "reflection/decorator", "experimental", optional),

        /// `@unroll` hint
        Unroll => (Newtype, "reflection/decorator", "unroll", optional),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_item_all() {
        // all items should be iterable
        let count = LanguageItem::all().count();
        assert!(count > 0);
    }

    #[test]
    fn test_language_item_required() {
        // required subset should be smaller than all
        let all_count = LanguageItem::all().count();
        let req_count = LanguageItem::required().count();
        assert!(req_count <= all_count);
        assert!(req_count > 0);
    }

    #[test]
    fn test_add_properties() {
        assert_eq!(LanguageItem::Add.module(), "operator/arithmetic");
        assert_eq!(LanguageItem::Add.export_name(), "Add");
        assert_eq!(LanguageItem::Add.kind(), LanguageItemKind::Interface);
        assert!(LanguageItem::Add.is_required());
    }
}
