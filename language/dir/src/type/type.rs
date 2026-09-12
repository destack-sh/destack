use std::borrow::Cow;
use std::cmp::Ordering;

use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::{
    Asynchrony, BinaryOperator, GlobalGenericParameterId, GlobalGenericTemplateId, GlobalStaticId,
    GlobalSymbolId, LanguageItem, Literal, MappedTypeModifier, RangeEnd, ScalarDomain, StaticKey,
    StringId, TypeFold, TypeLiteral, UnaryOperator,
};

use super::{FloatType, IntegerType, PrimitiveType};

/// A canonical solved type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Type {
    /// One open inference variable.
    Variable(TypeVariableId),

    /// Placeholder type for an already-reported error.
    Error,
    /// Never type `never`.
    Never,
    /// Unknown type.
    Unknown,
    /// Void type.
    Void,
    /// Null type and value.
    Null,
    /// Undefined type and value.
    Undefined,
    /// Concrete object class of one exact shape.
    Object(ObjectType),
    /// Primitive type, like `string` or `int32`.
    Primitive(PrimitiveType),
    /// Scalar literal type, like `"id"` or `42`.
    Literal(Literal),
    /// Singleton static property key type.
    Key(StaticKey),
    /// Singleton type of one committed static value.
    Static(GlobalStaticId),
    /// Compiler intrinsic type body.
    Intrinsic,
    /// Erased generic argument captured by a runtime head test.
    Erased(GlobalGenericParameterId),

    /// Generic parameter, like the `T` in `class Box<T>`.
    Parameter(GlobalGenericParameterId),
    /// Type declaration reference before application, like `Box` in `Box.empty`.
    Reference(TypeReference),
    /// Applied type declaration, like `User` or `Map<string, User>`.
    Application(GenericApplication),
    /// This type in a method signature, like `this` in `clone(): this`.
    This,
    /// Member type selected from an owner type, like `T.Output`.
    Member(MemberTypeId),
    /// Applied type refined by one associated member equality.
    Refined(RefinedTypeId),
    /// One selected enum variant, like `Mode.Read`.
    Variant(VariantType),

    /// One reference region: the referent's lifetime extent and space set.
    Region(RegionType),
    /// Canonical memory or access form, like `^User` or `&User`.
    Form(FormType),
    /// Explicit runtime `Dynamic<T>` representation, like `Dynamic<Printable>`.
    Dynamic(DynamicType),

    /// Type-level operation.
    Operation(TypeOperationId),

    /// Fixed-length array type, like `[uint8; 4]`.
    FixedArray(FixedArrayType),
    /// Compact scalar interval type, like `0..10`.
    Range(RangeType),
    /// Runtime-length homogeneous view type, like `[uint8]`.
    Slice(SliceType),
    /// Tuple type, like `(string, int32)`.
    Tuple(TupleType),

    /// Function signature type, like `(value: int32) => string`, interned in the segment.
    FunctionSignature(FunctionSignatureId),
    /// Fat callable value.
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
            TypeLiteral::Undefined => Self::Undefined,
            TypeLiteral::Unknown => Self::Unknown,
            TypeLiteral::Void => Self::Void,
            TypeLiteral::Null => Self::Null,
            TypeLiteral::Boolean => Self::Primitive(PrimitiveType::Boolean),
            TypeLiteral::Character => Self::Primitive(PrimitiveType::Character),
            TypeLiteral::String => Self::Primitive(PrimitiveType::String),
            TypeLiteral::Bigint => Self::Primitive(PrimitiveType::Bigint),
            TypeLiteral::Number => Self::Primitive(PrimitiveType::Float(FloatType::Float64)),
            TypeLiteral::Alias(alias) => Self::Primitive(alias.primitive()),
            TypeLiteral::Integer(integer) => Self::Primitive(PrimitiveType::Integer(integer)),
            TypeLiteral::Float(float) => Self::Primitive(PrimitiveType::Float(float)),
        }
    }
}

impl From<Literal> for Type {
    /// Convert a scalar literal expression into its exact type.
    fn from(value: Literal) -> Self {
        Self::from(&value)
    }
}

impl From<&Literal> for Type {
    /// Convert a scalar literal expression into its exact type.
    fn from(value: &Literal) -> Self {
        match value {
            Literal::Null => Self::Null,
            Literal::Undefined => Self::Undefined,
            Literal::Boolean(_) => Self::Primitive(PrimitiveType::Boolean),
            Literal::Character(_) => Self::Primitive(PrimitiveType::Character),
            Literal::String(_) | Literal::Integer(_) | Literal::Float(_) | Literal::Bigint(_) => {
                Self::Literal(*value)
            }
            Literal::RegexString { .. } => Self::Error,
        }
    }
}

impl Type {
    /// Return whether an alias of this type declares a representation of its own.
    pub fn is_structural(&self) -> bool {
        matches!(
            self,
            Self::Object(_)
                | Self::Union(_)
                | Self::Tuple(_)
                | Self::Slice(_)
                | Self::FixedArray(_)
                | Self::Function(_)
        )
    }

    /// Return whether this is a boolean type or boolean singleton.
    pub fn is_boolean(&self) -> bool {
        matches!(
            self,
            Self::Primitive(PrimitiveType::Boolean) | Self::Literal(Literal::Boolean(_))
        )
    }

