use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Asynchrony, BinaryOperator, GlobalGenericParameterId, GlobalStaticId, GlobalSymbolId,
    MappedTypeModifier, ScalarLiteral, StaticKey, StringId, TypeLiteral, UnaryOperator,
};

use super::{FloatType, PrimitiveType};

/// Compiler-provided string mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StringMapping {
    /// Uppercase string mapping.
    Uppercase,
    /// Lowercase string mapping.
    Lowercase,
    /// Capitalize string mapping.
    Capitalize,
    /// Uncapitalize string mapping.
    Uncapitalize,
}

impl TryFrom<&str> for StringMapping {
    /// The error type for string mapping parsing.
    type Error = ();

    /// Parse a string mapping from its standard name.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Uppercase" => Ok(Self::Uppercase),
            "Lowercase" => Ok(Self::Lowercase),
            "Capitalize" => Ok(Self::Capitalize),
            "Uncapitalize" => Ok(Self::Uncapitalize),
            _ => Err(()),
        }
    }
}

/// Mapped-type modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappedTypeModifiers {
    /// The readonly modifier.
    pub readonly: MappedTypeModifier,
    /// The optional modifier.
    pub optional: MappedTypeModifier,
}

/// A mapped-type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappedTypeParameter {
    /// The parameter name like `K`.
    pub name: StringId,
    /// The parameter symbol.
    pub symbol: GlobalSymbolId,
    /// The constraint type like `keyof T`.
    pub constraint: GlobalTypeId,
    /// The optional key remap like `as Foo<K>`.
    pub key_remap: Option<GlobalTypeId>,
}

/// One declaration applied to its complete positional arguments.
/// A non-generic reference is an instance with no arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericInstance {
    /// The referenced declaration symbol.
    pub symbol: GlobalSymbolId,
    /// The complete positional arguments in declaration order.
    pub arguments: Vec<GlobalTypeId>,
}

/// Member type selected from an owner type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberType {
    /// The owner type.
    pub owner: GlobalTypeId,
    /// The selected member key.
    pub key: StaticKey,
    /// The complete positional arguments applied to the member.
    pub arguments: Vec<GlobalTypeId>,
}

/// Explicit runtime `Dynamic<T>` representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicType {
    /// The `Dynamic<T>` constraint.
    pub constraint: GlobalTypeId,
}

/// Canonical memory or access form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormType {
    /// The form constructor.
    pub form: Form,
    /// The type carried by the form.
    pub value: GlobalTypeId,
}

/// Canonical memory or access form constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form {
    /// Automatically managed runtime value.
    Managed,
    /// Owned value.
    Owned,
    /// Borrowed value.
    Borrowed {
        /// The solved borrow lifetime singleton.
        lifetime: GlobalTypeId,
        /// The solved borrow access singleton.
        access: GlobalTypeId,
    },
    /// Raw pointer value.
    Raw,
    /// Placed value.
    Placed {
        /// The solved concrete or ambient place singleton.
        place: GlobalTypeId,
    },
    /// Readonly view.
    Readonly,
}

/// Singleton type of one normalized memory value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryLiteral {
    /// Memory access singleton.
    Access(Access),
    /// Storage space singleton.
    Space(Space),
    /// Placement singleton.
    Place(Place),
    /// Lifetime singleton.
    Lifetime(Lifetime),
}

/// Normalized memory access value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Access {
    /// Shared readonly access.
    Readonly,
    /// Mutable access.
    Mutable,
    /// Exclusive access.
    Exclusive,
}

/// Normalized storage space value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Space {
    /// Local storage.
    Local,
    /// Shared storage.
    Shared,
    /// Static storage.
    Static,
    /// Frame storage.
    Frame,
}

/// Normalized place value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Place {
    /// Ambient placement.
    Ambient,
    /// Concrete storage space.
    Space(Space),
}

/// Normalized lifetime value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lifetime {
    /// Static lifetime.
    Static,
    /// The enclosing frame's lifetime.
    Frame,
    /// Symbolic lifetime parameter or associated constant.
    Symbol(GlobalSymbolId),
}

/// An index signature in an object type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeIndexSignature {
    /// The parameter name like `K`.
    pub name: StringId,
    /// The key type.
    pub key_type: GlobalTypeId,
    /// The value type.
    pub value_type: GlobalTypeId,
    /// Whether the index signature is optional.
    pub is_optional: bool,
    /// Whether the index signature is readonly.
    pub is_readonly: bool,
}

