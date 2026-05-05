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

    /// Reflection types for type handles and shapes.
    reflect {
        /// Opaque `Type<T>` handle.
        Type => (Newtype, "reflect/type", "Type"),

        /// Stable type identifier.
        TypeId => (Newtype, "reflect/type", "TypeId"),

        /// Stable type symbol identifier.
        TypeSymbolId => (Newtype, "reflect/type", "TypeSymbolId"),

        /// Type declaration kind alias.
        TypeDeclarationKind => (TypeAlias, "reflect/type", "TypeDeclarationKind"),

        /// Type symbol descriptor.
        TypeSymbol => (Struct, "reflect/type", "TypeSymbol"),

        /// Static key alias.
        StaticKey => (TypeAlias, "reflect/type", "StaticKey"),

        /// Static argument alias.
        StaticArgument => (TypeAlias, "reflect/type", "StaticArgument"),

        /// Type argument descriptor.
        TypeArgument => (Struct, "reflect/type", "TypeArgument"),

        /// Value argument descriptor.
        ValueArgument => (Struct, "reflect/type", "ValueArgument"),

        /// Type literal alias.
        TypeLiteral => (TypeAlias, "reflect/type", "TypeLiteral"),

        /// Intrinsic type alias.
        IntrinsicType => (TypeAlias, "reflect/type", "IntrinsicType"),

        /// Primitive category alias.
        Primitive => (TypeAlias, "reflect/type", "Primitive"),

        /// Integer type descriptor.
        IntegerType => (Struct, "reflect/type", "IntegerType"),

        /// Float type alias.
        FloatType => (TypeAlias, "reflect/type", "FloatType"),

        /// Scalar literal alias.
        ScalarLiteral => (TypeAlias, "reflect/type", "ScalarLiteral"),

        /// Type shape alias.
        TypeShape => (TypeAlias, "reflect/type", "TypeShape"),

        /// Type literal shape.
        LiteralType => (Struct, "reflect/type", "LiteralType"),

        /// Primitive type shape.
        PrimitiveType => (Struct, "reflect/type", "PrimitiveType"),

        /// Scalar literal type shape.
        ScalarLiteralType => (Struct, "reflect/type", "ScalarLiteralType"),

        /// This type shape.
        ThisType => (Struct, "reflect/type", "ThisType"),

        /// Reference type shape.
        ReferenceType => (Struct, "reflect/type", "ReferenceType"),

        /// Value type shape.
        ValueType => (Struct, "reflect/type", "ValueType"),

        /// Conditional type shape.
        ConditionalType => (Struct, "reflect/type", "ConditionalType"),

        /// Mapped type modifier alias.
        MappedTypeModifier => (TypeAlias, "reflect/type", "MappedTypeModifier"),

        /// Mapped type modifiers.
        MappedTypeModifiers => (Struct, "reflect/type", "MappedTypeModifiers"),

        /// Mapped type parameter.
        MappedTypeParameter => (Struct, "reflect/type", "MappedTypeParameter"),

        /// Mapped type shape.
        MappedType => (Struct, "reflect/type", "MappedType"),

        /// Index type shape.
        IndexType => (Struct, "reflect/type", "IndexType"),

        /// Template literal type shape.
        TemplateLiteralType => (Struct, "reflect/type", "TemplateLiteralType"),

        /// Import type shape.
        ImportType => (Struct, "reflect/type", "ImportType"),

        /// Infer type shape.
        InferType => (Struct, "reflect/type", "InferType"),

        /// Predicate subject alias.
        PredicateSubject => (TypeAlias, "reflect/type", "PredicateSubject"),

        /// Predicate type shape.
        PredicateType => (Struct, "reflect/type", "PredicateType"),

        /// Readonly type shape.
        ReadonlyType => (Struct, "reflect/type", "ReadonlyType"),

        /// Keyof type shape.
        KeyOfType => (Struct, "reflect/type", "KeyOfType"),

        /// Must type shape.
        MustType => (Struct, "reflect/type", "MustType"),

        /// Comptime type shape.
        ComptimeType => (Struct, "reflect/type", "ComptimeType"),

        /// Not type shape.
        NotType => (Struct, "reflect/type", "NotType"),

        /// Mutability alias.
        Mutability => (TypeAlias, "reflect/type", "Mutability"),

        /// Variance bound alias.
        VarianceBound => (TypeAlias, "reflect/type", "VarianceBound"),

        /// Value-of type shape.
        ValueOfType => (Struct, "reflect/type", "ValueOfType"),

        /// Reference-of type shape.
        ReferenceOfType => (Struct, "reflect/type", "ReferenceOfType"),

        /// Pointer-of type shape.
        PointerOfType => (Struct, "reflect/type", "PointerOfType"),

        /// In type shape.
        InType => (Struct, "reflect/type", "InType"),

        /// Extends type shape.
        ExtendsType => (Struct, "reflect/type", "ExtendsType"),

        /// Implements type shape.
        ImplementsType => (Struct, "reflect/type", "ImplementsType"),

        /// Fixed array type shape.
        FixedArrayType => (Struct, "reflect/type", "FixedArrayType"),

        /// Array type shape.
        ArrayType => (Struct, "reflect/type", "ArrayType"),

        /// Slice type shape.
        SliceType => (Struct, "reflect/type", "SliceType"),

        /// Tuple element shape.
        TupleElement => (Struct, "reflect/type", "TupleElement"),

        /// Tuple type shape.
        TupleType => (Struct, "reflect/type", "TupleType"),

        /// Object field shape.
        ObjectField => (Struct, "reflect/type", "ObjectField"),

        /// Object index signature shape.
        ObjectIndexSignature => (Struct, "reflect/type", "ObjectIndexSignature"),

        /// Object type shape.
        ObjectType => (Struct, "reflect/type", "ObjectType"),

        /// Function asynchrony alias.
        FunctionAsynchrony => (TypeAlias, "reflect/type", "FunctionAsynchrony"),

        /// Function cardinality alias.
        FunctionCardinality => (TypeAlias, "reflect/type", "FunctionCardinality"),

        /// Function parameter shape.
        FunctionParameter => (Struct, "reflect/type", "FunctionParameter"),

        /// Function type shape.
        FunctionType => (Struct, "reflect/type", "FunctionType"),

        /// Union type shape.
        UnionType => (Struct, "reflect/type", "UnionType"),

        /// Intersection type shape.
        IntersectionType => (Struct, "reflect/type", "IntersectionType"),

        /// Form type shape.
        FormType => (Struct, "reflect/type", "FormType"),

        /// Layout kind alias.
        LayoutKind => (TypeAlias, "reflect/type", "LayoutKind"),

        /// Layout descriptor.
        LayoutDescriptor => (Struct, "reflect/type", "LayoutDescriptor"),

        /// Layout field descriptor.
        LayoutFieldDescriptor => (Struct, "reflect/type", "LayoutFieldDescriptor"),

        /// Runtime type handle intrinsic.
        TypeOf => (Function, "reflect/type", "typeOf"),

        /// Stable type identity intrinsic.
        IdOf => (Function, "reflect/type", "idOf"),

        /// Type symbol intrinsic.
        SymbolOf => (Function, "reflect/type", "symbolOf"),

        /// Type shape intrinsic.
        ShapeOf => (Function, "reflect/type", "shapeOf"),

        /// Type display name intrinsic.
        DisplayNameOf => (Function, "reflect/type", "displayNameOf"),

        /// Size query intrinsic.
        SizeOf => (Function, "reflect/type", "sizeOf"),

        /// Alignment query intrinsic.
        AlignOf => (Function, "reflect/type", "alignOf"),

        /// Stride query intrinsic.
        StrideOf => (Function, "reflect/type", "strideOf"),

        /// Layout query intrinsic.
        LayoutOf => (Function, "reflect/type", "layoutOf"),

        /// Generic argument kind alias.
        GenericArgumentKind => (TypeAlias, "reflect/property", "GenericArgumentKind"),

        /// Signature asynchrony alias.
        SignatureAsynchrony => (TypeAlias, "reflect/property", "SignatureAsynchrony"),

        /// Signature cardinality alias.
        SignatureCardinality => (TypeAlias, "reflect/property", "SignatureCardinality"),

        /// Generic argument descriptor.
        GenericArgumentDescriptor => (Struct, "reflect/property", "GenericArgumentDescriptor"),

        /// Field descriptor.
        FieldDescriptor => (Struct, "reflect/property", "FieldDescriptor"),

        /// Method descriptor.
        MethodDescriptor => (Struct, "reflect/property", "MethodDescriptor"),

        /// Parameter descriptor.
        ParameterDescriptor => (Struct, "reflect/property", "ParameterDescriptor"),

        /// Signature descriptor.
        SignatureDescriptor => (Struct, "reflect/property", "SignatureDescriptor"),

        /// Tuple element descriptor.
        TupleElementDescriptor => (Struct, "reflect/property", "TupleElementDescriptor"),

        /// Index signature descriptor.
        IndexSignatureDescriptor => (Struct, "reflect/property", "IndexSignatureDescriptor"),

        /// Variant descriptor.
        VariantDescriptor => (Struct, "reflect/property", "VariantDescriptor"),

        /// Generic argument reflection intrinsic.
        GenericArgumentsOf => (Function, "reflect/property", "genericArgumentsOf"),

        /// Field reflection intrinsic.
        FieldsOf => (Function, "reflect/property", "fieldsOf"),

        /// Method reflection intrinsic.
        MethodsOf => (Function, "reflect/property", "methodsOf"),

        /// Call signature reflection intrinsic.
        CallSignaturesOf => (Function, "reflect/property", "callSignaturesOf"),

        /// Construct signature reflection intrinsic.
        ConstructSignaturesOf => (Function, "reflect/property", "constructSignaturesOf"),

        /// Tuple element reflection intrinsic.
        ElementsOf => (Function, "reflect/property", "elementsOf"),

        /// Index signature reflection intrinsic.
        IndexSignaturesOf => (Function, "reflect/property", "indexSignaturesOf"),

        /// Variant reflection intrinsic.
        VariantsOf => (Function, "reflect/property", "variantsOf"),
    }

    /// Annotation reflection metadata.
    annotation_metadata {
        /// Annotation effect alias.
        AnnotationEffect => (TypeAlias, "reflect/annotation", "AnnotationEffect"),

        /// Documentation descriptor.
        DocumentationDescriptor => (Struct, "reflect/annotation", "DocumentationDescriptor"),

        /// Documentation tag descriptor.
        DocumentationTagDescriptor => (Struct, "reflect/annotation", "DocumentationTagDescriptor"),

        /// Annotation descriptor.
        AnnotationDescriptor => (Struct, "reflect/annotation", "AnnotationDescriptor"),

        /// Annotation reflection intrinsic.
        AnnotationsOf => (Function, "reflect/annotation", "annotationsOf"),
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
        assert_eq!(LanguageSymbol::Add.module(), "operator/plus");
        assert_eq!(LanguageSymbol::Add.export_name(), "Add");
        assert_eq!(LanguageSymbol::Add.kind(), LanguageSymbolKind::Interface);
    }
}
