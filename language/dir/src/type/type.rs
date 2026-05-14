use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Asynchrony, GlobalSymbolId, LocalNodeId, ScalarLiteral, StaticArgument, StaticKey, StringId,
    TypeExpression,
};

use super::PrimitiveType;

/// A scalar, primitive, intrinsic, or built-in type literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralType {
    /// Never type `never`.
    Never,
    /// Any type `any`.
    Any,
    /// Infer placeholder `_`.
    Infer,
    /// Undefined type and value.
    Undefined,
    /// Unknown type.
    Unknown,
    /// Object type (any non-primitive).
    Object,
    /// Void type.
    Void,
    /// Null type and value.
    Null,
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Intrinsic type.
    Intrinsic(IntrinsicType),
    /// Scalar literal.
    ScalarLiteral(ScalarLiteral),
}

/// An intrinsic type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntrinsicType {
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

impl TryFrom<&str> for IntrinsicType {
    /// The error type for intrinsic parsing.
    type Error = ();

    /// Parse an intrinsic type from a standard intrinsic name.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Uppercase" => Ok(IntrinsicType::Uppercase),
            "Lowercase" => Ok(IntrinsicType::Lowercase),
            "Capitalize" => Ok(IntrinsicType::Capitalize),
            "Uncapitalize" => Ok(IntrinsicType::Uncapitalize),
            "NoInfer" => Ok(IntrinsicType::NoInfer),
            "BuiltinIteratorReturn" => Ok(IntrinsicType::BuiltinIteratorReturn),
            _ => Err(()),
        }
    }
}

/// A mapped-type modifier in evaluated type space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeMappedModifier {
    /// The plain modifier without an explicit sign.
    Present,
    /// Add a modifier with an explicit `+` sign.
    Add,
    /// Remove a modifier (like `-readonly` or `-?`).
    Remove,
    /// No modifier specified.
    None,
}

/// Evaluated mapped-type modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappedTypeModifiers {
    /// The readonly modifier.
    pub readonly: TypeMappedModifier,
    /// The optional modifier.
    pub optional: TypeMappedModifier,
}

/// An evaluated mapped-type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappedTypeParameter {
    /// The parameter name (like `K`).
    pub name: StringId,
    /// The parameter symbol.
    pub symbol: GlobalSymbolId,
    /// The constraint type (like `keyof T`).
    pub constraint: LocalTypeId,
    /// The optional key remap (like `as Foo<K>`).
    pub key_remap: Option<LocalTypeId>,
}

/// A type bound for a reference operation.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeVarianceBound {
    /// The left type must implement the right type.
    Implements,
    /// The left type must extend the right type.
    Extends,
    /// The left type must be a supertype of the right type.
    Super,
}

/// Canonical qualified storage form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeForm {
    /// The unqualified base type.
    pub base: LocalTypeId,
    /// The ownership axis type.
    pub ownership: LocalTypeId,
    /// The placement axis type.
    pub place: LocalTypeId,
    /// The lifetime axis type.
    pub lifetime: LocalTypeId,
    /// The access axis type.
    pub access: LocalTypeId,
}

/// A predicate subject in evaluated type space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredicateSubject {
    /// Symbol subject (like `x` in `x is T`).
    Symbol(GlobalSymbolId),
    /// `this` subject.
    This,
}

/// An index signature in an object type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeIndexSignature {
    /// The parameter name (like `K`).
    pub name: StringId,
    /// The key type (like `string`).
    pub key_type: LocalTypeId,
    /// The value type (like `T`).
    pub value_type: LocalTypeId,
    /// Whether the index signature is optional.
    pub is_optional: bool,
    /// Whether the index signature is readonly.
    pub is_readonly: bool,
}

/// An inference variable type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferVariableType {
    /// The inference variable id.
    pub id: InferVarId,
}

/// Runtime representation of a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueType {
    /// The represented type.
    pub value: LocalTypeId,
}

