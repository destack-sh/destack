use destack_core::{
    EntryRange, EntryStore, Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice,
    StringId,
};
use destack_mir::{Access, FloatType};
use destack_serde::Reflect;
use destack_source as source;
use serde::{Deserialize, Serialize};

use super::{EntryPoint, FunctionId, LayoutId, TypeId};

/// Reflected view of one program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ProgramInfo {
    /// Reflected modules in this program.
    modules: SectionSlice<ModuleInfo>,
    /// Reflected types keyed by program type id.
    types: SectionSlice<TypeInfo>,
    /// Reflected functions keyed by program function id.
    functions: SectionSlice<Optional<FunctionInfo>>,
    /// Reflected frame layouts keyed by frame layout id.
    frames: SectionSlice<FrameInfo>,
    /// Reflected globals keyed by program global id.
    globals: SectionSlice<GlobalInfo>,
    /// Reflected entrypoints.
    entries: SectionSlice<EntryInfo>,
    /// Flattened child type operands used by variable-arity type payloads.
    type_operands: SectionSlice<TypeId>,
    /// Flattened object-like fields.
    fields: SectionSlice<TypeField>,
    /// Flattened tuple elements.
    tuple_elements: SectionSlice<TypeElement>,
    /// Flattened reflected members.
    members: SectionSlice<MemberType>,
    /// Flattened reflected index signatures.
    index_signatures: SectionSlice<TypeIndexSignature>,
    /// Flattened reflected function parameters.
    function_parameters: SectionSlice<FunctionParameterType>,
    /// Flattened reflected variant cases.
    variant_cases: SectionSlice<VariantCase>,
    /// Flattened reflected frame slots.
    frame_slots: SectionSlice<FrameSlotInfo>,
}

impl ProgramInfo {
    /// Pack one reflected program view.
    pub fn pack(
        sections: &mut SectionPacker,
        modules: Vec<ModuleInfo>,
        types: Vec<TypeInfoBuilder>,
        functions: Vec<Option<FunctionInfo>>,
        frames: Vec<FrameInfoBuilder>,
        globals: Vec<GlobalInfo>,
        entries: Vec<EntryInfo>,
    ) -> Self {
        let mut type_operands = EntryStore::new();
        let mut fields = EntryStore::new();
        let mut tuple_elements = EntryStore::new();
        let mut members = EntryStore::new();
        let mut index_signatures = EntryStore::new();
        let mut function_parameters = EntryStore::new();
        let mut variant_cases = EntryStore::new();
        let mut frame_slots = EntryStore::new();

        // flatten reflected type payloads
        let types = types
            .into_iter()
            .map(|ty| {
                ty.build(
                    &mut type_operands,
                    &mut fields,
                    &mut tuple_elements,
                    &mut members,
                    &mut index_signatures,
                    &mut function_parameters,
                    &mut variant_cases,
                )
            })
            .collect::<Vec<_>>();

        // flatten reflected frame payloads
        let frames = frames
            .into_iter()
            .map(|frame| {
                let slots = frame_slots.append(frame.slots);

                FrameInfo {
                    function: frame.function,
                    slots,
                }
            })
            .collect::<Vec<_>>();

        Self {
            modules: sections.insert(modules),
            types: sections.insert(types),
            functions: sections.insert(functions.into_iter().map(Into::into).collect::<Vec<_>>()),
            frames: sections.insert(frames),
            globals: sections.insert(globals),
            entries: sections.insert(entries),
            type_operands: sections.insert(type_operands.into_entries()),
            fields: sections.insert(fields.into_entries()),
            tuple_elements: sections.insert(tuple_elements.into_entries()),
            members: sections.insert(members.into_entries()),
            index_signatures: sections.insert(index_signatures.into_entries()),
            function_parameters: sections.insert(function_parameters.into_entries()),
            variant_cases: sections.insert(variant_cases.into_entries()),
            frame_slots: sections.insert(frame_slots.into_entries()),
        }
    }