/// A conditional type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalType {
    /// The left operand.
    pub left: GlobalTypeId,
    /// The right operand.
    pub right: GlobalTypeId,
    /// The type selected when the condition holds.
    pub then_type: GlobalTypeId,
    /// The type selected when the condition does not hold.
    pub else_type: GlobalTypeId,
    /// Whether the conditional distributes over union-valued left operands.
    pub is_distributive: bool,
}

/// A mapped type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappedType {
    /// The mapped parameter.
    pub parameter: MappedTypeParameter,
    /// The mapped modifiers.
    pub modifiers: MappedTypeModifiers,
    /// The mapped value type.
    pub value: GlobalTypeId,
}

/// Indexed access type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexType {
    /// The indexed type.
    pub left: GlobalTypeId,
    /// The index type.
    pub index: GlobalTypeId,
}

/// A template literal type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateLiteralType {
    /// The literal string segments.
    pub strings: Vec<StringId>,
    /// The interpolated type spans.
    pub spans: Vec<GlobalTypeId>,
}

/// An infer binding inside a conditional type pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferType {
    /// The inferred binding name.
    pub name: Option<StringId>,
    /// The optional inferred constraint.
    pub constraint: Option<GlobalTypeId>,
}

/// A unary type operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnaryType {
    /// The target type.
    pub target: GlobalTypeId,
}

/// Homogeneous array type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrayType {
    /// The element type.
    pub element: GlobalTypeId,
}

/// A fixed-length array type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedArrayType {
    /// The element type.
    pub element: GlobalTypeId,
    /// The static array length singleton.
    pub count: GlobalTypeId,
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
    pub element: GlobalTypeId,
}

/// A tuple type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TupleType {
    /// The tuple source form.
    pub form: TupleForm,
    /// The tuple elements.
    pub elements: Vec<TypeElement>,
}

/// The source form of a tuple type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TupleForm {
    /// Parenthesized tuple form.
    Tuple,
    /// Bracket tuple form.
    Array,
}

/// A structural object shape type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeType {
    /// The shape fields.
    pub fields: Vec<TypeField>,
    /// The call signatures.
    pub call_signatures: Vec<GlobalTypeId>,
    /// The construct signatures.
    pub construct_signatures: Vec<GlobalTypeId>,
    /// The index signatures.
    pub index_signatures: Vec<TypeIndexSignature>,
}

/// A function type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionType {
    /// The function asynchrony.
    pub asynchrony: Asynchrony,
    /// The generic parameter types.
    pub generic_parameters: Vec<GlobalTypeId>,
    /// The optional `this` parameter type.
    pub this_parameter: Option<GlobalTypeId>,
    /// The runtime parameters.
    pub parameters: Vec<FunctionParameterType>,
    /// The optional return type.
    pub return_type: Option<GlobalTypeId>,
    /// Whether this is a generator function.
    pub is_generator: bool,
}

/// A runtime parameter in a function type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionParameterType {
    /// The parameter type.
    pub ty: GlobalTypeId,
    /// The static generic parameter supplied by this runtime argument.
    pub static_parameter: Option<GlobalGenericParameterId>,
    /// Whether the parameter may be omitted at the call site.
    pub is_optional: bool,
    /// Whether the parameter captures remaining call arguments.
    pub is_rest: bool,
}

/// A closure type with its function contract and captured environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClosureType {
    /// The function contract.
    pub function: GlobalTypeId,
    /// The captured environment type.
    pub environment: GlobalTypeId,
}

/// A union type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionType {
    /// The union elements.
    pub elements: Vec<GlobalTypeId>,
}

/// An intersection type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntersectionType {
    /// The intersection elements.
    pub elements: Vec<GlobalTypeId>,
}

/// One static binary operation over singleton operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticBinaryType {
    /// The applied operator.
    pub operator: StaticBinaryOperator,
    /// The left operand.
    pub left: GlobalTypeId,
    /// The right operand.
    pub right: GlobalTypeId,
}

/// One static binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StaticBinaryOperator {
    /// `left + right`.
    Add,
    /// `left - right`.
    Subtract,
    /// `left * right`.
    Multiply,
    /// `left / right`.
    Divide,
    /// `left % right`.
    Remainder,
    /// `left ** right`.
    Exponent,
    /// `left << right`.
    ShiftLeft,
    /// `left >> right`.
    ShiftRight,
    /// `left >>> right`.
    UnsignedShiftRight,
    /// `left & right`.
    BitwiseAnd,
    /// `left ^ right`.
    BitwiseXor,
    /// `left | right`.
    BitwiseOr,
    /// `left == right`.
    Equal,
    /// `left === right`.
    EqualStrict,
    /// `left != right`.
    NotEqual,
    /// `left !== right`.
    NotEqualStrict,
    /// `left < right`.
    LessThan,
    /// `left <= right`.
    LessThanOrEqual,
    /// `left > right`.
    GreaterThan,
    /// `left >= right`.
    GreaterThanOrEqual,
    /// `left && right`.
    And,
    /// `left || right`.
    Or,
}