/// Reference to one declared type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclaredType {
    /// The referenced declaration symbol.
    pub symbol: GlobalSymbolId,
    /// The static arguments applied to the reference.
    pub generic_arguments: Option<Vec<StaticArgument>>,
}

/// Source type expression that has not been evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UnevaluatedType {
    /// The source type expression.
    pub expression: LocalNodeId<TypeExpression>,
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

/// An infer binding type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferType {
    /// The inferred binding name.
    pub name: StringId,
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
    pub target_type: LocalTypeId,
}

/// A binary type relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryType {
    /// The left operand.
    pub left: LocalTypeId,
    /// The right operand.
    pub right: LocalTypeId,
}

/// A fixed-length array type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedArrayType {
    /// The element type.
    pub element: LocalTypeId,
    /// The length type.
    pub count: LocalTypeId,
    /// Whether the array is readonly.
    pub is_readonly: bool,
}

/// Runtime-length homogeneous view type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceType {
    /// The element type when known.
    pub element: Option<LocalTypeId>,
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

/// An object type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectType {
    /// The object fields.
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

/// A canonical type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    /// Scalar type literal.
    Literal(LiteralType),

    /// Inference variable used during type analysis (should not be used outside of analysis).
    InferVariable(InferVariableType),

    /// Runtime representation of a type.
    Value(ValueType),

    /// This type in a type predicate or method signature.
    This,

    /// Reference to one declared type.
    Reference(DeclaredType),
    /// Unevaluated type expression that resolves to a type.
    Unevaluated(UnevaluatedType),

    /// Conditional type expression.
    Conditional(ConditionalType),
    /// Mapped type expression.
    Mapped(MappedType),
    /// Indexed access type expression.
    Index(IndexType),
    /// Template literal type expression.
    TemplateLiteral(TemplateLiteralType),
    /// Type infer binding.
    Infer(InferType),
    /// Type predicate expression.
    Predicate(PredicateType),

    /// Canonical qualified storage form.
    Form(TypeForm),

    /// `keyof T`.
    KeyOf(UnaryType),
    /// `T!`.
    Must(UnaryType),
    /// `T as comptime`.
    AsComptime(UnaryType),
    /// `!T`.
    Not(UnaryType),
    /// `left in right`.
    In(BinaryType),
    /// `left extends right`.
    Extends(BinaryType),
    /// `left implements right`.
    Implements(BinaryType),

    /// Fixed-length array type.
    FixedArray(FixedArrayType),
    /// Runtime-length homogeneous view type.
    Slice(SliceType),
    /// Tuple type.
    Tuple(TupleType),
    /// Object type.
    Object(ObjectType),
    /// Function type.
    Function(FunctionType),

    /// Union type `A | B | C`.
    Union(UnionType),
    /// Intersection type `A & B & C`.
    Intersection(IntersectionType),

    /// Error type that could not be resolved.
    Error,
}

impl Type {
    /// Whether the type is evaluated.
    pub fn is_evaluated(&self) -> bool {
        !matches!(self, Type::Unevaluated(_) | Type::InferVariable(_))
    }

    /// Whether the type is unevaluated.
    pub fn is_unevaluated(&self) -> bool {
        matches!(self, Type::Unevaluated(_))
    }

    /// Whether the type is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    /// Whether the type is an unknown literal.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Type::Literal(LiteralType::Unknown))
    }

    /// Whether the type is one infer-owned placeholder variant.
    pub fn is_infer(&self) -> bool {
        matches!(self, Type::InferVariable(_) | Type::Infer(_))
    }

    /// Whether the type is an error or unknown literal.
    pub fn is_error_or_unknown(&self) -> bool {
        self.is_error() || self.is_unknown()
    }

    /// Return the symbol if this type directly references one declaration.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Type::Reference(reference) => Some(reference.symbol),
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

/// Unique identifier for inference variables.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InferVarId(pub u32);

impl InferVarId {
    /// Wrap a raw id as an InferVarId.
    pub fn new(id: u32) -> Self {
        Self(id)
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
