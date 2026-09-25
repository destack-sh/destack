use serde::{Deserialize, Serialize};
use tspp_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
    StringId,
};
use tspp_mir::{Access, FloatType, Space};
use tspp_serde::Reflect;
use tspp_source as source;

use super::{BindingId, EntryPoint, FunctionId, LayoutId, TypeId};

/// Reflected view of one program.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ProgramInfo {
    /// Reflected modules in this program.
    modules: SectionSlice<ModuleInfo>,
    /// Reflected types keyed by program type id.
    types: SectionSlice<TypeInfo>,
    /// Reflected functions keyed by program function id.
    functions: SectionSlice<FunctionInfo>,
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

/// Build-time reflected program table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProgramInfoBuilder {
    /// Reflected modules.
    modules: Vec<ModuleInfo>,
    /// Reflected types.
    types: Vec<TypeInfoBuilder>,
    /// Reflected functions.
    functions: Vec<FunctionInfo>,
    /// Reflected frame layouts.
    frames: Vec<FrameInfoBuilder>,
    /// Reflected globals.
    globals: Vec<GlobalInfo>,
    /// Reflected entrypoints.
    entries: Vec<EntryInfo>,
}

impl ProgramInfoBuilder {
    /// Create an empty reflected program builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set reflected modules.
    pub fn modules(mut self, modules: impl IntoIterator<Item = ModuleInfo>) -> Self {
        self.modules = modules.into_iter().collect();

        self
    }

    /// Set reflected types.
    pub fn types(mut self, types: impl IntoIterator<Item = TypeInfoBuilder>) -> Self {
        self.types = types.into_iter().collect();

        self
    }

    /// Set reflected functions.
    pub fn functions(mut self, functions: impl IntoIterator<Item = FunctionInfo>) -> Self {
        self.functions = functions.into_iter().collect();

        self
    }

    /// Set reflected frame layouts.
    pub fn frames(mut self, frames: impl IntoIterator<Item = FrameInfoBuilder>) -> Self {
        self.frames = frames.into_iter().collect();

        self
    }

    /// Set reflected globals.
    pub fn globals(mut self, globals: impl IntoIterator<Item = GlobalInfo>) -> Self {
        self.globals = globals.into_iter().collect();

        self
    }

    /// Set reflected entrypoints.
    pub fn entries(mut self, entries: impl IntoIterator<Item = EntryInfo>) -> Self {
        self.entries = entries.into_iter().collect();

        self
    }

    /// Build this reflection table into program sections.
    pub(crate) fn build(self, sections: &mut SectionBuilder) -> ProgramInfo {
        let Self {
            modules,
            types,
            functions,
            frames,
            globals,
            entries,
        } = self;
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

        ProgramInfo {
            modules: sections.insert(modules),
            types: sections.insert(types),
            functions: sections.insert(functions),
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
}

impl ProgramInfo {
    /// Return reflected modules.
    pub fn modules<'a>(&self, sections: SectionImage<'a>) -> &'a [ModuleInfo] {
        sections.entries(self.modules)
    }

    /// Return reflected types.
    pub fn types<'a>(&self, sections: SectionImage<'a>) -> &'a [TypeInfo] {
        sections.entries(self.types)
    }

    /// Return reflected functions.
    pub fn functions<'a>(&self, sections: SectionImage<'a>) -> &'a [FunctionInfo] {
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

    /// Return whether every reflected payload range fits its flattened column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let type_operands = sections.entries(self.type_operands).len();
        let fields = sections.entries(self.fields).len();
        let elements = sections.entries(self.tuple_elements).len();
        let members = sections.entries(self.members);
        let index_signatures = sections.entries(self.index_signatures).len();
        let parameters = sections.entries(self.function_parameters).len();
        let cases = sections.entries(self.variant_cases).len();
        let frame_slots = sections.entries(self.frame_slots).len();

        // check every variable type payload
        let types_fit = sections.entries(self.types).iter().all(|ty| {
            let payload = ty.payload;

            payload.type_operands.fits(type_operands)
                && payload.fields.fits(fields)
                && payload.elements.fits(elements)
                && payload.members.fits(members.len())
                && payload.index_signatures.fits(index_signatures)
                && payload.function_parameters.fits(parameters)
                && payload.variant_cases.fits(cases)
        });
        if !types_fit {
            return false;
        }

        // check ranges nested inside reflected member and frame entries
        let members_fit = members
            .iter()
            .all(|member| member.arguments.fits(type_operands));
        let frames_fit = sections
            .entries(self.frames)
            .iter()
            .all(|frame| frame.slots.fits(frame_slots));

        members_fit && frames_fit
    }
}

/// Build-time reflected type entry.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfoBuilder {
    /// Type display name when one exists.
    name: Option<StringId>,
    /// Source module containing the type when one exists.
    module: Option<source::ModuleId>,
    /// Transparent program representation.
    representation: TypeId,
    /// Concrete program layout when one exists.
    layout: Option<LayoutId>,
    /// Normalized reflected program type tag.
    tag: TypeTag,
    /// Normalized reflected program type payload.
    payload: TypePayloadBuilder,
}

