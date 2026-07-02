use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Asynchrony, BinaryOperator, GlobalGenericParameterId, GlobalGenericTemplateId, GlobalNodeIdAny,
    GlobalStaticId, GlobalSymbolId, LanguageItem, MappedTypeModifier, RangeEnd, ScalarDomain,
    ScalarLiteral, StaticKey, StringId, TypeLiteral, UnaryOperator,
};

use super::{FloatType, PrimitiveType};

/// A canonical solved type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Type {
    /// One open inference variable.
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
    /// Primitive type, like `string` or `int32`.
    Primitive(PrimitiveType),
    /// Scalar literal type, like `"id"` or `42`.
    Literal(ScalarLiteral),
    /// Singleton property key type, like `Symbol.for("id")`.
    Key(StaticKey),
    /// Singleton type of one normalized memory value.
    Memory(MemoryLiteral),
    /// Singleton type of one committed static value.
    Static(GlobalStaticId),
    /// Compiler intrinsic type body.
    Intrinsic,

    /// Generic parameter, like the `T` in `class Box<T>`.
    Parameter(GlobalGenericParameterId),
    /// Type declaration reference before application, like `Box` in `Box.empty`.
    Reference(TypeReference),
    /// Applied type declaration, like `User` or `Map<string, User>`.
    Instance(GenericInstance),
    /// This type in a method signature, like `this` in `clone(): this`.
    This,
    /// Member type selected from an owner type, like `T.Output`.
    Member(MemberType),
    /// Singleton enum member type, like `Mode.Read`.
    EnumMember(EnumMemberType),

    /// Canonical memory or access form, like `^User` or `&exclusive User`.
    Form(FormType),
    /// Explicit runtime `Dynamic<T>` representation, like `Dynamic<Printable>`.
    Dynamic(DynamicType),

    /// Type-level operation preserved by check.
    Operation(TypeOperation),

    /// Homogeneous array type, like `int32[]`.
    Array(ArrayType),
    /// Fixed-length array type, like `[uint8; 4]`.
    FixedArray(FixedArrayType),
    /// Compact scalar interval type, like `0..10`.
    Range(RangeType),
    /// Runtime-length homogeneous view type, like `[uint8]`.
    Slice(SliceType),
    /// Tuple type, like `(string, int32)`.
    Tuple(TupleType),
    /// Structural object shape type, like `{ name: string }`.
    Shape(ShapeType),

    /// Function signature type, like `(value: int32) => string`.
    FunctionSignature(FunctionSignatureType),
    /// Fat callable value with an explicit captured environment.
    Function(FunctionType),
    /// Thin callable value with no captured environment.
    FunctionPointer(FunctionPointerType),

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
    /// Return whether this type is the undefined singleton.
    pub fn is_undefined(&self) -> bool {
        matches!(
            self,
            Self::Undefined | Self::Literal(ScalarLiteral::Undefined)
        )
    }

    /// Return this type's direct scalar domain.
    pub fn scalar_domain(&self) -> Option<ScalarDomain> {
        let domain = match self {
            Self::Null => ScalarDomain::Null,
            Self::Undefined => ScalarDomain::Undefined,
            Self::Primitive(PrimitiveType::Boolean) => ScalarDomain::Boolean,
            Self::Primitive(PrimitiveType::Character) => ScalarDomain::Character,
            Self::Primitive(PrimitiveType::String) => ScalarDomain::String,
            Self::Primitive(PrimitiveType::Symbol | PrimitiveType::UniqueSymbol) => {
                ScalarDomain::Symbol
            }
            Self::Primitive(PrimitiveType::Bigint) => ScalarDomain::Bigint,
            Self::Primitive(PrimitiveType::Integer(_) | PrimitiveType::Float(_)) => {
                ScalarDomain::Numeric
            }
            Self::Literal(literal) => return literal.scalar_domain(),
            Self::Key(key) if key.is_string_like() => ScalarDomain::String,
            Self::Key(key) if key.is_number_like() => ScalarDomain::Numeric,
            Self::Key(key) if key.is_symbol_like() => ScalarDomain::Symbol,
            Self::Range(range) => return range.scalar_domain(),
            _ => return None,
        };

        Some(domain)
    }

    /// Return the symbolic leaf kind contributed by this type alone.
    /// A stored type joins this bit with every child type's flags.
    pub fn own_flags(&self) -> TypeFlags {
        match self {
            // symbolic leaves, one bit each
            Self::Variable(_) => TypeFlags::HAS_VARIABLE,
            Self::Error => TypeFlags::HAS_ERROR,
            Self::Parameter(_) => TypeFlags::HAS_PARAMETER,
            Self::This => TypeFlags::HAS_THIS,
            Self::Reference(_) => TypeFlags::HAS_REFERENCE,
            Self::Member(_) => TypeFlags::HAS_MEMBER,
            Self::Operation(TypeOperation::Infer(_)) => {
                TypeFlags::HAS_OPERATION | TypeFlags::HAS_INFER
            }
            Self::Operation(_) => TypeFlags::HAS_OPERATION,

            // concrete heads contribute nothing of their own
            Self::Never
            | Self::Any
            | Self::Unknown
            | Self::Void
            | Self::Null
            | Self::Undefined
            | Self::Object
            | Self::Primitive(_)
            | Self::Literal(_)
            | Self::Key(_)
            | Self::Memory(_)
            | Self::Static(_)
            | Self::Intrinsic
            | Self::Instance(_)
            | Self::EnumMember(_)
            | Self::Form(_)
            | Self::Dynamic(_)
            | Self::Array(_)
            | Self::FixedArray(_)
            | Self::Range(_)
            | Self::Slice(_)
            | Self::Tuple(_)
            | Self::Shape(_)
            | Self::FunctionSignature(_)
            | Self::Function(_)
            | Self::FunctionPointer(_)
            | Self::Union(_)
            | Self::Intersection(_) => TypeFlags::EMPTY,
        }
    }

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
            Self::Instance(instance) => Some(instance.symbol),
            _ => None,
        }
    }
}