impl TryFrom<BinaryOperator> for StaticBinaryOperator {
    type Error = ();

    /// Map one source binary operator onto its static operator.
    fn try_from(operator: BinaryOperator) -> Result<Self, ()> {
        let operator = match operator {
            BinaryOperator::Add => Self::Add,
            BinaryOperator::Subtract => Self::Subtract,
            BinaryOperator::Multiply => Self::Multiply,
            BinaryOperator::Divide => Self::Divide,
            BinaryOperator::Remainder => Self::Remainder,
            BinaryOperator::Exponent => Self::Exponent,
            BinaryOperator::ShiftLeft => Self::ShiftLeft,
            BinaryOperator::ShiftRight => Self::ShiftRight,
            BinaryOperator::UnsignedShiftRight => Self::UnsignedShiftRight,
            BinaryOperator::ElementwiseAnd => Self::BitwiseAnd,
            BinaryOperator::ElementwiseXor => Self::BitwiseXor,
            BinaryOperator::ElementwiseOr => Self::BitwiseOr,
            BinaryOperator::Equal => Self::Equal,
            BinaryOperator::EqualStrict => Self::EqualStrict,
            BinaryOperator::NotEqual => Self::NotEqual,
            BinaryOperator::NotEqualStrict => Self::NotEqualStrict,
            BinaryOperator::LessThan => Self::LessThan,
            BinaryOperator::LessThanOrEqual => Self::LessThanOrEqual,
            BinaryOperator::GreaterThan => Self::GreaterThan,
            BinaryOperator::GreaterThanOrEqual => Self::GreaterThanOrEqual,
            BinaryOperator::And => Self::And,
            BinaryOperator::Or => Self::Or,
            _ => return Err(()),
        };

        Ok(operator)
    }
}

/// One static unary operation over one singleton operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticUnaryType {
    /// The applied operator.
    pub operator: StaticUnaryOperator,
    /// The operand.
    pub target: GlobalTypeId,
}

/// One static unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StaticUnaryOperator {
    /// `!target`.
    Not,
    /// `-target`.
    Negate,
    /// `~target`.
    BitwiseNot,
}

impl TryFrom<UnaryOperator> for StaticUnaryOperator {
    type Error = ();

    /// Map one source unary operator onto its static operator.
    fn try_from(operator: UnaryOperator) -> Result<Self, ()> {
        let operator = match operator {
            UnaryOperator::Not => Self::Not,
            UnaryOperator::Negate => Self::Negate,
            UnaryOperator::ElementwiseNot => Self::BitwiseNot,
            _ => return Err(()),
        };

        Ok(operator)
    }
}

/// Type-level operation preserved by check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeOperation {
    /// Compiler-known string mapping type.
    StringMapping {
        /// The string mapping operation.
        mapping: StringMapping,
        /// The mapped string type.
        target: GlobalTypeId,
    },
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
    /// Try success projection like `value?` continuing evaluation.
    TryOutput {
        /// The tried value type.
        value: GlobalTypeId,
    },
    /// Try failure projection like `value?` propagating its residual.
    TryResidual {
        /// The tried value type.
        value: GlobalTypeId,
    },
    /// Static binary operation like `N * 2` or `Mode == "inline"`.
    StaticBinary(StaticBinaryType),
    /// Static unary operation like `!Wide`.
    StaticUnary(StaticUnaryType),
}

