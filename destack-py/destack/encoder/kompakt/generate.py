import textwrap
from datetime import UTC, date, datetime, time, timedelta
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    BuiltinObject,
    Encoding,
    NodeType,
    ObjectKind,
    PrimitiveType,
    ScalarType,
    Session,
    StructType,
    TypeCardinality,
    TypeDeclaration,
)
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from destack.utils.code import exec_
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass

from .core import KompaktObjectEncoder

# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


def _generate_kompakt_object_encoder(cls: type["BuiltinObject"]) -> tuple[str, str, dict[str, Any]]:
    """Generate the KompaktObjectEncoder class for a BuiltinObject."""

    pack_kompakt = textwrap.indent(_generate_pack_kompakt(cls), " " * 8)
    unpack_kompakt = textwrap.indent(_generate_unpack_kompakt(cls), " " * 8)
    encoder_name = f"{cls.__name__}KompaktEncoder"

    impl = f"""
class {encoder_name}(KompaktObjectEncoder):
    @override
    def pack_object(
        self, 
        _encoder: "KompaktEncoder",
        _object: "{cls.__name__}", 
        _writer: BinaryWriter,
        _options: "EncoderOptions"
    ) -> None:
{pack_kompakt}

    @override
    def unpack_object(
        self, 
        _encoder: "KompaktEncoder",
        _reader: BinaryReader,
        _session: "Session | None",
        _options: "EncoderOptions",
    ) -> "{cls.__name__}":
{unpack_kompakt}
"""

    return (
        encoder_name,
        impl,
        {
            "KompaktObjectEncoder": KompaktObjectEncoder,
            "BinaryReader": BinaryReader,
            "BinaryWriter": BinaryWriter,
            "datetime": datetime,
            "timedelta": timedelta,
            "date": date,
            "time": time,
            "UTC": UTC,
            "UUID": UUID,
            "override": override,
            "Self": cls,
            "cls": cls,
            "BuiltinObject": BuiltinObject,
            "Encoding": Encoding,
            "Session": Session,
        },
    )


def _generate_pack_kompakt(cls: type["BuiltinObject"]) -> str:
    """Generate the pack_object method implementation."""
    lines: list[str] = [
        f"_writer.write_uint32({cls.metatype.value})",
    ]

    return "\n".join(lines)


def _generate_unpack_kompakt(cls: type["BuiltinObject"]) -> str:
    """Generate the unpack_object method implementation."""
    lines: list[str] = [
        "metatype = _reader.read_uint32()",
        f"assert metatype == {cls.metatype.value}",
    ]

    return "\n".join(lines)


def _generate_pack_kompakt_type(type: TypeDeclaration, value_expr: str) -> str:
    """Generate the pack_kompakt method implementation for a property."""

    ...


def _generate_unpack_kompakt_type(type: TypeDeclaration, target_expr: str) -> tuple[str, bool]:
    """Generate the unpack_kompakt method implementation for a property."""

    ...