    /// Return this type's variant name.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Self::Variable(_) => "Variable",
            Self::Error => "Error",
            Self::Never => "Never",
            Self::Unknown => "Unknown",
            Self::Void => "Void",
            Self::Null => "Null",
            Self::Undefined => "Undefined",
            Self::Object(_) => "Object",
            Self::Primitive(_) => "Primitive",
            Self::Literal(_) => "Literal",
            Self::Key(_) => "Key",
            Self::Reference(_) => "Reference",
            Self::Static(_) => "Static",
            Self::Intrinsic => "Intrinsic",
            Self::Parameter(_) => "Parameter",
            Self::Erased(_) => "Erased",
            Self::This => "This",
            Self::Range(_) => "Range",
            Self::Application(_) => "Application",
            Self::Refined(_) => "Refined",
            Self::Member(_) => "Member",
            Self::Variant(_) => "Variant",
            Self::Form(_) => "Form",
            Self::Region(_) => "Region",
            Self::Dynamic(_) => "Dynamic",
            Self::Operation(_) => "Operation",
            Self::FixedArray(_) => "FixedArray",
            Self::Slice(_) => "Slice",
            Self::Tuple(_) => "Tuple",
            Self::FunctionSignature(_) => "FunctionSignature",
            Self::Function(_) => "Function",
            Self::FunctionPointer(_) => "FunctionPointer",
            Self::Union(_) => "Union",
            Self::Intersection(_) => "Intersection",
        }
    }

    /// Return whether this type is the undefined singleton.
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined | Self::Literal(Literal::Undefined))
    }

    /// Return whether this is the unit type.
    pub fn is_unit(&self) -> bool {
        matches!(self, Self::Void)
            || matches!(
                self,
                Self::Tuple(tuple)
                    if tuple.form == TupleForm::Tuple && tuple.elements.is_empty()
            )
    }

    /// Return whether this type names one constant value.
    pub fn is_constant(&self) -> bool {
        matches!(
            self,
            Self::Literal(_) | Self::Variant(_) | Self::Null | Self::Undefined
        )
    }

    /// Return whether runtime values of this type can carry memory placement.
    pub fn is_placeable(&self) -> bool {
        !matches!(
            self,
            Self::Error
                | Self::Never
                | Self::Void
                | Self::Null
                | Self::Undefined
                | Self::Static(_)
                | Self::Intrinsic
                | Self::FunctionSignature(_)
        )
    }

    /// Return whether this type can change shape after solving or substitution.
    pub fn is_open(&self) -> bool {
        matches!(
            self,
            Self::Parameter(_)
                | Self::Variable(_)
                | Self::This
                | Self::Member(_)
                | Self::Operation(_)
        )
    }

    /// Return this type's direct scalar domain.
    pub fn scalar_domain(&self) -> Option<ScalarDomain> {
        let domain = match self {
            Self::Null => ScalarDomain::Null,
            Self::Undefined => ScalarDomain::Undefined,
            Self::Primitive(primitive) => primitive.scalar_domain(),
            Self::Literal(literal) => return literal.scalar_domain(),
            Self::Key(key) if key.is_string_like() => ScalarDomain::String,
            Self::Key(key) if key.is_number_like() => ScalarDomain::Integer,
            Self::Range(range) => return range.scalar_domain(),
            _ => return None,
        };

        Some(domain)
    }

    /// Return the primitive representation selected when this scalar must store a runtime value.
    pub fn scalar_representation(&self) -> Option<Self> {
        let primitive = match self {
            Self::Primitive(_) => return Some(*self),
            Self::Literal(literal) => return Some(literal.widen()),
            Self::Key(StaticKey::Name(_)) => PrimitiveType::String,
            Self::Key(StaticKey::Index(_)) => {
                PrimitiveType::Integer(IntegerType::Pointer { is_signed: false })
            }
            _ => return None,
        };

        Some(Self::Primitive(primitive))
    }

    /// Return the language declaration that owns this built-in type's members.
    pub fn member_owner_item(&self) -> Option<LanguageItem> {
        if let Some(domain) = self.scalar_domain() {
            return domain.member_owner_item();
        }

        self.builtin_item()
    }

    /// Return the language declaration that carries this built-in type at runtime.
    pub fn representation_item(&self) -> Option<LanguageItem> {
        if let Some(domain) = self.scalar_domain() {
            return domain.representation_item();
        }

        self.builtin_item()
    }

    /// Return the language declaration backing this slice, fixed array, or function type.
    fn builtin_item(&self) -> Option<LanguageItem> {
        match self {
            Self::Slice(_) => Some(LanguageItem::Slice),
            Self::FixedArray(_) => Some(LanguageItem::FixedArray),
            Self::Function(_) | Self::FunctionSignature(_) => Some(LanguageItem::Function),
            _ => None,
        }
    }

    /// Iterate over language declarations that may own built-in type members.
    pub fn member_owner_items() -> impl Iterator<Item = LanguageItem> {
        [
            LanguageItem::Number,
            LanguageItem::BigInt,
            LanguageItem::String,
            LanguageItem::Array,
            LanguageItem::Slice,
            LanguageItem::FixedArray,
        ]
        .into_iter()
    }

    /// Return the only scalar literal inhabiting this type.
    pub fn singleton_literal(&self) -> Option<Literal> {
        match self {
            Self::Null => Some(Literal::Null),
            Self::Undefined => Some(Literal::Undefined),
            Self::Literal(literal) => Some(*literal),
            _ => None,
        }
    }

    /// Return every inhabitant when this type has a finite literal set.
    pub fn finite_literals(&self) -> Option<SmallVec<[Literal; 2]>> {
        // return the exact singleton directly
        if let Some(literal) = self.singleton_literal() {
            return Some(smallvec![literal]);
        }

        // enumerate the remaining finite domains
        match self {
            Self::Never => Some(SmallVec::new()),
            Self::Primitive(PrimitiveType::Boolean) => {
                Some(smallvec![Literal::Boolean(false), Literal::Boolean(true)])
            }
            _ => None,
        }
    }

    /// Return the symbolic leaf kind contributed by this type alone.
    ///
    /// A stored type joins this bit with every child type's flags.
    pub fn own_flags(&self) -> TypeFlags {
        match self {
            // symbolic leaves, one bit each
            Self::Variable(_) => TypeFlags::HAS_VARIABLE,
            Self::Error => TypeFlags::HAS_ERROR,
            Self::Parameter(_) => TypeFlags::HAS_PARAMETER,
            Self::Erased(_) => TypeFlags::HAS_PARAMETER,
            Self::This => TypeFlags::HAS_THIS,
            Self::Reference(_) | Self::Application(_) => TypeFlags::HAS_REFERENCE,
            Self::Member(_) => TypeFlags::HAS_MEMBER,
            Self::Refined(_) => TypeFlags::HAS_MEMBER,
            Self::Operation(_) => TypeFlags::HAS_OPERATION,

            // concrete heads contribute nothing of their own
            Self::Never
            | Self::Unknown
            | Self::Void
            | Self::Null
            | Self::Undefined
            | Self::Primitive(_)
            | Self::Literal(_)
            | Self::Key(_)
            | Self::Static(_)
            | Self::Intrinsic
            | Self::Variant(_)
            | Self::Form(_)
            | Self::Region(_)
            | Self::Dynamic(_)
            | Self::FixedArray(_)
            | Self::Range(_)
            | Self::Slice(_)
            | Self::Tuple(_)
            | Self::Object(_)
            | Self::FunctionSignature(_)
            | Self::Function(_)
            | Self::FunctionPointer(_)
            | Self::Union(_)
            | Self::Intersection(_) => TypeFlags::EMPTY,
        }
    }

    /// Collect every module id this entry mentions directly.
    ///
    /// List and pool contents live in the segment and are scanned there.
    /// This visits the ids embedded in the entry itself.
    pub fn referenced_modules(&self, collect: &mut impl FnMut(ModuleId)) {
        // collect the module of every embedded id
        match self {
            // parameter and symbol heads
            Self::Erased(parameter) | Self::Parameter(parameter) => collect(parameter.module_id),
            Self::Reference(reference) => collect(reference.symbol.module_id),
            Self::Application(application) => collect(application.symbol.module_id),
            Self::Variant(variant) => {
                collect(variant.owner.module_id);
                collect(variant.variant.module_id);
            }
            Self::Static(value) => collect(value.module_id),

            // wrapped value heads
            Self::Form(form) => collect(form.value.module_id),
            Self::Region(region) => {
                collect(region.extent.module_id);
                collect(region.space.module_id);
            }
            Self::Dynamic(dynamic) => collect(dynamic.constraint.module_id),
            Self::FixedArray(array) => {
                collect(array.element.module_id);
                collect(array.count.module_id);
            }
            Self::Slice(slice) => collect(slice.element.module_id),
            Self::Function(function) => collect(function.signature.module_id),
            Self::FunctionPointer(function) => collect(function.signature.module_id),

            // list and pool heads scan through the segment
            Self::Object(_)
            | Self::Tuple(_)
            | Self::Union(_)
            | Self::Intersection(_)
            | Self::Member(_)
            | Self::Refined(_)
            | Self::Operation(_)
            | Self::FunctionSignature(_) => {}

            // idless leaves
            Self::Error
            | Self::Variable(_)
            | Self::Never
            | Self::Unknown
            | Self::Void
            | Self::Null
            | Self::Undefined
            | Self::Primitive(_)
            | Self::Literal(_)
            | Self::Key(_)
            | Self::Intrinsic
            | Self::Range(_)
            | Self::This => {}
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
            Self::Application(instance) => Some(instance.symbol),
            _ => None,
        }
    }
}

