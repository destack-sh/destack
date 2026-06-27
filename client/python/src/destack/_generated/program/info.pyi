# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.mir.tree.lifetime
import destack._generated.mir.tree.type
import destack._generated.program.entry
import destack._generated.program.function
import destack._generated.program.layout
import destack._generated.program.type
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ProgramInfo:
    """Cold reflected view of one executable program."""

    # reflected modules in this program
    modules: Sequence[ModuleInfo]
    # reflected types keyed by program type id
    types: Sequence[TypeInfo | None]
    # reflected functions keyed by program function id
    functions: Sequence[FunctionInfo | None]
    # reflected frame layouts keyed by frame layout id
    frames: Sequence[FrameInfo]
    # reflected globals keyed by program global id
    globals: Sequence[GlobalInfo]
    # reflected entry points
    entries: Sequence[EntryInfo]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProgramInfo: ...

def encode_program_info(writer: BinaryWriter, value: ProgramInfo) -> None: ...
def decode_program_info(reader: BinaryReader) -> ProgramInfo: ...
def to_json_program_info(value: ProgramInfo) -> Json: ...
def from_json_program_info(value: Json) -> ProgramInfo: ...

@dataclass(frozen=True, slots=True)
class ModuleInfo:
    """Reflected module in one executable program."""

    # source module id when one exists
    id: destack._generated.source.file.model.module.ModuleId | None
    # module display name when one exists
    name: destack._generated.core.string.StringId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ModuleInfo: ...

def encode_module_info(writer: BinaryWriter, value: ModuleInfo) -> None: ...
def decode_module_info(reader: BinaryReader) -> ModuleInfo: ...
def to_json_module_info(value: ModuleInfo) -> Json: ...
def from_json_module_info(value: Json) -> ModuleInfo: ...

@dataclass(frozen=True, slots=True)
class TypeInfo:
    """Reflected type in one executable program."""

    # type display name when one exists
    name: destack._generated.core.string.StringId | None
    # source module containing the type when one exists
    module: destack._generated.source.file.model.module.ModuleId | None
    # transparent executable representation
    repr: destack._generated.program.type.TypeId
    # concrete executable layout when one exists
    layout: destack._generated.program.layout.LayoutId | None
    # normalized reflected program type
    ty: ProgramType

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeInfo: ...

def encode_type_info(writer: BinaryWriter, value: TypeInfo) -> None: ...
def decode_type_info(reader: BinaryReader) -> TypeInfo: ...
def to_json_type_info(value: TypeInfo) -> Json: ...
def from_json_type_info(value: Json) -> TypeInfo: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeNever:
    """Never type."""

    kind: typing.Literal["never"] = "never"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeUnknown:
    """Unknown type."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeVoid:
    """Void type."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeNull:
    """Null singleton type."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeUndefined:
    """Undefined singleton type."""

    kind: typing.Literal["undefined"] = "undefined"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeObject:
    """Object constraint type."""

    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypePrimitive:
    """Primitive type."""

    primitive: PrimitiveInfo
    kind: typing.Literal["primitive"] = "primitive"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeLiteral:
    """Scalar literal type."""

    literal: LiteralInfo
    kind: typing.Literal["literal"] = "literal"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeMemory:
    """Normalized memory singleton type."""

    memory: MemoryInfo
    kind: typing.Literal["memory"] = "memory"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeReference:
    """Declaration reference with applied type arguments."""

    reference: GenericInfo
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeMember:
    """Member type selected from an owner type."""

    member: MemberInfo
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeForm:
    """Canonical ownership or access form."""

    form: FormInfo
    kind: typing.Literal["form"] = "form"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeDynamic:
    """Explicit erased dynamic value."""

    dynamic: DynamicInfo
    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeArray:
    """Homogeneous dynamic-length array."""

    array: ArrayInfo
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeFixedArray:
    """Fixed-length array."""

    fixed_array: FixedArrayInfo
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeRange:
    """Compact scalar interval."""

    range: RangeInfo
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeSlice:
    """Runtime-length homogeneous view."""

    slice: SliceInfo
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeTuple:
    """Tuple value."""

    tuple: TupleInfo
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeShape:
    """Structural object shape."""

    shape: ShapeInfo
    kind: typing.Literal["shape"] = "shape"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeFunctionSignature:
    """Callable signature."""

    function_signature: FunctionSignatureInfo
    kind: typing.Literal["functionSignature"] = "functionSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeFunction:
    """Fat callable value with captured environment."""

    function: FunctionTypeInfo
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeFunctionPointer:
    """Thin callable value."""

    function_pointer: FunctionPointerInfo
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeUnion:
    """Union type."""

    union: UnionInfo
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeIntersection:
    """Intersection type."""

    intersection: IntersectionInfo
    kind: typing.Literal["intersection"] = "intersection"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeNewtype:
    """Transparent nominal type."""

    newtype: NewtypeInfo
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeStruct:
    """Struct declaration type."""

    struct: StructInfo
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeClass:
    """Class declaration type."""

    class_: ClassInfo
    kind: typing.Literal["class"] = "class"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeInterface:
    """Interface declaration type."""

    interface: InterfaceInfo
    kind: typing.Literal["interface"] = "interface"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProgramTypeVariant:
    """Variant declaration type."""

    variant: VariantInfo
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Normalized program type visible through program reflection."""
ProgramType: typing.TypeAlias = (
    ProgramTypeNever
    | ProgramTypeUnknown
    | ProgramTypeVoid
    | ProgramTypeNull
    | ProgramTypeUndefined
    | ProgramTypeObject
    | ProgramTypePrimitive
    | ProgramTypeLiteral
    | ProgramTypeMemory
    | ProgramTypeReference
    | ProgramTypeMember
    | ProgramTypeForm
    | ProgramTypeDynamic
    | ProgramTypeArray
    | ProgramTypeFixedArray
    | ProgramTypeRange
    | ProgramTypeSlice
    | ProgramTypeTuple
    | ProgramTypeShape
    | ProgramTypeFunctionSignature
    | ProgramTypeFunction
    | ProgramTypeFunctionPointer
    | ProgramTypeUnion
    | ProgramTypeIntersection
    | ProgramTypeNewtype
    | ProgramTypeStruct
    | ProgramTypeClass
    | ProgramTypeInterface
    | ProgramTypeVariant
)

