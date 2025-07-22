import textwrap
from datetime import UTC
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never, override

import structlog
from google.protobuf.duration_pb2 import Duration
from google.protobuf.timestamp_pb2 import Timestamp
from opentelemetry import trace

from destack import proto
from destack.language.core import (
    BuiltinObject,
    Graph,
    GraphConnection,
    NodeType,
    ObjectKind,
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
from destack.utils.uuid import UUID

from .utils import (
    pack_proto_duration,
    pack_proto_json,
    pack_proto_timestamp,
    unpack_proto_duration,
    unpack_proto_json,
    unpack_proto_timestamp,
)

if TYPE_CHECKING:
    pass

# ruff: noqa: FURB113
# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)
type_ = type


class _ProtoObjectEncoder:
    def pack_object(self, object: BuiltinObject) -> Any:
        raise NotImplementedError

    def unpack_object(
        self,
        proto: Any,
        session: Session,
        graph: Graph,
        connection: GraphConnection | None,
    ) -> BuiltinObject:
        raise NotImplementedError


def _generate_encoder_impl(cls: type["BuiltinObject"]) -> tuple[str, str, dict[str, Any]]:
    """Generate the ProtoObjectEncoder class for a BuiltinObject."""

    pack_proto = textwrap.indent(_generate_pack_proto(cls), " " * 8)
    unpack_proto = textwrap.indent(_generate_unpack_proto(cls), " " * 8)
    encoder_name = f"{cls.__name__}ProtoEncoder"

    impl = f"""
class {encoder_name}(ProtoObjectEncoder):
    
    @override
    def pack_object(self, _object: "{cls.__name__}") -> "{cls.__name__}Proto":
{pack_proto}

    @override
    def unpack_object(
        self, 
        _object_proto: "{cls.__name__}Proto",
        _session: "Session | None" = None,
        _graph: "Graph | None" = None,
        _connection: "GraphConnection | None" = None,
    ) -> "{cls.__name__}":
{unpack_proto}
"""
    return (
        encoder_name,
        impl,
        {
            "ProtoObjectEncoder": _ProtoObjectEncoder,
            "Timestamp": Timestamp,
            "Duration": Duration,
            "UTC": UTC,
            "pack_proto_json": pack_proto_json,
            "unpack_proto_json": unpack_proto_json,
            "pack_proto_timestamp": pack_proto_timestamp,
            "unpack_proto_timestamp": unpack_proto_timestamp,
            "pack_proto_duration": pack_proto_duration,
            "unpack_proto_duration": unpack_proto_duration,
            "override": override,
            "Self": cls,
            "BuiltinObject": BuiltinObject,
            "UUID": UUID,
        },
    )


def _generate_pack_proto(cls: type["BuiltinObject"]) -> str:
    """Generate the pack_object method for a BuiltinObject."""
    metatype = get_builtin_type(cls)
    pack_method_parts: list[str] = []
    pack_method_parts.append(f"_object_proto = {cls.__name__}Proto(metatype={metatype.value})")

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)
    for prop in wired_properties_in_order:
        if prop.name == "metatype":
            continue  # already set
        pack_code = _generate_pack_property(prop)
        if pack_code:
            pack_method_parts.extend(pack_code)

    pack_method_parts.append("return _object_proto")
    return "\n".join(pack_method_parts)


def _generate_unpack_proto(cls: type["BuiltinObject"]) -> str:
    """Generate the unpack_object method for a BuiltinObject."""
    unpack_assignments: list[str] = []
    unpack_method_parts: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.is_computed:
            continue  # set implicitly
        prop_name = (
            prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
        )
        unpack_code = _generate_unpack_property(prop)
        if unpack_code:
            if len(unpack_code) == 1:
                unpack_assignments.append(f"{prop_name}={unpack_code[0].split(' = ', 1)[1]}")
            else:
                unpack_method_parts.extend(unpack_code)
                unpack_assignments.append(f"{prop_name}=_unpacked_{prop_name}")
        else:
            unpack_assignments.append(f"{prop_name}=_object_proto.{prop_name}")
    if cls.__is_frozen__ and not cls.__is_node__:
        unpack_assignments.append("_proto=_object_proto")

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


"""'Primitives' that actually map to Proto structs (Messages)"""
_PROTO_PRIMITIVE_MESSAGE_TYPES = (
    PrimitiveType.DATE,
    PrimitiveType.TIME,
    PrimitiveType.DATETIME,
    PrimitiveType.DURATION,
    PrimitiveType.JSON,
    PrimitiveType.CSON,
)


def _is_proto_primitive(prop: "PropertyDeclaration | TypeDeclaration") -> bool:
    """Check if a property is a proto primitive type."""
    return prop.cardinality == TypeCardinality.SCALAR and (
        prop.scalar_type == ScalarType.ENUM
        or (
            prop.scalar_type == ScalarType.PRIMITIVE
            and prop.primitive_type not in _PROTO_PRIMITIVE_MESSAGE_TYPES
        )
    )