/// The symbolic leaf kinds contained in one type graph, computed once at intern time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct TypeFlags(u16);

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
    /// The graph names a declaration, a bare reference or an application, which may expand.
    pub const HAS_REFERENCE: Self = Self(1 << 4);
    /// The graph contains a member projection like `T.Output`.
    pub const HAS_MEMBER: Self = Self(1 << 5);
    /// The graph contains a type-level operation.
    pub const HAS_OPERATION: Self = Self(1 << 6);
    /// The graph contains a conditional infer binding.
    pub const HAS_INFER: Self = Self(1 << 7);
    /// The graph contains a generic parameter outside the region kind.
    pub const HAS_TYPE_PARAMETER: Self = Self(1 << 8);

    /// Return whether every bit of `other` is set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Return whether the graph contains no open or symbolic leaves.
    pub fn is_ground(self) -> bool {
        let symbolic = Self::HAS_VARIABLE
            | Self::HAS_PARAMETER
            | Self::HAS_THIS
            | Self::HAS_REFERENCE
            | Self::HAS_MEMBER
            | Self::HAS_OPERATION
            | Self::HAS_INFER;

        !self.contains_any(symbolic)
    }

    /// Return whether any bit of `other` is set.
    fn contains_any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Return whether the graph contains an open inference variable.
    pub fn has_variable(self) -> bool {
        self.contains(Self::HAS_VARIABLE)
    }

    /// Return whether the graph contains a generic parameter.
    pub fn has_parameter(self) -> bool {
        self.contains(Self::HAS_PARAMETER)
    }

    /// Return whether the graph contains a generic parameter outside the region kind.
    pub fn has_type_parameter(self) -> bool {
        self.contains(Self::HAS_TYPE_PARAMETER)
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

/// Unique identifier for one interned borrow form payload.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct BorrowFormId(pub u32);

impl BorrowFormId {
    /// Wrap a raw id as a BorrowFormId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// One borrow form's region and access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct BorrowForm {
    /// The solved borrow region: a region pair, parameter, or variable.
    pub region: GlobalTypeId,
    /// The borrow access, a singleton, parameter, or inference variable.
    pub access: GlobalTypeId,
}

/// Unique identifier for one interned member projection payload.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct MemberTypeId(pub u32);

impl MemberTypeId {
    /// Wrap a raw id as a MemberTypeId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Unique identifier for one interned refined application payload.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct RefinedTypeId(pub u32);

impl RefinedTypeId {
    /// Wrap a raw id as a RefinedTypeId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Unique identifier for one interned function signature payload.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct FunctionSignatureId(pub u32);

impl FunctionSignatureId {
    /// Wrap a raw id as a FunctionSignatureId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Unique identifier for one interned type operation payload.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct TypeOperationId(pub u32);

impl TypeOperationId {
    /// Wrap a raw id as a TypeOperationId.
    pub fn new(id: u32) -> Self {
        Self(id)
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct TypeVariableId(pub u32);

/// One interned list inside the owning module's type storage.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct TypeListId {
    /// The index of the list across the module's pools of its kind.
    pub index: u32,
    /// The number of elements in the list.
    pub count: u32,
}

impl TypeListId {
    /// The canonical empty list.
    pub const EMPTY: Self = Self { index: 0, count: 0 };

    /// Create a new list id.
    pub fn new(index: u32, count: u32) -> Self {
        Self { index, count }
    }

    /// Return the number of elements in the list.
    pub fn len(&self) -> u32 {
        self.count
    }

    /// Return whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Access to borrowed storage.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum Access {
    /// Read access while aliases may write.
    Readonly,
    /// Read and write access while aliases may access the storage.
    Mutable,
    /// Read access that excludes conflicting writes.
    Immutable,
    /// Read and write access that excludes conflicting accesses.
    Exclusive,
}

impl Access {
    /// Parse one canonical access name.
    pub fn from_text(value: &str) -> Option<Self> {
        [
            Self::Readonly,
            Self::Mutable,
            Self::Immutable,
            Self::Exclusive,
        ]
        .into_iter()
        .find(|access| access.text() == value)
    }

    /// Return whether this access grants the requested access.
    pub fn grants(self, requested: Self) -> bool {
        self == requested || self == Self::Exclusive || requested == Self::Readonly
    }

    /// Return the canonical text of this access.
    pub const fn text(self) -> &'static str {
        match self {
            Self::Readonly => "readonly",
            Self::Mutable => "mutable",
            Self::Immutable => "immutable",
            Self::Exclusive => "exclusive",
        }
    }
}

/// Normalized storage space value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Space {
    /// Local storage.
    Local,
    /// Shared storage.
    Shared,
    /// Immutable link-written storage, derived by the linker.
    Constant,
}

impl Space {
    /// Return the canonical text of this space.
    pub const fn text(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Shared => "shared",
            Self::Constant => "constant",
        }
    }

    /// Parse one canonical space name.
    pub fn from_text(value: &str) -> Option<Self> {
        [Self::Local, Self::Shared, Self::Constant]
            .into_iter()
            .find(|space| space.text() == value)
    }
}

/// Normalized lifetime value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Lifetime {
    /// Static lifetime ('static).
    Static,
    /// The enclosing frame's lifetime ('frame).
    Frame,
    /// The extent of managed storage, alive while reachable ('managed).
    Managed,
    /// The region bound at one position of an instance's arguments ('bound0).
    Bound(u32),
}

impl Lifetime {
    /// Return the canonical text of this lifetime.
    pub fn text(self) -> Cow<'static, str> {
        match self {
            Self::Static => Cow::Borrowed("static"),
            Self::Frame => Cow::Borrowed("frame"),
            Self::Managed => Cow::Borrowed("managed"),
            Self::Bound(position) => Cow::Owned(format!("bound{position}")),
        }
    }

    /// Parse one canonical lifetime name, bound positions included.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "static" => Some(Self::Static),
            "frame" => Some(Self::Frame),
            "managed" => Some(Self::Managed),
            other => other.strip_prefix("bound")?.parse().ok().map(Self::Bound),
        }
    }
}

