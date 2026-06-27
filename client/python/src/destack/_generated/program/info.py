# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramInfo:
        """Decode one ProgramInfo."""
        return decode_program_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_info(self)

    @classmethod
    def from_json(cls, value: Json) -> ProgramInfo:
        """Return one ProgramInfo from one JSON value."""
        return from_json_program_info(value)


def encode_program_info(writer: BinaryWriter, value: ProgramInfo) -> None:
    """Encode one ProgramInfo."""
    writer.write_unsigned(len(value.modules))
    for item_value_modules_0 in value.modules:
        encode_module_info(writer, item_value_modules_0)
    writer.write_unsigned(len(value.types))
    for item_value_types_0 in value.types:
        if item_value_types_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_type_info(writer, item_value_types_0)
    writer.write_unsigned(len(value.functions))
    for item_value_functions_0 in value.functions:
        if item_value_functions_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_function_info(writer, item_value_functions_0)
    writer.write_unsigned(len(value.frames))
    for item_value_frames_0 in value.frames:
        encode_frame_info(writer, item_value_frames_0)
    writer.write_unsigned(len(value.globals))
    for item_value_globals_0 in value.globals:
        encode_global_info(writer, item_value_globals_0)
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_entry_info(writer, item_value_entries_0)


def decode_program_info(reader: BinaryReader) -> ProgramInfo:
    """Decode one ProgramInfo."""
    modules = [decode_module_info(reader) for _ in range(reader.read_number())]
    types = [
        reader.read_option(lambda: decode_type_info(reader))
        for _ in range(reader.read_number())
    ]
    functions = [
        reader.read_option(lambda: decode_function_info(reader))
        for _ in range(reader.read_number())
    ]
    frames = [decode_frame_info(reader) for _ in range(reader.read_number())]
    globals = [decode_global_info(reader) for _ in range(reader.read_number())]
    entries = [decode_entry_info(reader) for _ in range(reader.read_number())]

    return ProgramInfo(
        modules=modules,
        types=types,
        functions=functions,
        frames=frames,
        globals=globals,
        entries=entries,
    )


def to_json_program_info(value: ProgramInfo) -> Json:
    """Return one JSON value for one ProgramInfo."""
    return {
        "modules": [to_json_module_info(item_0) for item_0 in value.modules],
        "types": [
            None if item_0 is None else to_json_type_info(item_0)
            for item_0 in value.types
        ],
        "functions": [
            None if item_0 is None else to_json_function_info(item_0)
            for item_0 in value.functions
        ],
        "frames": [to_json_frame_info(item_0) for item_0 in value.frames],
        "globals": [to_json_global_info(item_0) for item_0 in value.globals],
        "entries": [to_json_entry_info(item_0) for item_0 in value.entries],
    }