    /// Return reflected modules.
    pub fn modules<'a>(&self, sections: SectionImage<'a>) -> &'a [ModuleInfo] {
        sections.entries(self.modules)
    }

    /// Return reflected types.
    pub fn types<'a>(&self, sections: SectionImage<'a>) -> &'a [TypeInfo] {
        sections.entries(self.types)
    }

    /// Return reflected functions.
    pub fn functions<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<FunctionInfo>] {
        sections.entries(self.functions)
    }

    /// Return reflected frames.
    pub fn frames<'a>(&self, sections: SectionImage<'a>) -> &'a [FrameInfo] {
        sections.entries(self.frames)
    }

    /// Return reflected globals.
    pub fn globals<'a>(&self, sections: SectionImage<'a>) -> &'a [GlobalInfo] {
        sections.entries(self.globals)
    }

    /// Return reflected entrypoints.
    pub fn entries<'a>(&self, sections: SectionImage<'a>) -> &'a [EntryInfo] {
        sections.entries(self.entries)
    }

    /// Return one type payload operand range.
    pub fn type_operands<'a>(
        &self,
        sections: SectionImage<'a>,
        range: EntryRange<TypeId>,
    ) -> &'a [TypeId] {
        range.slice(sections.entries(self.type_operands))
    }

    /// Return one type payload field range.
    pub fn fields<'a>(
        &self,
        sections: SectionImage<'a>,
        range: EntryRange<TypeField>,
    ) -> &'a [TypeField] {
        range.slice(sections.entries(self.fields))
    }

    /// Return one type payload element range.
    pub fn tuple_elements<'a>(
        &self,
        sections: SectionImage<'a>,
        range: EntryRange<TypeElement>,
    ) -> &'a [TypeElement] {
        range.slice(sections.entries(self.tuple_elements))
    }

    /// Return one type payload member range.
    pub fn members<'a>(
        &self,
        sections: SectionImage<'a>,
        range: EntryRange<MemberType>,
    ) -> &'a [MemberType] {
        range.slice(sections.entries(self.members))
    }

    /// Return one type payload index-signature range.
    pub fn index_signatures<'a>(
        &self,
        sections: SectionImage<'a>,
        range: EntryRange<TypeIndexSignature>,
    ) -> &'a [TypeIndexSignature] {
        range.slice(sections.entries(self.index_signatures))
    }

    /// Return one type payload parameter range.
    pub fn function_parameters<'a>(
        &self,
        sections: SectionImage<'a>,
        range: EntryRange<FunctionParameterType>,
    ) -> &'a [FunctionParameterType] {
        range.slice(sections.entries(self.function_parameters))
    }

    /// Return one type payload variant-case range.
    pub fn variant_cases<'a>(
        &self,
        sections: SectionImage<'a>,
        range: EntryRange<VariantCase>,
    ) -> &'a [VariantCase] {
        range.slice(sections.entries(self.variant_cases))
    }

    /// Return one frame slot range.
    pub fn frame_slots<'a>(
        &self,
        sections: SectionImage<'a>,
        frame: &FrameInfo,
    ) -> &'a [FrameSlotInfo] {
        frame.slots.slice(sections.entries(self.frame_slots))
    }
}

/// Build-time reflected type entry.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfoBuilder {
    /// Type display name when one exists.
    pub name: Option<StringId>,
    /// Source module containing the type when one exists.
    pub module: Option<source::ModuleId>,
    /// Transparent program representation.
    pub representation: TypeId,
    /// Concrete program layout when one exists.
    pub layout: Option<LayoutId>,
    /// Normalized reflected program type tag.
    pub tag: TypeTag,
    /// Normalized reflected program type payload.
    pub payload: TypePayloadBuilder,
}

impl TypeInfoBuilder {
    /// Build one section entry.
    fn build(
        self,
        type_operands: &mut EntryStore<TypeId>,
        fields: &mut EntryStore<TypeField>,
        tuple_elements: &mut EntryStore<TypeElement>,
        members: &mut EntryStore<MemberType>,
        index_signatures: &mut EntryStore<TypeIndexSignature>,
        function_parameters: &mut EntryStore<FunctionParameterType>,
        variant_cases: &mut EntryStore<VariantCase>,
    ) -> TypeInfo {
        TypeInfo {
            name: self.name.into(),
            module: self.module.map(Into::into).into(),
            representation: self.representation,
            layout: self.layout.into(),
            tag: self.tag,
            payload: self.payload.build(
                type_operands,
                fields,
                tuple_elements,
                members,
                index_signatures,
                function_parameters,
                variant_cases,
            ),
        }
    }
}

/// Reflected module in one program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ModuleInfo {
    /// Source module id.
    pub id: ModuleId,
    /// Module display name.
    pub name: StringId,
}

impl ModuleInfo {
    /// Create one reflected module entry.
    pub fn new(id: source::ModuleId, name: StringId) -> Self {
        Self {
            id: id.into(),
            name,
        }
    }
}