/// A canonical solved type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    /// One open inference variable.
    /// Only present in working types during check, never in committed tables.
    Variable(TypeVariableId),

    /// Error type that could not be resolved.
    Error,
    /// Never type `never`.
    Never,
    /// TypeScript `any` compatibility marker.
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
    /// Singleton type of one normalized memory value.
    /// The only committed spelling: check normalizes literal spellings like
    /// `"shared"` to memory singletons at language-item-typed positions.
    Memory(MemoryLiteral),
    /// Singleton type of one committed static value.
    Static(GlobalStaticId),
    /// Compiler intrinsic type body.
    Intrinsic,

    /// Generic parameter.
    Parameter(GlobalGenericParameterId),
    /// Type declaration reference.
    Reference(GenericInstance),
    /// This type in a method signature.
    This,
    /// Member type selected from an owner type.
    Member(MemberType),

    /// Canonical memory or access form.
    Form(FormType),
    /// Explicit runtime `Dynamic<T>` representation.
    Dynamic(DynamicType),

    /// Type-level operation preserved by check.
    Operation(TypeOperation),

    /// Homogeneous array type.
    Array(ArrayType),
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
    /// Closure type with an explicit captured environment.
    Closure(ClosureType),

    /// Union type `A | B | C`.
    Union(UnionType),
    /// Intersection type `A & B & C`.
    Intersection(IntersectionType),
}

impl From<TypeLiteral> for Type {
    /// Convert a source type literal into a type.
    fn from(value: TypeLiteral) -> Self {
        match value {
            TypeLiteral::Never => Self::Never,
            TypeLiteral::Any => Self::Any,
            TypeLiteral::Undefined => Self::Undefined,
            TypeLiteral::Unknown => Self::Unknown,
            TypeLiteral::Object => Self::Object,
            TypeLiteral::Void => Self::Void,
            TypeLiteral::Null => Self::Null,
            TypeLiteral::Boolean => Self::Primitive(PrimitiveType::Boolean),
            TypeLiteral::Character => Self::Primitive(PrimitiveType::Character),
            TypeLiteral::String => Self::Primitive(PrimitiveType::String),
            TypeLiteral::Bigint => Self::Primitive(PrimitiveType::Bigint),
            TypeLiteral::Number => Self::Primitive(PrimitiveType::Float(FloatType::Float64)),
            TypeLiteral::Integer(integer) => Self::Primitive(PrimitiveType::Integer(integer)),
            TypeLiteral::Float(float) => Self::Primitive(PrimitiveType::Float(float)),
            TypeLiteral::Symbol => Self::Primitive(PrimitiveType::Symbol),
            TypeLiteral::UniqueSymbol => Self::Primitive(PrimitiveType::UniqueSymbol),
        }
    }
}

impl From<ScalarLiteral> for Type {
    /// Convert a scalar literal expression into its fresh type.
    fn from(value: ScalarLiteral) -> Self {
        Self::from(&value)
    }
}

impl From<&ScalarLiteral> for Type {
    /// Convert a scalar literal expression into its fresh type.
    fn from(value: &ScalarLiteral) -> Self {
        match value {
            ScalarLiteral::Null => Self::Null,
            ScalarLiteral::Undefined => Self::Undefined,
            ScalarLiteral::Boolean(_) => Self::Primitive(PrimitiveType::Boolean),
            ScalarLiteral::Character(_) => Self::Primitive(PrimitiveType::Character),
            ScalarLiteral::String(_)
            | ScalarLiteral::Integer(_)
            | ScalarLiteral::Float(_)
            | ScalarLiteral::Bigint(_) => Self::Literal(*value),
            ScalarLiteral::RegexString { .. } => Self::Object,
        }
    }
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
            Self::Reference(reference) => Some(reference.symbol),
            _ => None,
        }
    }
}

/// A field in an object-like type.
/// Methods are represented as fields whose `ty` is a `Type::Function`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeField {
    /// The key of the field.
    pub key: StaticKey,
    /// The type of the field.
    pub ty: GlobalTypeId,
    /// Whether the field is optional.
    pub is_optional: bool,
    /// Whether the field is readonly.
    pub is_readonly: bool,
}

/// An element in a tuple type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeElement {
    /// The optional label for the element.
    pub label: Option<StringId>,
    /// The element type.
    pub ty: GlobalTypeId,
    /// Whether the element is optional.
    pub is_optional: bool,
    /// Whether the element is readonly.
    pub is_readonly: bool,
    /// Whether the element is a rest element.
    pub is_rest: bool,
}

impl TypeElement {
    /// Create a default tuple element for a type.
    pub fn new(ty: GlobalTypeId) -> Self {
        Self {
            label: None,
            ty,
            is_optional: false,
            is_readonly: false,
            is_rest: false,
        }
    }
}

/// Identifier for one open inference variable inside a checked component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TypeVariableId {
    /// The module that allocated the variable.
    pub module_id: ModuleId,
    /// The variable index inside the module.
    pub index: u32,
}

impl TypeVariableId {
    /// Create a new type variable id.
    pub fn new(module_id: ModuleId, index: u32) -> Self {
        Self { module_id, index }
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