def _generate_pack_kompakt_scalar(type: TypeDeclaration, value_expr: str) -> str:
    """Generate the pack_kompakt method implementation for a scalar property."""
    lines: list[str] = []

    assert type.cardinality == TypeCardinality.SCALAR, f"cannot pack non-scalar: {type!r}"
    assert type.scalar_type is not None, f"no scalar type for {type!r}"

    # primitive
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None, f"no primitive type for {type!r}"
        if type.primitive_type == PrimitiveType.NONE:
            raise NotImplementedError(f"cannot pack none: {type!r}")
        elif type.primitive_type == PrimitiveType.BOOLEAN:
            lines.append(f"_writer.write_bool({value_expr})")
        elif type.primitive_type == PrimitiveType.INT8:
            lines.append(f"_writer.write_int8({value_expr})")
        elif type.primitive_type == PrimitiveType.INT16:
            lines.append(f"_writer.write_int16({value_expr})")
        elif type.primitive_type == PrimitiveType.INT32:
            lines.append(f"_writer.write_int32({value_expr})")
        elif type.primitive_type == PrimitiveType.INT64:
            lines.append(f"_writer.write_int64({value_expr})")
        elif type.primitive_type == PrimitiveType.INT128:
            lines.append(f"_writer.write_int128({value_expr})")
        elif type.primitive_type == PrimitiveType.UINT8:
            lines.append(f"_writer.write_uint8({value_expr})")
        elif type.primitive_type == PrimitiveType.UINT16:
            lines.append(f"_writer.write_uint16({value_expr})")
        elif type.primitive_type == PrimitiveType.UINT32:
            lines.append(f"_writer.write_uint32({value_expr})")
        elif type.primitive_type == PrimitiveType.UINT64:
            lines.append(f"_writer.write_uint64({value_expr})")
        elif type.primitive_type == PrimitiveType.UINT128:
            lines.append(f"_writer.write_uint128({value_expr})")
        elif type.primitive_type == PrimitiveType.FLOAT16:
            lines.append(f"_writer.write_float16({value_expr})")
        elif type.primitive_type == PrimitiveType.FLOAT32:
            lines.append(f"_writer.write_float32({value_expr})")
        elif type.primitive_type == PrimitiveType.FLOAT64:
            lines.append(f"_writer.write_float64({value_expr})")
        elif type.primitive_type == PrimitiveType.DATETIME:
            lines.append(f"_writer.write_datetime({value_expr})")
        elif type.primitive_type == PrimitiveType.DATE:
            lines.append(f"_writer.write_date({value_expr})")
        elif type.primitive_type == PrimitiveType.TIME:
            lines.append(f"_writer.write_time({value_expr})")
        elif type.primitive_type == PrimitiveType.DURATION:
            lines.append(f"_writer.write_duration({value_expr})")
        elif type.primitive_type == PrimitiveType.STRING:
            lines.append(f"_writer.write_string({value_expr})")
        elif type.primitive_type == PrimitiveType.UUID:
            lines.append(f"_writer.write_uuid({value_expr})")
        elif type.primitive_type == PrimitiveType.BYTES:
            lines.append(f"_writer.write_bytes({value_expr})")
        elif type.primitive_type == PrimitiveType.JSON:
            lines.append(f"_writer.write_json({value_expr})")
        else:
            assert_never(type.primitive_type)

    # enum
    elif type.scalar_type == ScalarType.ENUM:
        lines.append(f"_writer.write_uint32({value_expr})")

    # struct
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct type for {type!r}"
        lines.append(
            f"_encoder.pack_object_binary({ObjectKind.STRUCT}, {type.struct_type}, {value_expr}, _writer, _options)"
        )

    # node reference
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        lines.append(
            f"_encoder.pack_object_binary({ObjectKind.STRUCT}, {StructType.NODE_REFERENCE}, {value_expr}, _writer, _options)"
        )

    # node value
    elif type.scalar_type == ScalarType.NODE_VALUE:
        lines.append(f"_writer.write_uint32({value_expr}.metatype.value)")
        lines.append(
            f"_encoder.pack_object_binary({ObjectKind.NODE}, {value_expr}.metatype, {value_expr}, _writer, _options)"
        )

    else:
        assert_never(type.scalar_type)

    return "\n".join(lines)