/// The symbolic leaf kinds contained in one type graph, computed once at intern time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct TypeFlags(u8);

impl TypeFlags {
    /// A ground type graph without symbolic leaves.
    pub const EMPTY: Self = Self(0);
    /// The graph contains an open inference variable.
    pub const HAS_VARIABLE: Self = Self(1 << 0);
    /// The graph contains a generic parameter.
    pub const HAS_PARAMETER: Self = Self(1 << 1);
    /// The graph contains an error type.
    pub const HAS_ERROR: Self = Self(1 << 2);
    /// The graph contains a `this` type.
    pub const HAS_THIS: Self = Self(1 << 3);
    /// The graph contains an unexpanded declaration reference.
    pub const HAS_REFERENCE: Self = Self(1 << 4);
    /// The graph contains a member projection like `T.Output`.
    pub const HAS_MEMBER: Self = Self(1 << 5);
    /// The graph contains a type-level operation.
    pub const HAS_OPERATION: Self = Self(1 << 6);
    /// The graph contains a conditional infer binding.
    pub const HAS_INFER: Self = Self(1 << 7);

    /// Return whether every bit of `other` is set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Return whether the graph contains an open inference variable.
    pub fn has_variable(self) -> bool {
        self.contains(Self::HAS_VARIABLE)
    }

    /// Return whether the graph contains a generic parameter.
    pub fn has_parameter(self) -> bool {
        self.contains(Self::HAS_PARAMETER)
    }

    /// Return whether the graph contains an error type.
    pub fn has_error(self) -> bool {
        self.contains(Self::HAS_ERROR)
    }

    /// Return whether the graph contains a `this` type.
    pub fn has_this(self) -> bool {
        self.contains(Self::HAS_THIS)
    }

    /// Return whether the graph contains an unexpanded declaration reference.
    pub fn has_reference(self) -> bool {
        self.contains(Self::HAS_REFERENCE)
    }

    /// Return whether the graph contains a member projection.
    pub fn has_member(self) -> bool {
        self.contains(Self::HAS_MEMBER)
    }

    /// Return whether the graph contains a type-level operation.
    pub fn has_operation(self) -> bool {
        self.contains(Self::HAS_OPERATION)
    }

    /// Return whether the graph contains a conditional infer binding.
    pub fn has_infer(self) -> bool {
        self.contains(Self::HAS_INFER)
    }
}

impl std::ops::BitOr for TypeFlags {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl std::ops::BitOrAssign for TypeFlags {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// Global type id across modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
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

/// Unique identifier for a local type.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
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

/// One open inference variable inside a checked component.
/// Component-scoped solver working state like `ConstraintId`: committed
/// tables never contain variables, so the id needs no module qualification.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct TypeVariableId(pub u32);

/// One interned list inside the owning module's type storage.
/// Composite types never own their payload lists: the id addresses elements
/// interned beside the type, which keeps every type small and `Copy`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct TypeListId {
    /// The first element of the list.
    pub start: u32,
    /// The number of elements in the list.
    pub count: u32,
}

impl TypeListId {
    /// The canonical empty list.
    pub const EMPTY: Self = Self { start: 0, count: 0 };

    /// Create a new list id.
    pub fn new(start: u32, count: u32) -> Self {
        Self { start, count }
    }

    /// Return the number of elements in the list.
    pub fn len(&self) -> u32 {
        self.count
    }

    /// Return whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Iterate the cumulative element indices of the list.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = u32> {
        self.start..self.start + self.count
    }
}

/// Singleton type of one normalized memory value.
/// Literal spellings at language-item-typed positions normalize here:
/// the `"exclusive"` in `Borrowed<User, L, "exclusive">` commits as
/// `MemoryLiteral::Access(Access::Exclusive)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemoryLiteral {
    /// Memory access singleton, like `"readonly"` or `"exclusive"`.
    Access(Access),
    /// Storage space singleton, like `"local"` or `"shared"`.
    Space(Space),
    /// Placement singleton, like `"ambient"` or a concrete space.
    Place(Place),
    /// Lifetime singleton, like `"static"` or a lifetime parameter.
    Lifetime(Lifetime),
}

impl MemoryLiteral {
    /// Return the language item naming this literal's singleton domain.
    pub fn domain_language_item(&self) -> LanguageItem {
        match self {
            Self::Access(_) => LanguageItem::Access,
            Self::Space(_) => LanguageItem::Space,
            Self::Place(_) => LanguageItem::Place,
            Self::Lifetime(_) => LanguageItem::Lifetime,
        }
    }