def from_json_program_info(value: Json) -> ProgramInfo:
    """Return one ProgramInfo from one JSON value."""
    object_ = json_object(value)

    return ProgramInfo(
        modules=[
            from_json_module_info(item_0)
            for item_0 in json_array(json_field(object_, "modules"))
        ],
        types=[
            None if item_0 is None else from_json_type_info(item_0)
            for item_0 in json_array(json_field(object_, "types"))
        ],
        functions=[
            None if item_0 is None else from_json_function_info(item_0)
            for item_0 in json_array(json_field(object_, "functions"))
        ],
        frames=[
            from_json_frame_info(item_0)
            for item_0 in json_array(json_field(object_, "frames"))
        ],
        globals=[
            from_json_global_info(item_0)
            for item_0 in json_array(json_field(object_, "globals"))
        ],
        entries=[
            from_json_entry_info(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ModuleInfo:
    """Reflected module in one executable program."""

    # source module id when one exists
    id: destack._generated.source.file.model.module.ModuleId | None
    # module display name when one exists
    name: destack._generated.core.string.StringId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleInfo:
        """Decode one ModuleInfo."""
        return decode_module_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module_info(self)

    @classmethod
    def from_json(cls, value: Json) -> ModuleInfo:
        """Return one ModuleInfo from one JSON value."""
        return from_json_module_info(value)


def encode_module_info(writer: BinaryWriter, value: ModuleInfo) -> None:
    """Encode one ModuleInfo."""
    if value.id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.module.encode_module_id(writer, value.id)
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)


def decode_module_info(reader: BinaryReader) -> ModuleInfo:
    """Decode one ModuleInfo."""
    id = reader.read_option(
        lambda: destack._generated.source.file.model.module.decode_module_id(reader)
    )
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )

    return ModuleInfo(
        id=id,
        name=name,
    )


def to_json_module_info(value: ModuleInfo) -> Json:
    """Return one JSON value for one ModuleInfo."""
    return {
        **(
            {}
            if value.id is None
            else {
                "id": destack._generated.source.file.model.module.to_json_module_id(
                    value.id
                )
            }
        ),
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
    }


def from_json_module_info(value: Json) -> ModuleInfo:
    """Return one ModuleInfo from one JSON value."""
    object_ = json_object(value)

    return ModuleInfo(
        id=json_optional(
            object_,
            "id",
            lambda value: (
                destack._generated.source.file.model.module.from_json_module_id(value)
            ),
        ),
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeInfo:
        """Decode one TypeInfo."""
        return decode_type_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_info(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeInfo:
        """Return one TypeInfo from one JSON value."""
        return from_json_type_info(value)


def encode_type_info(writer: BinaryWriter, value: TypeInfo) -> None:
    """Encode one TypeInfo."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    if value.module is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
    destack._generated.program.type.encode_type_id(writer, value.repr)
    if value.layout is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.program.layout.encode_layout_id(writer, value.layout)
    encode_program_type(writer, value.ty)


def decode_type_info(reader: BinaryReader) -> TypeInfo:
    """Decode one TypeInfo."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    module = reader.read_option(
        lambda: destack._generated.source.file.model.module.decode_module_id(reader)
    )
    repr = destack._generated.program.type.decode_type_id(reader)
    layout = reader.read_option(
        lambda: destack._generated.program.layout.decode_layout_id(reader)
    )
    ty = decode_program_type(reader)

    return TypeInfo(
        name=name,
        module=module,
        repr=repr,
        layout=layout,
        ty=ty,
    )


def to_json_type_info(value: TypeInfo) -> Json:
    """Return one JSON value for one TypeInfo."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        **(
            {}
            if value.module is None
            else {
                "module": destack._generated.source.file.model.module.to_json_module_id(
                    value.module
                )
            }
        ),
        "repr": destack._generated.program.type.to_json_type_id(value.repr),
        **(
            {}
            if value.layout is None
            else {
                "layout": destack._generated.program.layout.to_json_layout_id(
                    value.layout
                )
            }
        ),
        "ty": to_json_program_type(value.ty),
    }


def from_json_type_info(value: Json) -> TypeInfo:
    """Return one TypeInfo from one JSON value."""
    object_ = json_object(value)

    return TypeInfo(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        module=json_optional(
            object_,
            "module",
            lambda value: (
                destack._generated.source.file.model.module.from_json_module_id(value)
            ),
        ),
        repr=destack._generated.program.type.from_json_type_id(
            json_field(object_, "repr")
        ),
        layout=json_optional(
            object_,
            "layout",
            lambda value: destack._generated.program.layout.from_json_layout_id(value),
        ),
        ty=from_json_program_type(json_field(object_, "ty")),
    )


@dataclass(frozen=True, slots=True)
class ProgramTypeNever:
    """Never type."""

    kind: typing.Literal["never"] = "never"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeUnknown:
    """Unknown type."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeVoid:
    """Void type."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeNull:
    """Null singleton type."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeUndefined:
    """Undefined singleton type."""

    kind: typing.Literal["undefined"] = "undefined"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeObject:
    """Object constraint type."""

    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypePrimitive:
    """Primitive type."""

    primitive: PrimitiveInfo
    kind: typing.Literal["primitive"] = "primitive"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeLiteral:
    """Scalar literal type."""

    literal: LiteralInfo
    kind: typing.Literal["literal"] = "literal"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeMemory:
    """Normalized memory singleton type."""

    memory: MemoryInfo
    kind: typing.Literal["memory"] = "memory"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeReference:
    """Declaration reference with applied type arguments."""

    reference: GenericInfo
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeMember:
    """Member type selected from an owner type."""

    member: MemberInfo
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeForm:
    """Canonical ownership or access form."""

    form: FormInfo
    kind: typing.Literal["form"] = "form"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeDynamic:
    """Explicit erased dynamic value."""

    dynamic: DynamicInfo
    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeArray:
    """Homogeneous dynamic-length array."""

    array: ArrayInfo
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeFixedArray:
    """Fixed-length array."""

    fixed_array: FixedArrayInfo
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeRange:
    """Compact scalar interval."""

    range: RangeInfo
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeSlice:
    """Runtime-length homogeneous view."""

    slice: SliceInfo
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeTuple:
    """Tuple value."""

    tuple: TupleInfo
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeShape:
    """Structural object shape."""

    shape: ShapeInfo
    kind: typing.Literal["shape"] = "shape"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeFunctionSignature:
    """Callable signature."""

    function_signature: FunctionSignatureInfo
    kind: typing.Literal["functionSignature"] = "functionSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeFunction:
    """Fat callable value with captured environment."""

    function: FunctionTypeInfo
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeFunctionPointer:
    """Thin callable value."""

    function_pointer: FunctionPointerInfo
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeUnion:
    """Union type."""

    union: UnionInfo
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeIntersection:
    """Intersection type."""

    intersection: IntersectionInfo
    kind: typing.Literal["intersection"] = "intersection"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeNewtype:
    """Transparent nominal type."""

    newtype: NewtypeInfo
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeStruct:
    """Struct declaration type."""

    struct: StructInfo
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeClass:
    """Class declaration type."""

    class_: ClassInfo
    kind: typing.Literal["class"] = "class"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeInterface:
    """Interface declaration type."""

    interface: InterfaceInfo
    kind: typing.Literal["interface"] = "interface"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


@dataclass(frozen=True, slots=True)
class ProgramTypeVariant:
    """Variant declaration type."""

    variant: VariantInfo
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_type(self)


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


def encode_program_type(writer: BinaryWriter, value: ProgramType) -> None:
    """Encode one ProgramType."""
    if value.kind == "never":
        writer.write_unsigned(0)
    elif value.kind == "unknown":
        writer.write_unsigned(1)
    elif value.kind == "void":
        writer.write_unsigned(2)
    elif value.kind == "null":
        writer.write_unsigned(3)
    elif value.kind == "undefined":
        writer.write_unsigned(4)
    elif value.kind == "object":
        writer.write_unsigned(5)
    elif value.kind == "primitive":
        writer.write_unsigned(6)
        encode_primitive_info(writer, value.primitive)
    elif value.kind == "literal":
        writer.write_unsigned(7)
        encode_literal_info(writer, value.literal)
    elif value.kind == "memory":
        writer.write_unsigned(8)
        encode_memory_info(writer, value.memory)
    elif value.kind == "reference":
        writer.write_unsigned(9)
        encode_generic_info(writer, value.reference)
    elif value.kind == "member":
        writer.write_unsigned(10)
        encode_member_info(writer, value.member)
    elif value.kind == "form":
        writer.write_unsigned(11)
        encode_form_info(writer, value.form)
    elif value.kind == "dynamic":
        writer.write_unsigned(12)
        encode_dynamic_info(writer, value.dynamic)
    elif value.kind == "array":
        writer.write_unsigned(13)
        encode_array_info(writer, value.array)
    elif value.kind == "fixedArray":
        writer.write_unsigned(14)
        encode_fixed_array_info(writer, value.fixed_array)
    elif value.kind == "range":
        writer.write_unsigned(15)
        encode_range_info(writer, value.range)
    elif value.kind == "slice":
        writer.write_unsigned(16)
        encode_slice_info(writer, value.slice)
    elif value.kind == "tuple":
        writer.write_unsigned(17)
        encode_tuple_info(writer, value.tuple)
    elif value.kind == "shape":
        writer.write_unsigned(18)
        encode_shape_info(writer, value.shape)
    elif value.kind == "functionSignature":
        writer.write_unsigned(19)
        encode_function_signature_info(writer, value.function_signature)
    elif value.kind == "function":
        writer.write_unsigned(20)
        encode_function_type_info(writer, value.function)
    elif value.kind == "functionPointer":
        writer.write_unsigned(21)
        encode_function_pointer_info(writer, value.function_pointer)
    elif value.kind == "union":
        writer.write_unsigned(22)
        encode_union_info(writer, value.union)
    elif value.kind == "intersection":
        writer.write_unsigned(23)
        encode_intersection_info(writer, value.intersection)
    elif value.kind == "newtype":
        writer.write_unsigned(24)
        encode_newtype_info(writer, value.newtype)
    elif value.kind == "struct":
        writer.write_unsigned(25)
        encode_struct_info(writer, value.struct)
    elif value.kind == "class":
        writer.write_unsigned(26)
        encode_class_info(writer, value.class_)
    elif value.kind == "interface":
        writer.write_unsigned(27)
        encode_interface_info(writer, value.interface)
    elif value.kind == "variant":
        writer.write_unsigned(28)
        encode_variant_info(writer, value.variant)
    else:
        raise SerdeError("unknown enum variant")


def decode_program_type(reader: BinaryReader) -> ProgramType:
    """Decode one ProgramType."""
    variant = reader.read_number()

    if variant == 0:
        return ProgramTypeNever()
    elif variant == 1:
        return ProgramTypeUnknown()
    elif variant == 2:
        return ProgramTypeVoid()
    elif variant == 3:
        return ProgramTypeNull()
    elif variant == 4:
        return ProgramTypeUndefined()
    elif variant == 5:
        return ProgramTypeObject()
    elif variant == 6:
        primitive = decode_primitive_info(reader)

        return ProgramTypePrimitive(primitive=primitive)
    elif variant == 7:
        literal = decode_literal_info(reader)

        return ProgramTypeLiteral(literal=literal)
    elif variant == 8:
        memory = decode_memory_info(reader)

        return ProgramTypeMemory(memory=memory)
    elif variant == 9:
        reference = decode_generic_info(reader)

        return ProgramTypeReference(reference=reference)
    elif variant == 10:
        member = decode_member_info(reader)

        return ProgramTypeMember(member=member)
    elif variant == 11:
        form = decode_form_info(reader)

        return ProgramTypeForm(form=form)
    elif variant == 12:
        dynamic = decode_dynamic_info(reader)

        return ProgramTypeDynamic(dynamic=dynamic)
    elif variant == 13:
        array = decode_array_info(reader)

        return ProgramTypeArray(array=array)
    elif variant == 14:
        fixed_array = decode_fixed_array_info(reader)

        return ProgramTypeFixedArray(fixed_array=fixed_array)
    elif variant == 15:
        range_ = decode_range_info(reader)

        return ProgramTypeRange(range=range_)
    elif variant == 16:
        slice = decode_slice_info(reader)

        return ProgramTypeSlice(slice=slice)
    elif variant == 17:
        tuple = decode_tuple_info(reader)

        return ProgramTypeTuple(tuple=tuple)
    elif variant == 18:
        shape = decode_shape_info(reader)

        return ProgramTypeShape(shape=shape)
    elif variant == 19:
        function_signature = decode_function_signature_info(reader)

        return ProgramTypeFunctionSignature(function_signature=function_signature)
    elif variant == 20:
        function = decode_function_type_info(reader)

        return ProgramTypeFunction(function=function)
    elif variant == 21:
        function_pointer = decode_function_pointer_info(reader)

        return ProgramTypeFunctionPointer(function_pointer=function_pointer)
    elif variant == 22:
        union = decode_union_info(reader)

        return ProgramTypeUnion(union=union)
    elif variant == 23:
        intersection = decode_intersection_info(reader)

        return ProgramTypeIntersection(intersection=intersection)
    elif variant == 24:
        newtype = decode_newtype_info(reader)

        return ProgramTypeNewtype(newtype=newtype)
    elif variant == 25:
        struct = decode_struct_info(reader)

        return ProgramTypeStruct(struct=struct)
    elif variant == 26:
        class_ = decode_class_info(reader)

        return ProgramTypeClass(class_=class_)
    elif variant == 27:
        interface = decode_interface_info(reader)

        return ProgramTypeInterface(interface=interface)
    elif variant == 28:
        variant = decode_variant_info(reader)

        return ProgramTypeVariant(variant=variant)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_program_type(value: ProgramType) -> Json:
    """Return one JSON value for one ProgramType."""
    if value.kind == "never":
        return {
            "kind": "never",
        }
    elif value.kind == "unknown":
        return {
            "kind": "unknown",
        }
    elif value.kind == "void":
        return {
            "kind": "void",
        }
    elif value.kind == "null":
        return {
            "kind": "null",
        }
    elif value.kind == "undefined":
        return {
            "kind": "undefined",
        }
    elif value.kind == "object":
        return {
            "kind": "object",
        }
    elif value.kind == "primitive":
        return {
            "kind": "primitive",
            "primitive": to_json_primitive_info(value.primitive),
        }
    elif value.kind == "literal":
        return {
            "kind": "literal",
            "literal": to_json_literal_info(value.literal),
        }
    elif value.kind == "memory":
        return {
            "kind": "memory",
            "memory": to_json_memory_info(value.memory),
        }
    elif value.kind == "reference":
        return {
            "kind": "reference",
            "reference": to_json_generic_info(value.reference),
        }
    elif value.kind == "member":
        return {
            "kind": "member",
            "member": to_json_member_info(value.member),
        }
    elif value.kind == "form":
        return {
            "kind": "form",
            "form": to_json_form_info(value.form),
        }
    elif value.kind == "dynamic":
        return {
            "kind": "dynamic",
            "dynamic": to_json_dynamic_info(value.dynamic),
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "array": to_json_array_info(value.array),
        }
    elif value.kind == "fixedArray":
        return {
            "kind": "fixedArray",
            "fixed_array": to_json_fixed_array_info(value.fixed_array),
        }
    elif value.kind == "range":
        return {
            "kind": "range",
            "range": to_json_range_info(value.range),
        }
    elif value.kind == "slice":
        return {
            "kind": "slice",
            "slice": to_json_slice_info(value.slice),
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "tuple": to_json_tuple_info(value.tuple),
        }
    elif value.kind == "shape":
        return {
            "kind": "shape",
            "shape": to_json_shape_info(value.shape),
        }
    elif value.kind == "functionSignature":
        return {
            "kind": "functionSignature",
            "function_signature": to_json_function_signature_info(
                value.function_signature
            ),
        }
    elif value.kind == "function":
        return {
            "kind": "function",
            "function": to_json_function_type_info(value.function),
        }
    elif value.kind == "functionPointer":
        return {
            "kind": "functionPointer",
            "function_pointer": to_json_function_pointer_info(value.function_pointer),
        }
    elif value.kind == "union":
        return {
            "kind": "union",
            "union": to_json_union_info(value.union),
        }
    elif value.kind == "intersection":
        return {
            "kind": "intersection",
            "intersection": to_json_intersection_info(value.intersection),
        }
    elif value.kind == "newtype":
        return {
            "kind": "newtype",
            "newtype": to_json_newtype_info(value.newtype),
        }
    elif value.kind == "struct":
        return {
            "kind": "struct",
            "struct": to_json_struct_info(value.struct),
        }
    elif value.kind == "class":
        return {
            "kind": "class",
            "class": to_json_class_info(value.class_),
        }
    elif value.kind == "interface":
        return {
            "kind": "interface",
            "interface": to_json_interface_info(value.interface),
        }
    elif value.kind == "variant":
        return {
            "kind": "variant",
            "variant": to_json_variant_info(value.variant),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_program_type(value: Json) -> ProgramType:
    """Return one ProgramType from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "never":
        return ProgramTypeNever()
    elif kind == "unknown":
        return ProgramTypeUnknown()
    elif kind == "void":
        return ProgramTypeVoid()
    elif kind == "null":
        return ProgramTypeNull()
    elif kind == "undefined":
        return ProgramTypeUndefined()
    elif kind == "object":
        return ProgramTypeObject()
    elif kind == "primitive":
        return ProgramTypePrimitive(
            primitive=from_json_primitive_info(json_field(object_, "primitive"))
        )
    elif kind == "literal":
        return ProgramTypeLiteral(
            literal=from_json_literal_info(json_field(object_, "literal"))
        )
    elif kind == "memory":
        return ProgramTypeMemory(
            memory=from_json_memory_info(json_field(object_, "memory"))
        )
    elif kind == "reference":
        return ProgramTypeReference(
            reference=from_json_generic_info(json_field(object_, "reference"))
        )
    elif kind == "member":
        return ProgramTypeMember(
            member=from_json_member_info(json_field(object_, "member"))
        )
    elif kind == "form":
        return ProgramTypeForm(form=from_json_form_info(json_field(object_, "form")))
    elif kind == "dynamic":
        return ProgramTypeDynamic(
            dynamic=from_json_dynamic_info(json_field(object_, "dynamic"))
        )
    elif kind == "array":
        return ProgramTypeArray(
            array=from_json_array_info(json_field(object_, "array"))
        )
    elif kind == "fixedArray":
        return ProgramTypeFixedArray(
            fixed_array=from_json_fixed_array_info(json_field(object_, "fixed_array"))
        )
    elif kind == "range":
        return ProgramTypeRange(
            range=from_json_range_info(json_field(object_, "range"))
        )
    elif kind == "slice":
        return ProgramTypeSlice(
            slice=from_json_slice_info(json_field(object_, "slice"))
        )
    elif kind == "tuple":
        return ProgramTypeTuple(
            tuple=from_json_tuple_info(json_field(object_, "tuple"))
        )
    elif kind == "shape":
        return ProgramTypeShape(
            shape=from_json_shape_info(json_field(object_, "shape"))
        )
    elif kind == "functionSignature":
        return ProgramTypeFunctionSignature(
            function_signature=from_json_function_signature_info(
                json_field(object_, "function_signature")
            )
        )
    elif kind == "function":
        return ProgramTypeFunction(
            function=from_json_function_type_info(json_field(object_, "function"))
        )
    elif kind == "functionPointer":
        return ProgramTypeFunctionPointer(
            function_pointer=from_json_function_pointer_info(
                json_field(object_, "function_pointer")
            )
        )
    elif kind == "union":
        return ProgramTypeUnion(
            union=from_json_union_info(json_field(object_, "union"))
        )
    elif kind == "intersection":
        return ProgramTypeIntersection(
            intersection=from_json_intersection_info(
                json_field(object_, "intersection")
            )
        )
    elif kind == "newtype":
        return ProgramTypeNewtype(
            newtype=from_json_newtype_info(json_field(object_, "newtype"))
        )
    elif kind == "struct":
        return ProgramTypeStruct(
            struct=from_json_struct_info(json_field(object_, "struct"))
        )
    elif kind == "class":
        return ProgramTypeClass(
            class_=from_json_class_info(json_field(object_, "class"))
        )
    elif kind == "interface":
        return ProgramTypeInterface(
            interface=from_json_interface_info(json_field(object_, "interface"))
        )
    elif kind == "variant":
        return ProgramTypeVariant(
            variant=from_json_variant_info(json_field(object_, "variant"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class PrimitiveInfoBoolean:
    """Boolean primitive."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_info(self)


@dataclass(frozen=True, slots=True)
class PrimitiveInfoCharacter:
    """Unicode scalar value primitive."""

    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_info(self)


@dataclass(frozen=True, slots=True)
class PrimitiveInfoString:
    """String primitive."""

    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_info(self)


@dataclass(frozen=True, slots=True)
class PrimitiveInfoBigint:
    """Bigint primitive."""

    kind: typing.Literal["bigint"] = "bigint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_info(self)


@dataclass(frozen=True, slots=True)
class PrimitiveInfoNumber:
    """Number primitive."""

    kind: typing.Literal["number"] = "number"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_info(self)


@dataclass(frozen=True, slots=True)
class PrimitiveInfoInt:
    """Integer primitive."""

    # integer bit width
    width: int
    # whether the integer is signed
    is_signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_info(self)


@dataclass(frozen=True, slots=True)
class PrimitiveInfoFloat:
    """Floating-point primitive."""

    # float format
    format: destack._generated.mir.tree.type.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_info(self)


@dataclass(frozen=True, slots=True)
class PrimitiveInfoSymbol:
    """Symbol primitive."""

    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_info(self)


@dataclass(frozen=True, slots=True)
class PrimitiveInfoUniqueSymbol:
    """Unique symbol primitive."""

    kind: typing.Literal["uniqueSymbol"] = "uniqueSymbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_info(self)


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


def encode_primitive_info(writer: BinaryWriter, value: PrimitiveInfo) -> None:
    """Encode one PrimitiveInfo."""
    if value.kind == "boolean":
        writer.write_unsigned(0)
    elif value.kind == "character":
        writer.write_unsigned(1)
    elif value.kind == "string":
        writer.write_unsigned(2)
    elif value.kind == "bigint":
        writer.write_unsigned(3)
    elif value.kind == "number":
        writer.write_unsigned(4)
    elif value.kind == "int":
        writer.write_unsigned(5)
        writer.write_unsigned(value.width)
        writer.write_bool(value.is_signed)
    elif value.kind == "float":
        writer.write_unsigned(6)
        destack._generated.mir.tree.type.encode_float_type(writer, value.format)
    elif value.kind == "symbol":
        writer.write_unsigned(7)
    elif value.kind == "uniqueSymbol":
        writer.write_unsigned(8)
    else:
        raise SerdeError("unknown enum variant")


def decode_primitive_info(reader: BinaryReader) -> PrimitiveInfo:
    """Decode one PrimitiveInfo."""
    variant = reader.read_number()

    if variant == 0:
        return PrimitiveInfoBoolean()
    elif variant == 1:
        return PrimitiveInfoCharacter()
    elif variant == 2:
        return PrimitiveInfoString()
    elif variant == 3:
        return PrimitiveInfoBigint()
    elif variant == 4:
        return PrimitiveInfoNumber()
    elif variant == 5:
        width = reader.read_number()
        is_signed = reader.read_bool()

        return PrimitiveInfoInt(
            width=width,
            is_signed=is_signed,
        )
    elif variant == 6:
        format = destack._generated.mir.tree.type.decode_float_type(reader)

        return PrimitiveInfoFloat(
            format=format,
        )
    elif variant == 7:
        return PrimitiveInfoSymbol()
    elif variant == 8:
        return PrimitiveInfoUniqueSymbol()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_primitive_info(value: PrimitiveInfo) -> Json:
    """Return one JSON value for one PrimitiveInfo."""
    if value.kind == "boolean":
        return {
            "kind": "boolean",
        }
    elif value.kind == "character":
        return {
            "kind": "character",
        }
    elif value.kind == "string":
        return {
            "kind": "string",
        }
    elif value.kind == "bigint":
        return {
            "kind": "bigint",
        }
    elif value.kind == "number":
        return {
            "kind": "number",
        }
    elif value.kind == "int":
        return {
            "kind": "int",
            "width": value.width,
            "isSigned": value.is_signed,
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "format": destack._generated.mir.tree.type.to_json_float_type(value.format),
        }
    elif value.kind == "symbol":
        return {
            "kind": "symbol",
        }
    elif value.kind == "uniqueSymbol":
        return {
            "kind": "uniqueSymbol",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_primitive_info(value: Json) -> PrimitiveInfo:
    """Return one PrimitiveInfo from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "boolean":
        return PrimitiveInfoBoolean()
    elif kind == "character":
        return PrimitiveInfoCharacter()
    elif kind == "string":
        return PrimitiveInfoString()
    elif kind == "bigint":
        return PrimitiveInfoBigint()
    elif kind == "number":
        return PrimitiveInfoNumber()
    elif kind == "int":
        return PrimitiveInfoInt(
            width=json_int(json_field(object_, "width")),
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "float":
        return PrimitiveInfoFloat(
            format=destack._generated.mir.tree.type.from_json_float_type(
                json_field(object_, "format")
            ),
        )
    elif kind == "symbol":
        return PrimitiveInfoSymbol()
    elif kind == "uniqueSymbol":
        return PrimitiveInfoUniqueSymbol()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class LiteralInfoBoolean:
    """Boolean literal."""

    boolean: bool
    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_literal_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_literal_info(self)


@dataclass(frozen=True, slots=True)
class LiteralInfoCharacter:
    """Character literal."""

    character: str
    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_literal_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_literal_info(self)


@dataclass(frozen=True, slots=True)
class LiteralInfoString:
    """String literal."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_literal_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_literal_info(self)


@dataclass(frozen=True, slots=True)
class LiteralInfoInteger:
    """Integer literal."""

    integer: int
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_literal_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_literal_info(self)


@dataclass(frozen=True, slots=True)
class LiteralInfoFloat:
    """Floating-point literal bits."""

    # float format
    format: destack._generated.mir.tree.type.FloatType
    # float bits
    bits: int
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_literal_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_literal_info(self)


@dataclass(frozen=True, slots=True)
class LiteralInfoBigint:
    """Bigint literal."""

    bigint: int
    kind: typing.Literal["bigint"] = "bigint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_literal_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_literal_info(self)


"""Reflected scalar literal program type."""
LiteralInfo: typing.TypeAlias = (
    LiteralInfoBoolean
    | LiteralInfoCharacter
    | LiteralInfoString
    | LiteralInfoInteger
    | LiteralInfoFloat
    | LiteralInfoBigint
)


def encode_literal_info(writer: BinaryWriter, value: LiteralInfo) -> None:
    """Encode one LiteralInfo."""
    if value.kind == "boolean":
        writer.write_unsigned(0)
        writer.write_bool(value.boolean)
    elif value.kind == "character":
        writer.write_unsigned(1)
        writer.write_char(value.character)
    elif value.kind == "string":
        writer.write_unsigned(2)
        destack._generated.core.string.encode_string_id(writer, value.string)
    elif value.kind == "integer":
        writer.write_unsigned(3)
        writer.write_signed(value.integer)
    elif value.kind == "float":
        writer.write_unsigned(4)
        destack._generated.mir.tree.type.encode_float_type(writer, value.format)
        writer.write_unsigned(value.bits)
    elif value.kind == "bigint":
        writer.write_unsigned(5)
        writer.write_signed(value.bigint)
    else:
        raise SerdeError("unknown enum variant")


def decode_literal_info(reader: BinaryReader) -> LiteralInfo:
    """Decode one LiteralInfo."""
    variant = reader.read_number()

    if variant == 0:
        boolean = reader.read_bool()

        return LiteralInfoBoolean(boolean=boolean)
    elif variant == 1:
        character = reader.read_char()

        return LiteralInfoCharacter(character=character)
    elif variant == 2:
        string = destack._generated.core.string.decode_string_id(reader)

        return LiteralInfoString(string=string)
    elif variant == 3:
        integer = reader.read_signed_number()

        return LiteralInfoInteger(integer=integer)
    elif variant == 4:
        format = destack._generated.mir.tree.type.decode_float_type(reader)
        bits = reader.read_unsigned()

        return LiteralInfoFloat(
            format=format,
            bits=bits,
        )
    elif variant == 5:
        bigint = reader.read_signed_number()

        return LiteralInfoBigint(bigint=bigint)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_literal_info(value: LiteralInfo) -> Json:
    """Return one JSON value for one LiteralInfo."""
    if value.kind == "boolean":
        return {
            "kind": "boolean",
            "boolean": value.boolean,
        }
    elif value.kind == "character":
        return {
            "kind": "character",
            "character": value.character,
        }
    elif value.kind == "string":
        return {
            "kind": "string",
            "string": destack._generated.core.string.to_json_string_id(value.string),
        }
    elif value.kind == "integer":
        return {
            "kind": "integer",
            "integer": value.integer,
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "format": destack._generated.mir.tree.type.to_json_float_type(value.format),
            "bits": value.bits,
        }
    elif value.kind == "bigint":
        return {
            "kind": "bigint",
            "bigint": value.bigint,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_literal_info(value: Json) -> LiteralInfo:
    """Return one LiteralInfo from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "boolean":
        return LiteralInfoBoolean(boolean=json_bool(json_field(object_, "boolean")))
    elif kind == "character":
        return LiteralInfoCharacter(
            character=json_string(json_field(object_, "character"))
        )
    elif kind == "string":
        return LiteralInfoString(
            string=destack._generated.core.string.from_json_string_id(
                json_field(object_, "string")
            )
        )
    elif kind == "integer":
        return LiteralInfoInteger(integer=json_int(json_field(object_, "integer")))
    elif kind == "float":
        return LiteralInfoFloat(
            format=destack._generated.mir.tree.type.from_json_float_type(
                json_field(object_, "format")
            ),
            bits=json_int(json_field(object_, "bits")),
        )
    elif kind == "bigint":
        return LiteralInfoBigint(bigint=json_int(json_field(object_, "bigint")))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class MemoryInfoAccess:
    """Reference access singleton."""

    access: destack._generated.mir.tree.type.Access
    kind: typing.Literal["access"] = "access"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_info(self)


@dataclass(frozen=True, slots=True)
class MemoryInfoSpace:
    """Concrete storage space singleton."""

    space: destack._generated.mir.tree.type.Space
    kind: typing.Literal["space"] = "space"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_info(self)


@dataclass(frozen=True, slots=True)
class MemoryInfoLifetime:
    """Lifetime singleton."""

    lifetime: destack._generated.mir.tree.lifetime.Lifetime
    kind: typing.Literal["lifetime"] = "lifetime"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_info(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_info(self)


"""Reflected memory singleton program type."""
MemoryInfo: typing.TypeAlias = MemoryInfoAccess | MemoryInfoSpace | MemoryInfoLifetime


def encode_memory_info(writer: BinaryWriter, value: MemoryInfo) -> None:
    """Encode one MemoryInfo."""
    if value.kind == "access":
        writer.write_unsigned(0)
        destack._generated.mir.tree.type.encode_access(writer, value.access)
    elif value.kind == "space":
        writer.write_unsigned(1)
        destack._generated.mir.tree.type.encode_space(writer, value.space)
    elif value.kind == "lifetime":
        writer.write_unsigned(2)
        destack._generated.mir.tree.lifetime.encode_lifetime(writer, value.lifetime)
    else:
        raise SerdeError("unknown enum variant")


def decode_memory_info(reader: BinaryReader) -> MemoryInfo:
    """Decode one MemoryInfo."""
    variant = reader.read_number()

    if variant == 0:
        access = destack._generated.mir.tree.type.decode_access(reader)

        return MemoryInfoAccess(access=access)
    elif variant == 1:
        space = destack._generated.mir.tree.type.decode_space(reader)

        return MemoryInfoSpace(space=space)
    elif variant == 2:
        lifetime = destack._generated.mir.tree.lifetime.decode_lifetime(reader)

        return MemoryInfoLifetime(lifetime=lifetime)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_memory_info(value: MemoryInfo) -> Json:
    """Return one JSON value for one MemoryInfo."""
    if value.kind == "access":
        return {
            "kind": "access",
            "access": destack._generated.mir.tree.type.to_json_access(value.access),
        }
    elif value.kind == "space":
        return {
            "kind": "space",
            "space": destack._generated.mir.tree.type.to_json_space(value.space),
        }
    elif value.kind == "lifetime":
        return {
            "kind": "lifetime",
            "lifetime": destack._generated.mir.tree.lifetime.to_json_lifetime(
                value.lifetime
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_memory_info(value: Json) -> MemoryInfo:
    """Return one MemoryInfo from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "access":
        return MemoryInfoAccess(
            access=destack._generated.mir.tree.type.from_json_access(
                json_field(object_, "access")
            )
        )
    elif kind == "space":
        return MemoryInfoSpace(
            space=destack._generated.mir.tree.type.from_json_space(
                json_field(object_, "space")
            )
        )
    elif kind == "lifetime":
        return MemoryInfoLifetime(
            lifetime=destack._generated.mir.tree.lifetime.from_json_lifetime(
                json_field(object_, "lifetime")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class GenericInfo:
    """Reflected generic declaration application."""

    # declaration name when one exists
    name: destack._generated.core.string.StringId | None
    # complete type arguments
    arguments: Sequence[destack._generated.program.type.TypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GenericInfo:
        """Decode one GenericInfo."""
        return decode_generic_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_info(self)

    @classmethod
    def from_json(cls, value: Json) -> GenericInfo:
        """Return one GenericInfo from one JSON value."""
        return from_json_generic_info(value)


def encode_generic_info(writer: BinaryWriter, value: GenericInfo) -> None:
    """Encode one GenericInfo."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.program.type.encode_type_id(writer, item_value_arguments_0)


def decode_generic_info(reader: BinaryReader) -> GenericInfo:
    """Decode one GenericInfo."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    arguments = [
        destack._generated.program.type.decode_type_id(reader)
        for _ in range(reader.read_number())
    ]

    return GenericInfo(
        name=name,
        arguments=arguments,
    )


def to_json_generic_info(value: GenericInfo) -> Json:
    """Return one JSON value for one GenericInfo."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        "arguments": [
            destack._generated.program.type.to_json_type_id(item_0)
            for item_0 in value.arguments
        ],
    }


def from_json_generic_info(value: Json) -> GenericInfo:
    """Return one GenericInfo from one JSON value."""
    object_ = json_object(value)

    return GenericInfo(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        arguments=[
            destack._generated.program.type.from_json_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class MemberInfo:
    """Reflected member type selection."""

    # owner type
    owner: destack._generated.program.type.TypeId
    # selected member name when one exists
    name: destack._generated.core.string.StringId | None
    # complete type arguments
    arguments: Sequence[destack._generated.program.type.TypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberInfo:
        """Decode one MemberInfo."""
        return decode_member_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_info(self)

    @classmethod
    def from_json(cls, value: Json) -> MemberInfo:
        """Return one MemberInfo from one JSON value."""
        return from_json_member_info(value)


def encode_member_info(writer: BinaryWriter, value: MemberInfo) -> None:
    """Encode one MemberInfo."""
    destack._generated.program.type.encode_type_id(writer, value.owner)
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.program.type.encode_type_id(writer, item_value_arguments_0)


def decode_member_info(reader: BinaryReader) -> MemberInfo:
    """Decode one MemberInfo."""
    owner = destack._generated.program.type.decode_type_id(reader)
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    arguments = [
        destack._generated.program.type.decode_type_id(reader)
        for _ in range(reader.read_number())
    ]

    return MemberInfo(
        owner=owner,
        name=name,
        arguments=arguments,
    )


def to_json_member_info(value: MemberInfo) -> Json:
    """Return one JSON value for one MemberInfo."""
    return {
        "owner": destack._generated.program.type.to_json_type_id(value.owner),
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        "arguments": [
            destack._generated.program.type.to_json_type_id(item_0)
            for item_0 in value.arguments
        ],
    }


def from_json_member_info(value: Json) -> MemberInfo:
    """Return one MemberInfo from one JSON value."""
    object_ = json_object(value)

    return MemberInfo(
        owner=destack._generated.program.type.from_json_type_id(
            json_field(object_, "owner")
        ),
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        arguments=[
            destack._generated.program.type.from_json_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FormInfo:
    """Reflected ownership or access form."""

    # form constructor
    form: Form
    # carried value type
    value: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FormInfo:
        """Decode one FormInfo."""
        return decode_form_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form_info(self)

    @classmethod
    def from_json(cls, value: Json) -> FormInfo:
        """Return one FormInfo from one JSON value."""
        return from_json_form_info(value)


def encode_form_info(writer: BinaryWriter, value: FormInfo) -> None:
    """Encode one FormInfo."""
    encode_form(writer, value.form)
    destack._generated.program.type.encode_type_id(writer, value.value)


def decode_form_info(reader: BinaryReader) -> FormInfo:
    """Decode one FormInfo."""
    form = decode_form(reader)
    value_ = destack._generated.program.type.decode_type_id(reader)

    return FormInfo(
        form=form,
        value=value_,
    )


def to_json_form_info(value: FormInfo) -> Json:
    """Return one JSON value for one FormInfo."""
    return {
        "form": to_json_form(value.form),
        "value": destack._generated.program.type.to_json_type_id(value.value),
    }


def from_json_form_info(value: Json) -> FormInfo:
    """Return one FormInfo from one JSON value."""
    object_ = json_object(value)

    return FormInfo(
        form=from_json_form(json_field(object_, "form")),
        value=destack._generated.program.type.from_json_type_id(
            json_field(object_, "value")
        ),
    )


@dataclass(frozen=True, slots=True)
class FormManaged:
    """Managed value."""

    kind: typing.Literal["managed"] = "managed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


@dataclass(frozen=True, slots=True)
class FormOwned:
    """Owned value."""

    kind: typing.Literal["owned"] = "owned"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


@dataclass(frozen=True, slots=True)
class FormBorrowed:
    """Borrowed value."""

    # borrow lifetime type
    lifetime: destack._generated.program.type.TypeId
    # borrow access type
    access: destack._generated.program.type.TypeId
    kind: typing.Literal["borrowed"] = "borrowed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


@dataclass(frozen=True, slots=True)
class FormRaw:
    """Raw pointer value."""

    kind: typing.Literal["raw"] = "raw"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


@dataclass(frozen=True, slots=True)
class FormPlaced:
    """Placed value."""

    # placement type
    place: destack._generated.program.type.TypeId
    kind: typing.Literal["placed"] = "placed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


@dataclass(frozen=True, slots=True)
class FormReadonly:
    """Readonly view."""

    kind: typing.Literal["readonly"] = "readonly"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


"""Reflected ownership or access form constructor."""
Form: typing.TypeAlias = (
    FormManaged | FormOwned | FormBorrowed | FormRaw | FormPlaced | FormReadonly
)


def encode_form(writer: BinaryWriter, value: Form) -> None:
    """Encode one Form."""
    if value.kind == "managed":
        writer.write_unsigned(0)
    elif value.kind == "owned":
        writer.write_unsigned(1)
    elif value.kind == "borrowed":
        writer.write_unsigned(2)
        destack._generated.program.type.encode_type_id(writer, value.lifetime)
        destack._generated.program.type.encode_type_id(writer, value.access)
    elif value.kind == "raw":
        writer.write_unsigned(3)
    elif value.kind == "placed":
        writer.write_unsigned(4)
        destack._generated.program.type.encode_type_id(writer, value.place)
    elif value.kind == "readonly":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_form(reader: BinaryReader) -> Form:
    """Decode one Form."""
    variant = reader.read_number()

    if variant == 0:
        return FormManaged()
    elif variant == 1:
        return FormOwned()
    elif variant == 2:
        lifetime = destack._generated.program.type.decode_type_id(reader)
        access = destack._generated.program.type.decode_type_id(reader)

        return FormBorrowed(
            lifetime=lifetime,
            access=access,
        )
    elif variant == 3:
        return FormRaw()
    elif variant == 4:
        place = destack._generated.program.type.decode_type_id(reader)

        return FormPlaced(
            place=place,
        )
    elif variant == 5:
        return FormReadonly()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_form(value: Form) -> Json:
    """Return one JSON value for one Form."""
    if value.kind == "managed":
        return {
            "kind": "managed",
        }
    elif value.kind == "owned":
        return {
            "kind": "owned",
        }
    elif value.kind == "borrowed":
        return {
            "kind": "borrowed",
            "lifetime": destack._generated.program.type.to_json_type_id(value.lifetime),
            "access": destack._generated.program.type.to_json_type_id(value.access),
        }
    elif value.kind == "raw":
        return {
            "kind": "raw",
        }
    elif value.kind == "placed":
        return {
            "kind": "placed",
            "place": destack._generated.program.type.to_json_type_id(value.place),
        }
    elif value.kind == "readonly":
        return {
            "kind": "readonly",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_form(value: Json) -> Form:
    """Return one Form from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "managed":
        return FormManaged()
    elif kind == "owned":
        return FormOwned()
    elif kind == "borrowed":
        return FormBorrowed(
            lifetime=destack._generated.program.type.from_json_type_id(
                json_field(object_, "lifetime")
            ),
            access=destack._generated.program.type.from_json_type_id(
                json_field(object_, "access")
            ),
        )
    elif kind == "raw":
        return FormRaw()
    elif kind == "placed":
        return FormPlaced(
            place=destack._generated.program.type.from_json_type_id(
                json_field(object_, "place")
            ),
        )
    elif kind == "readonly":
        return FormReadonly()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DynamicInfo:
    """Reflected erased dynamic value type."""

    # erased constraint type
    constraint: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DynamicInfo:
        """Decode one DynamicInfo."""
        return decode_dynamic_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_info(self)

    @classmethod
    def from_json(cls, value: Json) -> DynamicInfo:
        """Return one DynamicInfo from one JSON value."""
        return from_json_dynamic_info(value)


def encode_dynamic_info(writer: BinaryWriter, value: DynamicInfo) -> None:
    """Encode one DynamicInfo."""
    destack._generated.program.type.encode_type_id(writer, value.constraint)


def decode_dynamic_info(reader: BinaryReader) -> DynamicInfo:
    """Decode one DynamicInfo."""
    constraint = destack._generated.program.type.decode_type_id(reader)

    return DynamicInfo(
        constraint=constraint,
    )


def to_json_dynamic_info(value: DynamicInfo) -> Json:
    """Return one JSON value for one DynamicInfo."""
    return {
        "constraint": destack._generated.program.type.to_json_type_id(value.constraint),
    }


def from_json_dynamic_info(value: Json) -> DynamicInfo:
    """Return one DynamicInfo from one JSON value."""
    object_ = json_object(value)

    return DynamicInfo(
        constraint=destack._generated.program.type.from_json_type_id(
            json_field(object_, "constraint")
        ),
    )


@dataclass(frozen=True, slots=True)
class ArrayInfo:
    """Reflected homogeneous array type."""

    # element type
    element: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_array_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArrayInfo:
        """Decode one ArrayInfo."""
        return decode_array_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_array_info(self)

    @classmethod
    def from_json(cls, value: Json) -> ArrayInfo:
        """Return one ArrayInfo from one JSON value."""
        return from_json_array_info(value)


def encode_array_info(writer: BinaryWriter, value: ArrayInfo) -> None:
    """Encode one ArrayInfo."""
    destack._generated.program.type.encode_type_id(writer, value.element)


def decode_array_info(reader: BinaryReader) -> ArrayInfo:
    """Decode one ArrayInfo."""
    element = destack._generated.program.type.decode_type_id(reader)

    return ArrayInfo(
        element=element,
    )


def to_json_array_info(value: ArrayInfo) -> Json:
    """Return one JSON value for one ArrayInfo."""
    return {
        "element": destack._generated.program.type.to_json_type_id(value.element),
    }


def from_json_array_info(value: Json) -> ArrayInfo:
    """Return one ArrayInfo from one JSON value."""
    object_ = json_object(value)

    return ArrayInfo(
        element=destack._generated.program.type.from_json_type_id(
            json_field(object_, "element")
        ),
    )


@dataclass(frozen=True, slots=True)
class FixedArrayInfo:
    """Reflected fixed-length array type."""

    # element type
    element: destack._generated.program.type.TypeId
    # static count type
    count: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_fixed_array_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FixedArrayInfo:
        """Decode one FixedArrayInfo."""
        return decode_fixed_array_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_fixed_array_info(self)

    @classmethod
    def from_json(cls, value: Json) -> FixedArrayInfo:
        """Return one FixedArrayInfo from one JSON value."""
        return from_json_fixed_array_info(value)


def encode_fixed_array_info(writer: BinaryWriter, value: FixedArrayInfo) -> None:
    """Encode one FixedArrayInfo."""
    destack._generated.program.type.encode_type_id(writer, value.element)
    destack._generated.program.type.encode_type_id(writer, value.count)


def decode_fixed_array_info(reader: BinaryReader) -> FixedArrayInfo:
    """Decode one FixedArrayInfo."""
    element = destack._generated.program.type.decode_type_id(reader)
    count = destack._generated.program.type.decode_type_id(reader)

    return FixedArrayInfo(
        element=element,
        count=count,
    )


def to_json_fixed_array_info(value: FixedArrayInfo) -> Json:
    """Return one JSON value for one FixedArrayInfo."""
    return {
        "element": destack._generated.program.type.to_json_type_id(value.element),
        "count": destack._generated.program.type.to_json_type_id(value.count),
    }


def from_json_fixed_array_info(value: Json) -> FixedArrayInfo:
    """Return one FixedArrayInfo from one JSON value."""
    object_ = json_object(value)

    return FixedArrayInfo(
        element=destack._generated.program.type.from_json_type_id(
            json_field(object_, "element")
        ),
        count=destack._generated.program.type.from_json_type_id(
            json_field(object_, "count")
        ),
    )


@dataclass(frozen=True, slots=True)
class RangeInfo:
    """Reflected scalar interval type."""

    # inclusive lower bound
    start: LiteralInfo | None
    # upper bound
    end: LiteralInfo | None
    # whether the upper bound is included
    is_inclusive: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_range_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RangeInfo:
        """Decode one RangeInfo."""
        return decode_range_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_range_info(self)

    @classmethod
    def from_json(cls, value: Json) -> RangeInfo:
        """Return one RangeInfo from one JSON value."""
        return from_json_range_info(value)


def encode_range_info(writer: BinaryWriter, value: RangeInfo) -> None:
    """Encode one RangeInfo."""
    if value.start is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_literal_info(writer, value.start)
    if value.end is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_literal_info(writer, value.end)
    writer.write_bool(value.is_inclusive)


def decode_range_info(reader: BinaryReader) -> RangeInfo:
    """Decode one RangeInfo."""
    start = reader.read_option(lambda: decode_literal_info(reader))
    end = reader.read_option(lambda: decode_literal_info(reader))
    is_inclusive = reader.read_bool()

    return RangeInfo(
        start=start,
        end=end,
        is_inclusive=is_inclusive,
    )


def to_json_range_info(value: RangeInfo) -> Json:
    """Return one JSON value for one RangeInfo."""
    return {
        **({} if value.start is None else {"start": to_json_literal_info(value.start)}),
        **({} if value.end is None else {"end": to_json_literal_info(value.end)}),
        "isInclusive": value.is_inclusive,
    }


def from_json_range_info(value: Json) -> RangeInfo:
    """Return one RangeInfo from one JSON value."""
    object_ = json_object(value)

    return RangeInfo(
        start=json_optional(
            object_, "start", lambda value: from_json_literal_info(value)
        ),
        end=json_optional(object_, "end", lambda value: from_json_literal_info(value)),
        is_inclusive=json_bool(json_field(object_, "isInclusive")),
    )


@dataclass(frozen=True, slots=True)
class SliceInfo:
    """Reflected runtime-length homogeneous view type."""

    # element type
    element: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_slice_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SliceInfo:
        """Decode one SliceInfo."""
        return decode_slice_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_slice_info(self)

    @classmethod
    def from_json(cls, value: Json) -> SliceInfo:
        """Return one SliceInfo from one JSON value."""
        return from_json_slice_info(value)


def encode_slice_info(writer: BinaryWriter, value: SliceInfo) -> None:
    """Encode one SliceInfo."""
    destack._generated.program.type.encode_type_id(writer, value.element)


def decode_slice_info(reader: BinaryReader) -> SliceInfo:
    """Decode one SliceInfo."""
    element = destack._generated.program.type.decode_type_id(reader)

    return SliceInfo(
        element=element,
    )


def to_json_slice_info(value: SliceInfo) -> Json:
    """Return one JSON value for one SliceInfo."""
    return {
        "element": destack._generated.program.type.to_json_type_id(value.element),
    }


def from_json_slice_info(value: Json) -> SliceInfo:
    """Return one SliceInfo from one JSON value."""
    object_ = json_object(value)

    return SliceInfo(
        element=destack._generated.program.type.from_json_type_id(
            json_field(object_, "element")
        ),
    )


@dataclass(frozen=True, slots=True)
class TupleInfo:
    """Reflected tuple type."""

    # tuple elements
    elements: Sequence[TypeElementInfo]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tuple_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TupleInfo:
        """Decode one TupleInfo."""
        return decode_tuple_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tuple_info(self)

    @classmethod
    def from_json(cls, value: Json) -> TupleInfo:
        """Return one TupleInfo from one JSON value."""
        return from_json_tuple_info(value)


def encode_tuple_info(writer: BinaryWriter, value: TupleInfo) -> None:
    """Encode one TupleInfo."""
    writer.write_unsigned(len(value.elements))
    for item_value_elements_0 in value.elements:
        encode_type_element_info(writer, item_value_elements_0)


def decode_tuple_info(reader: BinaryReader) -> TupleInfo:
    """Decode one TupleInfo."""
    elements = [decode_type_element_info(reader) for _ in range(reader.read_number())]

    return TupleInfo(
        elements=elements,
    )


def to_json_tuple_info(value: TupleInfo) -> Json:
    """Return one JSON value for one TupleInfo."""
    return {
        "elements": [to_json_type_element_info(item_0) for item_0 in value.elements],
    }


def from_json_tuple_info(value: Json) -> TupleInfo:
    """Return one TupleInfo from one JSON value."""
    object_ = json_object(value)

    return TupleInfo(
        elements=[
            from_json_type_element_info(item_0)
            for item_0 in json_array(json_field(object_, "elements"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_element_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeElementInfo:
        """Decode one TypeElementInfo."""
        return decode_type_element_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_element_info(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeElementInfo:
        """Return one TypeElementInfo from one JSON value."""
        return from_json_type_element_info(value)


def encode_type_element_info(writer: BinaryWriter, value: TypeElementInfo) -> None:
    """Encode one TypeElementInfo."""
    if value.label is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.label)
    destack._generated.program.type.encode_type_id(writer, value.ty)
    writer.write_bool(value.is_optional)
    writer.write_bool(value.is_readonly)
    writer.write_bool(value.is_rest)


def decode_type_element_info(reader: BinaryReader) -> TypeElementInfo:
    """Decode one TypeElementInfo."""
    label = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    ty = destack._generated.program.type.decode_type_id(reader)
    is_optional = reader.read_bool()
    is_readonly = reader.read_bool()
    is_rest = reader.read_bool()

    return TypeElementInfo(
        label=label,
        ty=ty,
        is_optional=is_optional,
        is_readonly=is_readonly,
        is_rest=is_rest,
    )


def to_json_type_element_info(value: TypeElementInfo) -> Json:
    """Return one JSON value for one TypeElementInfo."""
    return {
        **(
            {}
            if value.label is None
            else {
                "label": destack._generated.core.string.to_json_string_id(value.label)
            }
        ),
        "ty": destack._generated.program.type.to_json_type_id(value.ty),
        "isOptional": value.is_optional,
        "isReadonly": value.is_readonly,
        "isRest": value.is_rest,
    }


def from_json_type_element_info(value: Json) -> TypeElementInfo:
    """Return one TypeElementInfo from one JSON value."""
    object_ = json_object(value)

    return TypeElementInfo(
        label=json_optional(
            object_,
            "label",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        ty=destack._generated.program.type.from_json_type_id(json_field(object_, "ty")),
        is_optional=json_bool(json_field(object_, "isOptional")),
        is_readonly=json_bool(json_field(object_, "isReadonly")),
        is_rest=json_bool(json_field(object_, "isRest")),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_shape_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ShapeInfo:
        """Decode one ShapeInfo."""
        return decode_shape_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_shape_info(self)

    @classmethod
    def from_json(cls, value: Json) -> ShapeInfo:
        """Return one ShapeInfo from one JSON value."""
        return from_json_shape_info(value)


def encode_shape_info(writer: BinaryWriter, value: ShapeInfo) -> None:
    """Encode one ShapeInfo."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_type_field_info(writer, item_value_fields_0)
    writer.write_unsigned(len(value.call_signatures))
    for item_value_call_signatures_0 in value.call_signatures:
        destack._generated.program.type.encode_type_id(
            writer, item_value_call_signatures_0
        )
    writer.write_unsigned(len(value.construct_signatures))
    for item_value_construct_signatures_0 in value.construct_signatures:
        destack._generated.program.type.encode_type_id(
            writer, item_value_construct_signatures_0
        )
    writer.write_unsigned(len(value.index_signatures))
    for item_value_index_signatures_0 in value.index_signatures:
        encode_type_index_signature_info(writer, item_value_index_signatures_0)


def decode_shape_info(reader: BinaryReader) -> ShapeInfo:
    """Decode one ShapeInfo."""
    fields = [decode_type_field_info(reader) for _ in range(reader.read_number())]
    call_signatures = [
        destack._generated.program.type.decode_type_id(reader)
        for _ in range(reader.read_number())
    ]
    construct_signatures = [
        destack._generated.program.type.decode_type_id(reader)
        for _ in range(reader.read_number())
    ]
    index_signatures = [
        decode_type_index_signature_info(reader) for _ in range(reader.read_number())
    ]

    return ShapeInfo(
        fields=fields,
        call_signatures=call_signatures,
        construct_signatures=construct_signatures,
        index_signatures=index_signatures,
    )


def to_json_shape_info(value: ShapeInfo) -> Json:
    """Return one JSON value for one ShapeInfo."""
    return {
        "fields": [to_json_type_field_info(item_0) for item_0 in value.fields],
        "callSignatures": [
            destack._generated.program.type.to_json_type_id(item_0)
            for item_0 in value.call_signatures
        ],
        "constructSignatures": [
            destack._generated.program.type.to_json_type_id(item_0)
            for item_0 in value.construct_signatures
        ],
        "indexSignatures": [
            to_json_type_index_signature_info(item_0)
            for item_0 in value.index_signatures
        ],
    }


def from_json_shape_info(value: Json) -> ShapeInfo:
    """Return one ShapeInfo from one JSON value."""
    object_ = json_object(value)

    return ShapeInfo(
        fields=[
            from_json_type_field_info(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
        call_signatures=[
            destack._generated.program.type.from_json_type_id(item_0)
            for item_0 in json_array(json_field(object_, "callSignatures"))
        ],
        construct_signatures=[
            destack._generated.program.type.from_json_type_id(item_0)
            for item_0 in json_array(json_field(object_, "constructSignatures"))
        ],
        index_signatures=[
            from_json_type_index_signature_info(item_0)
            for item_0 in json_array(json_field(object_, "indexSignatures"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_field_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeFieldInfo:
        """Decode one TypeFieldInfo."""
        return decode_type_field_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_field_info(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeFieldInfo:
        """Return one TypeFieldInfo from one JSON value."""
        return from_json_type_field_info(value)


def encode_type_field_info(writer: BinaryWriter, value: TypeFieldInfo) -> None:
    """Encode one TypeFieldInfo."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    destack._generated.program.type.encode_type_id(writer, value.ty)
    writer.write_bool(value.is_optional)
    writer.write_bool(value.is_readonly)


def decode_type_field_info(reader: BinaryReader) -> TypeFieldInfo:
    """Decode one TypeFieldInfo."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    ty = destack._generated.program.type.decode_type_id(reader)
    is_optional = reader.read_bool()
    is_readonly = reader.read_bool()

    return TypeFieldInfo(
        name=name,
        ty=ty,
        is_optional=is_optional,
        is_readonly=is_readonly,
    )


def to_json_type_field_info(value: TypeFieldInfo) -> Json:
    """Return one JSON value for one TypeFieldInfo."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        "ty": destack._generated.program.type.to_json_type_id(value.ty),
        "isOptional": value.is_optional,
        "isReadonly": value.is_readonly,
    }


def from_json_type_field_info(value: Json) -> TypeFieldInfo:
    """Return one TypeFieldInfo from one JSON value."""
    object_ = json_object(value)

    return TypeFieldInfo(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        ty=destack._generated.program.type.from_json_type_id(json_field(object_, "ty")),
        is_optional=json_bool(json_field(object_, "isOptional")),
        is_readonly=json_bool(json_field(object_, "isReadonly")),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_index_signature_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeIndexSignatureInfo:
        """Decode one TypeIndexSignatureInfo."""
        return decode_type_index_signature_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_index_signature_info(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeIndexSignatureInfo:
        """Return one TypeIndexSignatureInfo from one JSON value."""
        return from_json_type_index_signature_info(value)


def encode_type_index_signature_info(
    writer: BinaryWriter, value: TypeIndexSignatureInfo
) -> None:
    """Encode one TypeIndexSignatureInfo."""
    destack._generated.program.type.encode_type_id(writer, value.key_type)
    destack._generated.program.type.encode_type_id(writer, value.value_type)
    writer.write_bool(value.is_optional)
    writer.write_bool(value.is_readonly)


def decode_type_index_signature_info(reader: BinaryReader) -> TypeIndexSignatureInfo:
    """Decode one TypeIndexSignatureInfo."""
    key_type = destack._generated.program.type.decode_type_id(reader)
    value_type = destack._generated.program.type.decode_type_id(reader)
    is_optional = reader.read_bool()
    is_readonly = reader.read_bool()

    return TypeIndexSignatureInfo(
        key_type=key_type,
        value_type=value_type,
        is_optional=is_optional,
        is_readonly=is_readonly,
    )


def to_json_type_index_signature_info(value: TypeIndexSignatureInfo) -> Json:
    """Return one JSON value for one TypeIndexSignatureInfo."""
    return {
        "keyType": destack._generated.program.type.to_json_type_id(value.key_type),
        "valueType": destack._generated.program.type.to_json_type_id(value.value_type),
        "isOptional": value.is_optional,
        "isReadonly": value.is_readonly,
    }


def from_json_type_index_signature_info(value: Json) -> TypeIndexSignatureInfo:
    """Return one TypeIndexSignatureInfo from one JSON value."""
    object_ = json_object(value)

    return TypeIndexSignatureInfo(
        key_type=destack._generated.program.type.from_json_type_id(
            json_field(object_, "keyType")
        ),
        value_type=destack._generated.program.type.from_json_type_id(
            json_field(object_, "valueType")
        ),
        is_optional=json_bool(json_field(object_, "isOptional")),
        is_readonly=json_bool(json_field(object_, "isReadonly")),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_signature_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionSignatureInfo:
        """Decode one FunctionSignatureInfo."""
        return decode_function_signature_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_signature_info(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionSignatureInfo:
        """Return one FunctionSignatureInfo from one JSON value."""
        return from_json_function_signature_info(value)


def encode_function_signature_info(
    writer: BinaryWriter, value: FunctionSignatureInfo
) -> None:
    """Encode one FunctionSignatureInfo."""
    if value.this_parameter is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.program.type.encode_type_id(writer, value.this_parameter)
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        encode_function_parameter_info(writer, item_value_parameters_0)
    if value.return_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.program.type.encode_type_id(writer, value.return_type)
    writer.write_bool(value.is_async)
    writer.write_bool(value.is_generator)


def decode_function_signature_info(reader: BinaryReader) -> FunctionSignatureInfo:
    """Decode one FunctionSignatureInfo."""
    this_parameter = reader.read_option(
        lambda: destack._generated.program.type.decode_type_id(reader)
    )
    parameters = [
        decode_function_parameter_info(reader) for _ in range(reader.read_number())
    ]
    return_type = reader.read_option(
        lambda: destack._generated.program.type.decode_type_id(reader)
    )
    is_async = reader.read_bool()
    is_generator = reader.read_bool()

    return FunctionSignatureInfo(
        this_parameter=this_parameter,
        parameters=parameters,
        return_type=return_type,
        is_async=is_async,
        is_generator=is_generator,
    )


def to_json_function_signature_info(value: FunctionSignatureInfo) -> Json:
    """Return one JSON value for one FunctionSignatureInfo."""
    return {
        **(
            {}
            if value.this_parameter is None
            else {
                "thisParameter": destack._generated.program.type.to_json_type_id(
                    value.this_parameter
                )
            }
        ),
        "parameters": [
            to_json_function_parameter_info(item_0) for item_0 in value.parameters
        ],
        **(
            {}
            if value.return_type is None
            else {
                "returnType": destack._generated.program.type.to_json_type_id(
                    value.return_type
                )
            }
        ),
        "isAsync": value.is_async,
        "isGenerator": value.is_generator,
    }


def from_json_function_signature_info(value: Json) -> FunctionSignatureInfo:
    """Return one FunctionSignatureInfo from one JSON value."""
    object_ = json_object(value)

    return FunctionSignatureInfo(
        this_parameter=json_optional(
            object_,
            "thisParameter",
            lambda value: destack._generated.program.type.from_json_type_id(value),
        ),
        parameters=[
            from_json_function_parameter_info(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        return_type=json_optional(
            object_,
            "returnType",
            lambda value: destack._generated.program.type.from_json_type_id(value),
        ),
        is_async=json_bool(json_field(object_, "isAsync")),
        is_generator=json_bool(json_field(object_, "isGenerator")),
    )


@dataclass(frozen=True, slots=True)
class FunctionParameterInfo:
    """Reflected runtime parameter in a function type."""

    # parameter type
    ty: destack._generated.program.type.TypeId
    # whether the parameter may be omitted
    is_optional: bool
    # whether the parameter captures remaining arguments
    is_rest: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_parameter_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionParameterInfo:
        """Decode one FunctionParameterInfo."""
        return decode_function_parameter_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_parameter_info(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionParameterInfo:
        """Return one FunctionParameterInfo from one JSON value."""
        return from_json_function_parameter_info(value)


def encode_function_parameter_info(
    writer: BinaryWriter, value: FunctionParameterInfo
) -> None:
    """Encode one FunctionParameterInfo."""
    destack._generated.program.type.encode_type_id(writer, value.ty)
    writer.write_bool(value.is_optional)
    writer.write_bool(value.is_rest)


def decode_function_parameter_info(reader: BinaryReader) -> FunctionParameterInfo:
    """Decode one FunctionParameterInfo."""
    ty = destack._generated.program.type.decode_type_id(reader)
    is_optional = reader.read_bool()
    is_rest = reader.read_bool()

    return FunctionParameterInfo(
        ty=ty,
        is_optional=is_optional,
        is_rest=is_rest,
    )


def to_json_function_parameter_info(value: FunctionParameterInfo) -> Json:
    """Return one JSON value for one FunctionParameterInfo."""
    return {
        "ty": destack._generated.program.type.to_json_type_id(value.ty),
        "isOptional": value.is_optional,
        "isRest": value.is_rest,
    }


def from_json_function_parameter_info(value: Json) -> FunctionParameterInfo:
    """Return one FunctionParameterInfo from one JSON value."""
    object_ = json_object(value)

    return FunctionParameterInfo(
        ty=destack._generated.program.type.from_json_type_id(json_field(object_, "ty")),
        is_optional=json_bool(json_field(object_, "isOptional")),
        is_rest=json_bool(json_field(object_, "isRest")),
    )


@dataclass(frozen=True, slots=True)
class FunctionTypeInfo:
    """Reflected fat callable value type."""

    # function signature type
    signature: destack._generated.program.type.TypeId
    # captured environment type
    environment: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_type_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionTypeInfo:
        """Decode one FunctionTypeInfo."""
        return decode_function_type_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_type_info(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionTypeInfo:
        """Return one FunctionTypeInfo from one JSON value."""
        return from_json_function_type_info(value)


def encode_function_type_info(writer: BinaryWriter, value: FunctionTypeInfo) -> None:
    """Encode one FunctionTypeInfo."""
    destack._generated.program.type.encode_type_id(writer, value.signature)
    destack._generated.program.type.encode_type_id(writer, value.environment)


def decode_function_type_info(reader: BinaryReader) -> FunctionTypeInfo:
    """Decode one FunctionTypeInfo."""
    signature = destack._generated.program.type.decode_type_id(reader)
    environment = destack._generated.program.type.decode_type_id(reader)

    return FunctionTypeInfo(
        signature=signature,
        environment=environment,
    )


def to_json_function_type_info(value: FunctionTypeInfo) -> Json:
    """Return one JSON value for one FunctionTypeInfo."""
    return {
        "signature": destack._generated.program.type.to_json_type_id(value.signature),
        "environment": destack._generated.program.type.to_json_type_id(
            value.environment
        ),
    }


def from_json_function_type_info(value: Json) -> FunctionTypeInfo:
    """Return one FunctionTypeInfo from one JSON value."""
    object_ = json_object(value)

    return FunctionTypeInfo(
        signature=destack._generated.program.type.from_json_type_id(
            json_field(object_, "signature")
        ),
        environment=destack._generated.program.type.from_json_type_id(
            json_field(object_, "environment")
        ),
    )


@dataclass(frozen=True, slots=True)
class FunctionPointerInfo:
    """Reflected thin callable value type."""

    # function signature type
    signature: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_pointer_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionPointerInfo:
        """Decode one FunctionPointerInfo."""
        return decode_function_pointer_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_pointer_info(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionPointerInfo:
        """Return one FunctionPointerInfo from one JSON value."""
        return from_json_function_pointer_info(value)


def encode_function_pointer_info(
    writer: BinaryWriter, value: FunctionPointerInfo
) -> None:
    """Encode one FunctionPointerInfo."""
    destack._generated.program.type.encode_type_id(writer, value.signature)


def decode_function_pointer_info(reader: BinaryReader) -> FunctionPointerInfo:
    """Decode one FunctionPointerInfo."""
    signature = destack._generated.program.type.decode_type_id(reader)

    return FunctionPointerInfo(
        signature=signature,
    )


def to_json_function_pointer_info(value: FunctionPointerInfo) -> Json:
    """Return one JSON value for one FunctionPointerInfo."""
    return {
        "signature": destack._generated.program.type.to_json_type_id(value.signature),
    }


def from_json_function_pointer_info(value: Json) -> FunctionPointerInfo:
    """Return one FunctionPointerInfo from one JSON value."""
    object_ = json_object(value)

    return FunctionPointerInfo(
        signature=destack._generated.program.type.from_json_type_id(
            json_field(object_, "signature")
        ),
    )


@dataclass(frozen=True, slots=True)
class UnionInfo:
    """Reflected union type."""

    # union elements
    elements: Sequence[destack._generated.program.type.TypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_union_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> UnionInfo:
        """Decode one UnionInfo."""
        return decode_union_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_union_info(self)

    @classmethod
    def from_json(cls, value: Json) -> UnionInfo:
        """Return one UnionInfo from one JSON value."""
        return from_json_union_info(value)


def encode_union_info(writer: BinaryWriter, value: UnionInfo) -> None:
    """Encode one UnionInfo."""
    writer.write_unsigned(len(value.elements))
    for item_value_elements_0 in value.elements:
        destack._generated.program.type.encode_type_id(writer, item_value_elements_0)


def decode_union_info(reader: BinaryReader) -> UnionInfo:
    """Decode one UnionInfo."""
    elements = [
        destack._generated.program.type.decode_type_id(reader)
        for _ in range(reader.read_number())
    ]

    return UnionInfo(
        elements=elements,
    )


def to_json_union_info(value: UnionInfo) -> Json:
    """Return one JSON value for one UnionInfo."""
    return {
        "elements": [
            destack._generated.program.type.to_json_type_id(item_0)
            for item_0 in value.elements
        ],
    }


def from_json_union_info(value: Json) -> UnionInfo:
    """Return one UnionInfo from one JSON value."""
    object_ = json_object(value)

    return UnionInfo(
        elements=[
            destack._generated.program.type.from_json_type_id(item_0)
            for item_0 in json_array(json_field(object_, "elements"))
        ],
    )


@dataclass(frozen=True, slots=True)
class IntersectionInfo:
    """Reflected intersection type."""

    # intersection elements
    elements: Sequence[destack._generated.program.type.TypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_intersection_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IntersectionInfo:
        """Decode one IntersectionInfo."""
        return decode_intersection_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_intersection_info(self)

    @classmethod
    def from_json(cls, value: Json) -> IntersectionInfo:
        """Return one IntersectionInfo from one JSON value."""
        return from_json_intersection_info(value)


def encode_intersection_info(writer: BinaryWriter, value: IntersectionInfo) -> None:
    """Encode one IntersectionInfo."""
    writer.write_unsigned(len(value.elements))
    for item_value_elements_0 in value.elements:
        destack._generated.program.type.encode_type_id(writer, item_value_elements_0)


def decode_intersection_info(reader: BinaryReader) -> IntersectionInfo:
    """Decode one IntersectionInfo."""
    elements = [
        destack._generated.program.type.decode_type_id(reader)
        for _ in range(reader.read_number())
    ]

    return IntersectionInfo(
        elements=elements,
    )


def to_json_intersection_info(value: IntersectionInfo) -> Json:
    """Return one JSON value for one IntersectionInfo."""
    return {
        "elements": [
            destack._generated.program.type.to_json_type_id(item_0)
            for item_0 in value.elements
        ],
    }


def from_json_intersection_info(value: Json) -> IntersectionInfo:
    """Return one IntersectionInfo from one JSON value."""
    object_ = json_object(value)

    return IntersectionInfo(
        elements=[
            destack._generated.program.type.from_json_type_id(item_0)
            for item_0 in json_array(json_field(object_, "elements"))
        ],
    )


@dataclass(frozen=True, slots=True)
class NewtypeInfo:
    """Reflected transparent nominal type."""

    # backing type
    backing: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_newtype_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NewtypeInfo:
        """Decode one NewtypeInfo."""
        return decode_newtype_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_newtype_info(self)

    @classmethod
    def from_json(cls, value: Json) -> NewtypeInfo:
        """Return one NewtypeInfo from one JSON value."""
        return from_json_newtype_info(value)


def encode_newtype_info(writer: BinaryWriter, value: NewtypeInfo) -> None:
    """Encode one NewtypeInfo."""
    destack._generated.program.type.encode_type_id(writer, value.backing)


def decode_newtype_info(reader: BinaryReader) -> NewtypeInfo:
    """Decode one NewtypeInfo."""
    backing = destack._generated.program.type.decode_type_id(reader)

    return NewtypeInfo(
        backing=backing,
    )


def to_json_newtype_info(value: NewtypeInfo) -> Json:
    """Return one JSON value for one NewtypeInfo."""
    return {
        "backing": destack._generated.program.type.to_json_type_id(value.backing),
    }


def from_json_newtype_info(value: Json) -> NewtypeInfo:
    """Return one NewtypeInfo from one JSON value."""
    object_ = json_object(value)

    return NewtypeInfo(
        backing=destack._generated.program.type.from_json_type_id(
            json_field(object_, "backing")
        ),
    )


@dataclass(frozen=True, slots=True)
class StructInfo:
    """Reflected struct declaration type."""

    # struct fields
    fields: Sequence[TypeFieldInfo]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_struct_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StructInfo:
        """Decode one StructInfo."""
        return decode_struct_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_struct_info(self)

    @classmethod
    def from_json(cls, value: Json) -> StructInfo:
        """Return one StructInfo from one JSON value."""
        return from_json_struct_info(value)


def encode_struct_info(writer: BinaryWriter, value: StructInfo) -> None:
    """Encode one StructInfo."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_type_field_info(writer, item_value_fields_0)


def decode_struct_info(reader: BinaryReader) -> StructInfo:
    """Decode one StructInfo."""
    fields = [decode_type_field_info(reader) for _ in range(reader.read_number())]

    return StructInfo(
        fields=fields,
    )


def to_json_struct_info(value: StructInfo) -> Json:
    """Return one JSON value for one StructInfo."""
    return {
        "fields": [to_json_type_field_info(item_0) for item_0 in value.fields],
    }


def from_json_struct_info(value: Json) -> StructInfo:
    """Return one StructInfo from one JSON value."""
    object_ = json_object(value)

    return StructInfo(
        fields=[
            from_json_type_field_info(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ClassInfo:
    """Reflected class declaration type."""

    # class fields
    fields: Sequence[TypeFieldInfo]
    # class members
    members: Sequence[MemberInfo]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_class_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ClassInfo:
        """Decode one ClassInfo."""
        return decode_class_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_class_info(self)

    @classmethod
    def from_json(cls, value: Json) -> ClassInfo:
        """Return one ClassInfo from one JSON value."""
        return from_json_class_info(value)


def encode_class_info(writer: BinaryWriter, value: ClassInfo) -> None:
    """Encode one ClassInfo."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_type_field_info(writer, item_value_fields_0)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        encode_member_info(writer, item_value_members_0)


def decode_class_info(reader: BinaryReader) -> ClassInfo:
    """Decode one ClassInfo."""
    fields = [decode_type_field_info(reader) for _ in range(reader.read_number())]
    members = [decode_member_info(reader) for _ in range(reader.read_number())]

    return ClassInfo(
        fields=fields,
        members=members,
    )


def to_json_class_info(value: ClassInfo) -> Json:
    """Return one JSON value for one ClassInfo."""
    return {
        "fields": [to_json_type_field_info(item_0) for item_0 in value.fields],
        "members": [to_json_member_info(item_0) for item_0 in value.members],
    }


def from_json_class_info(value: Json) -> ClassInfo:
    """Return one ClassInfo from one JSON value."""
    object_ = json_object(value)

    return ClassInfo(
        fields=[
            from_json_type_field_info(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
        members=[
            from_json_member_info(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


@dataclass(frozen=True, slots=True)
class InterfaceInfo:
    """Reflected interface declaration type."""

    # whether the interface is nominal
    is_nominal: bool
    # interface members
    members: Sequence[MemberInfo]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_interface_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InterfaceInfo:
        """Decode one InterfaceInfo."""
        return decode_interface_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_interface_info(self)

    @classmethod
    def from_json(cls, value: Json) -> InterfaceInfo:
        """Return one InterfaceInfo from one JSON value."""
        return from_json_interface_info(value)


def encode_interface_info(writer: BinaryWriter, value: InterfaceInfo) -> None:
    """Encode one InterfaceInfo."""
    writer.write_bool(value.is_nominal)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        encode_member_info(writer, item_value_members_0)


def decode_interface_info(reader: BinaryReader) -> InterfaceInfo:
    """Decode one InterfaceInfo."""
    is_nominal = reader.read_bool()
    members = [decode_member_info(reader) for _ in range(reader.read_number())]

    return InterfaceInfo(
        is_nominal=is_nominal,
        members=members,
    )


def to_json_interface_info(value: InterfaceInfo) -> Json:
    """Return one JSON value for one InterfaceInfo."""
    return {
        "isNominal": value.is_nominal,
        "members": [to_json_member_info(item_0) for item_0 in value.members],
    }


def from_json_interface_info(value: Json) -> InterfaceInfo:
    """Return one InterfaceInfo from one JSON value."""
    object_ = json_object(value)

    return InterfaceInfo(
        is_nominal=json_bool(json_field(object_, "isNominal")),
        members=[
            from_json_member_info(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


@dataclass(frozen=True, slots=True)
class VariantInfo:
    """Reflected variant declaration type."""

    # variant cases
    cases: Sequence[VariantCaseInfo]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_variant_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantInfo:
        """Decode one VariantInfo."""
        return decode_variant_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_variant_info(self)

    @classmethod
    def from_json(cls, value: Json) -> VariantInfo:
        """Return one VariantInfo from one JSON value."""
        return from_json_variant_info(value)


def encode_variant_info(writer: BinaryWriter, value: VariantInfo) -> None:
    """Encode one VariantInfo."""
    writer.write_unsigned(len(value.cases))
    for item_value_cases_0 in value.cases:
        encode_variant_case_info(writer, item_value_cases_0)


def decode_variant_info(reader: BinaryReader) -> VariantInfo:
    """Decode one VariantInfo."""
    cases = [decode_variant_case_info(reader) for _ in range(reader.read_number())]

    return VariantInfo(
        cases=cases,
    )


def to_json_variant_info(value: VariantInfo) -> Json:
    """Return one JSON value for one VariantInfo."""
    return {
        "cases": [to_json_variant_case_info(item_0) for item_0 in value.cases],
    }


def from_json_variant_info(value: Json) -> VariantInfo:
    """Return one VariantInfo from one JSON value."""
    object_ = json_object(value)

    return VariantInfo(
        cases=[
            from_json_variant_case_info(item_0)
            for item_0 in json_array(json_field(object_, "cases"))
        ],
    )


@dataclass(frozen=True, slots=True)
class VariantCaseInfo:
    """Reflected variant case."""

    # case name when one exists
    name: destack._generated.core.string.StringId | None
    # case payload type
    ty: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_variant_case_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantCaseInfo:
        """Decode one VariantCaseInfo."""
        return decode_variant_case_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_variant_case_info(self)

    @classmethod
    def from_json(cls, value: Json) -> VariantCaseInfo:
        """Return one VariantCaseInfo from one JSON value."""
        return from_json_variant_case_info(value)


def encode_variant_case_info(writer: BinaryWriter, value: VariantCaseInfo) -> None:
    """Encode one VariantCaseInfo."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    destack._generated.program.type.encode_type_id(writer, value.ty)


def decode_variant_case_info(reader: BinaryReader) -> VariantCaseInfo:
    """Decode one VariantCaseInfo."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    ty = destack._generated.program.type.decode_type_id(reader)

    return VariantCaseInfo(
        name=name,
        ty=ty,
    )


def to_json_variant_case_info(value: VariantCaseInfo) -> Json:
    """Return one JSON value for one VariantCaseInfo."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        "ty": destack._generated.program.type.to_json_type_id(value.ty),
    }


def from_json_variant_case_info(value: Json) -> VariantCaseInfo:
    """Return one VariantCaseInfo from one JSON value."""
    object_ = json_object(value)

    return VariantCaseInfo(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        ty=destack._generated.program.type.from_json_type_id(json_field(object_, "ty")),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionInfo:
        """Decode one FunctionInfo."""
        return decode_function_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_info(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionInfo:
        """Return one FunctionInfo from one JSON value."""
        return from_json_function_info(value)


def encode_function_info(writer: BinaryWriter, value: FunctionInfo) -> None:
    """Encode one FunctionInfo."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    if value.module is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
    destack._generated.program.type.encode_type_id(writer, value.signature)
    if value.environment is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.program.type.encode_type_id(writer, value.environment)


def decode_function_info(reader: BinaryReader) -> FunctionInfo:
    """Decode one FunctionInfo."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    module = reader.read_option(
        lambda: destack._generated.source.file.model.module.decode_module_id(reader)
    )
    signature = destack._generated.program.type.decode_type_id(reader)
    environment = reader.read_option(
        lambda: destack._generated.program.type.decode_type_id(reader)
    )

    return FunctionInfo(
        name=name,
        module=module,
        signature=signature,
        environment=environment,
    )


def to_json_function_info(value: FunctionInfo) -> Json:
    """Return one JSON value for one FunctionInfo."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        **(
            {}
            if value.module is None
            else {
                "module": destack._generated.source.file.model.module.to_json_module_id(
                    value.module
                )
            }
        ),
        "signature": destack._generated.program.type.to_json_type_id(value.signature),
        **(
            {}
            if value.environment is None
            else {
                "environment": destack._generated.program.type.to_json_type_id(
                    value.environment
                )
            }
        ),
    }


def from_json_function_info(value: Json) -> FunctionInfo:
    """Return one FunctionInfo from one JSON value."""
    object_ = json_object(value)

    return FunctionInfo(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        module=json_optional(
            object_,
            "module",
            lambda value: (
                destack._generated.source.file.model.module.from_json_module_id(value)
            ),
        ),
        signature=destack._generated.program.type.from_json_type_id(
            json_field(object_, "signature")
        ),
        environment=json_optional(
            object_,
            "environment",
            lambda value: destack._generated.program.type.from_json_type_id(value),
        ),
    )


@dataclass(frozen=True, slots=True)
class FrameInfo:
    """Reflected executable frame layout."""

    # function owning this frame
    function: destack._generated.program.function.FunctionId
    # frame slots
    slots: Sequence[FrameSlotInfo]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_frame_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameInfo:
        """Decode one FrameInfo."""
        return decode_frame_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_frame_info(self)

    @classmethod
    def from_json(cls, value: Json) -> FrameInfo:
        """Return one FrameInfo from one JSON value."""
        return from_json_frame_info(value)


def encode_frame_info(writer: BinaryWriter, value: FrameInfo) -> None:
    """Encode one FrameInfo."""
    destack._generated.program.function.encode_function_id(writer, value.function)
    writer.write_unsigned(len(value.slots))
    for item_value_slots_0 in value.slots:
        encode_frame_slot_info(writer, item_value_slots_0)


def decode_frame_info(reader: BinaryReader) -> FrameInfo:
    """Decode one FrameInfo."""
    function = destack._generated.program.function.decode_function_id(reader)
    slots = [decode_frame_slot_info(reader) for _ in range(reader.read_number())]

    return FrameInfo(
        function=function,
        slots=slots,
    )


def to_json_frame_info(value: FrameInfo) -> Json:
    """Return one JSON value for one FrameInfo."""
    return {
        "function": destack._generated.program.function.to_json_function_id(
            value.function
        ),
        "slots": [to_json_frame_slot_info(item_0) for item_0 in value.slots],
    }


def from_json_frame_info(value: Json) -> FrameInfo:
    """Return one FrameInfo from one JSON value."""
    object_ = json_object(value)

    return FrameInfo(
        function=destack._generated.program.function.from_json_function_id(
            json_field(object_, "function")
        ),
        slots=[
            from_json_frame_slot_info(item_0)
            for item_0 in json_array(json_field(object_, "slots"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FrameSlotInfo:
    """Reflected executable frame slot."""

    # slot type
    ty: destack._generated.program.type.TypeId
    # slot byte offset
    offset: int
    # slot byte length
    byte_len: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_frame_slot_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameSlotInfo:
        """Decode one FrameSlotInfo."""
        return decode_frame_slot_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_frame_slot_info(self)

    @classmethod
    def from_json(cls, value: Json) -> FrameSlotInfo:
        """Return one FrameSlotInfo from one JSON value."""
        return from_json_frame_slot_info(value)


def encode_frame_slot_info(writer: BinaryWriter, value: FrameSlotInfo) -> None:
    """Encode one FrameSlotInfo."""
    destack._generated.program.type.encode_type_id(writer, value.ty)
    writer.write_unsigned(value.offset)
    writer.write_unsigned(value.byte_len)


def decode_frame_slot_info(reader: BinaryReader) -> FrameSlotInfo:
    """Decode one FrameSlotInfo."""
    ty = destack._generated.program.type.decode_type_id(reader)
    offset = reader.read_number()
    byte_len = reader.read_number()

    return FrameSlotInfo(
        ty=ty,
        offset=offset,
        byte_len=byte_len,
    )


def to_json_frame_slot_info(value: FrameSlotInfo) -> Json:
    """Return one JSON value for one FrameSlotInfo."""
    return {
        "ty": destack._generated.program.type.to_json_type_id(value.ty),
        "offset": value.offset,
        "byteLen": value.byte_len,
    }


def from_json_frame_slot_info(value: Json) -> FrameSlotInfo:
    """Return one FrameSlotInfo from one JSON value."""
    object_ = json_object(value)

    return FrameSlotInfo(
        ty=destack._generated.program.type.from_json_type_id(json_field(object_, "ty")),
        offset=json_int(json_field(object_, "offset")),
        byte_len=json_int(json_field(object_, "byteLen")),
    )


@dataclass(frozen=True, slots=True)
class GlobalInfo:
    """Reflected global in one executable program."""

    # global display name when one exists
    name: destack._generated.core.string.StringId | None
    # global type
    ty: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalInfo:
        """Decode one GlobalInfo."""
        return decode_global_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_info(self)

    @classmethod
    def from_json(cls, value: Json) -> GlobalInfo:
        """Return one GlobalInfo from one JSON value."""
        return from_json_global_info(value)


def encode_global_info(writer: BinaryWriter, value: GlobalInfo) -> None:
    """Encode one GlobalInfo."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    destack._generated.program.type.encode_type_id(writer, value.ty)


def decode_global_info(reader: BinaryReader) -> GlobalInfo:
    """Decode one GlobalInfo."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    ty = destack._generated.program.type.decode_type_id(reader)

    return GlobalInfo(
        name=name,
        ty=ty,
    )


def to_json_global_info(value: GlobalInfo) -> Json:
    """Return one JSON value for one GlobalInfo."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        "ty": destack._generated.program.type.to_json_type_id(value.ty),
    }


def from_json_global_info(value: Json) -> GlobalInfo:
    """Return one GlobalInfo from one JSON value."""
    object_ = json_object(value)

    return GlobalInfo(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        ty=destack._generated.program.type.from_json_type_id(json_field(object_, "ty")),
    )


@dataclass(frozen=True, slots=True)
class EntryInfo:
    """Reflected entry point."""

    # entry point
    entry: destack._generated.program.entry.EntryPoint

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_entry_info(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> EntryInfo:
        """Decode one EntryInfo."""
        return decode_entry_info(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_entry_info(self)

    @classmethod
    def from_json(cls, value: Json) -> EntryInfo:
        """Return one EntryInfo from one JSON value."""
        return from_json_entry_info(value)


def encode_entry_info(writer: BinaryWriter, value: EntryInfo) -> None:
    """Encode one EntryInfo."""
    destack._generated.program.entry.encode_entry_point(writer, value.entry)


def decode_entry_info(reader: BinaryReader) -> EntryInfo:
    """Decode one EntryInfo."""
    entry = destack._generated.program.entry.decode_entry_point(reader)

    return EntryInfo(
        entry=entry,
    )


def to_json_entry_info(value: EntryInfo) -> Json:
    """Return one JSON value for one EntryInfo."""
    return {
        "entry": destack._generated.program.entry.to_json_entry_point(value.entry),
    }


def from_json_entry_info(value: Json) -> EntryInfo:
    """Return one EntryInfo from one JSON value."""
    object_ = json_object(value)

    return EntryInfo(
        entry=destack._generated.program.entry.from_json_entry_point(
            json_field(object_, "entry")
        ),
    )


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
