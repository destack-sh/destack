import base64
import textwrap
from datetime import UTC, date, datetime, time, timedelta
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never, override

from destack.language.core import (
    METATYPE_PROPERTY_KEY,
    BuiltinObject,
    Encoding,
    Jsonc,
    NodeType,
    ObjectKind,
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
from destack.utils.telemetry import get_tracer
from destack.utils.time import timedelta_from_isoformat, timedelta_to_isoformat
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass

from .core import JsoncObjectEncoder

# ruff: noqa: FURB113
# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


def _get_encoder_name(cls: type["BuiltinObject"]) -> str:
    """Get the name of the encoder class for a BuiltinObject."""
    return f"{cls.__name__}JsoncEncoder"


def _generate_jsonc_object_encoder(cls: type["BuiltinObject"]) -> tuple[str, str, dict[str, Any]]:
    """Generate the JsoncObjectEncoder class for a BuiltinObject."""

    pack_jsonc = textwrap.indent(_generate_pack_jsonc(cls), " " * 8)
    unpack_jsonc = textwrap.indent(_generate_unpack_jsonc(cls), " " * 8)
    encoder_name = _get_encoder_name(cls)

    impl = f"""
class {encoder_name}(JsoncObjectEncoder):
    
    @override
    def pack_object(
        self, 
        _encoder: "JsoncEncoder",
        _object: "{cls.__name__}", 
        _options: "EncoderOptions"
    ) -> Jsonc:
{pack_jsonc}

    @override
    def unpack_object(
        self, 
        _encoder: "JsoncEncoder",
        _object_jsonc: Jsonc,
        _session: "Session | None",
        _options: "EncoderOptions",
    ) -> "{cls.__name__}":
{unpack_jsonc}
"""
    return (
        encoder_name,
        impl,
        {
            "JsoncObjectEncoder": JsoncObjectEncoder,
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
            "Jsonc": Jsonc,
            "Self": cls,
            "cls": cls,
            "BuiltinObject": BuiltinObject,
            "Encoding": Encoding,
        },
    )


def _generate_pack_jsonc(cls: type["BuiltinObject"]) -> str:
    """Generate the pack_object method for a BuiltinObject."""
    lines: list[str] = [
        f"_object_jsonc = {{'{METATYPE_PROPERTY_KEY}': {cls.metatype.value}}}",
    ]

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)
    for prop in wired_properties_in_order:
        if prop.is_computed:
            continue
        elif cls.metatype == StructType.VALUE and prop.name == "value":
            # generic value
            lines.append(
                f"_object_jsonc['{prop.id}'] = _encoder.pack_value(_object.type, _object.value, _options)"
            )
        else:
            # normal property
            pack_code = _generate_pack_jsonc_property(prop)
            lines.extend(pack_code)

    lines.append("return _object_jsonc")
    return "\n".join(lines)


def _generate_unpack_jsonc(cls: type["BuiltinObject"]) -> str:
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
                f"_unpacked_value = _encoder.unpack_value(_unpacked_type, _object_jsonc.get('{prop.id}'), _session, _options)"
            )
            assignments.append("value = _unpacked_value")
        else:
            # normal property
            prop_name = (
                prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
            )
            unpack_code = _generate_unpack_jsonc_property(prop)
            lines.extend(unpack_code)
            assignments.append(f"{prop_name}=_unpacked_{prop_name}")

    lines.append("return cls(")
    for assignment in assignments:
        lines.append(f"    {assignment},")
    lines.append("    _session=_session,")
    lines.append(")")

    return "\n".join(lines)