    /// Return the canonical source text of one memory literal.
    pub fn text(&self) -> &'static str {
        match self {
            Self::Access(Access::Readonly) => "readonly",
            Self::Access(Access::Mutable) => "mutable",
            Self::Access(Access::Exclusive) => "exclusive",
            Self::Space(Space::Local) | Self::Place(Place::Space(Space::Local)) => "local",
            Self::Space(Space::Shared) | Self::Place(Place::Space(Space::Shared)) => "shared",
            Self::Space(Space::Static) | Self::Place(Place::Space(Space::Static)) => "static",
            Self::Space(Space::Frame) | Self::Place(Place::Space(Space::Frame)) => "frame",
            Self::Place(Place::Ambient) => "ambient",
            Self::Lifetime(Lifetime::Frame) => "frame",
            Self::Lifetime(_) => "static",
        }
    }
}

/// Normalized memory access value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Access {
    /// Shared readonly access.
    Readonly,
    /// Mutable access.
    Mutable,
    /// Exclusive access.
    Exclusive,
}

/// Normalized storage space value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// Normalized memory placement value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Place {
    /// Ambient placement.
    Ambient,
    /// Concrete storage space.
    Space(Space),
}

/// Normalized lifetime value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Lifetime {
    /// Static lifetime.
    Static,
    /// The enclosing frame's lifetime.
    Frame,
    /// Symbolic lifetime parameter or associated constant.
    Symbol(GlobalSymbolId),
}

/// One written reference to a type declaration before application.
///
/// Examples:
/// ```ds
/// Box                 // static declaration receiver in `Box.empty`
/// Box.Output          // owner of a static associated type projection
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct TypeReference {
    /// The referenced declaration symbol.
    pub symbol: GlobalSymbolId,
}

/// One declaration applied to its complete positional arguments.
/// A non-generic reference is an instance with no arguments.
///
/// Examples:
/// ```ds
/// User                  // no arguments
/// Map<string, User>     // two positional arguments
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct GenericInstance {
    /// The referenced declaration symbol.
    pub symbol: GlobalSymbolId,
    /// The complete positional argument list in declaration order.
    pub arguments: TypeListId,
}

/// Member type selected from an owner type.
///
/// Examples:
/// ```ds
/// T.Output              // the associated type selected on T
/// Ordering.Less         // the enum member selected on Ordering
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemberType {
    /// The owner type.
    pub owner: GlobalTypeId,
    /// The selected member key.
    pub key: StaticKey,
    /// The complete positional argument list applied to the member.
    pub arguments: TypeListId,
}

/// Singleton type of one enum member.
///
/// Examples:
/// ```ds
/// Mode.Read
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct EnumMemberType {
    /// The enum declaration instance.
    pub owner: GlobalTypeId,
    /// The selected enum variant symbol.
    pub member: GlobalSymbolId,
}

/// Canonical memory or access form.
/// Surface sigils spell these forms: `^User` is `Owned<User>`,
/// `&exclusive User` is `Borrowed<User, L, "exclusive">`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FormType {
    /// The form constructor.
    pub form: Form,
    /// The type carried by the form.
    pub value: GlobalTypeId,
}

/// Canonical memory or access form constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Form {
    /// Automatically managed runtime value, the unqualified `User`.
    Managed,
    /// Owned value, like `^User`.
    Owned,
    /// Borrowed value, like `&User`, `&readonly User`, or `&exclusive User`.
    Borrowed {
        /// The solved borrow lifetime singleton.
        lifetime: GlobalTypeId,
        /// The solved borrow access singleton.
        access: GlobalTypeId,
    },
    /// Raw pointer value, like `*User`.
    Raw,
    /// Placed value, like `local User` or `shared User`.
    Placed {
        /// The solved concrete or ambient place singleton.
        place: GlobalTypeId,
    },
    /// Readonly view, like `readonly User`.
    Readonly,
}

/// Explicit runtime `Dynamic<T>` representation.
///
/// Examples:
/// ```ds
/// Dynamic<Printable>    // a boxed value known to satisfy Printable
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct DynamicType {
    /// The `Dynamic<T>` constraint.
    pub constraint: GlobalTypeId,
}

/// Type-level operation preserved by check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum TypeOperation {
    /// Compiler-known string mapping type, like `Uppercase<S>`.
    StringMapping {
        /// The string mapping operation.
        mapping: StringMapping,
        /// The mapped string type.
        target: GlobalTypeId,
    },
    /// Conditional type expression, like `T extends string ? A : B`.
    Conditional(ConditionalType),
    /// Runtime guard narrowing, like the true or false branch of `value is T`.
    Narrow(NarrowType),
    /// Mapped type expression, like `{ [K in keyof T]: T[K] }`.
    Mapped(MappedType),
    /// Indexed access type expression, like `User["name"]`.
    Index(IndexType),
    /// Template literal type expression, like `` `get${Name}` ``.
    TemplateLiteral(TemplateLiteralType),
    /// Type infer binding in a conditional type pattern, like `infer E`.
    Infer(InferType),
    /// Type query expression, like `typeof value`.
    TypeOf(TypeOfType),
    /// `keyof T`.
    KeyOf(UnaryType),
    /// Inference blocker like `NoInfer<T>`.
    NoInfer(UnaryType),
    /// Awaited value type, like `Awaited<Promise<T>>`.
    Awaited(UnaryType),
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

