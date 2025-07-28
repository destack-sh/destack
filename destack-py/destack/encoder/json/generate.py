import base64
import textwrap
from datetime import UTC, date, datetime, time, timedelta
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never, override

from destack.language.core import (
    UNSET,
    BuiltinObject,
    Entity,
    Materialization,
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

    is_entity = issubclass(cls, Entity)
    pack_json = _generate_pack_object_json(cls, is_entity=is_entity)
    unpack_json = _generate_unpack_object_json(cls, is_entity=is_entity)
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
{textwrap.indent(pack_json, " " * 8)}

    @override
    def unpack_object(
        self,
        _encoder: "JsonEncoder",
        _object_json: "dict[str, Any]",
        _session: "Session | None",
        _options: "EncoderOptions",
    ) -> "{cls.__name__}":
{textwrap.indent(unpack_json, " " * 8)}
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
            "UNSET": UNSET,
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
        },
    )


def _get_property_name(prop: PropertyDeclaration) -> str:
    """Get the name of a property."""
    return prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"


def _generate_pack_object_json(
    cls: type["BuiltinObject"],
    *,
    is_entity: bool,
) -> str:
    """Generate the pack method for a BuiltinObject."""
    lines: list[str] = [
        f"_object_json: dict[str, Any] = {{'metatype': '{cls.metatype.name}'}}",
    ]

    # collect properties
    properties = list(cls.__wired_properties__.values())
    properties.sort(key=lambda p: p.id or 0)
    if is_entity:
        set_properties = [p for p in properties if p.is_identity]
        maybe_set_properties = [p for p in properties if not p.is_identity]
    else:
        set_properties = properties
        maybe_set_properties = []

    # pack always set properties
    for prop in set_properties:
        if cls.metatype == StructType.VALUE and prop.name == "value":
            # special case: generic Value.value
            lines.append(
                f"_object_json['{prop.name}'] = _encoder.pack_value(_object.type, _object.value, _options)"
            )
            continue

        prop_name = _get_property_name(prop)
        pack_code = _generate_pack_json_value(
            prop,
            key=f"_{prop_name}",
            source_expr=f"_object.{prop_name}",
            target_expr=f"_object_json['{to_casing(prop.name, Casing.LOWER_CAMEL)}']",
        )
        lines.append(pack_code)

    # pack partial properties
    if is_entity and maybe_set_properties:
        maybe_set_lines: list[str] = []
        set_lines: list[str] = []
        for prop in maybe_set_properties:
            prop_name = _get_property_name(prop)
            pack_code = _generate_pack_json_value(
                prop,
                key=f"_{prop_name}",
                source_expr=f"_object.{prop_name}",
                target_expr=f"_object_json['{to_casing(prop.name, Casing.LOWER_CAMEL)}']",
            )
            set_lines.append(pack_code)
            maybe_set_lines.append(f"""\
if _object.is_set("{prop.name}"):
{textwrap.indent(pack_code, " " * 4)}
""")

        maybe_set_code = "\n".join(maybe_set_lines)
        set_code = "\n".join(set_lines)
        lines.append(f"""\
if _object.is_partial:
{textwrap.indent(maybe_set_code, " " * 4)}
else:
{textwrap.indent(set_code, " " * 4)}
""")

    lines.append("return _object_json")
    return "\n".join(lines)