def _generate_pack_jsonc_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the packing code for a property value."""
    lines: list[str] = []
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    obj_jsonc = f"_object.{prop_name}"

    # scalar
    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_pack_jsonc_scalar(prop, obj_jsonc)
            lines.append(f'_object_jsonc["{prop.id}"] = {value_expr}')
        else:
            lines.append(f"if ({prop_name} := {obj_jsonc}) is not None:")
            value_expr = _generate_pack_jsonc_scalar(prop, prop_name)
            lines.append(f'    _object_jsonc["{prop.id}"] = {value_expr}')

    # list
    elif prop.cardinality == TypeCardinality.LIST:
        assert prop.value_type is not None, f"no value type for {prop!r}"
        item_expr = _generate_pack_jsonc_scalar(prop.value_type, "_item")
        if prop.is_required:
            lines.append(f"_packed_{prop_name} = []")
            lines.append(f"for _item in {obj_jsonc}:")
            lines.append(f"    _packed_{prop_name}.append({item_expr})")
            lines.append(f'_object_jsonc["{prop.id}"] = _packed_{prop_name}')
        else:
            lines.append(f"if {obj_jsonc} is not None:")
            lines.append(f"    _packed_{prop_name} = []")
            lines.append(f"    for _item in {obj_jsonc}:")
            lines.append(f"        _packed_{prop_name}.append({item_expr})")
            lines.append(f'    _object_jsonc["{prop.id}"] = _packed_{prop_name}')

    # tuple
    elif prop.cardinality == TypeCardinality.TUPLE:
        raise NotImplementedError(f"cannot pack tuple: {prop!r}")

    # map
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        assert prop.value_type is not None, f"no value type for {prop!r}"
        key_expr = _generate_pack_jsonc_scalar(prop.key_type, "_key")
        value_expr = _generate_pack_jsonc_scalar(prop.value_type, "_jsonc")
        if prop.is_required:
            lines.append(f"_packed_{prop_name} = {{}}")
            lines.append(f"for _key, _jsonc in {obj_jsonc}.items():")
            lines.append(f"    _packed_{prop_name}[str({key_expr})] = {value_expr}")
            lines.append(f'_object_jsonc["{prop.id}"] = _packed_{prop_name}')
        else:
            lines.append(f"if {obj_jsonc} is not None:")
            lines.append(f"    _packed_{prop_name} = {{}}")
            lines.append(f"    for _key, _jsonc in {obj_jsonc}.items():")
            lines.append(f"        _packed_{prop_name}[str({key_expr})] = {value_expr}")
            lines.append(f'    _object_jsonc["{prop.id}"] = _packed_{prop_name}')

    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_jsonc_scalar(
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
        elif (
            prop.primitive_type == PrimitiveType.BOOLEAN
            or prop.primitive_type
            in (
                PrimitiveType.INT8,
                PrimitiveType.INT16,
                PrimitiveType.INT32,
                PrimitiveType.INT64,
                PrimitiveType.INT128,
                PrimitiveType.UINT8,
                PrimitiveType.UINT16,
                PrimitiveType.UINT32,
                PrimitiveType.UINT64,
                PrimitiveType.UINT128,
            )
            or prop.primitive_type
            in (
                PrimitiveType.FLOAT16,
                PrimitiveType.FLOAT32,
                PrimitiveType.FLOAT64,
            )
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
        elif (
            prop.primitive_type == PrimitiveType.STRING or prop.primitive_type == PrimitiveType.UUID
        ):
            return f"str({value_expr})"
        elif prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64encode({value_expr}).decode()"
        elif prop.primitive_type == PrimitiveType.JSON:
            return value_expr
        else:
            assert_never(prop.primitive_type)

    # enum
    elif prop.scalar_type == ScalarType.ENUM:
        return f"{value_expr}.value"

    # struct
    elif prop.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
        return f"{value_expr}.pack(Encoding.JSONC)"

    #
    else:
        assert_never(prop.scalar_type)


def _generate_unpack_jsonc_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the unpacking code for a property value."""
    lines: list[str] = []
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    data_jsonc = f'_object_jsonc.get("{prop.id}")'

    # scalar
    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_unpack_jsonc_scalar(prop, data_jsonc)
            lines.append(f"_unpacked_{prop_name} = {value_expr}")
        else:
            value_expr = _generate_unpack_jsonc_scalar(prop, prop_name)
            lines.append(
                f"_unpacked_{prop_name} = {value_expr} if ({prop_name} := {data_jsonc}) is not None else None"
            )

    # list
    elif prop.cardinality == TypeCardinality.LIST:
        assert prop.value_type is not None, f"no value type for {prop!r}"
        item_expr = _generate_unpack_jsonc_scalar(prop.value_type, "_item")
        if prop.is_required:
            lines.append(f"_unpacked_{prop_name} = []")
            lines.append(f"for _item in {data_jsonc}:")
            lines.append(f"    _unpacked_{prop_name}.append({item_expr})")
        else:
            lines.append(f"_unpacked_{prop_name} = []")
            lines.append(f"if {data_jsonc} is not None:")
            lines.append(f"    for _item in {data_jsonc}:")
            lines.append(f"        _unpacked_{prop_name}.append({item_expr})")

    # tuple
    elif prop.cardinality == TypeCardinality.TUPLE:
        raise NotImplementedError(f"cannot unpack tuple: {prop!r}")

    # map
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        assert prop.value_type is not None, f"no value type for {prop!r}"
        key_expr = _generate_unpack_jsonc_scalar(prop.key_type, "_key")
        value_expr = _generate_unpack_jsonc_scalar(prop.value_type, "_jsonc")
        if prop.is_required:
            lines.append(f"_unpacked_{prop_name} = {{}}")
            lines.append(f"for _key, _jsonc in {data_jsonc}.items():")
            lines.append(f"    _unpacked_{prop_name}[{key_expr}] = {value_expr}")
            lines.append("else:")
            lines.append(f"    _unpacked_{prop_name} = None")
        else:
            lines.append(f"if {data_jsonc} is not None:")
            lines.append(f"    _unpacked_{prop_name} = {{}}")
            lines.append(f"    for _key, _jsonc in {data_jsonc}.items():")
            lines.append(f"        _unpacked_{prop_name}[{key_expr}] = {value_expr}")
            lines.append("else:")
            lines.append(f"    _unpacked_{prop_name} = None")

    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_jsonc_scalar(
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
        if prop.primitive_type == PrimitiveType.BOOLEAN:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.INT8,
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.INT128,
        ):
            return f"int({value_expr})"  # cast JSONC floats to ints
        elif prop.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return f"int({value_expr})"
        elif prop.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return value_expr
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"datetime.fromisoformat({value_expr}).astimezone(UTC)"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"date.fromisoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"time.fromisoformat({value_expr}).astimezone(UTC)"
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
        enum_type_name = prop.enum_type.camel_name
        return f"{enum_type_name}(int({value_expr}))"

    # struct
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"{struct_cls.__name__}.unpack(Encoding.JSONC, {value_expr}, _session)"

    # node reference
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"NodeReference.unpack(Encoding.JSONC, {value_expr}, _session)"

    # node value
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"Node.unpack(Encoding.JSONC, {value_expr}, _session)"

    #
    else:
        assert_never(prop.scalar_type)


JSONC_OBJECT_ENCODERS: dict[tuple[ObjectKind, NodeType | StructType], "JsoncObjectEncoder"] = {}


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
        encoder_name, impl, extra_glbls = _generate_jsonc_object_encoder(node_cls)
        locals_ = {}
        exec_(
            impl,
            {**builtin_class_by_name, **extra_glbls},
            locals_,
            encoder_name,
        )
        encoder_cls = locals_[encoder_name]
        JSONC_OBJECT_ENCODERS[node_cls.__kind__, node_cls.metatype] = encoder_cls()


_generate()