def encode_program_type(writer: BinaryWriter, value: ProgramType) -> None: ...
def decode_program_type(reader: BinaryReader) -> ProgramType: ...
def to_json_program_type(value: ProgramType) -> Json: ...
def from_json_program_type(value: Json) -> ProgramType: ...

@dataclass(frozen=True, slots=True)
class PrimitiveInfoBoolean:
    """Boolean primitive."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveInfoCharacter:
    """Unicode scalar value primitive."""

    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveInfoString:
    """String primitive."""

    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveInfoBigint:
    """Bigint primitive."""

    kind: typing.Literal["bigint"] = "bigint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveInfoNumber:
    """Number primitive."""

    kind: typing.Literal["number"] = "number"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveInfoInt:
    """Integer primitive."""

    # integer bit width
    width: int
    # whether the integer is signed
    is_signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveInfoFloat:
    """Floating-point primitive."""

    # float format
    format: destack._generated.mir.tree.type.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveInfoSymbol:
    """Symbol primitive."""

    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveInfoUniqueSymbol:
    """Unique symbol primitive."""

    kind: typing.Literal["uniqueSymbol"] = "uniqueSymbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Reflected primitive program type."""
PrimitiveInfo: typing.TypeAlias = (
    PrimitiveInfoBoolean
    | PrimitiveInfoCharacter
    | PrimitiveInfoString
    | PrimitiveInfoBigint
    | PrimitiveInfoNumber
    | PrimitiveInfoInt
    | PrimitiveInfoFloat
    | PrimitiveInfoSymbol
    | PrimitiveInfoUniqueSymbol
)

def encode_primitive_info(writer: BinaryWriter, value: PrimitiveInfo) -> None: ...
def decode_primitive_info(reader: BinaryReader) -> PrimitiveInfo: ...
def to_json_primitive_info(value: PrimitiveInfo) -> Json: ...
def from_json_primitive_info(value: Json) -> PrimitiveInfo: ...

@dataclass(frozen=True, slots=True)
class LiteralInfoBoolean:
    """Boolean literal."""

    boolean: bool
    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LiteralInfoCharacter:
    """Character literal."""

    character: str
    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LiteralInfoString:
    """String literal."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LiteralInfoInteger:
    """Integer literal."""

    integer: int
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LiteralInfoFloat:
    """Floating-point literal bits."""

    # float format
    format: destack._generated.mir.tree.type.FloatType
    # float bits
    bits: int
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LiteralInfoBigint:
    """Bigint literal."""

    bigint: int
    kind: typing.Literal["bigint"] = "bigint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Reflected scalar literal program type."""
LiteralInfo: typing.TypeAlias = (
    LiteralInfoBoolean
    | LiteralInfoCharacter
    | LiteralInfoString
    | LiteralInfoInteger
    | LiteralInfoFloat
    | LiteralInfoBigint
)

def encode_literal_info(writer: BinaryWriter, value: LiteralInfo) -> None: ...
def decode_literal_info(reader: BinaryReader) -> LiteralInfo: ...
def to_json_literal_info(value: LiteralInfo) -> Json: ...
def from_json_literal_info(value: Json) -> LiteralInfo: ...