/// Section-safe source module id.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ModuleId {
    /// Stable source package id.
    pub package: u128,
    /// Stable source module key.
    pub module: u128,
}

impl From<source::ModuleId> for ModuleId {
    /// Convert one source module id.
    fn from(id: source::ModuleId) -> Self {
        Self {
            package: id.package_id.0,
            module: id.module_key.0,
        }
    }
}

/// Reflected type in one program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeInfo {
    /// Type display name when one exists.
    pub name: Optional<StringId>,
    /// Source module containing the type when one exists.
    pub module: Optional<ModuleId>,
    /// Transparent program representation.
    pub representation: TypeId,
    /// Concrete program layout when one exists.
    pub layout: Optional<LayoutId>,
    /// Normalized reflected program type tag.
    pub tag: TypeTag,
    /// Normalized reflected program type payload.
    pub payload: TypePayload,
}

/// Normalized program type tag visible through program reflection.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum TypeTag {
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
    Primitive,
    /// Scalar literal type.
    Literal,
    /// Normalized memory singleton type.
    Memory,
    /// Declaration reference with applied type arguments.
    Reference,
    /// Member type selected from an owner type.
    Member,
    /// Canonical ownership or access form.
    Form,
    /// Explicit erased dynamic value.
    Dynamic,
    /// Homogeneous dynamic-length array.
    Array,
    /// Fixed-length array.
    FixedArray,
    /// Compact scalar interval.
    Range,
    /// Runtime-length homogeneous view.
    Slice,
    /// Tuple value.
    Tuple,
    /// Structural object shape.
    Shape,
    /// Callable signature.
    FunctionSignature,
    /// Fat callable value with captured environment.
    Function,
    /// Thin callable value.
    FunctionPointer,
    /// Union type.
    Union,
    /// Intersection type.
    Intersection,
    /// Transparent nominal type.
    Newtype,
    /// Struct declaration type.
    Struct,
    /// Class declaration type.
    Class,
    /// Interface declaration type.
    Interface,
    /// Variant declaration type.
    Variant,
}

/// Normalized program type payload.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypePayload {
    /// Primitive payload.
    pub primitive: PrimitiveType,
    /// Literal payload.
    pub literal: ScalarLiteral,
    /// Memory singleton payload.
    pub memory: MemoryLiteral,
    /// Type-form payload.
    pub form: FormType,
    /// Dynamic payload.
    pub dynamic: DynamicType,
    /// Array payload.
    pub array: ArrayType,
    /// Fixed array payload.
    pub fixed_array: FixedArrayType,
    /// Range payload.
    pub range: RangeType,
    /// Slice payload.
    pub slice: SliceType,
    /// Callable value payload.
    pub function: FunctionType,
    /// Thin function pointer payload.
    pub function_pointer: FunctionPointerType,
    /// Newtype payload.
    pub newtype: NewtypeType,
    /// Reference or member name.
    pub name: Optional<StringId>,
    /// Primary type id payload.
    pub primary_type: Optional<TypeId>,
    /// Secondary type id payload.
    pub secondary_type: Optional<TypeId>,
    /// Generic arguments, union elements, or intersection elements.
    pub type_operands: EntryRange<TypeId>,
    /// Object-like fields.
    pub fields: EntryRange<TypeField>,
    /// Tuple elements.
    pub elements: EntryRange<TypeElement>,
    /// Class or interface members.
    pub members: EntryRange<MemberType>,
    /// Object-like index signatures.
    pub index_signatures: EntryRange<TypeIndexSignature>,
    /// Function signature parameters.
    pub function_parameters: EntryRange<FunctionParameterType>,
    /// Variant cases.
    pub variant_cases: EntryRange<VariantCase>,
}

