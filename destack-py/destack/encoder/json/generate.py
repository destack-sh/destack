import base64
import textwrap
from datetime import UTC, date, datetime, time, timedelta
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never, override

from destack.language.core import (
    METATYPE_PROPERTY_KEY,
    BuiltinObject,
    NodeType,
    ObjectKind,
    PackedObjectCache,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
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
from destack.utils.string import Casing, to_casing
from destack.utils.telemetry import get_tracer
from destack.utils.time import timedelta_from_isoformat, timedelta_to_isoformat
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass

from .core import JsonObjectEncoder

# ruff: noqa: FURB113, SIM114
# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


def _generate_json_object_encoder(cls: type["BuiltinObject"]) -> tuple[str, str, dict[str, Any]]:
    """Generate the JsonObjectEncoder class for a BuiltinObject."""

    pack_json = textwrap.indent(_generate_pack_json(cls), " " * 8)
    unpack_json = textwrap.indent(_generate_unpack_json(cls), " " * 8)
    encoder_name = f"{cls.__name__}JsonEncoder"

    impl = f"""
class {encoder_name}(JsonObjectEncoder):
    
    @override
    def pack_object(
        self,
        _encoder: "JsonEncoder",
        _object: "{cls.__name__}",
        _options: "EncoderOptions",
    ) -> "dict[str, Any]":
{pack_json}

    @override
    def unpack_object(
        self,
        _encoder: "JsonEncoder",
        _object_json: "dict[str, Any]",
        _session: "Session | None",
        _options: "EncoderOptions",
    ) -> "{cls.__name__}":
{unpack_json}
"""
    return (
        encoder_name,
        impl,
        {
            "JsonObjectEncoder": JsonObjectEncoder,
            "timedelta_from_isoformat": timedelta_from_isoformat,
            "timedelta_to_isoformat": timedelta_to_isoformat,
            "datetime": datetime,
            "timedelta": timedelta,
            "date": date,
            "time": time,
            "UTC": UTC,
            "base64": base64,
            "UUID": UUID,
            "override": override,
            "dict": dict,
            "Any": Any,
            "Self": cls,
            "cls": cls,
            "BuiltinObject": BuiltinObject,
            "PackedObjectCache": PackedObjectCache,
        },
    )


def _generate_pack_json(cls: type["BuiltinObject"]) -> str:
    """Generate the pack_object method for a BuiltinObject."""
    lines: list[str] = [
        f"_object_json: dict[str, Any] = {{{METATYPE_PROPERTY_KEY}: {cls.metatype.value}}}",
    ]

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)
    for prop in wired_properties_in_order:
        if prop.is_computed:
            continue
        elif cls.metatype == StructType.VALUE and prop.name == "value":
            # generic value
            lines.append(
                f"_object_json['{prop.id}'] = _encoder.pack_value(_object.type, _object.value, _options)"
            )
        else:
            # normal property
            pack_code = _generate_pack_json_property(prop)
            lines.extend(pack_code)

    lines.append("return _object_json")
    return "\n".join(lines)


def _generate_unpack_json(cls: type["BuiltinObject"]) -> str:
    """Generate the unpack_object method for a BuiltinObject."""
    assignments: list[str] = []
    lines: list[str] = []

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)
    for prop in wired_properties_in_order:
        if prop.is_computed:
            continue  # set implicitly
        elif cls.metatype == StructType.VALUE and prop.name == "value":
            # generic value
            lines.append(
                f"_unpacked_value = _encoder.unpack_value(_object_json.get({prop.id}), _session, _options)"
            )
        else:
            # normal property
            prop_name = (
                prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
            )
            unpack_code = _generate_unpack_json_property(prop)
            if len(unpack_code) == 1:
                assignments.append(f"{prop_name}={unpack_code[0].split(' = ', 1)[1]}")
            else:
                lines.extend(unpack_code)
                assignments.append(f"{prop_name}=_unpacked_{prop_name}")
    if cls.__is_frozen__ and not cls.__is_node__:
        assignments.append(
            "_packed_cache=PackedObjectCache(encoding=Encoding.JSON, is_bytes=False, packed=_object_json)"
        )

    lines.append("return cls(")
    for assignment in assignments:
        lines.append(f"    {assignment},")
    lines.append("    _session=_session,")
    lines.append(")")

    return "\n".join(lines)