/// Type query expression, like `typeof value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct TypeOfType {
    /// The queried value reference.
    pub value: GlobalNodeIdAny,
}

/// Compiler-provided string mapping.
///
/// Examples:
/// ```ds
/// Uppercase<"id">      // "ID"
/// Capitalize<"name">   // "Name"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum StringMapping {
    /// Uppercase string mapping, like `Uppercase<"id">` reducing to `"ID"`.
    Uppercase,
    /// Lowercase string mapping, like `Lowercase<"ID">` reducing to `"id"`.
    Lowercase,
    /// Capitalize string mapping, like `Capitalize<"name">` reducing to `"Name"`.
    Capitalize,
    /// Uncapitalize string mapping, like `Uncapitalize<"Name">` reducing to `"name"`.
    Uncapitalize,
}

impl StringMapping {
    /// Apply this mapping to one string.
    pub fn apply(self, text: &str) -> String {
        match self {
            Self::Uppercase => text.to_uppercase(),
            Self::Lowercase => text.to_lowercase(),
            Self::Capitalize => Self::recase(text, true),
            Self::Uncapitalize => Self::recase(text, false),
        }
    }

    /// Recase the first character of one string.
    fn recase(text: &str, upper: bool) -> String {
        let mut characters = text.chars();

        match characters.next() {
            Some(first) if upper => first.to_uppercase().collect::<String>() + characters.as_str(),
            Some(first) => first.to_lowercase().collect::<String>() + characters.as_str(),
            None => String::new(),
        }
    }
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

/// A conditional type.
///
/// Examples:
/// ```ds
/// T extends string ? Text : Raw
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// A runtime guard narrowing applied to one source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct NarrowType {
    /// The source type being narrowed.
    pub source: GlobalTypeId,
    /// The runtime-tested target type.
    pub target: GlobalTypeId,
    /// Whether matching arms are kept or removed.
    pub is_positive: bool,
}

/// A mapped type.
///
/// Examples:
/// ```ds
/// { [K in keyof T]: T[K] }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MappedType {
    /// The mapped parameter.
    pub parameter: MappedTypeParameter,
    /// The mapped modifiers.
    pub modifiers: MappedTypeModifiers,
    /// The mapped value type.
    pub value: GlobalTypeId,
}

/// A mapped-type parameter.
///
/// Examples:
/// ```ds
/// [K in keyof T]
/// [K in "name" | "age" as Uppercase<K>]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MappedTypeParameter {
    /// The parameter name like `K`.
    pub name: StringId,
    /// The binder's generic parameter.
    pub parameter: GlobalGenericParameterId,
    /// The constraint type like `keyof T`.
    pub constraint: GlobalTypeId,
    /// The optional key remap like `as Foo<K>`.
    pub key_remap: Option<GlobalTypeId>,
}

/// Mapped-type modifiers.
///
/// Examples:
/// ```ds
/// { [K in keyof T]?: T[K] }              // optional: Present
/// { -readonly [K in keyof T]-?: T[K] }   // readonly and optional: Remove
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MappedTypeModifiers {
    /// The readonly modifier.
    pub readonly: MappedTypeModifier,
    /// The optional modifier.
    pub optional: MappedTypeModifier,
}

/// Indexed access type.
///
/// Examples:
/// ```ds
/// User["name"]
/// Pair[0]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct IndexType {
    /// The indexed type.
    pub left: GlobalTypeId,
    /// The index type.
    pub index: GlobalTypeId,
}

/// A template literal type.
///
/// Examples:
/// ```ds
/// `get${Name}`
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct TemplateLiteralType {
    /// The literal string segment list.
    pub strings: TypeListId,
    /// The interpolated type span list.
    pub spans: TypeListId,
}

/// An infer binding inside a conditional type pattern.
///
/// Examples:
/// ```ds
/// T extends Array<infer E> ? E : never
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct InferType {
    /// The inferred binding name.
    pub name: Option<StringId>,
    /// The inferred binding symbol.
    pub symbol: Option<GlobalSymbolId>,
    /// The optional inferred constraint.
    pub constraint: Option<GlobalTypeId>,
}

/// A unary type operator.
///
/// Examples:
/// ```ds
/// keyof User
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct UnaryType {
    /// The target type.
    pub target: GlobalTypeId,
}

/// One static binary operation over singleton operands.
///
/// Examples:
/// ```ds
/// N * 2
/// Mode == "inline"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct StaticBinaryType {
    /// The applied operator.
    pub operator: StaticBinaryOperator,
    /// The left operand.
    pub left: GlobalTypeId,
    /// The right operand.
    pub right: GlobalTypeId,
}

/// One static binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

impl StaticBinaryOperator {
    /// Evaluate this operator over two scalar literals.
    pub fn apply(
        self,
        left: ScalarLiteral,
        right: ScalarLiteral,
    ) -> Result<ScalarLiteral, &'static str> {
        use ScalarLiteral as Literal;
        use StaticBinaryOperator as Operator;

