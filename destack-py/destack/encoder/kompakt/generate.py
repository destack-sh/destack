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
    PackedObjectCache,
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

# ruff: noqa: SIM114, FURB113
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
    def pack_object(self, _object: "{cls.__name__}", writer: BinaryWriter) -> None:
{pack_kompakt}

    @override
    def unpack_object(self, _reader: BinaryReader, _session: "Session | None") -> "{cls.__name__}":
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
            "PackedObjectCache": PackedObjectCache,
        },
    )


def _generate_pack_kompakt(cls: type["BuiltinObject"]) -> str:
    """Generate the pack_object method implementation."""
    pack_method_parts: list[str] = [
        f"writer.write_uint32({cls.metatype.value})",
    ]

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)
    for prop in wired_properties_in_order:
        if prop.is_computed:
            continue
        pack_code = _generate_pack_kompakt_type(prop, f"_object.{prop.name}")
        if pack_code:
            pack_method_parts.append(pack_code)

    return "\n".join(pack_method_parts)


def _generate_unpack_kompakt(cls: type["BuiltinObject"]) -> str:
    """Generate the unpack_object method implementation."""
    unpack_method_parts: list[str] = [
        "metatype = reader.read_uint32()",
        f"assert metatype == {cls.metatype.value}",
    ]

    # build up the object
    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)
    for prop in wired_properties_in_order:
        if prop.is_computed:
            continue
        target_expr = f"_object_{prop.name}"
        unpack_code, is_simple = _generate_unpack_kompakt_type(prop, target_expr)
        if is_simple:
            unpack_method_parts.append(f"{target_expr} = {unpack_code}")
        else:
            unpack_method_parts.append(unpack_code)

    # constructor
    constructor_args = ",\n".join(
        f"    {prop.name}=_object_{prop.name}" for prop in wired_properties_in_order
    )
    unpack_method_parts.append(f"_object = {cls.__name__}(\n{constructor_args}\n)")
    unpack_method_parts.append("return _object")

    return "\n".join(unpack_method_parts)


def _generate_pack_kompakt_type(type: TypeDeclaration, value_expr: str) -> str:
    """Generate the pack_kompakt method implementation for a property."""

    # scalar
    if type.cardinality == TypeCardinality.SCALAR:
        return _generate_pack_kompakt_scalar(type, value_expr)

    # list
    elif type.cardinality == TypeCardinality.LIST:
        assert type.value_type is not None, f"no value type for {type!r}"
        item_expr = _generate_pack_kompakt_type(type.value_type, "_item")
        lines: list[str] = []
        lines.append(f"writer.write_uint32({len(value_expr)})")
        lines.append(f"for _item in {value_expr}:")
        lines.append(textwrap.indent(item_expr, " " * 4))

    # tuple
    elif type.cardinality == TypeCardinality.TUPLE:
        raise NotImplementedError(f"cannot pack tuple: {type!r}")

    # map
    elif type.cardinality == TypeCardinality.MAP:
        assert type.key_type is not None, f"no key type for {type!r}"
        assert type.value_type is not None, f"no value type for {type!r}"
        key_expr = _generate_pack_kompakt_type(type.key_type, "_key")
        item_expr = _generate_pack_kompakt_type(type.value_type, "_value")
        lines: list[str] = []
        lines.append(f"writer.write_uint32({len(value_expr)})")
        lines.append(f"for _key, _value in {value_expr}.items():")
        lines.append(textwrap.indent(key_expr, " " * 4))
        lines.append(textwrap.indent(item_expr, " " * 4))

    else:
        assert_never(type.cardinality)

    return "\n".join(lines)