def _generate_pack_json_property(prop: PropertyDeclaration) -> list[str]:
    """Generate code to pack a property into JSON."""
    lines: list[str] = []
    json_key = to_casing(prop.name, Casing.LOWER_CAMEL)
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    obj_json = f"_object.{prop_name}"

    # scalar
    if prop.cardinality == TypeCardinality.SCALAR:
        value_expr = _generate_pack_json_scalar(prop, obj_json)
        if prop.is_required:
            lines.append(f'_object_json["{json_key}"] = {value_expr}')
        else:
            lines.append(f"if ({prop_name} := {obj_json}) is not None:")
            lines.append(f'    _object_json["{json_key}"] = {value_expr}')

    # list
    elif prop.cardinality == TypeCardinality.LIST:
        assert prop.value_type is not None, f"no value type for {prop!r}"
        item_expr = _generate_pack_json_scalar(prop.value_type, "_item")
        if prop.is_required:
            lines.append(f"_packed_{prop_name} = []")
            lines.append(f"for _item in {obj_json}:")
            lines.append(f"    _packed_{prop_name}.append({item_expr})")
            lines.append(f'_object_json["{json_key}"] = _packed_{prop_name}')
        else:
            lines.append(f"if {obj_json} is not None:")
            lines.append(f"    _packed_{prop_name} = []")
            lines.append(f"    for _item in {obj_json}:")
            lines.append(f"        _packed_{prop_name}.append({item_expr})")
            lines.append(f'    _object_json["{json_key}"] = _packed_{prop_name}')

    # tuple
    elif prop.cardinality == TypeCardinality.TUPLE:
        raise NotImplementedError(f"cannot pack tuple: {prop!r}")

    # map
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        assert prop.value_type is not None, f"no value type for {prop!r}"
        key_expr = _generate_pack_json_scalar(prop.key_type, "_key")
        value_expr = _generate_pack_json_scalar(prop.value_type, "_value")
        if prop.is_required:
            lines.append(f"_packed_{prop_name} = {{}}")
            lines.append(f"for _key, _value in {obj_json}.items():")
            lines.append(f"    _packed_{prop_name}[str({key_expr})] = {value_expr}")
            lines.append(f'_object_json["{json_key}"] = _packed_{prop_name}')
        else:
            lines.append(f"if {obj_json} is not None:")
            lines.append(f"    _packed_{prop_name} = {{}}")
            lines.append(f"    for _key, _value in {obj_json}.items():")
            lines.append(f"        _packed_{prop_name}[str({key_expr})] = {value_expr}")
            lines.append(f'    _object_json["{json_key}"] = _packed_{prop_name}')

    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_json_property(prop: PropertyDeclaration) -> list[str]:
    """Generate code to unpack a property from JSON."""
    lines: list[str] = []
    json_key = to_casing(prop.name, Casing.LOWER_CAMEL)
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    data_json = f'_object_json.get("{json_key}")'

    # scalar
    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_unpack_json_scalar(prop, data_json)
            lines.append(f"_unpacked_{prop_name} = {value_expr}")
        else:
            value_expr = _generate_unpack_json_scalar(prop, prop_name)
            lines.append(
                f"_unpacked_{prop_name} = {value_expr} if ({prop_name} := {data_json}) is not None else None"
            )

    # list
    elif prop.cardinality == TypeCardinality.LIST:
        assert prop.value_type is not None, f"no value type for {prop!r}"
        item_expr = _generate_unpack_json_scalar(prop.value_type, "_item")
        if prop.is_required:
            lines.append(f"_unpacked_{prop_name} = []")
            lines.append(f"for _item in {data_json}:")
            lines.append(f"    _unpacked_{prop_name}.append({item_expr})")
        else:
            lines.append(f"if {data_json} is not None:")
            lines.append(f"    _unpacked_{prop_name} = []")
            lines.append(f"    for _item in {data_json}:")
            lines.append(f"        _unpacked_{prop_name}.append({item_expr})")
            lines.append("else:")
            lines.append(f"    _unpacked_{prop_name} = None")

    # tuple
    elif prop.cardinality == TypeCardinality.TUPLE:
        raise NotImplementedError(f"cannot unpack tuple: {prop!r}")

    # map
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        assert prop.value_type is not None, f"no value type for {prop!r}"
        key_expr = _generate_unpack_json_scalar(prop.key_type, "_key")
        value_expr = _generate_unpack_json_scalar(prop.value_type, "_value")
        if prop.is_required:
            lines.append(f"_unpacked_{prop_name} = {{}}")
            lines.append(f"for _key, _value in {data_json}.items():")
            lines.append(f"    _unpacked_{prop_name}[{key_expr}] = {value_expr}")
        else:
            lines.append(f"if {data_json} is not None:")
            lines.append(f"    _unpacked_{prop_name} = {{}}")
            lines.append(f"    for _key, _value in {data_json}.items():")
            lines.append(f"        _unpacked_{prop_name}[{key_expr}] = {value_expr}")
            lines.append("else:")
            lines.append(f"    _unpacked_{prop_name} = None")

    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_json_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the packing code for a scalar value."""

    assert prop.cardinality == TypeCardinality.SCALAR, f"cannot pack non-scalar: {prop!r}"
    assert prop.scalar_type is not None, f"no scalar type for {prop!r}"

    # primitive
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.NONE:
            return "None"
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.INT8,
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.INT128,
        ):
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return value_expr
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"{value_expr}.astimezone(UTC).isoformat()"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"{value_expr}.isoformat()"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"{value_expr}.astimezone(UTC).replace(tzinfo=None).isoformat()"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedelta_to_isoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.STRING:
            return value_expr
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"str({value_expr})"
        elif prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64encode({value_expr}).decode()"
        elif prop.primitive_type == PrimitiveType.JSON:
            return value_expr
        else:
            assert_never(prop.primitive_type)

    # enum
    elif prop.scalar_type == ScalarType.ENUM:
        # use the enum name in CONSTANT_UPPER_CASE for JSON
        return f"{value_expr}.name"

    # struct
    elif prop.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
        return f"{value_expr}.pack(Encoding.JSON)"

    # node reference
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"{value_expr}.pack(Encoding.JSON)"

    # node value
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"{value_expr}.pack(Encoding.JSON)"

    # literal
    elif prop.scalar_type == ScalarType.LITERAL:
        raise NotImplementedError(f"cannot pack literal: {prop!r}")

    # union
    elif prop.scalar_type == ScalarType.UNION:
        raise NotImplementedError(f"cannot pack union: {prop!r}")

    #
    else:
        assert_never(prop.scalar_type)


def _generate_unpack_json_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the unpacking code for a scalar value."""

    assert prop.cardinality == TypeCardinality.SCALAR, f"cannot unpack non-scalar: {prop!r}"
    assert prop.scalar_type is not None, f"no scalar type for {prop!r}"

    # primitive
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.NONE:
            return "None"
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.INT8,
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.INT128,
        ):
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return value_expr
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"datetime.fromisoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"date.fromisoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"time.fromisoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedelta_from_isoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.STRING:
            return value_expr
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"UUID({value_expr})"
        elif prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64decode({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return value_expr
        else:
            assert_never(prop.primitive_type)

    # enum
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
        return f"{enum_cls.__name__}[{value_expr}]"

    # struct
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"{struct_cls.__name__}.unpack(Encoding.JSON, {value_expr}, _session)"

    # node reference
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"NodeReference.unpack(Encoding.JSON, {value_expr}, _session)"

    # node value
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"Node.unpack(Encoding.JSON, {value_expr}, _session)"

    # literal
    elif prop.scalar_type == ScalarType.LITERAL:
        raise NotImplementedError(f"cannot unpack literal: {prop!r}")

    # union
    elif prop.scalar_type == ScalarType.UNION:
        raise NotImplementedError(f"cannot unpack union: {prop!r}")

    #
    else:
        assert_never(prop.scalar_type)


# registry of JSON encoders by (ObjectKind, NodeType|StructType)
JSON_OBJECT_ENCODERS: dict[tuple[ObjectKind, NodeType | StructType], JsonObjectEncoder] = {}


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
        encoder_name, impl, extra_glbls = _generate_json_object_encoder(node_cls)
        locals_ = {}
        exec_(
            impl,
            {**builtin_class_by_name, **extra_glbls},
            locals_,
            f"{node_cls.__name__}:json",
        )
        encoder_cls = locals_[encoder_name]
        JSON_OBJECT_ENCODERS[node_cls.__kind__, node_cls.metatype] = encoder_cls()


_generate()
