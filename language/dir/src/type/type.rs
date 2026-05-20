use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Asynchrony, GlobalSymbolId, LocalStaticId, ScalarLiteral, StaticArgument, StaticKey, StringId,
};

use super::PrimitiveType;

/// Compiler-provided type function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltinTypeFunction {
    /// Uppercase string intrinsic.
    Uppercase,
    /// Lowercase string intrinsic.
    Lowercase,
    /// Capitalize string intrinsic.
    Capitalize,
    /// Uncapitalize string intrinsic.
    Uncapitalize,
    /// NoInfer intrinsic.
    NoInfer,
    /// Builtin iterator return intrinsic.
    BuiltinIteratorReturn,
}

impl TryFrom<&str> for BuiltinTypeFunction {
    /// The error type for intrinsic parsing.
    type Error = ();

    /// Parse a builtin type function from its standard name.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Uppercase" => Ok(Self::Uppercase),
            "Lowercase" => Ok(Self::Lowercase),
            "Capitalize" => Ok(Self::Capitalize),
            "Uncapitalize" => Ok(Self::Uncapitalize),
            "NoInfer" => Ok(Self::NoInfer),
            "BuiltinIteratorReturn" => Ok(Self::BuiltinIteratorReturn),
            _ => Err(()),
        }
    }
}

/// A mapped-type modifier in semantic type space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeMappedModifier {
    /// The plain modifier without an explicit sign.
    Present,
    /// Add a modifier with an explicit `+` sign.
    Add,
    /// Remove a modifier like `-readonly` or `-?`.
    Remove,
    /// No modifier specified.
    None,
}

/// Semantic mapped-type modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappedTypeModifiers {
    /// The readonly modifier.
    pub readonly: TypeMappedModifier,
    /// The optional modifier.
    pub optional: TypeMappedModifier,
}

/// A semantic mapped-type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappedTypeParameter {
    /// The parameter name like `K`.
    pub name: StringId,
    /// The parameter symbol.
    pub symbol: GlobalSymbolId,
    /// The constraint type like `keyof T`.
    pub constraint: LocalTypeId,
    /// The optional key remap like `as Foo<K>`.
    pub key_remap: Option<LocalTypeId>,
}

/// A semantic type parameter reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterType {
    /// The referenced generic parameter symbol.
    pub symbol: GlobalSymbolId,
}

/// Reference to one named type declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedType {
    /// The referenced declaration symbol.
    pub symbol: GlobalSymbolId,
    /// The static arguments applied to the reference.
    pub arguments: Vec<StaticArgument>,
}

/// Explicit erased runtime `Any<T>` representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErasedAnyType {
    /// The erased `Any<T>` constraint.
    pub constraint: LocalTypeId,
}

/// Canonical memory or access form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormType {
    /// The form constructor.
    pub form: Form,
    /// The type carried by the form.
    pub value: LocalTypeId,
}

/// Canonical memory or access form constructor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Form {
    /// Automatically managed runtime value.
    Managed,
    /// Owned value.
    Owned,
    /// Borrowed value.
    Borrowed {
        /// The solved borrow lifetime value.
        lifetime: LocalStaticId,
        /// The solved borrow access value.
        access: LocalStaticId,
    },
    /// Raw pointer value.
    Raw,
    /// Placed value.
    Placed {
        /// The solved concrete or ambient place value.
        place: LocalStaticId,
    },
    /// Readonly view.
    Readonly,
}

/// A predicate subject in semantic type space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredicateSubject {
    /// Symbol subject like `x` in `x is T`.
    Symbol(GlobalSymbolId),
    /// `this` subject.
    This,
}

/// An index signature in an object type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeIndexSignature {
    /// The parameter name like `K`.
    pub name: StringId,
    /// The key type.
    pub key_type: LocalTypeId,
    /// The value type.
    pub value_type: LocalTypeId,
    /// Whether the index signature is optional.
    pub is_optional: bool,
    /// Whether the index signature is readonly.
    pub is_readonly: bool,
}