/// One declaration reference with its explicitly bound generic arguments.
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
    /// The explicit generic arguments, with omitted parameters left for invocation.
    pub arguments: TypeListId,
}

impl TypeReference {
    /// Create a declaration reference with no explicit generic arguments.
    pub fn new(symbol: GlobalSymbolId) -> Self {
        Self {
            symbol,
            arguments: TypeListId::EMPTY,
        }
    }
}

/// One declaration applied to its complete positional arguments.
/// A non-generic reference is an application with no arguments.
///
/// Examples:
/// ```ds
/// User                  // no arguments
/// Map<string, User>     // two positional arguments
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct GenericApplication {
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
    /// The declaring scope qualifying the projection.
    pub qualifier: Option<GlobalTypeId>,
}

/// One associated member equality refining an applied type.
///
/// Examples:
/// ```ds
/// function nextByte<I: Iterator<type Item = uint8>>(iter: I): uint8;
/// function stream<S: Source<type Chunk = string, type Error = E>>(source: S): E;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct RefinedType {
    /// The refined base application.
    pub base: GlobalTypeId,
    /// The refined associated member key.
    pub key: StaticKey,
    /// The required associated member type.
    pub value: GlobalTypeId,
}

/// One selected enum variant type.
///
/// Examples:
/// ```ds
/// Mode.Read
/// Mode.Write
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct VariantType {
    /// The instantiated enum.
    pub owner: GlobalTypeId,
    /// The selected variant declaration.
    pub variant: GlobalSymbolId,
}

/// One region pair: the lifetime extent and the referent space set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct RegionType {
    /// The lifetime extent: a lifetime singleton, parameter, or variable.
    pub extent: GlobalTypeId,
    /// The referent space: a space literal, parameter, or variable.
    pub space: GlobalTypeId,
}

/// Canonical memory or access form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FormType {
    /// The form constructor.
    pub form: Form,
    /// The type carried by the form.
    pub value: GlobalTypeId,
}

/// Canonical memory or access form constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub enum Ownership {
    /// Automatically managed reference ownership.
    Managed,
    /// Owned value ownership.
    Owned,
    /// Borrowed view ownership.
    Borrowed,
    /// Raw pointer ownership.
    Raw,
}

impl Ownership {
    /// Parse one canonical ownership name.
    pub fn from_text(value: StringId) -> Option<Self> {
        [Self::Managed, Self::Owned, Self::Borrowed, Self::Raw]
            .into_iter()
            .find(|ownership| value == StringId::for_text(ownership.text()))
    }

    /// Return the canonical singleton text.
    pub fn text(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Owned => "owned",
            Self::Borrowed => "borrowed",
            Self::Raw => "raw",
        }
    }
}

/// Canonical memory or access form constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Form {
    /// Automatically managed runtime value with its referent place,
    /// the unqualified `User` or the qualified `shared User`.
    Managed {
        /// The solved concrete or ambient referent place singleton.
        place: GlobalTypeId,
    },
    /// Owned value, like `^User`.
    Owned,
    /// Borrowed value, like `&User`, `&readonly User`, or `&User`,
    /// with its lifetime, access, and referent place interned in the segment.
    Borrowed(BorrowFormId),
    /// Raw pointer value, like `*User`.
    Raw,
    /// Readonly view, like `readonly User`.
    Readonly,
}