/// Build-time normalized program type payload.
#[derive(Debug, Clone, PartialEq)]
pub struct TypePayloadBuilder {
    /// Primitive payload.
    pub primitive: PrimitiveType,
    /// Literal payload.
    pub literal: ScalarLiteral,
    /// Memory singleton payload.
    pub memory: MemoryLiteral,
    /// Type-form payload.
    pub form: FormType,
    /// Dynamic payload.
    pub dynamic: DynamicType,
    /// Array payload.
    pub array: ArrayType,
    /// Fixed array payload.
    pub fixed_array: FixedArrayType,
    /// Range payload.
    pub range: RangeType,
    /// Slice payload.
    pub slice: SliceType,
    /// Callable value payload.
    pub function: FunctionType,
    /// Thin function pointer payload.
    pub function_pointer: FunctionPointerType,
    /// Newtype payload.
    pub newtype: NewtypeType,
    /// Reference or member name.
    pub name: Option<StringId>,
    /// Primary type id payload.
    pub primary_type: Option<TypeId>,
    /// Secondary type id payload.
    pub secondary_type: Option<TypeId>,
    /// Generic arguments, union elements, or intersection elements.
    pub type_operands: Vec<TypeId>,
    /// Object-like fields.
    pub fields: Vec<TypeField>,
    /// Tuple elements.
    pub elements: Vec<TypeElement>,
    /// Class or interface members.
    pub members: Vec<MemberType>,
    /// Object-like index signatures.
    pub index_signatures: Vec<TypeIndexSignature>,
    /// Function signature parameters.
    pub function_parameters: Vec<FunctionParameterType>,
    /// Variant cases.
    pub variant_cases: Vec<VariantCase>,
}

impl TypePayloadBuilder {
    /// Build one section entry.
    fn build(
        self,
        type_operands: &mut EntryStore<TypeId>,
        fields: &mut EntryStore<TypeField>,
        tuple_elements: &mut EntryStore<TypeElement>,
        members: &mut EntryStore<MemberType>,
        index_signatures: &mut EntryStore<TypeIndexSignature>,
        function_parameters: &mut EntryStore<FunctionParameterType>,
        variant_cases: &mut EntryStore<VariantCase>,
    ) -> TypePayload {
        TypePayload {
            primitive: self.primitive,
            literal: self.literal,
            memory: self.memory,
            form: self.form,
            dynamic: self.dynamic,
            array: self.array,
            fixed_array: self.fixed_array,
            range: self.range,
            slice: self.slice,
            function: self.function,
            function_pointer: self.function_pointer,
            newtype: self.newtype,
            name: self.name.into(),
            primary_type: self.primary_type.into(),
            secondary_type: self.secondary_type.into(),
            type_operands: type_operands.append(self.type_operands),
            fields: fields.append(self.fields),
            elements: tuple_elements.append(self.elements),
            members: members.append(self.members),
            index_signatures: index_signatures.append(self.index_signatures),
            function_parameters: function_parameters.append(self.function_parameters),
            variant_cases: variant_cases.append(self.variant_cases),
        }
    }
}

/// Reflected primitive program type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct PrimitiveType {
    /// Primitive tag.
    pub tag: PrimitiveTag,
    /// Integer bit width.
    pub width: u16,
    /// Whether the integer is signed.
    pub is_signed: u32,
    /// Float format.
    pub float: FloatType,
}

/// Reflected primitive tag.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum PrimitiveTag {
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
    Int,
    /// Floating-point primitive.
    Float,
    /// Symbol primitive.
    Symbol,
    /// Unique symbol primitive.
    UniqueSymbol,
}

/// Reflected scalar literal program type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ScalarLiteral {
    /// Literal tag.
    pub tag: ScalarLiteralTag,
    /// String literal value when present.
    pub string: Optional<StringId>,
    /// Integer or bigint literal value.
    pub integer: i64,
    /// Floating-point format.
    pub float: FloatType,
    /// Floating-point literal bits.
    pub bits: u128,
}

/// Reflected literal tag.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ScalarLiteralTag {
    /// Boolean false literal.
    False,
    /// Boolean true literal.
    True,
    /// Character literal.
    Character,
    /// String literal.
    String,
    /// Integer literal.
    Integer,
    /// Floating-point literal.
    Float,
    /// Bigint literal.
    Bigint,
}

/// Reflected memory singleton program type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemoryLiteral {
    /// Memory singleton tag.
    pub tag: MemoryLiteralTag,
    /// Reference access singleton.
    pub access: Access,
    /// Concrete storage space singleton.
    pub space: Space,
    /// Concrete placement singleton.
    pub place: Place,
    /// Lifetime singleton.
    pub lifetime: Lifetime,
}

/// Reflected memory singleton tag.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum MemoryLiteralTag {
    /// Reference access singleton.
    Access,
    /// Concrete storage space singleton.
    Space,
    /// Placement singleton.
    Place,
    /// Lifetime singleton.
    Lifetime,
}

/// Reflected storage space singleton.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Space {
    /// Local runtime storage.
    Local,
    /// Shared runtime storage.
    Shared,
    /// Frame-slot storage inside one activation.
    Frame,
    /// Static memory.
    Static,
}

