import base64
import textwrap
from datetime import UTC, date, datetime, time, timedelta
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never, override

import structlog
from opentelemetry import trace

from destack.language.core import (
    BuiltinObject,
    Cson,
    Graph,
    GraphConnection,
    NodeType,
    ObjectKind,
    PackedCache,
    PrimitiveType,
    PropertyDeclaration,
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
    get_builtin_type,
)
from destack.utils.code import exec_
from destack.utils.time import timedelta_from_isoformat, timedelta_to_isoformat
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass


# ruff: noqa: FURB113
# pyright: reportIncompatibleVariableOverride=false


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)
type_ = type


class _CsonObjectEncoder:
    def pack_object(self, object: BuiltinObject) -> Cson:
        raise NotImplementedError

    def unpack_object(
        self,
        cson: Cson,
        session: Session,
        graph: Graph,
        connection: GraphConnection | None,
    ) -> BuiltinObject:
        raise NotImplementedError


def _generate_encoder_impl(cls: type["BuiltinObject"]) -> tuple[str, str, dict[str, Any]]:
    """Generate the CsonObjectEncoder class for a BuiltinObject."""

    pack_cson = textwrap.indent(_generate_pack_cson(cls), " " * 8)
    unpack_cson = textwrap.indent(_generate_unpack_cson(cls), " " * 8)
    encoder_name = f"{cls.__name__}CsonEncoder"

    impl = f"""
class {encoder_name}(CsonObjectEncoder):
    
    @override
    def pack_object(self, _object: "{cls.__name__}") -> Cson:
{pack_cson}

    @override
    def unpack_object(
        self, 
        _object_cson: Cson,
        _session: "Session | None" = None,
        _graph: "Graph | None" = None,
        _connection: "GraphConnection | None" = None,
    ) -> "{cls.__name__}":
{unpack_cson}
"""
    return (
        encoder_name,
        impl,
        {
            "CsonObjectEncoder": _CsonObjectEncoder,
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
            "Cson": Cson,
            "Self": cls,
            "cls": cls,
            "BuiltinObject": BuiltinObject,
            "PackedCache": PackedCache,
        },
    )


def _generate_pack_cson(cls: type["BuiltinObject"]) -> str:
    """Generate the pack_object method for a BuiltinObject."""
    pack_method_parts: list[str] = []
    pack_method_parts.append("_object_cson = {}")

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)
    for prop in wired_properties_in_order:
        if prop.name == "metatype":
            metatype = get_builtin_type(cls)
            pack_method_parts.append(f'_object_cson["{prop.id}"] = {metatype.value}')
            continue

        pack_code = _generate_pack_cson_property(prop)
        if pack_code:
            pack_method_parts.extend(pack_code)

    pack_method_parts.append("return _object_cson")
    return "\n".join(pack_method_parts)


def _generate_unpack_cson(cls: type["BuiltinObject"]) -> str:
    """Generate the unpack_object method for a BuiltinObject."""
    unpack_assignments: list[str] = []
    unpack_method_parts: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.is_computed:
            continue  # set implicitly
        prop_name = (
            prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
        )
        unpack_code = _generate_unpack_cson_property(prop)
        if len(unpack_code) == 1:
            unpack_assignments.append(f"{prop_name}={unpack_code[0].split(' = ', 1)[1]}")
        else:
            unpack_method_parts.extend(unpack_code)
            unpack_assignments.append(f"{prop_name}=_unpacked_{prop_name}")
    if cls.__is_frozen__ and not cls.__is_node__:
        unpack_assignments.append(
            "_packed_cache = (PackedCache(Encoding.CSON, False, _object_cson),)"
        )

    unpack_method_parts.append("return cls(")
    for assignment in unpack_assignments:
        unpack_method_parts.append(f"    {assignment},")
    if cls.__is_node__:
        unpack_method_parts.append("    _session=_session,")
        unpack_method_parts.append("    _graph=_graph,")
        unpack_method_parts.append("    _connection=_connection,")
    else:
        unpack_method_parts.append("    _graph=_graph,")
    unpack_method_parts.append(")")

    return "\n".join(unpack_method_parts)


