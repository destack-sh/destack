import base64
import textwrap
from datetime import UTC, date, datetime, time, timedelta
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never, override

from destack.language.core import (
    BuiltinObject,
    NodeType,
    ObjectKind,
    PackedCache,
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
    get_builtin_type,
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
    def pack_object(self, _object: "{cls.__name__}") -> "dict[str, Any]":
{pack_json}

    @override
    def unpack_object(
        self, 
        _object_json: "dict[str, Any]",
        _session: "Session | None" = None,
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
            "PackedCache": PackedCache,
        },
    )


def _generate_pack_json(cls: type["BuiltinObject"]) -> str:
    """Generate the pack_object method for a BuiltinObject."""
    pack_method_parts: list[str] = []
    pack_method_parts.append("_object_json: dict[str, Any] = {}")

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)
    for prop in wired_properties_in_order:
        if prop.name == "metatype":
            metatype = get_builtin_type(cls)
            # use the enum name in CONSTANT_UPPER_CASE for metatype
            if isinstance(metatype, NodeType):
                metatype_name = NodeType(metatype).name
            else:
                metatype_name = StructType(metatype).name
            pack_method_parts.append(f'_object_json["metatype"] = "{metatype_name}"')
            continue

        pack_code = _generate_pack_json_property(prop)
        if pack_code:
            pack_method_parts.extend(pack_code)

    pack_method_parts.append("return _object_json")
    return "\n".join(pack_method_parts)


def _generate_unpack_json(cls: type["BuiltinObject"]) -> str:
    """Generate the unpack_object method for a BuiltinObject."""
    unpack_assignments: list[str] = []
    unpack_method_parts: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.is_computed:
            continue  # set implicitly
        prop_name = (
            prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
        )
        unpack_code = _generate_unpack_json_property(prop)
        if len(unpack_code) == 1:
            unpack_assignments.append(f"{prop_name}={unpack_code[0].split(' = ', 1)[1]}")
        else:
            unpack_method_parts.extend(unpack_code)
            unpack_assignments.append(f"{prop_name}=_unpacked_{prop_name}")
    if cls.__is_frozen__ and not cls.__is_node__:
        unpack_assignments.append(
            "_packed_cache=PackedCache(encoding=Encoding.JSON, is_bytes=False, packed=_object_json)"
        )

    unpack_method_parts.append("return cls(")
    for assignment in unpack_assignments:
        unpack_method_parts.append(f"    {assignment},")
    unpack_method_parts.append("    _session=_session,")
    unpack_method_parts.append(")")

    return "\n".join(unpack_method_parts)


def _generate_pack_json_property(prop: PropertyDeclaration) -> list[str]:
    """Generate code to pack a property into JSON."""
    lines: list[str] = []
    json_key = to_casing(prop.name, Casing.LOWER_CAMEL)
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    obj_json = f"_object.{prop_name}"

    if prop.cardinality == TypeCardinality.SCALAR:
        value_expr = _generate_pack_json_scalar(prop, obj_json)
        if prop.is_required:
            lines.append(f'_object_json["{json_key}"] = {value_expr}')
        else:
            lines.append(f"if ({prop_name} := {obj_json}) is not None:")
            lines.append(f'    _object_json["{json_key}"] = {value_expr}')
    elif prop.cardinality == TypeCardinality.LIST:
        item_expr = _generate_pack_json_scalar(prop, "_item")
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
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        key_expr = _generate_pack_json_scalar(prop.key_type, "_key")
        value_expr = _generate_pack_json_scalar(prop, "_value")
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


def _generate_pack_json_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the packing code for a scalar value."""

    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.NONE:
            return "None"
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.SINT8,
            PrimitiveType.SINT16,
            PrimitiveType.SINT32,
            PrimitiveType.SINT64,
            PrimitiveType.SINT128,
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
    elif prop.scalar_type == ScalarType.ENUM:
        # use the enum name in CONSTANT_UPPER_CASE for JSON
        return f"{value_expr}.name"
    elif prop.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
        return f"{value_expr}.pack(Encoding.JSON)"
    else:
        assert_never(prop.scalar_type)


def _generate_unpack_json_property(prop: PropertyDeclaration) -> list[str]:
    """Generate code to unpack a property from JSON."""
    lines: list[str] = []
    json_key = to_casing(prop.name, Casing.LOWER_CAMEL)
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    data_json = f'_object_json.get("{json_key}")'

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_unpack_json_scalar(prop, data_json)
            lines.append(f"_unpacked_{prop_name} = {value_expr}")
        else:
            value_expr = _generate_unpack_json_scalar(prop, prop_name)
            lines.append(
                f"_unpacked_{prop_name} = {value_expr} if ({prop_name} := {data_json}) is not None else None"
            )
    elif prop.cardinality == TypeCardinality.LIST:
        item_expr = _generate_unpack_json_scalar(prop, "_item")
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
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        key_expr = _generate_unpack_json_scalar(prop.key_type, "_key")
        value_expr = _generate_unpack_json_scalar(prop, "_value")
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


def _generate_unpack_json_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the unpacking code for a scalar value."""

    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.NONE:
            return "None"
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.SINT8,
            PrimitiveType.SINT16,
            PrimitiveType.SINT32,
            PrimitiveType.SINT64,
            PrimitiveType.SINT128,
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
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
        return f"{enum_cls.__name__}[{value_expr}]"
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"{struct_cls.__name__}.unpack(Encoding.JSON, {value_expr}, _session)"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"NodeReference.unpack(Encoding.JSON, {value_expr}, _session)"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"Node.unpack(Encoding.JSON, {value_expr}, _session)"
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