impl Form {
    /// Return whether this form is a non-owning view over its payload.
    pub fn is_view(self) -> bool {
        matches!(self, Self::Borrowed(_) | Self::Raw | Self::Readonly)
    }

    /// Return the language item constructing this form.
    pub fn language_item(self) -> LanguageItem {
        match self {
            Self::Managed { .. } => LanguageItem::Managed,
            Self::Owned => LanguageItem::Owned,
            Self::Borrowed(_) => LanguageItem::Borrowed,
            Self::Raw => LanguageItem::Raw,
            Self::Readonly => LanguageItem::Readonly,
        }
    }

    /// Return whether one receiver form adjusts to a declared target form.
    pub fn adjusts_to(self, target: Form) -> bool {
        match (self, target) {
            // family-default targets accept every receiver
            (_, Form::Managed { .. }) => true,
            // owned targets consume, only owned receivers match them
            (Form::Owned, Form::Owned) => true,
            (_, Form::Owned) => false,
            // borrow targets accept reborrowable receivers
            (Form::Owned | Form::Managed { .. }, Form::Borrowed(_)) => true,
            (Form::Borrowed(_), Form::Borrowed(_)) => true,
            // raw pointers match only raw targets
            (Form::Raw, Form::Raw) => true,
            _ => false,
        }
    }

    /// Return this form's ownership constructor, when it carries one.
    pub fn ownership(self) -> Option<Ownership> {
        match self {
            Self::Managed { .. } => Some(Ownership::Managed),
            Self::Owned => Some(Ownership::Owned),
            Self::Borrowed(_) => Some(Ownership::Borrowed),
            Self::Raw => Some(Ownership::Raw),
            Self::Readonly => None,
        }
    }

    /// Return whether two forms share one constructor.
    pub fn same_constructor(&self, other: &Form) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// Explicit runtime `Dynamic<T>` representation.
///
/// Examples:
/// ```ds
/// Dynamic<Printable>    // a boxed value known to satisfy Printable
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct DynamicType {
    /// The `Dynamic<T>` constraint.
    pub constraint: GlobalTypeId,
    /// The place of the erased referent.
    pub place: GlobalTypeId,
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
    /// Explicit generic application to a value type, like `typeof create<int32>`.
    Instantiation(InstantiationType),
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

impl TypeOperation {
    /// Collect every module id this operation mentions directly.
    pub fn referenced_modules(&self, collect: &mut impl FnMut(ModuleId)) {
        match self {
            Self::StringMapping { target, .. } => collect(target.module_id),
            Self::Conditional(conditional) => {
                collect(conditional.left.module_id);
                collect(conditional.right.module_id);
                collect(conditional.then_type.module_id);
                collect(conditional.else_type.module_id);
            }
            Self::Narrow(narrow) => {
                collect(narrow.source.module_id);
                collect(narrow.target.module_id);
            }
            Self::Mapped(mapped) => {
                collect(mapped.parameter.parameter.module_id);
                collect(mapped.parameter.constraint.module_id);
                if let Some(remap) = mapped.parameter.key_remap {
                    collect(remap.module_id);
                }
                if let Some(modifiers) = mapped.parameter.modifiers_type {
                    collect(modifiers.module_id);
                }
                collect(mapped.value.module_id);
            }
            Self::Index(index) => {
                collect(index.left.module_id);
                collect(index.index.module_id);
            }
            Self::TemplateLiteral(_) => {}
            Self::Infer(infer) => {
                if let Some(symbol) = infer.symbol {
                    collect(symbol.module_id);
                }
                if let Some(constraint) = infer.constraint {
                    collect(constraint.module_id);
                }
            }
            Self::TypeOf(_) => {}
            Self::Instantiation(application) => collect(application.target.module_id),
            Self::KeyOf(unary) | Self::NoInfer(unary) | Self::Awaited(unary) => {
                collect(unary.target.module_id);
            }
            Self::TryOutput { value } => collect(value.module_id),
            Self::TryResidual { value } => collect(value.module_id),
            Self::StaticBinary(binary) => {
                collect(binary.left.module_id);
                collect(binary.right.module_id);
            }
            Self::StaticUnary(unary) => collect(unary.target.module_id),
        }
    }

    /// Return the structural flags this operation contributes to its type.
    pub fn own_flags(&self) -> TypeFlags {
        match self {
            Self::Infer(_) => TypeFlags::HAS_INFER,
            _ => TypeFlags::EMPTY,
        }
    }
}

/// Type query expression, like `typeof value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct TypeOfType {
    /// The queried value declaration.
    pub symbol: GlobalSymbolId,
}

/// Generic arguments applied to one value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct InstantiationType {
    /// The unapplied value type.
    pub target: GlobalTypeId,
    /// The written generic arguments.
    pub arguments: TypeListId,
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
    /// Return the source name for this string mapping.
    pub fn text(self) -> &'static str {
        match self {
            Self::Uppercase => "Uppercase",
            Self::Lowercase => "Lowercase",
            Self::Capitalize => "Capitalize",
            Self::Uncapitalize => "Uncapitalize",
        }
    }

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

        // read the escape each character names
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct ConditionalType {
    /// The left operand.
    pub left: GlobalTypeId,
    /// The right operand.
    pub right: GlobalTypeId,
    /// The type selected when the condition holds.
    pub then_type: GlobalTypeId,
    /// The type selected when the condition fails.
    pub else_type: GlobalTypeId,
    /// Whether the conditional distributes over union-valued left operands.
    pub is_distributive: bool,
}