@dataclass(frozen=True, slots=True)
class MemoryInfoAccess:
    """Reference access singleton."""

    access: destack._generated.mir.tree.type.Access
    kind: typing.Literal["access"] = "access"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryInfoSpace:
    """Concrete storage space singleton."""

    space: destack._generated.mir.tree.type.Space
    kind: typing.Literal["space"] = "space"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryInfoLifetime:
    """Lifetime singleton."""

    lifetime: destack._generated.mir.tree.lifetime.Lifetime
    kind: typing.Literal["lifetime"] = "lifetime"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Reflected memory singleton program type."""
MemoryInfo: typing.TypeAlias = MemoryInfoAccess | MemoryInfoSpace | MemoryInfoLifetime

def encode_memory_info(writer: BinaryWriter, value: MemoryInfo) -> None: ...
def decode_memory_info(reader: BinaryReader) -> MemoryInfo: ...
def to_json_memory_info(value: MemoryInfo) -> Json: ...
def from_json_memory_info(value: Json) -> MemoryInfo: ...

@dataclass(frozen=True, slots=True)
class GenericInfo:
    """Reflected generic declaration application."""

    # declaration name when one exists
    name: destack._generated.core.string.StringId | None
    # complete type arguments
    arguments: Sequence[destack._generated.program.type.TypeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GenericInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GenericInfo: ...

def encode_generic_info(writer: BinaryWriter, value: GenericInfo) -> None: ...
def decode_generic_info(reader: BinaryReader) -> GenericInfo: ...
def to_json_generic_info(value: GenericInfo) -> Json: ...
def from_json_generic_info(value: Json) -> GenericInfo: ...

@dataclass(frozen=True, slots=True)
class MemberInfo:
    """Reflected member type selection."""

    # owner type
    owner: destack._generated.program.type.TypeId
    # selected member name when one exists
    name: destack._generated.core.string.StringId | None
    # complete type arguments
    arguments: Sequence[destack._generated.program.type.TypeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemberInfo: ...

def encode_member_info(writer: BinaryWriter, value: MemberInfo) -> None: ...
def decode_member_info(reader: BinaryReader) -> MemberInfo: ...
def to_json_member_info(value: MemberInfo) -> Json: ...
def from_json_member_info(value: Json) -> MemberInfo: ...

@dataclass(frozen=True, slots=True)
class FormInfo:
    """Reflected ownership or access form."""

    # form constructor
    form: Form
    # carried value type
    value: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FormInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FormInfo: ...

def encode_form_info(writer: BinaryWriter, value: FormInfo) -> None: ...
def decode_form_info(reader: BinaryReader) -> FormInfo: ...
def to_json_form_info(value: FormInfo) -> Json: ...
def from_json_form_info(value: Json) -> FormInfo: ...

@dataclass(frozen=True, slots=True)
class FormManaged:
    """Managed value."""

    kind: typing.Literal["managed"] = "managed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class FormOwned:
    """Owned value."""

    kind: typing.Literal["owned"] = "owned"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class FormBorrowed:
    """Borrowed value."""

    # borrow lifetime type
    lifetime: destack._generated.program.type.TypeId
    # borrow access type
    access: destack._generated.program.type.TypeId
    kind: typing.Literal["borrowed"] = "borrowed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class FormRaw:
    """Raw pointer value."""

    kind: typing.Literal["raw"] = "raw"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class FormPlaced:
    """Placed value."""

    # placement type
    place: destack._generated.program.type.TypeId
    kind: typing.Literal["placed"] = "placed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class FormReadonly:
    """Readonly view."""

    kind: typing.Literal["readonly"] = "readonly"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Reflected ownership or access form constructor."""
Form: typing.TypeAlias = (
    FormManaged | FormOwned | FormBorrowed | FormRaw | FormPlaced | FormReadonly
)

def encode_form(writer: BinaryWriter, value: Form) -> None: ...
def decode_form(reader: BinaryReader) -> Form: ...
def to_json_form(value: Form) -> Json: ...
def from_json_form(value: Json) -> Form: ...

@dataclass(frozen=True, slots=True)
class DynamicInfo:
    """Reflected erased dynamic value type."""

    # erased constraint type
    constraint: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DynamicInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DynamicInfo: ...

def encode_dynamic_info(writer: BinaryWriter, value: DynamicInfo) -> None: ...
def decode_dynamic_info(reader: BinaryReader) -> DynamicInfo: ...
def to_json_dynamic_info(value: DynamicInfo) -> Json: ...
def from_json_dynamic_info(value: Json) -> DynamicInfo: ...