        let literal = match (self, left, right) {
            // integer arithmetic is checked
            (Operator::Add, Literal::Integer(left), Literal::Integer(right)) => Literal::Integer(
                left.checked_add(right)
                    .ok_or("integer addition overflows")?,
            ),
            (Operator::Subtract, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(
                    left.checked_sub(right)
                        .ok_or("integer subtraction overflows")?,
                )
            }
            (Operator::Multiply, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(
                    left.checked_mul(right)
                        .ok_or("integer multiplication overflows")?,
                )
            }
            (Operator::Divide, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(left.checked_div(right).ok_or("integer division by zero")?)
            }
            (Operator::Remainder, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(left.checked_rem(right).ok_or("integer remainder by zero")?)
            }
            (Operator::Exponent, Literal::Integer(left), Literal::Integer(right)) => {
                let exponent =
                    u32::try_from(right).map_err(|_| "integer exponent must be non-negative")?;

                Literal::Integer(
                    left.checked_pow(exponent)
                        .ok_or("integer exponentiation overflows")?,
                )
            }

            // shifts and bitwise operations stay in integer space
            (Operator::ShiftLeft, Literal::Integer(left), Literal::Integer(right)) => {
                let amount =
                    u32::try_from(right).map_err(|_| "shift amount must be non-negative")?;

                Literal::Integer(left.checked_shl(amount).ok_or("shift amount too large")?)
            }
            (Operator::ShiftRight, Literal::Integer(left), Literal::Integer(right)) => {
                let amount =
                    u32::try_from(right).map_err(|_| "shift amount must be non-negative")?;

                Literal::Integer(left.checked_shr(amount).ok_or("shift amount too large")?)
            }
            (Operator::UnsignedShiftRight, Literal::Integer(left), Literal::Integer(right)) => {
                let amount =
                    u32::try_from(right).map_err(|_| "shift amount must be non-negative")?;
                let shifted = (left as u64)
                    .checked_shr(amount)
                    .ok_or("shift amount too large")?;

                Literal::Integer(shifted as i64)
            }
            (Operator::BitwiseAnd, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(left & right)
            }
            (Operator::BitwiseXor, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(left ^ right)
            }
            (Operator::BitwiseOr, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(left | right)
            }

            // float arithmetic follows ieee semantics
            (Operator::Add, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left + right)
            }
            (Operator::Subtract, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left - right)
            }
            (Operator::Multiply, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left * right)
            }
            (Operator::Divide, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left / right)
            }
            (Operator::Remainder, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left % right)
            }
            (Operator::Exponent, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left.powf(right))
            }

            // ordering compares within one operand kind
            (
                Operator::LessThan
                | Operator::LessThanOrEqual
                | Operator::GreaterThan
                | Operator::GreaterThanOrEqual,
                left,
                right,
            ) => {
                let ordering = match (left, right) {
                    (Literal::Integer(left), Literal::Integer(right)) => left.cmp(&right),
                    (Literal::Float(left), Literal::Float(right)) => left
                        .partial_cmp(&right)
                        .ok_or("float comparison is undefined for nan")?,
                    (Literal::Character(left), Literal::Character(right)) => left.cmp(&right),
                    _ => return Err("static comparison requires matching operand kinds"),
                };

                Literal::Boolean(match self {
                    Operator::LessThan => ordering.is_lt(),
                    Operator::LessThanOrEqual => ordering.is_le(),
                    Operator::GreaterThan => ordering.is_gt(),
                    _ => ordering.is_ge(),
                })
            }

            // equality compares across kinds, distinct kinds compare unequal
            (
                Operator::Equal
                | Operator::EqualStrict
                | Operator::NotEqual
                | Operator::NotEqualStrict,
                left,
                right,
            ) => {
                let equal = match (left, right) {
                    (Literal::Integer(left), Literal::Integer(right)) => left == right,
                    (Literal::Float(left), Literal::Float(right)) => left == right,
                    (Literal::Boolean(left), Literal::Boolean(right)) => left == right,
                    (Literal::String(left), Literal::String(right)) => left == right,
                    (Literal::Character(left), Literal::Character(right)) => left == right,
                    (Literal::Null, Literal::Null) => true,
                    (Literal::Undefined, Literal::Undefined) => true,
                    _ => false,
                };

                let negated = matches!(self, Operator::NotEqual | Operator::NotEqualStrict);

                Literal::Boolean(equal != negated)
            }

            // logical joins reach here only with non-boolean operands
            (Operator::And | Operator::Or, _, _) => {
                return Err("logical operator requires boolean operands");
            }

            _ => return Err("static operator does not apply to its operand kinds"),
        };

        Ok(literal)
    }
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
///
/// Examples:
/// ```ds
/// !Wide
/// -Offset
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct StaticUnaryType {
    /// The applied operator.
    pub operator: StaticUnaryOperator,
    /// The operand.
    pub target: GlobalTypeId,
}

/// One static unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// Homogeneous array type.
///
/// Examples:
/// ```ds
/// int32[]
/// Array<int32>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ArrayType {
    /// The element type.
    pub element: GlobalTypeId,
}