/// Reflected placement singleton.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Place {
    /// Placement tag.
    pub tag: PlaceTag,
    /// Concrete storage space when present.
    pub space: Space,
}

/// Reflected placement tag.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum PlaceTag {
    /// Ambient placement.
    Ambient,
    /// Concrete storage space.
    Space,
}

/// Reflected lifetime singleton.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Lifetime {
    /// Lifetime tag.
    pub tag: LifetimeTag,
    /// Symbolic lifetime name when present.
    pub symbol: Optional<StringId>,
}

/// Reflected lifetime tag.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum LifetimeTag {
    /// Static storage lifetime.
    Static,
    /// The enclosing frame's lifetime.
    Frame,
    /// Symbolic lifetime parameter or associated constant.
    Symbol,
}

/// Reflected ownership or access form.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FormType {
    /// Form tag.
    pub tag: Form,
    /// Carried value type.
    pub value: TypeId,
    /// Borrow lifetime type.
    pub lifetime: Optional<TypeId>,
    /// Borrow access type.
    pub access: Optional<TypeId>,
    /// Placement type.
    pub place: Optional<TypeId>,
}

/// Reflected ownership or access form tag.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Form {
    /// Managed value.
    Managed,
    /// Owned value.
    Owned,
    /// Borrowed value.
    Borrowed,
    /// Raw pointer value.
    Raw,
    /// Placed value.
    Placed,
    /// Readonly view.
    Readonly,
}

/// Reflected erased dynamic value type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicType {
    /// Erased constraint type.
    pub constraint: TypeId,
}

/// Reflected homogeneous array type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ArrayType {
    /// Element type.
    pub element: TypeId,
}

/// Reflected fixed-length array type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FixedArrayType {
    /// Element type.
    pub element: TypeId,
    /// Static count type.
    pub count: TypeId,
}

/// Reflected scalar interval type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RangeType {
    /// Inclusive lower bound.
    pub start: Optional<ScalarLiteral>,
    /// Upper bound.
    pub end: Optional<ScalarLiteral>,
    /// Whether the upper bound is included.
    pub is_inclusive: u32,
}

/// Reflected runtime-length homogeneous view type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SliceType {
    /// Element type.
    pub element: TypeId,
}

/// Reflected fat callable value type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionType {
    /// Function signature type.
    pub signature: TypeId,
    /// Captured environment type.
    pub environment: TypeId,
}

/// Reflected thin callable value type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionPointerType {
    /// Function signature type.
    pub signature: TypeId,
}

/// Reflected transparent nominal type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct NewtypeType {
    /// Backing type.
    pub backing: TypeId,
}

/// Reflected static object key.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StaticKey {
    /// Static key tag.
    pub tag: StaticKeyTag,
    /// Name key when present.
    pub name: Optional<StringId>,
    /// Positional index key when present.
    pub index: u64,
}

impl StaticKey {
    /// Create one name key.
    pub fn name(name: StringId) -> Self {
        Self {
            tag: StaticKeyTag::Name,
            name: Some(name).into(),
            index: 0,
        }
    }

    /// Create one index key.
    pub fn index(index: u64) -> Self {
        Self {
            tag: StaticKeyTag::Index,
            name: Optional::none(),
            index,
        }
    }
}

/// Reflected static object key tag.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum StaticKeyTag {
    /// String-like object key.
    Name,
    /// Number-like object key.
    Index,
}

/// Reflected object-like type field.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypeField {
    /// Field key.
    pub key: StaticKey,
    /// Field type.
    pub ty: TypeId,
    /// Whether the field is optional.
    pub is_optional: u32,
    /// Whether the field is readonly.
    pub is_readonly: u32,
}

/// Reflected tuple element.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypeElement {
    /// Element label when one exists.
    pub label: Optional<StringId>,
    /// Element type.
    pub ty: TypeId,
    /// Whether the element is optional.
    pub is_optional: u32,
    /// Whether the element is readonly.
    pub is_readonly: u32,
    /// Whether the element captures remaining tuple elements.
    pub is_rest: u32,
}

/// Reflected index signature in an object-like type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypeIndexSignature {
    /// Parameter name.
    pub name: StringId,
    /// Key type.
    pub key_type: TypeId,
    /// Value type.
    pub value_type: TypeId,
    /// Whether the index signature is optional.
    pub is_optional: u32,
    /// Whether the index signature is readonly.
    pub is_readonly: u32,
}