@dataclass(frozen=True, slots=True)
class ArrayInfo:
    """Reflected homogeneous array type."""

    # element type
    element: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArrayInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArrayInfo: ...

def encode_array_info(writer: BinaryWriter, value: ArrayInfo) -> None: ...
def decode_array_info(reader: BinaryReader) -> ArrayInfo: ...
def to_json_array_info(value: ArrayInfo) -> Json: ...
def from_json_array_info(value: Json) -> ArrayInfo: ...

@dataclass(frozen=True, slots=True)
class FixedArrayInfo:
    """Reflected fixed-length array type."""

    # element type
    element: destack._generated.program.type.TypeId
    # static count type
    count: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FixedArrayInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FixedArrayInfo: ...

def encode_fixed_array_info(writer: BinaryWriter, value: FixedArrayInfo) -> None: ...
def decode_fixed_array_info(reader: BinaryReader) -> FixedArrayInfo: ...
def to_json_fixed_array_info(value: FixedArrayInfo) -> Json: ...
def from_json_fixed_array_info(value: Json) -> FixedArrayInfo: ...

@dataclass(frozen=True, slots=True)
class RangeInfo:
    """Reflected scalar interval type."""

    # inclusive lower bound
    start: LiteralInfo | None
    # upper bound
    end: LiteralInfo | None
    # whether the upper bound is included
    is_inclusive: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RangeInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RangeInfo: ...

def encode_range_info(writer: BinaryWriter, value: RangeInfo) -> None: ...
def decode_range_info(reader: BinaryReader) -> RangeInfo: ...
def to_json_range_info(value: RangeInfo) -> Json: ...
def from_json_range_info(value: Json) -> RangeInfo: ...

@dataclass(frozen=True, slots=True)
class SliceInfo:
    """Reflected runtime-length homogeneous view type."""

    # element type
    element: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SliceInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SliceInfo: ...

def encode_slice_info(writer: BinaryWriter, value: SliceInfo) -> None: ...
def decode_slice_info(reader: BinaryReader) -> SliceInfo: ...
def to_json_slice_info(value: SliceInfo) -> Json: ...
def from_json_slice_info(value: Json) -> SliceInfo: ...

@dataclass(frozen=True, slots=True)
class TupleInfo:
    """Reflected tuple type."""

    # tuple elements
    elements: Sequence[TypeElementInfo]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TupleInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TupleInfo: ...

def encode_tuple_info(writer: BinaryWriter, value: TupleInfo) -> None: ...
def decode_tuple_info(reader: BinaryReader) -> TupleInfo: ...
def to_json_tuple_info(value: TupleInfo) -> Json: ...
def from_json_tuple_info(value: Json) -> TupleInfo: ...

@dataclass(frozen=True, slots=True)
class TypeElementInfo:
    """Reflected tuple element."""

    # element label when one exists
    label: destack._generated.core.string.StringId | None
    # element type
    ty: destack._generated.program.type.TypeId
    # whether the element is optional
    is_optional: bool
    # whether the element is readonly
    is_readonly: bool
    # whether the element captures remaining tuple elements
    is_rest: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeElementInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeElementInfo: ...

def encode_type_element_info(writer: BinaryWriter, value: TypeElementInfo) -> None: ...
def decode_type_element_info(reader: BinaryReader) -> TypeElementInfo: ...
def to_json_type_element_info(value: TypeElementInfo) -> Json: ...
def from_json_type_element_info(value: Json) -> TypeElementInfo: ...

@dataclass(frozen=True, slots=True)
class ShapeInfo:
    """Reflected structural object shape type."""

    # shape fields
    fields: Sequence[TypeFieldInfo]
    # call signature types
    call_signatures: Sequence[destack._generated.program.type.TypeId]
    # construct signature types
    construct_signatures: Sequence[destack._generated.program.type.TypeId]
    # index signatures
    index_signatures: Sequence[TypeIndexSignatureInfo]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ShapeInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ShapeInfo: ...

def encode_shape_info(writer: BinaryWriter, value: ShapeInfo) -> None: ...
def decode_shape_info(reader: BinaryReader) -> ShapeInfo: ...
def to_json_shape_info(value: ShapeInfo) -> Json: ...
def from_json_shape_info(value: Json) -> ShapeInfo: ...

@dataclass(frozen=True, slots=True)
class TypeFieldInfo:
    """Reflected object-like type field."""

    # field name when one exists
    name: destack._generated.core.string.StringId | None
    # field type
    ty: destack._generated.program.type.TypeId
    # whether the field is optional
    is_optional: bool
    # whether the field is readonly
    is_readonly: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeFieldInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeFieldInfo: ...