def _generate_unpack_kompakt_scalar(type: TypeDeclaration, target_expr: str) -> tuple[str, bool]:
    """Generate the unpack_kompakt method implementation for a scalar property."""
    assert type.cardinality == TypeCardinality.SCALAR, f"cannot pack non-scalar: {type!r}"
    assert type.scalar_type is not None, f"no scalar type for {type!r}"

    # primitive
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None, f"no primitive type for {type!r}"
        if type.primitive_type == PrimitiveType.NONE:
            raise NotImplementedError(f"cannot pack none: {type!r}")
        elif type.primitive_type == PrimitiveType.BOOLEAN:
            return "_reader.read_bool()", True
        elif type.primitive_type == PrimitiveType.INT8:
            return "_reader.read_int8()", True
        elif type.primitive_type == PrimitiveType.INT16:
            return "_reader.read_int16()", True
        elif type.primitive_type == PrimitiveType.INT32:
            return "_reader.read_int32()", True
        elif type.primitive_type == PrimitiveType.INT64:
            return "_reader.read_int64()", True
        elif type.primitive_type == PrimitiveType.INT128:
            return "_reader.read_int128()", True
        elif type.primitive_type == PrimitiveType.UINT8:
            return "_reader.read_uint8()", True
        elif type.primitive_type == PrimitiveType.UINT16:
            return "_reader.read_uint16()", True
        elif type.primitive_type == PrimitiveType.UINT32:
            return "_reader.read_uint32()", True
        elif type.primitive_type == PrimitiveType.UINT64:
            return "_reader.read_uint64()", True
        elif type.primitive_type == PrimitiveType.UINT128:
            return "_reader.read_uint128()", True
        elif type.primitive_type == PrimitiveType.FLOAT16:
            return "_reader.read_float16()", True
        elif type.primitive_type == PrimitiveType.FLOAT32:
            return "_reader.read_float32()", True
        elif type.primitive_type == PrimitiveType.FLOAT64:
            return "_reader.read_float64()", True
        elif type.primitive_type == PrimitiveType.DATETIME:
            return "_reader.read_datetime()", True
        elif type.primitive_type == PrimitiveType.DATE:
            return "_reader.read_date()", True
        elif type.primitive_type == PrimitiveType.TIME:
            return "_reader.read_time()", True
        elif type.primitive_type == PrimitiveType.DURATION:
            return "_reader.read_duration()", True
        elif type.primitive_type == PrimitiveType.STRING:
            return "_reader.read_string()", True
        elif type.primitive_type == PrimitiveType.UUID:
            return "_reader.read_uuid()", True
        elif type.primitive_type == PrimitiveType.BYTES:
            return "_reader.read_bytes()", True
        elif type.primitive_type == PrimitiveType.JSON:
            return "_reader.read_json()", True
        else:
            assert_never(type.primitive_type)

    # enum
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        return f"{enum_cls.__name__}[_reader.read_uint32()]", True

    # struct
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct type for {type!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
        return (
            f"{struct_cls.__name__}.unpack_binary({Encoding.KOMPAKT.value}, _reader, _session)",
            True,
        )

    # node reference
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        return (
            f"_encoder.unpack_object_binary({ObjectKind.STRUCT}, {StructType.NODE_REFERENCE}, _reader, _session, _options)",
            True,
        )

    # node value
    elif type.scalar_type == ScalarType.NODE_VALUE:
        return (
            f"_encoder.unpack_object_binary({ObjectKind.NODE}, _reader.read_uint32(), _reader, _session, _options)",
            True,
        )

    else:
        assert_never(type.scalar_type)


# registry of Kompakt encoders by (ObjectKind, NodeType|StructType)
KOMPAKT_OBJECT_ENCODERS: dict[tuple[ObjectKind, NodeType | StructType], KompaktObjectEncoder] = {}


def _generate():
    # generate pack/unpack methods
    builtin_class_by_name: dict[str, Any] = {"UUID": UUID}
    builtin_class_by_name.update(
        {
            cls.__name__: cls
            for cls in chain(
                NODE_CLASS_BY_TYPE.values(),
                STRUCT_CLASS_BY_TYPE.values(),
                ENUM_CLASS_BY_TYPE.values(),
            )
        }
    )
    for node_cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        if node_cls.__is_abstract__:
            continue
        encoder_name, impl, extra_glbls = _generate_kompakt_object_encoder(node_cls)
        locals_ = {}
        # print("=" * 80)
        # print(node_cls.__name__ + ":kompakt")
        # print("=" * 80)
        # print(impl)
        # print("=" * 80)
        exec_(
            impl,
            {**builtin_class_by_name, **extra_glbls},
            locals_,
            encoder_name,
        )
        encoder_cls = locals_[encoder_name]
        KOMPAKT_OBJECT_ENCODERS[node_cls.__kind__, node_cls.metatype] = encoder_cls()


_generate()
