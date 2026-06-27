use destack_core::StringId;
use destack_mir::{Access, FloatType, Lifetime, Space};
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use super::{EntryPoint, FunctionId, LayoutId, TypeId};

/// Cold reflected view of one executable program.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ProgramInfo {
    /// Reflected modules in this program.
    pub modules: Vec<ModuleInfo>,
    /// Reflected types keyed by program type id.
    pub types: Vec<Option<TypeInfo>>,
    /// Reflected functions keyed by program function id.
    pub functions: Vec<Option<FunctionInfo>>,
    /// Reflected frame layouts keyed by frame layout id.
    pub frames: Vec<FrameInfo>,
    /// Reflected globals keyed by program global id.
    pub globals: Vec<GlobalInfo>,
    /// Reflected entry points.
    pub entries: Vec<EntryInfo>,
}

/// Reflected module in one executable program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ModuleInfo {
    /// Source module id when one exists.
    pub id: Option<ModuleId>,
    /// Module display name when one exists.
    pub name: Option<StringId>,
}

/// Reflected type in one executable program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeInfo {
    /// Type display name when one exists.
    pub name: Option<StringId>,
    /// Source module containing the type when one exists.
    pub module: Option<ModuleId>,
    /// Transparent executable representation.
    pub repr: TypeId,
    /// Concrete executable layout when one exists.
    pub layout: Option<LayoutId>,
    /// Normalized reflected program type.
    pub ty: ProgramType,
}

/// Normalized program type visible through program reflection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ProgramType {
    /// Never type.
    Never,
    /// Unknown type.
    Unknown,
    /// Void type.
    Void,
    /// Null singleton type.
    Null,
    /// Undefined singleton type.
    Undefined,
    /// Object constraint type.
    Object,
    /// Primitive type.
    Primitive(PrimitiveInfo),
    /// Scalar literal type.
    Literal(LiteralInfo),
    /// Normalized memory singleton type.
    Memory(MemoryInfo),
    /// Declaration reference with applied type arguments.
    Reference(GenericInfo),
    /// Member type selected from an owner type.
    Member(MemberInfo),
    /// Canonical ownership or access form.
    Form(FormInfo),
    /// Explicit erased dynamic value.
    Dynamic(DynamicInfo),
    /// Homogeneous dynamic-length array.
    Array(ArrayInfo),
    /// Fixed-length array.
    FixedArray(FixedArrayInfo),
    /// Compact scalar interval.
    Range(RangeInfo),
    /// Runtime-length homogeneous view.
    Slice(SliceInfo),
    /// Tuple value.
    Tuple(TupleInfo),
    /// Structural object shape.
    Shape(ShapeInfo),
    /// Callable signature.
    FunctionSignature(FunctionSignatureInfo),
    /// Fat callable value with captured environment.
    Function(FunctionTypeInfo),
    /// Thin callable value.
    FunctionPointer(FunctionPointerInfo),
    /// Union type.
    Union(UnionInfo),
    /// Intersection type.
    Intersection(IntersectionInfo),
    /// Transparent nominal type.
    Newtype(NewtypeInfo),
    /// Struct declaration type.
    Struct(StructInfo),
    /// Class declaration type.
    Class(ClassInfo),
    /// Interface declaration type.
    Interface(InterfaceInfo),
    /// Variant declaration type.
    Variant(VariantInfo),
}

/// Reflected primitive program type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum PrimitiveInfo {
    /// Boolean primitive.
    Boolean,
    /// Unicode scalar value primitive.
    Character,
    /// String primitive.
    String,
    /// Bigint primitive.
    Bigint,
    /// Number primitive.
    Number,
    /// Integer primitive.
    Int {
        /// Integer bit width.
        width: u16,
        /// Whether the integer is signed.
        is_signed: bool,
    },
    /// Floating-point primitive.
    Float {
        /// Float format.
        format: FloatType,
    },
    /// Symbol primitive.
    Symbol,
    /// Unique symbol primitive.
    UniqueSymbol,
}

/// Reflected scalar literal program type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum LiteralInfo {
    /// Boolean literal.
    Boolean(bool),
    /// Character literal.
    Character(char),
    /// String literal.
    String(StringId),
    /// Integer literal.
    Integer(i64),
    /// Floating-point literal bits.
    Float {
        /// Float format.
        format: FloatType,
        /// Float bits.
        bits: u128,
    },
    /// Bigint literal.
    Bigint(i64),
}

/// Reflected memory singleton program type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum MemoryInfo {
    /// Reference access singleton.
    Access(Access),
    /// Concrete storage space singleton.
    Space(Space),
    /// Lifetime singleton.
    Lifetime(Lifetime),
}

/// Reflected generic declaration application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GenericInfo {
    /// Declaration name when one exists.
    pub name: Option<StringId>,
    /// Complete type arguments.
    pub arguments: Vec<TypeId>,
}

/// Reflected member type selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberInfo {
    /// Owner type.
    pub owner: TypeId,
    /// Selected member name when one exists.
    pub name: Option<StringId>,
    /// Complete type arguments.
    pub arguments: Vec<TypeId>,
}

/// Reflected ownership or access form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FormInfo {
    /// Form constructor.
    pub form: Form,
    /// Carried value type.
    pub value: TypeId,
}

/// Reflected ownership or access form constructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Form {
    /// Managed value.
    Managed,
    /// Owned value.
    Owned,
    /// Borrowed value.
    Borrowed {
        /// Borrow lifetime type.
        lifetime: TypeId,
        /// Borrow access type.
        access: TypeId,
    },
    /// Raw pointer value.
    Raw,
    /// Placed value.
    Placed {
        /// Placement type.
        place: TypeId,
    },
    /// Readonly view.
    Readonly,
}