/// A conditional type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalType {
    /// The symbol distributed over the conditional when present.
    pub distributive_symbol: Option<GlobalSymbolId>,
    /// The left operand.
    pub left: LocalTypeId,
    /// The right operand.
    pub right: LocalTypeId,
    /// The type selected when the condition holds.
    pub then_type: LocalTypeId,
    /// The type selected when the condition does not hold.
    pub else_type: LocalTypeId,
}

/// A mapped type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappedType {
    /// The mapped parameter.
    pub parameter: MappedTypeParameter,
    /// The mapped modifiers.
    pub modifiers: MappedTypeModifiers,
    /// The mapped value type.
    pub value: LocalTypeId,
}

/// Indexed access type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexType {
    /// The indexed type.
    pub left: LocalTypeId,
    /// The index type.
    pub index: LocalTypeId,
}

/// A template literal type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateLiteralType {
    /// The literal string segments.
    pub strings: Vec<StringId>,
    /// The interpolated type spans.
    pub spans: Vec<LocalTypeId>,
}

/// An infer binding inside a conditional type pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferType {
    /// The inferred binding name.
    pub name: Option<StringId>,
    /// The optional inferred constraint.
    pub constraint: Option<LocalTypeId>,
}

/// A type predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredicateType {
    /// Whether this is an assertion predicate.
    pub asserts: bool,
    /// The predicate subject.
    pub subject: PredicateSubject,
    /// The predicate target type.
    pub target: Option<LocalTypeId>,
}

/// A unary type operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnaryType {
    /// The target type.
    pub target: LocalTypeId,
}

/// A fixed-length array type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedArrayType {
    /// The element type.
    pub element: LocalTypeId,
    /// The static array length.
    pub count: LocalStaticId,
    /// Whether the array is readonly.
    pub is_readonly: bool,
}

/// Compact scalar interval type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeType {
    /// The inclusive lower bound.
    pub start: Option<ScalarLiteral>,
    /// The upper bound.
    pub end: Option<ScalarLiteral>,
    /// Whether the upper bound is included.
    pub is_inclusive: bool,
}

/// Runtime-length homogeneous view type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceType {
    /// The element type.
    pub element: LocalTypeId,
    /// Whether the slice is readonly.
    pub is_readonly: bool,
}

/// A tuple type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TupleType {
    /// The tuple elements.
    pub elements: Vec<TypeElement>,
    /// Whether the tuple is readonly.
    pub is_readonly: bool,
}

/// A structural object shape type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeType {
    /// The shape fields.
    pub fields: Vec<TypeField>,
    /// The call signatures.
    pub call_signatures: Vec<LocalTypeId>,
    /// The construct signatures.
    pub construct_signatures: Vec<LocalTypeId>,
    /// The index signatures.
    pub index_signatures: Vec<TypeIndexSignature>,
}

/// A function type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionType {
    /// The function asynchrony.
    pub asynchrony: Asynchrony,
    /// The generic parameter types.
    pub generic_parameters: Vec<LocalTypeId>,
    /// The optional `this` parameter type.
    pub this_parameter: Option<LocalTypeId>,
    /// The parameter types.
    pub parameters: Vec<LocalTypeId>,
    /// The optional return type.
    pub return_type: Option<LocalTypeId>,
    /// Whether this is a generator function.
    pub is_generator: bool,
}

/// A union type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionType {
    /// The union elements.
    pub elements: Vec<LocalTypeId>,
}

/// An intersection type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntersectionType {
    /// The intersection elements.
    pub elements: Vec<LocalTypeId>,
}

/// Type-level operation reduced by check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeOperation {
    /// Compiler-known builtin type function.
    BuiltinTypeFunction(BuiltinTypeFunction),
    /// Conditional type expression.
    Conditional(ConditionalType),
    /// Mapped type expression.
    Mapped(MappedType),
    /// Indexed access type expression.
    Index(IndexType),
    /// Template literal type expression.
    TemplateLiteral(TemplateLiteralType),
    /// Type infer binding in a conditional type pattern.
    Infer(InferType),
    /// `keyof T`.
    KeyOf(UnaryType),
}