/// A runtime guard narrowing applied to one source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct MappedTypeParameter {
    /// The parameter name like `K`.
    pub name: StringId,
    /// The binder's generic parameter.
    pub parameter: GlobalGenericParameterId,
    /// The constraint type like `keyof T`.
    pub constraint: GlobalTypeId,
    /// The optional key remap like `as Foo<K>`.
    pub key_remap: Option<GlobalTypeId>,
    /// The keyed source whose field modifiers carry, like `T` in `[K in keyof T]`.
    pub modifiers_type: Option<GlobalTypeId>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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
    /// Return the source text for this static binary operator.
    pub fn text(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::Exponent => "**",
            Self::ShiftLeft => "<<",
            Self::ShiftRight => ">>",
            Self::UnsignedShiftRight => ">>>",
            Self::BitwiseAnd => "&",
            Self::BitwiseXor => "^",
            Self::BitwiseOr => "|",
            Self::Equal => "==",
            Self::EqualStrict => "===",
            Self::NotEqual => "!=",
            Self::NotEqualStrict => "!==",
            Self::LessThan => "<",
            Self::LessThanOrEqual => "<=",
            Self::GreaterThan => ">",
            Self::GreaterThanOrEqual => ">=",
            Self::And => "&&",
            Self::Or => "||",
        }
    }

    /// Return whether this operator yields a boolean result.
    ///
    /// Comparisons and logical operators yield booleans; arithmetic,
    /// shift, and bitwise operators stay within their operand type.
    pub fn yields_boolean(self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::EqualStrict
                | Self::NotEqual
                | Self::NotEqualStrict
                | Self::LessThan
                | Self::LessThanOrEqual
                | Self::GreaterThan
                | Self::GreaterThanOrEqual
                | Self::And
                | Self::Or
        )
    }

    /// Evaluate this operator over two scalar literals.
    pub fn apply(self, left: Literal, right: Literal) -> Result<Literal, &'static str> {
        use Literal;
        use StaticBinaryOperator as Operator;

        // bigint pairs evaluate over their shared integer payload and rewrap
        if let (Literal::Bigint(left), Literal::Bigint(right)) = (left, right) {
            let evaluated = self.apply(Literal::Integer(left), Literal::Integer(right))?;

            return Ok(match evaluated {
                Literal::Integer(value) => Literal::Bigint(value),
                evaluated => evaluated,
            });
        }

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
                let invalid = if left.is_nan() || right.is_nan() {
                    "float comparison is undefined for nan"
                } else {
                    "static comparison requires matching operand kinds"
                };
                let ordering = left.ordering(&right).ok_or(invalid)?;

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

            // logical joins arrive here only with non-boolean operands
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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

impl StaticUnaryOperator {
    /// Return the source text for this static unary operator.
    pub fn text(self) -> &'static str {
        match self {
            Self::Not => "!",
            Self::Negate => "-",
            Self::BitwiseNot => "~",
        }
    }

    /// Return whether this operator yields a boolean result.
    pub fn yields_boolean(self) -> bool {
        matches!(self, Self::Not)
    }

    /// Evaluate this operator over one scalar literal.
    pub fn apply(self, target: Literal) -> Result<Literal, &'static str> {
        let literal = match (self, target) {
            (Self::Not, Literal::Boolean(value)) => Literal::Boolean(!value),
            (Self::Negate, Literal::Integer(value)) => {
                Literal::Integer(value.checked_neg().ok_or("integer negation overflows")?)
            }
            (Self::Negate, Literal::Float(value)) => Literal::Float(-value),
            (Self::Negate, Literal::Bigint(value)) => {
                Literal::Bigint(value.checked_neg().ok_or("bigint negation overflows")?)
            }
            (Self::BitwiseNot, Literal::Integer(value)) => Literal::Integer(!value),
            (Self::BitwiseNot, Literal::Bigint(value)) => Literal::Bigint(!value),
            _ => return Err("static operator does not apply to its operand kind"),
        };

        Ok(literal)
    }
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

/// A fixed-length array type.
///
/// Examples:
/// ```ds
/// [uint8; 4]
/// FixedArray<uint8, 4>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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
    pub start: Option<Literal>,
    /// The upper bound.
    pub end: Option<Literal>,
    /// Whether the upper bound is included.
    pub is_inclusive: bool,
}

impl RangeType {
    /// Create an interval from pattern bounds.
    pub fn new(start: Option<Literal>, end: Option<Literal>, end_kind: RangeEnd) -> Self {
        Self {
            start,
            end,
            is_inclusive: matches!(end_kind, RangeEnd::Inclusive),
        }
    }

    /// Return this interval's scalar domain.
    pub fn scalar_domain(&self) -> Option<ScalarDomain> {
        let start = self.start.as_ref().and_then(Literal::interval_domain);
        let end = self.end.as_ref().and_then(Literal::interval_domain);

        // require both written bounds to name one domain
        match (start, end) {
            (Some(start), Some(end)) if start == end => Some(start),
            (Some(start), None) => Some(start),
            (None, Some(end)) => Some(end),
            _ => None,
        }
    }