def _generate_pack_cson_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the packing code for a property value."""
    lines: list[str] = []
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    obj_cson = f"_object.{prop_name}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_pack_cson_scalar(prop, obj_cson)
            lines.append(f'_object_cson["{prop.id}"] = {value_expr}')
        else:
            lines.append(f"if ({prop_name} := {obj_cson}) is not None:")
            value_expr = _generate_pack_cson_scalar(prop, prop_name)
            lines.append(f'    _object_cson["{prop.id}"] = {value_expr}')
    elif prop.cardinality == TypeCardinality.LIST:
        lines.append(f"if {obj_cson}:")
        lines.append(f"    _packed_{prop_name} = []")
        lines.append(f"    for _item in {obj_cson}:")
        item_expr = _generate_pack_cson_scalar(prop, "_item")
        lines.append(f"        _packed_{prop_name}.append({item_expr})")
        lines.append(f'    _object_cson["{prop.id}"] = _packed_{prop_name}')
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"if {obj_cson}:")
        lines.append(f"    _packed_{prop_name} = {{}}")
        lines.append(f"    for _key, _cson in {obj_cson}.items():")
        key_expr = _generate_pack_cson_scalar(prop.key_type, "_key")
        value_expr = _generate_pack_cson_scalar(prop, "_cson")
        lines.append(f"        _packed_{prop_name}[str({key_expr})] = {value_expr}")
        lines.append(f'    _object_cson["{prop.id}"] = _packed_{prop_name}')
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_cson_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the unpacking code for a property value."""
    lines: list[str] = []
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    data_cson = f'_object_cson.get("{prop.id}")'

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_unpack_cson_scalar(prop, data_cson)
            lines.append(f"_unpacked_{prop_name} = {value_expr}")
        else:
            value_expr = _generate_unpack_cson_scalar(prop, prop_name)
            lines.append(
                f"_unpacked_{prop_name} = {value_expr} if ({prop_name} := {data_cson}) is not None else None"
            )
    elif prop.cardinality == TypeCardinality.LIST:
        lines.append(f"_unpacked_{prop_name} = []")
        lines.append(f"if {data_cson} is not None:")
        lines.append(f"    for _item in {data_cson}:")
        item_expr = _generate_unpack_cson_scalar(prop, "_item")
        lines.append(f"        _unpacked_{prop_name}.append({item_expr})")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"_unpacked_{prop_name} = {{}}")
        lines.append(f"if {data_cson} is not None:")
        lines.append(f"    for _key, _cson in {data_cson}.items():")
        key_expr = _generate_unpack_cson_scalar(prop.key_type, "_key")
        value_expr = _generate_unpack_cson_scalar(prop, "_cson")
        lines.append(f"        _unpacked_{prop_name}[{key_expr}] = {value_expr}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_cson_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the packing code for a scalar value."""

    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64encode({value_expr}).decode()"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"str({value_expr})"
        elif prop.primitive_type == PrimitiveType.CSON:
            return value_expr
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"{value_expr}.astimezone(UTC).isoformat()"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"{value_expr}.isoformat()"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"{value_expr}.astimezone(UTC).replace(tzinfo=None).isoformat()"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedelta_to_isoformat({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        return f"{value_expr}.value"
    elif prop.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
        return f"{value_expr}.pack(Encoding.CSON)"
    else:
        assert_never(prop.scalar_type)


def _generate_unpack_cson_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the unpacking code for a scalar value."""

    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64decode({value_expr})"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"UUID({value_expr})"
        elif prop.primitive_type == PrimitiveType.CSON:
            return value_expr
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"datetime.fromisoformat({value_expr}).astimezone(UTC)"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"date.fromisoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"time.fromisoformat({value_expr}).astimezone(UTC)"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedelta_from_isoformat({value_expr})"
        elif prop.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return f"int({value_expr})"  # cast CSON floats to ints
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        enum_type_name = prop.enum_type.camel_name
        return f"{enum_type_name}(int({value_expr}))"
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"{struct_cls.__name__}.unpack(Encoding.CSON, {value_expr}, _session=_session, _graph=_graph, _connection=_connection)"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"NodeReference.unpack(Encoding.CSON, {value_expr}, _session=_session, _graph=_graph, _connection=_connection)"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"Node.unpack(Encoding.CSON, {value_expr}, _session=_session, _graph=_graph, _connection=_connection)"
    else:
        assert_never(prop.scalar_type)


CSON_OBJECT_ENCODERS: dict[tuple[ObjectKind, NodeType | StructType], "_CsonObjectEncoder"] = {}


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
        encoder_name, impl, extra_glbls = _generate_encoder_impl(node_cls)
        locals_ = {}
        exec_(
            impl,
            {**builtin_class_by_name, **extra_glbls},
            locals_,
            f"{node_cls.__name__}:cson",
        )
        encoder_cls = locals_[encoder_name]
        CSON_OBJECT_ENCODERS[node_cls.__kind__, node_cls.metatype] = encoder_cls()


_generate()