/// A canonical solved semantic type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    /// Error type that could not be resolved.
    Error,
    /// Never type `never`.
    Never,
    /// Any type `any`.
    Any,
    /// Unknown type.
    Unknown,
    /// Void type.
    Void,
    /// Null type and value.
    Null,
    /// Undefined type and value.
    Undefined,
    /// TypeScript `object` constraint.
    Object,
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Scalar literal type.
    Literal(ScalarLiteral),

    /// Generic parameter reference.
    Parameter(ParameterType),
    /// Named type declaration reference.
    Named(NamedType),
    /// This type in a type predicate or method signature.
    This,

    /// Canonical memory or access form.
    Form(FormType),
    /// Explicit erased runtime `Any<T>` representation.
    ErasedAny(ErasedAnyType),

    /// Type predicate expression.
    Predicate(PredicateType),
    /// Type-level operation reduced by check.
    Operation(TypeOperation),

    /// Fixed-length array type.
    FixedArray(FixedArrayType),
    /// Compact scalar interval type.
    Range(RangeType),
    /// Runtime-length homogeneous view type.
    Slice(SliceType),
    /// Tuple type.
    Tuple(TupleType),
    /// Structural object shape type.
    Shape(ShapeType),
    /// Function type.
    Function(FunctionType),

    /// Union type `A | B | C`.
    Union(UnionType),
    /// Intersection type `A & B & C`.
    Intersection(IntersectionType),
}

impl Type {
    /// Whether the type is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// Whether the type is unknown.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Whether the type is an error or unknown.
    pub fn is_error_or_unknown(&self) -> bool {
        self.is_error() || self.is_unknown()
    }

    /// Return the symbol if this type directly references one declaration.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Parameter(parameter) => Some(parameter.symbol),
            Self::Named(named) => Some(named.symbol),
            _ => None,
        }
    }
}

/// A field in an object-like type.
/// Methods are represented as fields whose `ty` is a `Type::Function`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeField {
    /// The key of the field.
    pub key: StaticKey,
    /// The type of the field.
    pub ty: LocalTypeId,
    /// Whether the field is optional.
    pub is_optional: bool,
    /// Whether the field is readonly.
    pub is_readonly: bool,
}

/// An element in a tuple type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeElement {
    /// The optional label for the element.
    pub label: Option<StringId>,
    /// The element type.
    pub ty: LocalTypeId,
    /// Whether the element is optional.
    pub is_optional: bool,
    /// Whether the element is readonly.
    pub is_readonly: bool,
    /// Whether the element is a rest element.
    pub is_rest: bool,
}

impl TypeElement {
    /// Create a default tuple element for a type.
    pub fn new(ty: LocalTypeId) -> Self {
        Self {
            label: None,
            ty,
            is_optional: false,
            is_readonly: false,
            is_rest: false,
        }
    }
}

/// Unique identifier for a local type.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalTypeId(pub u32);

impl LocalTypeId {
    /// Wrap a raw id as a LocalTypeId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Convert this id into a global type id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalTypeId {
        GlobalTypeId {
            module_id,
            local_id: self,
        }
    }
}

/// Global type id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalTypeId {
    /// The module id of the global type.
    pub module_id: ModuleId,
    /// The local id of the global type.
    pub local_id: LocalTypeId,
}

impl GlobalTypeId {
    /// Create a new global type id.
    pub fn new(module_id: ModuleId, local_id: LocalTypeId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Convert this id into a local type id.
    pub fn into_local(self) -> LocalTypeId {
        self.local_id
    }
}

impl From<GlobalTypeId> for LocalTypeId {
    fn from(id: GlobalTypeId) -> Self {
        id.local_id
    }
}