def encode_type_field_info(writer: BinaryWriter, value: TypeFieldInfo) -> None: ...
def decode_type_field_info(reader: BinaryReader) -> TypeFieldInfo: ...
def to_json_type_field_info(value: TypeFieldInfo) -> Json: ...
def from_json_type_field_info(value: Json) -> TypeFieldInfo: ...

@dataclass(frozen=True, slots=True)
class TypeIndexSignatureInfo:
    """Reflected index signature in an object-like type."""

    # key type
    key_type: destack._generated.program.type.TypeId
    # value type
    value_type: destack._generated.program.type.TypeId
    # whether the index signature is optional
    is_optional: bool
    # whether the index signature is readonly
    is_readonly: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeIndexSignatureInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeIndexSignatureInfo: ...

def encode_type_index_signature_info(
    writer: BinaryWriter, value: TypeIndexSignatureInfo
) -> None: ...
def decode_type_index_signature_info(
    reader: BinaryReader,
) -> TypeIndexSignatureInfo: ...
def to_json_type_index_signature_info(value: TypeIndexSignatureInfo) -> Json: ...
def from_json_type_index_signature_info(value: Json) -> TypeIndexSignatureInfo: ...

@dataclass(frozen=True, slots=True)
class FunctionSignatureInfo:
    """Reflected function signature type."""

    # optional receiver type
    this_parameter: destack._generated.program.type.TypeId | None
    # runtime parameters
    parameters: Sequence[FunctionParameterInfo]
    # optional return type
    return_type: destack._generated.program.type.TypeId | None
    # whether this signature is async
    is_async: bool
    # whether this signature is a generator
    is_generator: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionSignatureInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionSignatureInfo: ...

def encode_function_signature_info(
    writer: BinaryWriter, value: FunctionSignatureInfo
) -> None: ...
def decode_function_signature_info(reader: BinaryReader) -> FunctionSignatureInfo: ...
def to_json_function_signature_info(value: FunctionSignatureInfo) -> Json: ...
def from_json_function_signature_info(value: Json) -> FunctionSignatureInfo: ...

@dataclass(frozen=True, slots=True)
class FunctionParameterInfo:
    """Reflected runtime parameter in a function type."""

    # parameter type
    ty: destack._generated.program.type.TypeId
    # whether the parameter may be omitted
    is_optional: bool
    # whether the parameter captures remaining arguments
    is_rest: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionParameterInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionParameterInfo: ...

def encode_function_parameter_info(
    writer: BinaryWriter, value: FunctionParameterInfo
) -> None: ...
def decode_function_parameter_info(reader: BinaryReader) -> FunctionParameterInfo: ...
def to_json_function_parameter_info(value: FunctionParameterInfo) -> Json: ...
def from_json_function_parameter_info(value: Json) -> FunctionParameterInfo: ...

@dataclass(frozen=True, slots=True)
class FunctionTypeInfo:
    """Reflected fat callable value type."""

    # function signature type
    signature: destack._generated.program.type.TypeId
    # captured environment type
    environment: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionTypeInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionTypeInfo: ...

def encode_function_type_info(
    writer: BinaryWriter, value: FunctionTypeInfo
) -> None: ...
def decode_function_type_info(reader: BinaryReader) -> FunctionTypeInfo: ...
def to_json_function_type_info(value: FunctionTypeInfo) -> Json: ...
def from_json_function_type_info(value: Json) -> FunctionTypeInfo: ...

@dataclass(frozen=True, slots=True)
class FunctionPointerInfo:
    """Reflected thin callable value type."""

    # function signature type
    signature: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionPointerInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionPointerInfo: ...

def encode_function_pointer_info(
    writer: BinaryWriter, value: FunctionPointerInfo
) -> None: ...
def decode_function_pointer_info(reader: BinaryReader) -> FunctionPointerInfo: ...
def to_json_function_pointer_info(value: FunctionPointerInfo) -> Json: ...
def from_json_function_pointer_info(value: Json) -> FunctionPointerInfo: ...