/// Reflected class or interface member.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberType {
    /// Owner type.
    pub owner: TypeId,
    /// Selected member key.
    pub key: StaticKey,
    /// Complete type arguments.
    pub arguments: EntryRange<TypeId>,
}

/// Reflected runtime parameter in a function type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionParameterType {
    /// Parameter type.
    pub ty: TypeId,
    /// Whether the parameter may be omitted.
    pub is_optional: u32,
    /// Whether the parameter captures remaining arguments.
    pub is_rest: u32,
}

/// Reflected variant case.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VariantCase {
    /// Case name when one exists.
    pub name: Optional<StringId>,
    /// Case payload type.
    pub ty: TypeId,
}

/// Reflected program function.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionInfo {
    /// Function display name.
    pub name: StringId,
    /// Source module containing the function.
    pub module: ModuleId,
    /// Function signature type.
    pub signature: TypeId,
}

impl FunctionInfo {
    /// Create one reflected function entry.
    pub fn new(name: StringId, module: source::ModuleId, signature: TypeId) -> Self {
        Self {
            name,
            module: module.into(),
            signature,
        }
    }
}

/// Build-time reflected program frame layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameInfoBuilder {
    /// Function owning this frame.
    pub function: FunctionId,
    /// Frame slots.
    pub slots: Vec<FrameSlotInfo>,
}

/// Reflected program frame layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameInfo {
    /// Function owning this frame.
    pub function: FunctionId,
    /// Frame slots.
    pub slots: EntryRange<FrameSlotInfo>,
}

/// Reflected program frame slot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameSlotInfo {
    /// Slot type.
    pub ty: TypeId,
    /// Slot byte offset.
    pub offset: u32,
    /// Slot byte length.
    pub byte_len: u32,
}

/// Reflected global in one program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GlobalInfo {
    /// Global display name when one exists.
    pub name: Optional<StringId>,
    /// Global type.
    pub ty: TypeId,
}

impl GlobalInfo {
    /// Create one reflected global entry.
    pub fn new(name: Option<StringId>, ty: TypeId) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

/// Reflected entry point.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EntryInfo {
    /// Entry point.
    pub entry: EntryPoint,
}

// SAFETY: program reflection entries contain only fixed-width ids, flags, and section ranges.
unsafe impl SectionEntry for ModuleId {}
unsafe impl SectionEntry for ModuleInfo {}
unsafe impl SectionEntry for TypeInfo {}
unsafe impl SectionEntry for TypeTag {}
unsafe impl SectionEntry for TypePayload {}
unsafe impl SectionEntry for PrimitiveType {}
unsafe impl SectionEntry for PrimitiveTag {}
unsafe impl SectionEntry for ScalarLiteral {}
unsafe impl SectionEntry for ScalarLiteralTag {}
unsafe impl SectionEntry for MemoryLiteral {}
unsafe impl SectionEntry for MemoryLiteralTag {}
unsafe impl SectionEntry for Space {}
unsafe impl SectionEntry for Place {}
unsafe impl SectionEntry for PlaceTag {}
unsafe impl SectionEntry for Lifetime {}
unsafe impl SectionEntry for LifetimeTag {}
unsafe impl SectionEntry for FormType {}
unsafe impl SectionEntry for Form {}
unsafe impl SectionEntry for DynamicType {}
unsafe impl SectionEntry for ArrayType {}
unsafe impl SectionEntry for FixedArrayType {}
unsafe impl SectionEntry for RangeType {}
unsafe impl SectionEntry for SliceType {}
unsafe impl SectionEntry for FunctionType {}
unsafe impl SectionEntry for FunctionPointerType {}
unsafe impl SectionEntry for NewtypeType {}
unsafe impl SectionEntry for StaticKey {}
unsafe impl SectionEntry for StaticKeyTag {}
unsafe impl SectionEntry for TypeField {}
unsafe impl SectionEntry for TypeElement {}
unsafe impl SectionEntry for TypeIndexSignature {}
unsafe impl SectionEntry for MemberType {}
unsafe impl SectionEntry for FunctionParameterType {}
unsafe impl SectionEntry for VariantCase {}
unsafe impl SectionEntry for FunctionInfo {}
unsafe impl SectionEntry for FrameInfo {}
unsafe impl SectionEntry for FrameSlotInfo {}
unsafe impl SectionEntry for GlobalInfo {}
unsafe impl SectionEntry for EntryInfo {}