    /// Return this interval's ordinary scalar type.
    pub fn widen(&self) -> Option<Type> {
        let ty = match self.scalar_domain()? {
            ScalarDomain::Integer => Type::Primitive(PrimitiveType::Integer(IntegerType::Fixed {
                width: 64,
                is_signed: true,
            })),
            ScalarDomain::Bigint => Type::Primitive(PrimitiveType::Bigint),
            ScalarDomain::Character => Type::Primitive(PrimitiveType::Character),
            ScalarDomain::Float
            | ScalarDomain::String
            | ScalarDomain::Boolean
            | ScalarDomain::Null
            | ScalarDomain::Undefined => return None,
        };

        Some(ty)
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
    pub fn contains_literal(&self, literal: Literal) -> bool {
        match literal {
            Literal::Integer(value) => self.contains_integer(value),
            Literal::Bigint(value) => self.contains_bigint(value),
            Literal::Character(value) => self.contains_character(value),
            _ => false,
        }
    }

    /// Return whether this interval contains one integer literal.
    pub fn contains_integer(&self, value: i64) -> bool {
        // test the value against the written start bound
        let start_holds = match self.start {
            Some(Literal::Integer(start)) => value >= start,
            Some(_) => false,
            None => true,
        };

        // test the value against the written end bound
        let end_holds = match self.end {
            Some(Literal::Integer(end)) => {
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
        // test the value against the written start bound
        let start_holds = match self.start {
            Some(Literal::Bigint(start)) => value >= start,
            Some(_) => false,
            None => true,
        };

        // test the value against the written end bound
        let end_holds = match self.end {
            Some(Literal::Bigint(end)) => {
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
        // test the value against the written start bound
        let start_holds = match self.start {
            Some(Literal::Character(start)) => value >= start,
            Some(_) => false,
            None => true,
        };

        // test the value against the written end bound
        let end_holds = match self.end {
            Some(Literal::Character(end)) => {
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
                    Some(Literal::Integer(start)) => integer.fits_literal(*start),
                    Some(_) | None => false,
                };
                let end_widens = match &self.end {
                    Some(Literal::Integer(end)) => integer.fits_literal(*end),
                    Some(_) | None => false,
                };

                start_widens && end_widens
            }
            // bigint intervals widen to the bigint primitive
            PrimitiveType::Bigint => matches!(
                (&self.start, &self.end),
                (
                    Some(Literal::Bigint(_)) | None,
                    Some(Literal::Bigint(_)) | None,
                )
            ),
            // character intervals widen to the character primitive
            PrimitiveType::Character => matches!(
                (&self.start, &self.end),
                (
                    Some(Literal::Character(_)) | None,
                    Some(Literal::Character(_)) | None,
                )
            ),
            _ => false,
        }
    }

    /// Return whether this interval contains another interval.
    pub fn contains_range(&self, inner: &RangeType) -> bool {
        // the outer start must stay at or below the inner start
        let start_holds = match (&self.start, &inner.start) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(Literal::Integer(outer)), Some(Literal::Integer(inner))) => outer <= inner,
            (Some(Literal::Bigint(outer)), Some(Literal::Bigint(inner))) => outer <= inner,
            (Some(Literal::Character(outer)), Some(Literal::Character(inner))) => outer <= inner,
            _ => false,
        };
        if !start_holds {
            return false;
        }

        // the outer end must stay at or above the inner end
        match (&self.end, &inner.end) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(Literal::Integer(outer_end)), Some(Literal::Integer(inner_end))) => {
                inner_end < outer_end
                    || (inner_end == outer_end && (self.is_inclusive || !inner.is_inclusive))
            }
            (Some(Literal::Bigint(outer_end)), Some(Literal::Bigint(inner_end))) => {
                inner_end < outer_end
                    || (inner_end == outer_end && (self.is_inclusive || !inner.is_inclusive))
            }
            (Some(Literal::Character(outer_end)), Some(Literal::Character(inner_end))) => {
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
    pub fn singleton_literal(&self) -> Option<Literal> {
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
    fn max_start_bound(left: &Option<Literal>, right: &Option<Literal>) -> Option<Option<Literal>> {
        let start = match (left, right) {
            (None, None) => None,
            (Some(left), None) => Some(*left),
            (None, Some(right)) => Some(*right),
            (Some(left), Some(right)) => match left.interval_ordering(right)? {
                Ordering::Less => Some(*right),
                Ordering::Equal | Ordering::Greater => Some(*left),
            },
        };

        Some(start)
    }

    /// Return the lesser upper bound.
    fn min_end_bound(
        left: &Option<Literal>,
        left_is_inclusive: bool,
        right: &Option<Literal>,
        right_is_inclusive: bool,
    ) -> Option<(Option<Literal>, bool)> {
        let end = match (left, right) {
            (None, None) => (None, left_is_inclusive && right_is_inclusive),
            (Some(left), None) => (Some(*left), left_is_inclusive),
            (None, Some(right)) => (Some(*right), right_is_inclusive),
            (Some(left), Some(right)) => match left.interval_ordering(right)? {
                Ordering::Less => (Some(*left), left_is_inclusive),
                Ordering::Greater => (Some(*right), right_is_inclusive),
                Ordering::Equal => (Some(*left), left_is_inclusive && right_is_inclusive),
            },
        };

        Some(end)
    }

    /// Return whether one upper bound excludes one lower bound.
    fn end_excludes_start(
        end: &Option<Literal>,
        is_inclusive: bool,
        start: &Option<Literal>,
    ) -> bool {
        match (end, start) {
            (Some(end), Some(start)) => match end.interval_ordering(start) {
                Some(Ordering::Less) => true,
                Some(Ordering::Equal) => !is_inclusive,
                Some(Ordering::Greater) => false,
                None => true,
            },
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct SliceType {
    /// The element type.
    pub element: GlobalTypeId,
    /// The place of the sliced elements.
    pub place: GlobalTypeId,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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

/// The members one anonymous object type declares.
///
/// Examples:
/// ```ds
/// { name: string; age?: int32 }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ObjectType {
    /// The declared property list.
    pub properties: TypeListId,
    /// The call signature list.
    pub call_signatures: TypeListId,
    /// The construct signature list.
    pub construct_signatures: TypeListId,
    /// The index signature list.
    pub index_signatures: TypeListId,
}

impl ObjectType {
    /// Return whether this object type declares call, construct, or index signatures.
    ///
    /// An index signature counts, since it makes the object type a keyed view over concrete
    /// objects, and mapped reduction produces one for `Record<K, V>`.
    pub fn declares_signatures(&self) -> bool {
        !self.call_signatures.is_empty()
            || !self.construct_signatures.is_empty()
            || !self.index_signatures.is_empty()
    }
}

/// One property in a structural object type.
///
/// Examples:
/// ```ds
/// {
///     readonly name: string;
///     get value(): string;
///     set value(input: string | number);
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct TypeProperty {
    /// The property key.
    pub key: StaticKey,
    /// The supported property operations and their value types.
    pub access: PropertyAccess,
    /// Whether the property may be absent.
    pub is_optional: bool,
}

impl TypeProperty {
    /// Compose complementary operations from another property.
    pub fn compose(&mut self, other: TypeProperty) -> bool {
        if self.key != other.key {
            return false;
        }
        let Some(access) = self.access.composed(other.access) else {
            return false;
        };

        self.access = access;
        self.is_optional &= other.is_optional;

        true
    }
}

/// The value types exposed by one structural property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub enum PropertyAccess {
    /// A readable property.
    Read(GlobalTypeId),
    /// A writable property.
    Write(GlobalTypeId),
    /// A readable and writable property.
    ReadWrite {
        /// The value type produced by a read.
        read: GlobalTypeId,
        /// The value type accepted by a write.
        write: GlobalTypeId,
    },
}

impl PropertyAccess {
    /// Return the value type produced by a read.
    pub fn read(self) -> Option<GlobalTypeId> {
        match self {
            Self::Read(ty) | Self::ReadWrite { read: ty, .. } => Some(ty),
            Self::Write(_) => None,
        }
    }

    /// Return the value type accepted by a write.
    pub fn write(self) -> Option<GlobalTypeId> {
        match self {
            Self::Write(ty) | Self::ReadWrite { write: ty, .. } => Some(ty),
            Self::Read(_) => None,
        }
    }

    /// Return whether the property supports reads.
    pub fn is_readable(self) -> bool {
        !matches!(self, Self::Write(_))
    }

    /// Return whether the property supports writes.
    pub fn is_writable(self) -> bool {
        !matches!(self, Self::Read(_))
    }

    /// Return the value type one construction write accepts.
    pub fn store(self) -> GlobalTypeId {
        match self {
            Self::Write(ty) | Self::ReadWrite { write: ty, .. } | Self::Read(ty) => ty,
        }
    }

    /// Iterate every value type this property exposes.
    pub fn types(self) -> impl Iterator<Item = GlobalTypeId> {
        let (read, write) = match self {
            Self::Read(ty) => (Some(ty), None),
            Self::Write(ty) => (None, Some(ty)),
            Self::ReadWrite { read, write } => (Some(read), Some(write)),
        };

        read.into_iter().chain(write)
    }

    /// Restrict this property to reads where possible.
    pub fn readonly(self) -> PropertyAccess {
        match self {
            Self::ReadWrite { read, .. } => Self::Read(read),
            access => access,
        }
    }

    /// Combine complementary property operations, rejecting overlapping operations.
    pub fn composed(self, other: PropertyAccess) -> Option<PropertyAccess> {
        let overlaps = self.read().is_some() && other.read().is_some()
            || self.write().is_some() && other.write().is_some();
        if overlaps {
            return None;
        }

        Some(self.merged(other))
    }

    /// Merge one key's accessor operations into a single access.
    pub fn merged(self, other: PropertyAccess) -> PropertyAccess {
        match (other.read().or(self.read()), other.write().or(self.write())) {
            (Some(read), Some(write)) => Self::ReadWrite { read, write },
            (Some(read), None) => Self::Read(read),
            (None, Some(write)) => Self::Write(write),
            (None, None) => self,
        }
    }
}

/// An index signature in an object type.
///
/// Examples:
/// ```ds
/// { [key: string]: int32 }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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
    /// The generic arguments fixed by explicit specialization.
    pub arguments: TypeListId,
    /// The optional `this` parameter type.
    pub this_parameter: Option<GlobalTypeId>,
    /// The runtime parameter list.
    pub parameters: TypeListId,
    /// The optional return type.
    pub return_type: Option<GlobalTypeId>,
    /// Whether this is a generator function.
    pub is_generator: bool,
    /// Whether this signature constructs its return type.
    pub is_construct: bool,
    /// Whether a call to this signature parks the current fiber.
    pub parks: bool,
}

/// A runtime parameter in a function type.
///
/// Examples:
/// ```ds
/// (value?: int32, ...rest: string[]) => void
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct FunctionParameterType {
    /// The authored parameter name, when the parameter declares one.
    pub name: Option<StringId>,
    /// The parameter type.
    pub ty: GlobalTypeId,
    /// Whether the parameter may be omitted at the call site.
    pub is_optional: bool,
    /// Whether the parameter captures remaining call arguments.
    pub is_rest: bool,
}

/// A fat callable value with one assignable function signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct FunctionType {
    /// The function signature.
    pub signature: GlobalTypeId,
    /// The mode a call takes the callable in, a receiver mode literal or an open access term.
    pub receiver: GlobalTypeId,
    /// The place of the captured environment.
    pub place: GlobalTypeId,
}

/// The ownership, access, and exclusion required to call a receiver.
///
/// Examples:
/// ```ds
/// (x: T) => R                     // mutable aliasable receiver when elided
/// (&readonly this, x: T) => R     // borrowed readonly
/// ^Function<(), void, "once">     // owned, callable once
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ReceiverMode {
    /// The receiver is borrowed with the required access.
    Borrowed {
        /// The access required to invoke the callable.
        access: Access,
    },
    /// The receiver is taken by value.
    Owned,
}

impl ReceiverMode {
    /// The mode an elided receiver takes.
    pub const ELIDED: Self = Self::Borrowed {
        access: Access::Mutable,
    };

    /// Parse one canonical receiver mode name.
    pub fn from_text(value: &str) -> Option<Self> {
        if value == "once" {
            Some(Self::Owned)
        } else {
            Access::from_text(value).map(|access| Self::Borrowed { access })
        }
    }

    /// Return whether this mode is the elided default.
    pub fn is_elided(self) -> bool {
        self == Self::ELIDED
    }

    /// Return whether this mode grants the requested mode.
    pub fn grants(self, requested: Self) -> bool {
        match (self, requested) {
            (Self::Owned, _) => true,
            (Self::Borrowed { .. }, Self::Owned) => false,
            (Self::Borrowed { access }, Self::Borrowed { access: requested }) => {
                access.grants(requested)
            }
        }
    }

    /// Return the access required by a borrowed receiver.
    pub fn access(self) -> Option<Access> {
        match self {
            Self::Borrowed { access } => Some(access),
            Self::Owned => None,
        }
    }

    /// Return the canonical text of this receiver mode.
    pub const fn text(self) -> &'static str {
        match self {
            Self::Borrowed { access } => access.text(),
            Self::Owned => "once",
        }
    }
}

/// A thin callable value with no captured environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
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

// lock the hot table shapes: one cache line per type, packed forms
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<Type>() <= 96);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<Form>() <= 32);