@dataclass(frozen=True, slots=True)
class UnionInfo:
    """Reflected union type."""

    # union elements
    elements: Sequence[destack._generated.program.type.TypeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> UnionInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> UnionInfo: ...

def encode_union_info(writer: BinaryWriter, value: UnionInfo) -> None: ...
def decode_union_info(reader: BinaryReader) -> UnionInfo: ...
def to_json_union_info(value: UnionInfo) -> Json: ...
def from_json_union_info(value: Json) -> UnionInfo: ...

@dataclass(frozen=True, slots=True)
class IntersectionInfo:
    """Reflected intersection type."""

    # intersection elements
    elements: Sequence[destack._generated.program.type.TypeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IntersectionInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IntersectionInfo: ...

def encode_intersection_info(writer: BinaryWriter, value: IntersectionInfo) -> None: ...
def decode_intersection_info(reader: BinaryReader) -> IntersectionInfo: ...
def to_json_intersection_info(value: IntersectionInfo) -> Json: ...
def from_json_intersection_info(value: Json) -> IntersectionInfo: ...

@dataclass(frozen=True, slots=True)
class NewtypeInfo:
    """Reflected transparent nominal type."""

    # backing type
    backing: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NewtypeInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NewtypeInfo: ...

def encode_newtype_info(writer: BinaryWriter, value: NewtypeInfo) -> None: ...
def decode_newtype_info(reader: BinaryReader) -> NewtypeInfo: ...
def to_json_newtype_info(value: NewtypeInfo) -> Json: ...
def from_json_newtype_info(value: Json) -> NewtypeInfo: ...

@dataclass(frozen=True, slots=True)
class StructInfo:
    """Reflected struct declaration type."""

    # struct fields
    fields: Sequence[TypeFieldInfo]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StructInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StructInfo: ...

def encode_struct_info(writer: BinaryWriter, value: StructInfo) -> None: ...
def decode_struct_info(reader: BinaryReader) -> StructInfo: ...
def to_json_struct_info(value: StructInfo) -> Json: ...
def from_json_struct_info(value: Json) -> StructInfo: ...

@dataclass(frozen=True, slots=True)
class ClassInfo:
    """Reflected class declaration type."""

    # class fields
    fields: Sequence[TypeFieldInfo]
    # class members
    members: Sequence[MemberInfo]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ClassInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ClassInfo: ...

def encode_class_info(writer: BinaryWriter, value: ClassInfo) -> None: ...
def decode_class_info(reader: BinaryReader) -> ClassInfo: ...
def to_json_class_info(value: ClassInfo) -> Json: ...
def from_json_class_info(value: Json) -> ClassInfo: ...

@dataclass(frozen=True, slots=True)
class InterfaceInfo:
    """Reflected interface declaration type."""

    # whether the interface is nominal
    is_nominal: bool
    # interface members
    members: Sequence[MemberInfo]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InterfaceInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InterfaceInfo: ...

def encode_interface_info(writer: BinaryWriter, value: InterfaceInfo) -> None: ...
def decode_interface_info(reader: BinaryReader) -> InterfaceInfo: ...
def to_json_interface_info(value: InterfaceInfo) -> Json: ...
def from_json_interface_info(value: Json) -> InterfaceInfo: ...

@dataclass(frozen=True, slots=True)
class VariantInfo:
    """Reflected variant declaration type."""

    # variant cases
    cases: Sequence[VariantCaseInfo]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VariantInfo: ...

def encode_variant_info(writer: BinaryWriter, value: VariantInfo) -> None: ...
def decode_variant_info(reader: BinaryReader) -> VariantInfo: ...
def to_json_variant_info(value: VariantInfo) -> Json: ...
def from_json_variant_info(value: Json) -> VariantInfo: ...

@dataclass(frozen=True, slots=True)
class VariantCaseInfo:
    """Reflected variant case."""

    # case name when one exists
    name: destack._generated.core.string.StringId | None
    # case payload type
    ty: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantCaseInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VariantCaseInfo: ...

def encode_variant_case_info(writer: BinaryWriter, value: VariantCaseInfo) -> None: ...
def decode_variant_case_info(reader: BinaryReader) -> VariantCaseInfo: ...
def to_json_variant_case_info(value: VariantCaseInfo) -> Json: ...
def from_json_variant_case_info(value: Json) -> VariantCaseInfo: ...

@dataclass(frozen=True, slots=True)
class FunctionInfo:
    """Reflected executable function."""

    # function display name when one exists
    name: destack._generated.core.string.StringId | None
    # source module containing the function when one exists
    module: destack._generated.source.file.model.module.ModuleId | None
    # function signature type
    signature: destack._generated.program.type.TypeId
    # captured environment type when one exists
    environment: destack._generated.program.type.TypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionInfo: ...

def encode_function_info(writer: BinaryWriter, value: FunctionInfo) -> None: ...
def decode_function_info(reader: BinaryReader) -> FunctionInfo: ...
def to_json_function_info(value: FunctionInfo) -> Json: ...
def from_json_function_info(value: Json) -> FunctionInfo: ...

@dataclass(frozen=True, slots=True)
class FrameInfo:
    """Reflected executable frame layout."""

    # function owning this frame
    function: destack._generated.program.function.FunctionId
    # frame slots
    slots: Sequence[FrameSlotInfo]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FrameInfo: ...

def encode_frame_info(writer: BinaryWriter, value: FrameInfo) -> None: ...
def decode_frame_info(reader: BinaryReader) -> FrameInfo: ...
def to_json_frame_info(value: FrameInfo) -> Json: ...
def from_json_frame_info(value: Json) -> FrameInfo: ...

@dataclass(frozen=True, slots=True)
class FrameSlotInfo:
    """Reflected executable frame slot."""

    # slot type
    ty: destack._generated.program.type.TypeId
    # slot byte offset
    offset: int
    # slot byte length
    byte_len: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameSlotInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FrameSlotInfo: ...

def encode_frame_slot_info(writer: BinaryWriter, value: FrameSlotInfo) -> None: ...
def decode_frame_slot_info(reader: BinaryReader) -> FrameSlotInfo: ...
def to_json_frame_slot_info(value: FrameSlotInfo) -> Json: ...
def from_json_frame_slot_info(value: Json) -> FrameSlotInfo: ...

@dataclass(frozen=True, slots=True)
class GlobalInfo:
    """Reflected global in one executable program."""

    # global display name when one exists
    name: destack._generated.core.string.StringId | None
    # global type
    ty: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalInfo: ...

def encode_global_info(writer: BinaryWriter, value: GlobalInfo) -> None: ...
def decode_global_info(reader: BinaryReader) -> GlobalInfo: ...
def to_json_global_info(value: GlobalInfo) -> Json: ...
def from_json_global_info(value: Json) -> GlobalInfo: ...

@dataclass(frozen=True, slots=True)
class EntryInfo:
    """Reflected entry point."""

    # entry point
    entry: destack._generated.program.entry.EntryPoint

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> EntryInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> EntryInfo: ...

def encode_entry_info(writer: BinaryWriter, value: EntryInfo) -> None: ...
def decode_entry_info(reader: BinaryReader) -> EntryInfo: ...
def to_json_entry_info(value: EntryInfo) -> Json: ...
def from_json_entry_info(value: Json) -> EntryInfo: ...

__all__ = [
    "ProgramInfo",
    "encode_program_info",
    "decode_program_info",
    "to_json_program_info",
    "from_json_program_info",
    "ModuleInfo",
    "encode_module_info",
    "decode_module_info",
    "to_json_module_info",
    "from_json_module_info",
    "TypeInfo",
    "encode_type_info",
    "decode_type_info",
    "to_json_type_info",
    "from_json_type_info",
    "ProgramType",
    "encode_program_type",
    "decode_program_type",
    "to_json_program_type",
    "from_json_program_type",
    "ProgramTypeNever",
    "ProgramTypeUnknown",
    "ProgramTypeVoid",
    "ProgramTypeNull",
    "ProgramTypeUndefined",
    "ProgramTypeObject",
    "ProgramTypePrimitive",
    "ProgramTypeLiteral",
    "ProgramTypeMemory",
    "ProgramTypeReference",
    "ProgramTypeMember",
    "ProgramTypeForm",
    "ProgramTypeDynamic",
    "ProgramTypeArray",
    "ProgramTypeFixedArray",
    "ProgramTypeRange",
    "ProgramTypeSlice",
    "ProgramTypeTuple",
    "ProgramTypeShape",
    "ProgramTypeFunctionSignature",
    "ProgramTypeFunction",
    "ProgramTypeFunctionPointer",
    "ProgramTypeUnion",
    "ProgramTypeIntersection",
    "ProgramTypeNewtype",
    "ProgramTypeStruct",
    "ProgramTypeClass",
    "ProgramTypeInterface",
    "ProgramTypeVariant",
    "PrimitiveInfo",
    "encode_primitive_info",
    "decode_primitive_info",
    "to_json_primitive_info",
    "from_json_primitive_info",
    "PrimitiveInfoBoolean",
    "PrimitiveInfoCharacter",
    "PrimitiveInfoString",
    "PrimitiveInfoBigint",
    "PrimitiveInfoNumber",
    "PrimitiveInfoInt",
    "PrimitiveInfoFloat",
    "PrimitiveInfoSymbol",
    "PrimitiveInfoUniqueSymbol",
    "LiteralInfo",
    "encode_literal_info",
    "decode_literal_info",
    "to_json_literal_info",
    "from_json_literal_info",
    "LiteralInfoBoolean",
    "LiteralInfoCharacter",
    "LiteralInfoString",
    "LiteralInfoInteger",
    "LiteralInfoFloat",
    "LiteralInfoBigint",
    "MemoryInfo",
    "encode_memory_info",
    "decode_memory_info",
    "to_json_memory_info",
    "from_json_memory_info",
    "MemoryInfoAccess",
    "MemoryInfoSpace",
    "MemoryInfoLifetime",
    "GenericInfo",
    "encode_generic_info",
    "decode_generic_info",
    "to_json_generic_info",
    "from_json_generic_info",
    "MemberInfo",
    "encode_member_info",
    "decode_member_info",
    "to_json_member_info",
    "from_json_member_info",
    "FormInfo",
    "encode_form_info",
    "decode_form_info",
    "to_json_form_info",
    "from_json_form_info",
    "Form",
    "encode_form",
    "decode_form",
    "to_json_form",
    "from_json_form",
    "FormManaged",
    "FormOwned",
    "FormBorrowed",
    "FormRaw",
    "FormPlaced",
    "FormReadonly",
    "DynamicInfo",
    "encode_dynamic_info",
    "decode_dynamic_info",
    "to_json_dynamic_info",
    "from_json_dynamic_info",
    "ArrayInfo",
    "encode_array_info",
    "decode_array_info",
    "to_json_array_info",
    "from_json_array_info",
    "FixedArrayInfo",
    "encode_fixed_array_info",
    "decode_fixed_array_info",
    "to_json_fixed_array_info",
    "from_json_fixed_array_info",
    "RangeInfo",
    "encode_range_info",
    "decode_range_info",
    "to_json_range_info",
    "from_json_range_info",
    "SliceInfo",
    "encode_slice_info",
    "decode_slice_info",
    "to_json_slice_info",
    "from_json_slice_info",
    "TupleInfo",
    "encode_tuple_info",
    "decode_tuple_info",
    "to_json_tuple_info",
    "from_json_tuple_info",
    "TypeElementInfo",
    "encode_type_element_info",
    "decode_type_element_info",
    "to_json_type_element_info",
    "from_json_type_element_info",
    "ShapeInfo",
    "encode_shape_info",
    "decode_shape_info",
    "to_json_shape_info",
    "from_json_shape_info",
    "TypeFieldInfo",
    "encode_type_field_info",
    "decode_type_field_info",
    "to_json_type_field_info",
    "from_json_type_field_info",
    "TypeIndexSignatureInfo",
    "encode_type_index_signature_info",
    "decode_type_index_signature_info",
    "to_json_type_index_signature_info",
    "from_json_type_index_signature_info",
    "FunctionSignatureInfo",
    "encode_function_signature_info",
    "decode_function_signature_info",
    "to_json_function_signature_info",
    "from_json_function_signature_info",
    "FunctionParameterInfo",
    "encode_function_parameter_info",
    "decode_function_parameter_info",
    "to_json_function_parameter_info",
    "from_json_function_parameter_info",
    "FunctionTypeInfo",
    "encode_function_type_info",
    "decode_function_type_info",
    "to_json_function_type_info",
    "from_json_function_type_info",
    "FunctionPointerInfo",
    "encode_function_pointer_info",
    "decode_function_pointer_info",
    "to_json_function_pointer_info",
    "from_json_function_pointer_info",
    "UnionInfo",
    "encode_union_info",
    "decode_union_info",
    "to_json_union_info",
    "from_json_union_info",
    "IntersectionInfo",
    "encode_intersection_info",
    "decode_intersection_info",
    "to_json_intersection_info",
    "from_json_intersection_info",
    "NewtypeInfo",
    "encode_newtype_info",
    "decode_newtype_info",
    "to_json_newtype_info",
    "from_json_newtype_info",
    "StructInfo",
    "encode_struct_info",
    "decode_struct_info",
    "to_json_struct_info",
    "from_json_struct_info",
    "ClassInfo",
    "encode_class_info",
    "decode_class_info",
    "to_json_class_info",
    "from_json_class_info",
    "InterfaceInfo",
    "encode_interface_info",
    "decode_interface_info",
    "to_json_interface_info",
    "from_json_interface_info",
    "VariantInfo",
    "encode_variant_info",
    "decode_variant_info",
    "to_json_variant_info",
    "from_json_variant_info",
    "VariantCaseInfo",
    "encode_variant_case_info",
    "decode_variant_case_info",
    "to_json_variant_case_info",
    "from_json_variant_case_info",
    "FunctionInfo",
    "encode_function_info",
    "decode_function_info",
    "to_json_function_info",
    "from_json_function_info",
    "FrameInfo",
    "encode_frame_info",
    "decode_frame_info",
    "to_json_frame_info",
    "from_json_frame_info",
    "FrameSlotInfo",
    "encode_frame_slot_info",
    "decode_frame_slot_info",
    "to_json_frame_slot_info",
    "from_json_frame_slot_info",
    "GlobalInfo",
    "encode_global_info",
    "decode_global_info",
    "to_json_global_info",
    "from_json_global_info",
    "EntryInfo",
    "encode_entry_info",
    "decode_entry_info",
    "to_json_entry_info",
    "from_json_entry_info",
]