def _generate_unpack_object_json(cls: type["BuiltinObject"], *, is_entity: bool) -> str:
    """Generate the unpack method for a BuiltinObject."""
    assignments: list[str] = []
    lines: list[str] = []
    if is_entity:
        lines.append(
            f"_is_partial = Materialization[_object_json.get('materialization')] < {Materialization.FULL}"
        )

    # collect properties
    properties = list(cls.__wired_properties__.values())
    properties.sort(key=lambda p: p.id or 0)
    if is_entity:
        set_properties = [p for p in properties if p.is_identity]
        maybe_set_properties = [p for p in properties if not p.is_identity]
    else:
        set_properties = properties
        maybe_set_properties = []

    # unpack always set properties
    for prop in set_properties:
        if cls.metatype == StructType.VALUE and prop.name == "value":
            # special case: generic Value.value
            lines.append(
                "_unpacked_value = _encoder.unpack_value(_unpacked_type, _object_json.get('value'), _session, _options)"
            )
            assignments.append("value = _unpacked_value")
            continue

        prop_name = _get_property_name(prop)
        unpack_code = _generate_unpack_json_value(
            prop,
            key=f"_{prop_name}",
            source_expr=f'_object_json.get("{to_casing(prop.name, Casing.LOWER_CAMEL)}")',
            target_expr=f"_unpacked_{prop_name}",
        )
        lines.append(unpack_code)
        assignments.append(f"{prop_name}=_unpacked_{prop_name}")

    # unpack partial properties
    if is_entity and maybe_set_properties:
        maybe_set_lines: list[str] = []
        set_lines: list[str] = []
        for prop in maybe_set_properties:
            prop_name = _get_property_name(prop)
            unpack_code = _generate_unpack_json_value(
                prop,
                key=f"_{prop_name}",
                source_expr=f'_object_json.get("{to_casing(prop.name, Casing.LOWER_CAMEL)}")',
                target_expr=f"_unpacked_{prop_name}",
            )
            set_lines.append(unpack_code)
            maybe_set_lines.append(f"""\
if "{to_casing(prop.name, Casing.LOWER_CAMEL)}" in _object_json:
{textwrap.indent(unpack_code, " " * 4)}
else:
    _unpacked_{prop_name} = UNSET
""")
            assignments.append(f"{prop_name}=_unpacked_{prop_name}")

        maybe_set_code = "\n".join(maybe_set_lines)
        set_code = "\n".join(set_lines)
        lines.append(f"""\
if _is_partial:
{textwrap.indent(maybe_set_code, " " * 4)}
else:
{textwrap.indent(set_code, " " * 4)}
""")

    lines.append(f"return {cls.__name__}(")
    for assignment in assignments:
        lines.append(f"    {assignment},")
    lines.append("    _session=_session,")
    lines.append(")")

    return "\n".join(lines)


def _generate_pack_json_value(
    prop: TypeDeclaration, key: str, source_expr: str, target_expr: str
) -> str:
    """Generate code to pack some value into the JSON encoding."""
    # scalar
    if prop.cardinality == TypeCardinality.SCALAR:
        value_packed = _generate_pack_json_scalar_value(prop, source_expr)
        if prop.is_required:
            return f"{target_expr} = {value_packed}"
        else:
            return f"""\
if {source_expr} is not None:
    {target_expr} = {value_packed}
else:
    {target_expr} = None"""

    # list
    elif prop.cardinality == TypeCardinality.LIST:
        assert prop.value_type is not None, f"no value type for {prop!r}"
        item_source_expr = f"{key}_item"
        item_target_expr = f"{key}_item_json"
        item_packed = _generate_pack_json_value(
            prop.value_type,
            key=f"{key}_value",
            source_expr=item_source_expr,
            target_expr=item_target_expr,
        )
        if prop.is_required:
            return f"""\
{target_expr} = []
for {item_source_expr} in {source_expr}:
{textwrap.indent(item_packed, " " * 4)}
    {target_expr}.append({item_target_expr})"""
        else:
            return f"""\
if {source_expr} is not None:
    {target_expr} = []
    for {item_source_expr} in {source_expr}:
{textwrap.indent(item_packed, " " * 8)}
        {target_expr}.append({item_target_expr})
else:
    {target_expr} = None"""

    # tuple
    elif prop.cardinality == TypeCardinality.TUPLE:
        assert prop.element_types is not None, f"no element types for {prop!r}"
        element_target_exprs = []
        element_packs = []
        for i, element_type in enumerate(prop.element_types):
            element_source_expr = f"{key}_element_{i}"
            element_target_expr = f"{key}_element_{i}_json"
            element_target_exprs.append(element_target_expr)
            element_packed = _generate_pack_json_value(
                element_type,
                key=f"{key}_element_{i}",
                source_expr=element_source_expr,
                target_expr=element_target_expr,
            )
            element_packs.append(element_packed)
        element_packed_str = "\n".join(element_packs)
        if prop.is_required:
            return f"""\
    {element_packed_str}
    {target_expr} = [{", ".join(element_target_exprs)}]"""
        else:
            return f"""\
if {source_expr} is not None:
{textwrap.indent(element_packed_str, " " * 4)}
    {target_expr} = [{", ".join(element_target_exprs)}]
elif not _options.omit_none:
    {target_expr} = None"""

    # map
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        assert prop.value_type is not None, f"no value type for {prop!r}"
        key_source_expr = f"{key}_key"
        key_target_expr = f"{key}_key_json"
        value_source_expr = f"{key}_value"
        value_target_expr = f"{key}_value_json"
        key_packed = _generate_pack_json_value(
            prop.key_type,
            key=f"{key}_key",
            source_expr=key_source_expr,
            target_expr=key_target_expr,
        )
        value_packed = _generate_pack_json_value(
            prop.value_type,
            key=f"{key}_value",
            source_expr=value_source_expr,
            target_expr=value_target_expr,
        )
        if prop.is_required:
            return f"""\
{target_expr} = {{}}
for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_packed, " " * 4)}
{textwrap.indent(value_packed, " " * 4)}
    {target_expr}[{key_target_expr}] = {value_target_expr}"""
        else:
            return f"""\
if {source_expr} is not None:
    {target_expr} = {{}}
    for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_packed, " " * 8)}
{textwrap.indent(value_packed, " " * 8)}
        {target_expr}[{key_target_expr}] = {value_target_expr}
elif not _options.omit_none:
    {target_expr} = None"""

    else:
        assert_never(prop.cardinality)