/// A fixed-length array type.
///
/// Examples:
/// ```ds
/// [uint8; 4]
/// FixedArray<uint8, 4>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FixedArrayType {
    /// The element type.
    pub element: GlobalTypeId,
    /// The static array length singleton.
    pub count: GlobalTypeId,
}

/// Compact discrete scalar interval type.
///
/// Examples:
/// ```ds
/// 0..10
/// 0..=255
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct RangeType {
    /// The inclusive lower bound.
    pub start: Option<ScalarLiteral>,
    /// The upper bound.
    pub end: Option<ScalarLiteral>,
    /// Whether the upper bound is included.
    pub is_inclusive: bool,
}

impl RangeType {
    /// Create an interval from pattern bounds.
    pub fn new(
        start: Option<ScalarLiteral>,
        end: Option<ScalarLiteral>,
        end_kind: RangeEnd,
    ) -> Self {
        Self {
            start,
            end,
            is_inclusive: matches!(end_kind, RangeEnd::Inclusive),
        }
    }

    /// Return this interval's scalar domain.
    pub fn scalar_domain(&self) -> Option<ScalarDomain> {
        let start = self.start.as_ref().and_then(ScalarLiteral::interval_domain);
        let end = self.end.as_ref().and_then(ScalarLiteral::interval_domain);

        match (start, end) {
            (Some(start), Some(end)) if start == end => Some(start),
            (Some(start), None) => Some(start),
            (None, Some(end)) => Some(end),
            _ => None,
        }
    }

    /// Return whether this interval can widen to one target type.
    pub fn widens_to(&self, target: &Type) -> bool {
        match target {
            Type::Primitive(primitive) => self.widens_to_primitive(*primitive),
            Type::Range(target) => target.contains_range(self),
            _ => false,
        }
    }

    /// Return whether this interval contains one scalar literal.
    pub fn contains_literal(&self, literal: ScalarLiteral) -> bool {
        match literal {
            ScalarLiteral::Integer(value) => self.contains_integer(value),
            ScalarLiteral::Bigint(value) => self.contains_bigint(value),
            ScalarLiteral::Character(value) => self.contains_character(value),
            _ => false,
        }
    }

    /// Return whether this interval contains one integer literal.
    pub fn contains_integer(&self, value: i64) -> bool {
        let start_holds = match self.start {
            Some(ScalarLiteral::Integer(start)) => value >= start,
            Some(_) => false,
            None => true,
        };

        let end_holds = match self.end {
            Some(ScalarLiteral::Integer(end)) => {
                if self.is_inclusive {
                    value <= end
                } else {
                    value < end
                }
            }
            Some(_) => false,
            None => true,
        };

        start_holds && end_holds
    }

    /// Return whether this interval contains one bigint literal.
    pub fn contains_bigint(&self, value: i64) -> bool {
        let start_holds = match self.start {
            Some(ScalarLiteral::Bigint(start)) => value >= start,
            Some(_) => false,
            None => true,
        };

        let end_holds = match self.end {
            Some(ScalarLiteral::Bigint(end)) => {
                if self.is_inclusive {
                    value <= end
                } else {
                    value < end
                }
            }
            Some(_) => false,
            None => true,
        };

        start_holds && end_holds
    }

    /// Return whether this interval contains one character literal.
    pub fn contains_character(&self, value: char) -> bool {
        let start_holds = match self.start {
            Some(ScalarLiteral::Character(start)) => value >= start,
            Some(_) => false,
            None => true,
        };

        let end_holds = match self.end {
            Some(ScalarLiteral::Character(end)) => {
                if self.is_inclusive {
                    value <= end
                } else {
                    value < end
                }
            }
            Some(_) => false,
            None => true,
        };

        start_holds && end_holds
    }

    /// Return whether this interval can widen to one primitive type.
    pub fn widens_to_primitive(&self, primitive: PrimitiveType) -> bool {
        match primitive {
            // integer intervals widen when both bounds fit
            PrimitiveType::Integer(integer) => {
                let start_widens = match &self.start {
                    Some(ScalarLiteral::Integer(start)) => integer.fits_literal(*start),
                    Some(_) | None => false,
                };
                let end_widens = match &self.end {
                    Some(ScalarLiteral::Integer(end)) => integer.fits_literal(*end),
                    Some(_) | None => false,
                };

                start_widens && end_widens
            }
            // bigint intervals widen to the bigint primitive
            PrimitiveType::Bigint => matches!(
                (&self.start, &self.end),
                (
                    Some(ScalarLiteral::Bigint(_)) | None,
                    Some(ScalarLiteral::Bigint(_)) | None,
                )
            ),
            // character intervals widen to the character primitive
            PrimitiveType::Character => matches!(
                (&self.start, &self.end),
                (
                    Some(ScalarLiteral::Character(_)) | None,
                    Some(ScalarLiteral::Character(_)) | None,
                )
            ),
            _ => false,
        }
    }