def _generate_pack_property(prop: "PropertyDeclaration") -> list[str] | None:
    """Generate the packing code for a property value."""
    lines: list[str] = []
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    obj_value = f"_object.{prop_name}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_optional:
            lines.append(f"if ({prop_name} := {obj_value}) is not None:")
            scalar_expr = _generate_pack_scalar(prop, prop_name)
            if _is_proto_primitive(prop):
                lines.append(f"    _object_proto.{prop_name} = {scalar_expr}")
            else:
                lines.append(f"    _object_proto.{prop_name}.CopyFrom({scalar_expr})")
        else:
            scalar_expr = _generate_pack_scalar(prop, obj_value)
            if _is_proto_primitive(prop):
                lines.append(f"_object_proto.{prop_name} = {scalar_expr}")
            else:
                lines.append(f"_object_proto.{prop_name}.CopyFrom({scalar_expr})")
    elif prop.cardinality == TypeCardinality.LIST:
        item_expr = _generate_pack_scalar(prop, "_item")
        if _is_proto_primitive(prop):
            lines.extend(
                f"""\
if {obj_value}:
    _packed_{prop_name} = []
    for _item in {obj_value}:
        _packed_{prop_name}.append({item_expr})
    _object_proto.{prop_name} = _packed_{prop_name}""".splitlines()
            )
        else:
            lines.extend(
                f"""\
if {obj_value}:
    for _item in {obj_value}:
        _object_proto.{prop_name}.append({item_expr})""".splitlines()
            )
    elif prop.cardinality == TypeCardinality.MAP:
        lines.extend(
            f"""\
if {obj_value}:
    for _key, _value in {obj_value}.items():""".splitlines()
        )
        assert prop.key_type is not None, f"no key_type for map: {prop!r}"
        key_expr = _generate_pack_scalar(prop.key_type, "_key")
        value_expr = _generate_pack_scalar(prop, "_value")
        if _is_proto_primitive(prop):
            lines.append(f"        _object_proto.{prop_name}[{key_expr}] = {value_expr}")
        else:
            lines.append(f"        _object_proto.{prop_name}[{key_expr}].CopyFrom({value_expr})")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_property(prop: "PropertyDeclaration") -> list[str] | None:
    """Generate the unpacking code for a property value."""

    def _wrap_with_null_check(code: str) -> str:
        """Wrap code with null check if property is optional."""
        if prop.is_required:
            return code
        else:
            return f"{code} if _object_proto.HasField('{prop_name}') else None"

    lines: list[str] = []
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    proto_value = f"_object_proto.{prop_name}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_optional:
            scalar_expr = _generate_unpack_scalar(prop, proto_value)
            lines.append(f"_unpacked_{prop_name} = {_wrap_with_null_check(scalar_expr)}")
        else:
            scalar_expr = _generate_unpack_scalar(prop, proto_value)
            lines.append(f"_unpacked_{prop_name} = {scalar_expr}")
    elif prop.cardinality == TypeCardinality.LIST:
        lines.extend(
            f"""\
_unpacked_{prop_name} = []
for _item in {proto_value}:""".splitlines()
        )
        item_expr = _generate_unpack_scalar(prop, "_item")
        lines.append(f"    _unpacked_{prop_name}.append({item_expr})")
    elif prop.cardinality == TypeCardinality.MAP:
        lines.extend(
            f"""\
_unpacked_{prop_name} = {{}}
for _key, _value in {proto_value}.items():""".splitlines()
        )
        assert prop.key_type is not None, f"no key_type for map: {prop!r}"
        key_expr = _generate_unpack_scalar(prop.key_type, "_key")
        value_expr = _generate_unpack_scalar(prop, "_value")
        lines.append(f"    _unpacked_{prop_name}[{key_expr}] = {value_expr}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_scalar(prop: "PropertyDeclaration | TypeDeclaration", value_expr: str) -> str:
    """Generate the packing code for a scalar value."""

    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.UUID:
            return f"str({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"pack_proto_timestamp({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"pack_proto_duration({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON or prop.primitive_type == PrimitiveType.CSON:
            return f"pack_proto_json({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        return f"{value_expr}.value"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"{value_expr}.pack(Encoding.PROTO)"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise RuntimeError(f"node_value cannot be wired directly: {prop!r}")
    elif prop.scalar_type == ScalarType.STRUCT:
        return f"{value_expr}.pack(Encoding.PROTO)"
    else:
        assert_never(prop.scalar_type)


def _generate_unpack_scalar(prop: "PropertyDeclaration | TypeDeclaration", value_expr: str) -> str:
    """Generate the unpacking code for a scalar value."""

    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.UUID:
            return f"UUID({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"unpack_proto_timestamp({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"unpack_proto_duration({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON or prop.primitive_type == PrimitiveType.CSON:
            return f"unpack_proto_json({value_expr})"
        else:
            return f"{value_expr}"
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None
        enum_type_name = prop.enum_type.camel_name
        return f"{enum_type_name}({value_expr})"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return (
            f"NodeReference.unpack(Encoding.PROTO, {value_expr}, _session=_session, _graph=_graph)"
        )
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise RuntimeError(f"node_value cannot be wired directly: {prop!r}")
    elif prop.scalar_type == ScalarType.STRUCT:
        return f"{value_expr}.unpack(Encoding.PROTO, _session=_session, _graph=_graph)"
    else:
        assert_never(prop.scalar_type)


PROTO_OBJECT_ENCODERS: dict[tuple[ObjectKind, NodeType | StructType], "_ProtoObjectEncoder"] = {}
PROTO_CLASSES: dict[tuple[ObjectKind, NodeType | StructType], type[proto.AnyObjectProto]] = {}


def _generate():
    # generate pack/unpack methods
    builtin_class_by_name: dict[str, Any] = {**proto.__dict__, "UUID": UUID}
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
        proto_cls = proto.__dict__.get(node_cls.__name__ + "Proto")
        if proto_cls is None:
            continue  # probably not yet generated
        PROTO_CLASSES[node_cls.__kind__, node_cls.metatype] = proto_cls
        encoder_name, impl, extra_glbls = _generate_encoder_impl(node_cls)
        locals_ = {}
        exec_(
            impl,
            {**builtin_class_by_name, **extra_glbls},
            locals_,
            f"{node_cls.__name__}:proto",
        )
        encoder_cls = locals_[encoder_name]
        PROTO_OBJECT_ENCODERS[node_cls.__kind__, node_cls.metatype] = encoder_cls()


_generate()