impl TypeInfoBuilder {
    /// Create one reflected type builder.
    pub fn new(representation: TypeId, tag: TypeTag, payload: TypePayloadBuilder) -> Self {
        Self {
            name: None,
            module: None,
            representation,
            layout: None,
            tag,
            payload,
        }
    }

    /// Set the type display name.
    pub fn name(mut self, name: StringId) -> Self {
        self.name = Some(name);

        self
    }

    /// Set the source module containing this type.
    pub fn module(mut self, module: source::ModuleId) -> Self {
        self.module = Some(module);

        self
    }

    /// Set the concrete program layout.
    pub fn layout(mut self, layout: LayoutId) -> Self {
        self.layout = Some(layout);

        self
    }

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ModuleId {
    /// Stable source package id.
    pub package: u64,
    /// Stable source module key.
    pub module: u64,
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct TypePayload {
    /// Primitive payload.
    pub primitive: PrimitiveType,
    /// Literal payload.
    pub literal: Literal,
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
    pub literal: Literal,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
}

/// Reflected scalar literal program type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Literal {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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

/// Reflected placement singleton.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Place {
    /// Placement tag.
    pub tag: PlaceTag,
    /// Concrete storage space when present.
    pub space: Space,
}

/// Reflected placement tag.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum PlaceTag {
    /// Ambient placement.
    Ambient,
    /// Concrete storage space.
    Space,
}

/// Reflected lifetime singleton.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Lifetime {
    /// Lifetime tag.
    pub tag: LifetimeTag,
    /// Symbolic lifetime name when present.
    pub symbol: Optional<StringId>,
}

/// Reflected lifetime tag.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct DynamicType {
    /// Erased constraint type.
    pub constraint: TypeId,
}

/// Reflected homogeneous array type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ArrayType {
    /// Element type.
    pub element: TypeId,
}

/// Reflected fixed-length array type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FixedArrayType {
    /// Element type.
    pub element: TypeId,
    /// Static count type.
    pub count: TypeId,
}

/// Reflected scalar interval type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct RangeType {
    /// Inclusive lower bound.
    pub start: Optional<Literal>,
    /// Upper bound.
    pub end: Optional<Literal>,
    /// Whether the upper bound is included.
    pub is_inclusive: u32,
}

/// Reflected runtime-length homogeneous view type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct SliceType {
    /// Element type.
    pub element: TypeId,
}

/// Reflected fat callable value type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FunctionType {
    /// Function signature type.
    pub signature: TypeId,
    /// Captured environment type.
    pub environment: TypeId,
}

/// Reflected thin callable value type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FunctionPointerType {
    /// Function signature type.
    pub signature: TypeId,
}

/// Reflected transparent nominal type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct NewtypeType {
    /// Backing type.
    pub backing: TypeId,
}

/// Reflected static object key.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum StaticKeyTag {
    /// String-like object key.
    Name,
    /// Number-like object key.
    Index,
}

/// Reflected object-like type field.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct VariantCase {
    /// Case name when one exists.
    pub name: Optional<StringId>,
    /// Case payload type.
    pub ty: TypeId,
}

/// Reflected program function.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FunctionInfo {
    /// Function display name.
    pub name: StringId,
    /// Source module containing the function.
    pub module: ModuleId,
    /// Function signature type.
    pub signature: TypeId,
    /// Runtime binding attached to this function when one exists.
    pub binding: Optional<BindingId>,
}

impl FunctionInfo {
    /// Create one reflected function entry.
    pub fn new(name: StringId, module: source::ModuleId, signature: TypeId) -> Self {
        Self {
            name,
            module: module.into(),
            signature,
            binding: Optional::none(),
        }
    }

    /// Create one reflected runtime binding function entry.
    pub fn binding(
        name: StringId,
        module: source::ModuleId,
        signature: TypeId,
        binding: BindingId,
    ) -> Self {
        Self {
            name,
            module: module.into(),
            signature,
            binding: Optional::some(binding),
        }
    }
}

/// Build-time reflected program frame layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameInfoBuilder {
    /// Function owning this frame.
    function: FunctionId,
    /// Frame slots.
    slots: Vec<FrameSlotInfo>,
}

impl FrameInfoBuilder {
    /// Create one reflected frame layout builder.
    pub fn new(function: FunctionId) -> Self {
        Self {
            function,
            slots: Vec::new(),
        }
    }

    /// Set reflected frame slots.
    pub fn slots(mut self, slots: impl IntoIterator<Item = FrameSlotInfo>) -> Self {
        self.slots = slots.into_iter().collect();

        self
    }
}

/// Reflected program frame layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameInfo {
    /// Function owning this frame.
    pub function: FunctionId,
    /// Frame slots.
    pub slots: EntryRange<FrameSlotInfo>,
}

/// Reflected program frame slot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct EntryInfo {
    /// Entry point.
    pub entry: EntryPoint,
}