    /// Return whether this interval contains another interval.
    pub fn contains_range(&self, inner: &RangeType) -> bool {
        // the outer start must not exceed the inner start
        let start_holds = match (&self.start, &inner.start) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(ScalarLiteral::Integer(outer)), Some(ScalarLiteral::Integer(inner))) => {
                outer <= inner
            }
            (Some(ScalarLiteral::Bigint(outer)), Some(ScalarLiteral::Bigint(inner))) => {
                outer <= inner
            }
            (Some(ScalarLiteral::Character(outer)), Some(ScalarLiteral::Character(inner))) => {
                outer <= inner
            }
            _ => false,
        };
        if !start_holds {
            return false;
        }

        // the outer end must not fall below the inner end
        match (&self.end, &inner.end) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(ScalarLiteral::Integer(outer_end)), Some(ScalarLiteral::Integer(inner_end))) => {
                inner_end < outer_end
                    || (inner_end == outer_end && (self.is_inclusive || !inner.is_inclusive))
            }
            (Some(ScalarLiteral::Bigint(outer_end)), Some(ScalarLiteral::Bigint(inner_end))) => {
                inner_end < outer_end
                    || (inner_end == outer_end && (self.is_inclusive || !inner.is_inclusive))
            }
            (
                Some(ScalarLiteral::Character(outer_end)),
                Some(ScalarLiteral::Character(inner_end)),
            ) => {
                inner_end < outer_end
                    || (inner_end == outer_end && (self.is_inclusive || !inner.is_inclusive))
            }
            _ => false,
        }
    }

    /// Return whether this interval shares any value with another interval.
    pub fn overlaps_range(&self, other: &RangeType) -> bool {
        let left_domain = self.scalar_domain();
        let right_domain = other.scalar_domain();
        if matches!((left_domain, right_domain), (Some(left), Some(right)) if left != right) {
            return false;
        }

        // reject ranges where the left upper bound falls before the right start
        if Self::end_excludes_start(&self.end, self.is_inclusive, &other.start) {
            return false;
        }

        // reject ranges where the right upper bound falls before the left start
        if Self::end_excludes_start(&other.end, other.is_inclusive, &self.start) {
            return false;
        }

        true
    }

    /// Return the interval shared with another interval.
    pub fn intersection(&self, other: &RangeType) -> Option<RangeType> {
        let left_domain = self.scalar_domain();
        let right_domain = other.scalar_domain();
        if matches!((left_domain, right_domain), (Some(left), Some(right)) if left != right) {
            return None;
        }

        let start = Self::max_start_bound(&self.start, &other.start)?;
        let (end, is_inclusive) =
            Self::min_end_bound(&self.end, self.is_inclusive, &other.end, other.is_inclusive)?;
        if Self::end_excludes_start(&end, is_inclusive, &start) {
            return None;
        }

        Some(Self {
            start,
            end,
            is_inclusive,
        })
    }

    /// Return the remaining intervals after removing another interval.
    pub fn subtract_range(&self, removed: &RangeType) -> [Option<RangeType>; 2] {
        let Some(overlap) = self.intersection(removed) else {
            return [Some(*self), None];
        };

        let left = overlap.start.map(|start| RangeType {
            start: self.start,
            end: Some(start),
            is_inclusive: false,
        });
        let left = left.filter(RangeType::is_non_empty);

        let right_start = match (&overlap.end, overlap.is_inclusive) {
            (Some(end), true) => end.successor(),
            (Some(end), false) => Some(*end),
            (None, _) => None,
        };
        let right = right_start.map(|start| RangeType {
            start: Some(start),
            end: self.end,
            is_inclusive: self.is_inclusive,
        });
        let right = right.filter(RangeType::is_non_empty);

        [left, right]
    }

    /// Return the literal when this interval contains exactly one discrete value.
    pub fn singleton_literal(&self) -> Option<ScalarLiteral> {
        let start = self.start?;
        let end = self.end?;

        if self.is_inclusive && start == end {
            return Some(start);
        }

        if !self.is_inclusive && start.successor()? == end {
            return Some(start);
        }

        None
    }

    /// Return whether this interval contains at least one value.
    pub fn is_non_empty(&self) -> bool {
        !Self::end_excludes_start(&self.end, self.is_inclusive, &self.start)
    }

    /// Return the greater inclusive lower bound.
    fn max_start_bound(
        left: &Option<ScalarLiteral>,
        right: &Option<ScalarLiteral>,
    ) -> Option<Option<ScalarLiteral>> {
        let start = match (left, right) {
            (None, None) => None,
            (Some(left), None) => Some(*left),
            (None, Some(right)) => Some(*right),
            (Some(ScalarLiteral::Integer(left)), Some(ScalarLiteral::Integer(right))) => {
                Some(ScalarLiteral::Integer((*left).max(*right)))
            }
            (Some(ScalarLiteral::Bigint(left)), Some(ScalarLiteral::Bigint(right))) => {
                Some(ScalarLiteral::Bigint((*left).max(*right)))
            }
            (Some(ScalarLiteral::Character(left)), Some(ScalarLiteral::Character(right))) => {
                Some(ScalarLiteral::Character((*left).max(*right)))
            }
            (Some(_), Some(_)) => return None,
        };

        Some(start)
    }

    /// Return the lesser upper bound.
    fn min_end_bound(
        left: &Option<ScalarLiteral>,
        left_is_inclusive: bool,
        right: &Option<ScalarLiteral>,
        right_is_inclusive: bool,
    ) -> Option<(Option<ScalarLiteral>, bool)> {
        let end = match (left, right) {
            (None, None) => (None, left_is_inclusive && right_is_inclusive),
            (Some(left), None) => (Some(*left), left_is_inclusive),
            (None, Some(right)) => (Some(*right), right_is_inclusive),
            (Some(ScalarLiteral::Integer(left)), Some(ScalarLiteral::Integer(right))) => {
                if left < right {
                    (Some(ScalarLiteral::Integer(*left)), left_is_inclusive)
                } else if right < left {
                    (Some(ScalarLiteral::Integer(*right)), right_is_inclusive)
                } else {
                    (
                        Some(ScalarLiteral::Integer(*left)),
                        left_is_inclusive && right_is_inclusive,
                    )
                }
            }
            (Some(ScalarLiteral::Bigint(left)), Some(ScalarLiteral::Bigint(right))) => {
                if left < right {
                    (Some(ScalarLiteral::Bigint(*left)), left_is_inclusive)
                } else if right < left {
                    (Some(ScalarLiteral::Bigint(*right)), right_is_inclusive)
                } else {
                    (
                        Some(ScalarLiteral::Bigint(*left)),
                        left_is_inclusive && right_is_inclusive,
                    )
                }
            }
            (Some(ScalarLiteral::Character(left)), Some(ScalarLiteral::Character(right))) => {
                if left < right {
                    (Some(ScalarLiteral::Character(*left)), left_is_inclusive)
                } else if right < left {
                    (Some(ScalarLiteral::Character(*right)), right_is_inclusive)
                } else {
                    (
                        Some(ScalarLiteral::Character(*left)),
                        left_is_inclusive && right_is_inclusive,
                    )
                }
            }
            (Some(_), Some(_)) => return None,
        };

        Some(end)
    }

    /// Return whether one upper bound excludes one lower bound.
    fn end_excludes_start(
        end: &Option<ScalarLiteral>,
        is_inclusive: bool,
        start: &Option<ScalarLiteral>,
    ) -> bool {
        match (end, start) {
            (Some(ScalarLiteral::Integer(end)), Some(ScalarLiteral::Integer(start))) => {
                end < start || (end == start && !is_inclusive)
            }
            (Some(ScalarLiteral::Bigint(end)), Some(ScalarLiteral::Bigint(start))) => {
                end < start || (end == start && !is_inclusive)
            }
            (Some(ScalarLiteral::Character(end)), Some(ScalarLiteral::Character(start))) => {
                end < start || (end == start && !is_inclusive)
            }
            (Some(_), Some(_)) => true,
            _ => false,
        }
    }
}