/// Reflected erased dynamic value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicInfo {
    /// Erased constraint type.
    pub constraint: TypeId,
}

/// Reflected homogeneous array type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ArrayInfo {
    /// Element type.
    pub element: TypeId,
}

/// Reflected fixed-length array type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FixedArrayInfo {
    /// Element type.
    pub element: TypeId,
    /// Static count type.
    pub count: TypeId,
}

/// Reflected scalar interval type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RangeInfo {
    /// Inclusive lower bound.
    pub start: Option<LiteralInfo>,
    /// Upper bound.
    pub end: Option<LiteralInfo>,
    /// Whether the upper bound is included.
    pub is_inclusive: bool,
}

/// Reflected runtime-length homogeneous view type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SliceInfo {
    /// Element type.
    pub element: TypeId,
}

/// Reflected tuple type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TupleInfo {
    /// Tuple elements.
    pub elements: Vec<TypeElementInfo>,
}

/// Reflected structural object shape type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ShapeInfo {
    /// Shape fields.
    pub fields: Vec<TypeFieldInfo>,
    /// Call signature types.
    pub call_signatures: Vec<TypeId>,
    /// Construct signature types.
    pub construct_signatures: Vec<TypeId>,
    /// Index signatures.
    pub index_signatures: Vec<TypeIndexSignatureInfo>,
}

/// Reflected function signature type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionSignatureInfo {
    /// Optional receiver type.
    pub this_parameter: Option<TypeId>,
    /// Runtime parameters.
    pub parameters: Vec<FunctionParameterInfo>,
    /// Optional return type.
    pub return_type: Option<TypeId>,
    /// Whether this signature is async.
    pub is_async: bool,
    /// Whether this signature is a generator.
    pub is_generator: bool,
}

/// Reflected runtime parameter in a function type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionParameterInfo {
    /// Parameter type.
    pub ty: TypeId,
    /// Whether the parameter may be omitted.
    pub is_optional: bool,
    /// Whether the parameter captures remaining arguments.
    pub is_rest: bool,
}

/// Reflected fat callable value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionTypeInfo {
    /// Function signature type.
    pub signature: TypeId,
    /// Captured environment type.
    pub environment: TypeId,
}

/// Reflected thin callable value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionPointerInfo {
    /// Function signature type.
    pub signature: TypeId,
}

/// Reflected union type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct UnionInfo {
    /// Union elements.
    pub elements: Vec<TypeId>,
}

/// Reflected intersection type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct IntersectionInfo {
    /// Intersection elements.
    pub elements: Vec<TypeId>,
}

/// Reflected transparent nominal type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct NewtypeInfo {
    /// Backing type.
    pub backing: TypeId,
}

/// Reflected struct declaration type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StructInfo {
    /// Struct fields.
    pub fields: Vec<TypeFieldInfo>,
}

/// Reflected class declaration type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ClassInfo {
    /// Class fields.
    pub fields: Vec<TypeFieldInfo>,
    /// Class members.
    pub members: Vec<MemberInfo>,
}

/// Reflected interface declaration type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct InterfaceInfo {
    /// Whether the interface is nominal.
    pub is_nominal: bool,
    /// Interface members.
    pub members: Vec<MemberInfo>,
}

/// Reflected variant declaration type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VariantInfo {
    /// Variant cases.
    pub cases: Vec<VariantCaseInfo>,
}

/// Reflected variant case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VariantCaseInfo {
    /// Case name when one exists.
    pub name: Option<StringId>,
    /// Case payload type.
    pub ty: TypeId,
}

/// Reflected object-like type field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypeFieldInfo {
    /// Field name when one exists.
    pub name: Option<StringId>,
    /// Field type.
    pub ty: TypeId,
    /// Whether the field is optional.
    pub is_optional: bool,
    /// Whether the field is readonly.
    pub is_readonly: bool,
}

/// Reflected tuple element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypeElementInfo {
    /// Element label when one exists.
    pub label: Option<StringId>,
    /// Element type.
    pub ty: TypeId,
    /// Whether the element is optional.
    pub is_optional: bool,
    /// Whether the element is readonly.
    pub is_readonly: bool,
    /// Whether the element captures remaining tuple elements.
    pub is_rest: bool,
}

/// Reflected index signature in an object-like type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypeIndexSignatureInfo {
    /// Key type.
    pub key_type: TypeId,
    /// Value type.
    pub value_type: TypeId,
    /// Whether the index signature is optional.
    pub is_optional: bool,
    /// Whether the index signature is readonly.
    pub is_readonly: bool,
}

/// Reflected executable function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionInfo {
    /// Function display name when one exists.
    pub name: Option<StringId>,
    /// Source module containing the function when one exists.
    pub module: Option<ModuleId>,
    /// Function signature type.
    pub signature: TypeId,
    /// Captured environment type when one exists.
    pub environment: Option<TypeId>,
}

/// Reflected executable frame layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameInfo {
    /// Function owning this frame.
    pub function: FunctionId,
    /// Frame slots.
    pub slots: Vec<FrameSlotInfo>,
}

/// Reflected executable frame slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameSlotInfo {
    /// Slot type.
    pub ty: TypeId,
    /// Slot byte offset.
    pub offset: u32,
    /// Slot byte length.
    pub byte_len: u32,
}

/// Reflected global in one executable program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GlobalInfo {
    /// Global display name when one exists.
    pub name: Option<StringId>,
    /// Global type.
    pub ty: TypeId,
}

/// Reflected entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EntryInfo {
    /// Entry point.
    pub entry: EntryPoint,
}