def _generate_unpack_kompakt_type(type: TypeDeclaration, target_expr: str) -> tuple[str, bool]:
    """Generate the unpack_kompakt method implementation for a property."""
    # scalar
    if type.cardinality == TypeCardinality.SCALAR:
        return _generate_unpack_kompakt_scalar(type, target_expr)

    # list
    elif type.cardinality == TypeCardinality.LIST:
        assert type.value_type is not None, f"no value type for {type!r}"
        item_expr, is_simple = _generate_unpack_kompakt_type(type.value_type, "_item")
        assert is_simple, f"cannot unpack non-simple list: {type!r}"
        accumulate_expr = f"{target_expr}.append({item_expr})"
        lines: list[str] = [
            f"{target_expr} = []",
            "for _ in range(reader.read_uint32()):",
            textwrap.indent(accumulate_expr, " " * 4),
        ]
        return "\n".join(lines), True

    # tuple
    elif type.cardinality == TypeCardinality.TUPLE:
        raise NotImplementedError(f"cannot pack tuple: {type!r}")

    # map
    elif type.cardinality == TypeCardinality.MAP:
        assert type.key_type is not None, f"no key type for {type!r}"
        assert type.value_type is not None, f"no value type for {type!r}"
        key_expr, is_simple = _generate_unpack_kompakt_type(type.key_type, "_key")
        value_expr, is_simple = _generate_unpack_kompakt_type(type.value_type, "_value")
        assert is_simple, f"cannot unpack non-simple map: {type!r}"
        accumulate_expr = f"{target_expr}[{key_expr}] = {value_expr}"
        lines: list[str] = [
            f"{target_expr} = {{}}",
            "for _ in range(reader.read_uint32()):",
            textwrap.indent(accumulate_expr, " " * 4),
        ]
        return "\n".join(lines), True

    else:
        assert_never(type.cardinality)


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
            lines.append(f"writer.write_bool({value_expr})")
        elif type.primitive_type == PrimitiveType.INT8:
            lines.append(f"writer.write_int8({value_expr})")
        elif type.primitive_type == PrimitiveType.INT16:
            lines.append(f"writer.write_int16({value_expr})")
        elif type.primitive_type == PrimitiveType.INT32:
            lines.append(f"writer.write_int32({value_expr})")
        elif type.primitive_type == PrimitiveType.INT64:
            lines.append(f"writer.write_int64({value_expr})")
        elif type.primitive_type == PrimitiveType.INT128:
            lines.append(f"writer.write_int128({value_expr})")
        elif type.primitive_type == PrimitiveType.UINT8:
            lines.append(f"writer.write_uint8({value_expr})")
        elif type.primitive_type == PrimitiveType.UINT16:
            lines.append(f"writer.write_uint16({value_expr})")
        elif type.primitive_type == PrimitiveType.UINT32:
            lines.append(f"writer.write_uint32({value_expr})")
        elif type.primitive_type == PrimitiveType.UINT64:
            lines.append(f"writer.write_uint64({value_expr})")
        elif type.primitive_type == PrimitiveType.UINT128:
            lines.append(f"writer.write_uint128({value_expr})")
        elif type.primitive_type == PrimitiveType.FLOAT16:
            lines.append(f"writer.write_float16({value_expr})")
        elif type.primitive_type == PrimitiveType.FLOAT32:
            lines.append(f"writer.write_float32({value_expr})")
        elif type.primitive_type == PrimitiveType.FLOAT64:
            lines.append(f"writer.write_float64({value_expr})")
        elif type.primitive_type == PrimitiveType.DATETIME:
            lines.append(f"writer.write_datetime({value_expr})")
        elif type.primitive_type == PrimitiveType.DATE:
            lines.append(f"writer.write_date({value_expr})")
        elif type.primitive_type == PrimitiveType.TIME:
            lines.append(f"writer.write_time({value_expr})")
        elif type.primitive_type == PrimitiveType.DURATION:
            lines.append(f"writer.write_duration({value_expr})")
        elif type.primitive_type == PrimitiveType.STRING:
            lines.append(f"writer.write_string({value_expr})")
        elif type.primitive_type == PrimitiveType.UUID:
            lines.append(f"writer.write_uuid({value_expr})")
        elif type.primitive_type == PrimitiveType.BYTES:
            lines.append(f"writer.write_bytes({value_expr})")
        elif type.primitive_type == PrimitiveType.JSON:
            lines.append(f"writer.write_json({value_expr})")
        else:
            assert_never(type.primitive_type)

    # enum
    elif type.scalar_type == ScalarType.ENUM:
        lines.append(f"writer.write_uint32({value_expr})")

    # struct
    elif type.scalar_type == ScalarType.STRUCT:
        lines.append(f"{value_expr}.pack_binary({Encoding.KOMPAKT.value}, writer)")

    # node reference
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        lines.append(f"{value_expr}.pack_binary({Encoding.KOMPAKT.value}, writer)")

    # node value
    elif type.scalar_type == ScalarType.NODE_VALUE:
        lines.append(f"{value_expr}.pack_binary({Encoding.KOMPAKT.value}, writer)")

    # literal
    elif type.scalar_type == ScalarType.LITERAL:
        raise NotImplementedError(f"cannot pack literal: {type!r}")

    # union
    elif type.scalar_type == ScalarType.UNION:
        raise NotImplementedError(f"cannot pack union: {type!r}")

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
            return "reader.read_bool()", True
        elif type.primitive_type == PrimitiveType.INT8:
            return "reader.read_int8()", True
        elif type.primitive_type == PrimitiveType.INT16:
            return "reader.read_int16()", True
        elif type.primitive_type == PrimitiveType.INT32:
            return "reader.read_int32()", True
        elif type.primitive_type == PrimitiveType.INT64:
            return "reader.read_int64()", True
        elif type.primitive_type == PrimitiveType.INT128:
            return "reader.read_int128()", True
        elif type.primitive_type == PrimitiveType.UINT8:
            return "reader.read_uint8()", True
        elif type.primitive_type == PrimitiveType.UINT16:
            return "reader.read_uint16()", True
        elif type.primitive_type == PrimitiveType.UINT32:
            return "reader.read_uint32()", True
        elif type.primitive_type == PrimitiveType.UINT64:
            return "reader.read_uint64()", True
        elif type.primitive_type == PrimitiveType.UINT128:
            return "reader.read_uint128()", True
        elif type.primitive_type == PrimitiveType.FLOAT16:
            return "reader.read_float16()", True
        elif type.primitive_type == PrimitiveType.FLOAT32:
            return "reader.read_float32()", True
        elif type.primitive_type == PrimitiveType.FLOAT64:
            return "reader.read_float64()", True
        elif type.primitive_type == PrimitiveType.DATETIME:
            return "reader.read_datetime()", True
        elif type.primitive_type == PrimitiveType.DATE:
            return "reader.read_date()", True
        elif type.primitive_type == PrimitiveType.TIME:
            return "reader.read_time()", True
        elif type.primitive_type == PrimitiveType.DURATION:
            return "reader.read_duration()", True
        elif type.primitive_type == PrimitiveType.STRING:
            return "reader.read_string()", True
        elif type.primitive_type == PrimitiveType.UUID:
            return "reader.read_uuid()", True
        elif type.primitive_type == PrimitiveType.BYTES:
            return "reader.read_bytes()", True
        elif type.primitive_type == PrimitiveType.JSON:
            return "reader.read_json()", True
        else:
            assert_never(type.primitive_type)

    # enum
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        return f"{enum_cls.__name__}[reader.read_uint32()]", True

    # struct
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct type for {type!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
        return (
            f"{struct_cls.__name__}.unpack_binary({Encoding.KOMPAKT.value}, reader, _session)",
            True,
        )

    # node reference
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        return f"NodeReference.unpack_binary({Encoding.KOMPAKT.value}, reader, _session)", True

    # node value
    elif type.scalar_type == ScalarType.NODE_VALUE:
        return f"Node.unpack_binary({Encoding.KOMPAKT.value}, reader, _session)", True

    # literal
    elif type.scalar_type == ScalarType.LITERAL:
        raise NotImplementedError(f"cannot unpack literal: {type!r}")

    # union
    elif type.scalar_type == ScalarType.UNION:
        raise NotImplementedError(f"cannot unpack union: {type!r}")

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
        print("=" * 80)
        print(node_cls.__name__ + ":kompakt")
        print("=" * 80)
        print(impl)
        print("=" * 80)
        exec_(
            impl,
            {**builtin_class_by_name, **extra_glbls},
            locals_,
            f"{node_cls.__name__}:kompakt",
        )
        encoder_cls = locals_[encoder_name]
        KOMPAKT_OBJECT_ENCODERS[node_cls.__kind__, node_cls.metatype] = encoder_cls()


_generate()