def _generate_unpack_json_value(
    prop: TypeDeclaration, key: str, source_expr: str, target_expr: str
) -> str:
    """Generate code to unpack a property from the JSON encoding."""

    # scalar
    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_unpacked = _generate_unpack_json_scalar_value(prop, source_expr)
            return f"{target_expr} = {value_unpacked}"
        else:
            value_unpacked = _generate_unpack_json_scalar_value(prop, source_expr)
            return f"""\
if {source_expr} is not None:
    {target_expr} = {value_unpacked}
else:
    {target_expr} = None"""

    # list
    elif prop.cardinality == TypeCardinality.LIST:
        assert prop.value_type is not None, f"no value type for {prop!r}"
        item_source_expr = f"{key}_item_json"
        item_target_expr = f"{key}_item"
        item_unpacked = _generate_unpack_json_value(
            prop.value_type,
            key=f"{key}_value",
            source_expr=item_source_expr,
            target_expr=item_target_expr,
        )
        if prop.is_required:
            return f"""\
{target_expr} = []
for {item_source_expr} in {source_expr}:
{textwrap.indent(item_unpacked, " " * 4)}
    {target_expr}.append({item_target_expr})"""
        else:
            return f"""\
if {source_expr} is not None:
    {target_expr} = []
    for {item_source_expr} in {source_expr}:
{textwrap.indent(item_unpacked, " " * 8)}
        {target_expr}.append({item_target_expr})
else:
    {target_expr} = None"""

    # tuple
    elif prop.cardinality == TypeCardinality.TUPLE:
        assert prop.element_types is not None, f"no element types for {prop!r}"
        element_target_exprs = []
        element_unpacks = []
        for i, element_type in enumerate(prop.element_types):
            element_source_expr = f"{key}_element_{i}_json"
            element_target_expr = f"{key}_element_{i}"
            element_target_exprs.append(element_target_expr)
            element_unpacked = _generate_unpack_json_value(
                element_type,
                key=f"{key}_element_{i}",
                source_expr=element_source_expr,
                target_expr=element_target_expr,
            )
            element_unpacks.append(element_unpacked)
        element_unpacked_str = "\n".join(element_unpacks)
        if prop.is_required:
            return f"""\
    {element_unpacked_str}
    {target_expr} = [{", ".join(element_target_exprs)}]"""
        else:
            return f"""\
if {source_expr} is not None:
    {textwrap.indent(element_unpacked_str, " " * 4)}
    {target_expr} = ({", ".join(element_target_exprs)})
else:
    {target_expr} = None"""

    # map
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        assert prop.value_type is not None, f"no value type for {prop!r}"
        key_source_expr = f"{key}_key_json"
        key_target_expr = f"{key}_key"
        value_source_expr = f"{key}_value_json"
        value_target_expr = f"{key}_value"
        key_unpacked = _generate_unpack_json_value(
            prop.key_type,
            key=f"{key}_key",
            source_expr=key_source_expr,
            target_expr=key_target_expr,
        )
        value_unpacked = _generate_unpack_json_value(
            prop.value_type,
            key=f"{key}_value",
            source_expr=value_source_expr,
            target_expr=value_target_expr,
        )
        if prop.is_required:
            return f"""\
{target_expr} = {{}}
for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_unpacked, " " * 4)}
{textwrap.indent(value_unpacked, " " * 4)}
    {target_expr}[{key_target_expr}] = {value_target_expr}"""
        else:
            return f"""\
if {source_expr} is not None:
    {target_expr} = {{}}
    for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_unpacked, " " * 8)}
{textwrap.indent(value_unpacked, " " * 8)}
        {target_expr}[{key_target_expr}] = {value_target_expr}
else:
    {target_expr} = None"""

    else:
        assert_never(prop.cardinality)