/// Runtime-length homogeneous view type.
///
/// Examples:
/// ```ds
/// [uint8]
/// Slice<uint8>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SliceType {
    /// The element type.
    pub element: GlobalTypeId,
}

/// A tuple type.
///
/// Examples:
/// ```ds
/// (string, int32)
/// ["id", 42]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct TupleType {
    /// The tuple source form.
    pub form: TupleForm,
    /// The tuple element list.
    pub elements: TypeListId,
}

/// The source form of a tuple type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum TupleForm {
    /// Parenthesized tuple form, like `(string, int32)`.
    Tuple,
    /// Bracket tuple form, like `["id", 42]`.
    Array,
}

/// An element in a tuple type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// A structural object shape type.
///
/// Examples:
/// ```ds
/// { name: string; age?: int32 }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ShapeType {
    /// The shape field list.
    pub fields: TypeListId,
    /// The call signature list.
    pub call_signatures: TypeListId,
    /// The construct signature list.
    pub construct_signatures: TypeListId,
    /// The index signature list.
    pub index_signatures: TypeListId,
}

/// A field in an object-like type.
/// Methods are represented as fields whose `ty` is a `Type::FunctionSignature`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// An index signature in an object type.
///
/// Examples:
/// ```ds
/// { [key: string]: int32 }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// A function signature type.
///
/// Examples:
/// ```ds
/// (value: int32) => string
/// async <T>(input: T) => Promise<T>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FunctionSignatureType {
    /// The function asynchrony.
    pub asynchrony: Asynchrony,
    /// The template that owns this signature's generic parameters.
    pub template: Option<GlobalGenericTemplateId>,
    /// The optional `this` parameter type.
    pub this_parameter: Option<GlobalTypeId>,
    /// The runtime parameter list.
    pub parameters: TypeListId,
    /// The optional return type.
    pub return_type: Option<GlobalTypeId>,
    /// Whether this is a generator function.
    pub is_generator: bool,
}

/// A runtime parameter in a function type.
///
/// Examples:
/// ```ds
/// (value?: int32, ...rest: string[]) => void
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// A fat callable value with a function signature and captured environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FunctionType {
    /// The function signature.
    pub signature: GlobalTypeId,
    /// The captured environment type.
    pub environment: GlobalTypeId,
}

/// A thin callable value with no captured environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FunctionPointerType {
    /// The function signature.
    pub signature: GlobalTypeId,
}

/// A union type.
///
/// Examples:
/// ```ds
/// string | int32
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct UnionType {
    /// The union element list.
    pub elements: TypeListId,
}

/// An intersection type.
///
/// Examples:
/// ```ds
/// Named & Aged
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct IntersectionType {
    /// The intersection element list.
    pub elements: TypeListId,
}