def _generate_pack_json_scalar_value(
    prop: "PropertyDeclaration | TypeDeclaration", source_expr: str
) -> str:
    """Generate the packing code for a scalar value in the JSON encoding."""

    assert prop.cardinality == TypeCardinality.SCALAR, f"cannot pack non-scalar: {prop!r}"
    assert prop.scalar_type is not None, f"no scalar type for {prop!r}"

    # primitive
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.NONE:
            return "None"
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return source_expr
        elif prop.primitive_type in (
            PrimitiveType.INT8,
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.INT128,
        ):
            return source_expr
        elif prop.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return source_expr
        elif prop.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return source_expr
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"{source_expr}.astimezone(UTC).isoformat()"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"{source_expr}.isoformat()"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"{source_expr}.astimezone(UTC).replace(tzinfo=None).isoformat()"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedelta_to_isoformat({source_expr})"
        elif prop.primitive_type == PrimitiveType.STRING:
            return source_expr
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"str({source_expr})"
        elif prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64encode({source_expr}).decode()"
        elif prop.primitive_type == PrimitiveType.JSON:
            return source_expr
        else:
            assert_never(prop.primitive_type)

    # enum
    elif prop.scalar_type == ScalarType.ENUM:
        # use the enum name in CONSTANT_UPPER_CASE for JSON
        return f"{source_expr}.name"

    # struct
    elif prop.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
        return f"""\
_encoder.pack_object({ObjectKind.STRUCT}, {source_expr}.metatype, {source_expr}, _options)"""

    # node reference
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"""\
_encoder.pack_object({ObjectKind.STRUCT}, {source_expr}.metatype, {source_expr}, _options)"""

    # node value
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"""\
_encoder.pack_object({ObjectKind.STRUCT}, {source_expr}.metatype, {source_expr}, _options)"""

    #
    else:
        assert_never(prop.scalar_type)


def _generate_unpack_json_scalar_value(prop: "TypeDeclaration", source_expr: str) -> str:
    """Generate the unpacking code for a scalar value in the JSON encoding."""

    assert prop.cardinality == TypeCardinality.SCALAR, f"cannot unpack non-scalar: {prop!r}"
    assert prop.scalar_type is not None, f"no scalar type for {prop!r}"

    # primitive
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.NONE:
            return "None"
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return source_expr
        elif prop.primitive_type in (
            PrimitiveType.INT8,
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.INT128,
        ):
            return source_expr
        elif prop.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return source_expr
        elif prop.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return source_expr
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"datetime.fromisoformat({source_expr})"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"date.fromisoformat({source_expr})"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"time.fromisoformat({source_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedelta_from_isoformat({source_expr})"
        elif prop.primitive_type == PrimitiveType.STRING:
            return source_expr
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"UUID({source_expr})"
        elif prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64decode({source_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return source_expr
        else:
            assert_never(prop.primitive_type)

    # enum
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
        return f"{enum_cls.__name__}[{source_expr}]"

    # struct
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        return f"""\
_encoder.unpack_object({ObjectKind.STRUCT}, StructType[{source_expr}['metatype']], {source_expr}, _session, _options)"""

    # node reference
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"""\
_encoder.unpack_object({ObjectKind.STRUCT}, {StructType.NODE_REFERENCE}, {source_expr}, _session, _options)"""

    # node value
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"""\
_encoder.unpack_object({ObjectKind.NODE}, NodeType[{source_expr}['metatype']], {source_expr}, _session, _options)"""

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
        print("=" * 80)
        print(node_cls.__name__ + ":json")
        print("=" * 80)
        print(impl)
        print("=" * 80)
        exec_(
            impl,
            {**builtin_class_by_name, **extra_glbls},
            locals_,
            encoder_name,
        )
        encoder_cls = locals_[encoder_name]
        JSON_OBJECT_ENCODERS[node_cls.__kind__, node_cls.metatype] = encoder_cls()


_generate()
