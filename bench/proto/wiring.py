import json
import textwrap
from base64 import b64decode, b64encode
from itertools import chain
from typing import TYPE_CHECKING, Mapping, Union, assert_never, cast

import structlog
from opentelemetry import trace

from bench import pb2
from bench.language.core import (
    BuiltinObject,
    EditType,
    NodeType,
    PrimitiveType,
    Property,
    StructType,
    TypeAnnotation,
)
from bench.language.registry import BUILTIN_OBJECT_CLASS_BY_TYPE, BUILTIN_OBJECT_TYPE_BY_CLASS
from bench.pb2 import AnyNodeData, AnyStructData, EditData, NodeReferenceData, RpcMetadata
from bench.utils.string import Casing, to_casing

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


PROTO_CLASS_BY_TYPE: dict[NodeType | StructType, type[Union[AnyNodeData, AnyStructData]]] = {
    object_type: getattr(pb2, object_type.bench_name + "Data")
    for object_type in chain(NodeType, StructType)
    if hasattr(pb2, object_type.bench_name + "Data")  # may just be creating a new class
}
OBJECT_TYPE_BY_PROTO_CLASS: dict[type[Union[AnyNodeData, AnyStructData]], NodeType | StructType] = {
    cls: object_type for object_type, cls in PROTO_CLASS_BY_TYPE.items()
}
BENCH_CLASS_BY_PROTO_CLASS: dict[type[Union[AnyNodeData, AnyStructData]], type[BuiltinObject]] = {
    cls: BUILTIN_OBJECT_CLASS_BY_TYPE[object_type]
    for cls, object_type in OBJECT_TYPE_BY_PROTO_CLASS.items()
}


def describe_node_ptr(ptr: NodeReferenceData) -> str:
    """Describe a pointer."""
    try:
        node_type_name = NodeType(ptr.node_type).name
    except ValueError:
        node_type_name = "???"
    if ptr.ck:
        return f"{node_type_name}[id={ptr.id}, ck={ptr.ck}]"
    else:
        return f"{node_type_name}[id={ptr.id}]"


def describe_node(node: AnyNodeData) -> str:
    """Describe a node."""
    node_type = NodeType(node.metatype)
    node_parts: list[str] = [f"id={node.id or '???'}"]
    if node.HasField("ck"):
        node_parts.append(f"ck={node.ck or '???'}")  # type: ignore
    if node.HasField("name") and node.name:  # type: ignore
        node_parts.append(f"name={node.name}")  # type: ignore
    if node.HasField("slug") and node.slug:  # type: ignore
        node_parts.append(f"slug={node.slug}")  # type: ignore
    return f"{node_type.name}({', '.join(node_parts)})"


def describe_edit(edit: EditData) -> str:
    """Describe an edit."""
    edit_type = EditType(edit.type)
    node_str = describe_node_ptr(edit.node_ptr)
    return f"{edit_type.name}[id={edit.id}, edited_at={edit.edited_at.ToJsonString()}, node={node_str}]"


def copy_struct[T: AnyStructData | AnyNodeData](data: T) -> T:
    """Deepcopy a struct data object."""
    copy = type(data)(metatype=data.metatype)  # type: ignore
    copy.CopyFrom(data)  # type: ignore
    return copy


def _generate_pack_proto_for_cls(cls: type["BuiltinObject"]) -> str:
    """
    Generate BuiltinObject.__pack_proto__ and __unpack_proto__ class methods.
    """
    pack_proto = textwrap.indent(_generate_pack_proto(cls), "    ")
    unpack_proto = textwrap.indent(_generate_unpack_proto(cls), "    ")

    if cls.__is_frozen__:
        to_proto = """\
def _to_proto(self: "Self") -> "StructDataT":
    if self._proto is None:
        self._proto = self.__pack_proto__(self)
    return self._proto
"""
    else:
        to_proto = """\
def _to_proto(self: "Self") -> "StructDataT":
    return self.__pack_proto__(self)
"""

    return f"""
@classmethod
def __pack_proto__(cls, object: "Self") -> "{cls.__name__}Data":
{pack_proto}

@classmethod
def __unpack_proto__(cls, object_data: "{cls.__name__}Data") -> "Self":
{unpack_proto}

{to_proto}

_from_proto = __unpack_proto__
"""


def _generate_pack_proto(cls: type["BuiltinObject"]) -> str:
    """Generate the BuiltinObject.__pack_proto__ method implementation."""
    pack_method_parts: list[str] = []
    pack_assignments: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.name == "metatype":
            metatype = BUILTIN_OBJECT_TYPE_BY_CLASS[cls]
            pack_assignments.append(f"{prop.name}={metatype.value}")
            continue
        pack_code = _generate_pack_property(prop)
        if pack_code:
            if len(pack_code) == 1:
                pack_assignments.append(f"{prop.name}={pack_code[0].split(' = ', 1)[1]}")
            else:
                pack_method_parts.extend(pack_code)
                pack_assignments.append(f"{prop.name}=_packed_{prop.name}")
        else:
            pack_assignments.append(f"{prop.name}=object.{prop.name}")

    pack_method_parts.append(f"return {cls.__name__}Data(")
    for i, assignment in enumerate(pack_assignments):
        comma = "," if i < len(pack_assignments) - 1 else ""
        pack_method_parts.append(f"    {assignment}{comma}")
    pack_method_parts.append(")")

    return "\n".join(pack_method_parts)


def _generate_unpack_proto(cls: type["BuiltinObject"]) -> str:
    """Generate the BuiltinObject.__unpack_proto__ method implementation."""
    unpack_assignments: list[str] = []
    unpack_method_parts: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.name == "metatype":
            continue  # set implicitly
        unpack_code = _generate_unpack_property(prop)
        if unpack_code:
            if len(unpack_code) == 1:
                unpack_assignments.append(f"{prop.name}={unpack_code[0].split(' = ', 1)[1]}")
            else:
                unpack_method_parts.extend(unpack_code)
                unpack_assignments.append(f"{prop.name}=_unpacked_{prop.name}")
        else:
            unpack_assignments.append(f"{prop.name}=object_data.{prop.name}")
    if cls.__is_frozen__:
        unpack_assignments.append("_proto=object_data")

    unpack_method_parts.append("return cls(")
    for i, assignment in enumerate(unpack_assignments):
        comma = "," if i < len(unpack_assignments) - 1 else ""
        unpack_method_parts.append(f"    {assignment}{comma}")

    unpack_method_parts.append(")")

    return "\n".join(unpack_method_parts)


def _generate_pack_property(prop: "Property") -> list[str] | None:
    """Generate the packing code for a property value."""
    lines: list[str] = []
    obj_value = f"object.{prop.name}"

    if prop.cardinality == "scalar":
        scalar_lines = _generate_pack_scalar(prop, obj_value, f"_packed_{prop.name}")
        if scalar_lines:
            lines.extend(scalar_lines)
        else:
            return None
    elif prop.cardinality == "list":
        lines.extend(
            f"""\
_packed_{prop.name} = []
if {obj_value} is not None:
    for _item in {obj_value}:""".splitlines()
        )
        item_lines = _generate_pack_scalar(prop, "_item", "_packed_item")
        if item_lines:
            for line in item_lines:
                lines.append(f"        {line}")
            lines.append(f"        _packed_{prop.name}.append(_packed_item)")
        else:
            lines.append(f"        _packed_{prop.name}.append(_item)")
    elif prop.cardinality == "map":
        lines.extend(
            f"""\
_packed_{prop.name} = {{}}
if {obj_value} is not None:
    for _key, _value in {obj_value}.items():""".splitlines()
        )
        key_lines = _generate_pack_scalar(prop, "_key", "_packed_key")
        value_lines = _generate_pack_scalar(prop, "_value", "_packed_value")
        if key_lines and value_lines:
            for line in key_lines + value_lines:
                lines.append(f"        {line}")
            lines.append(f"        _packed_{prop.name}[_packed_key] = _packed_value")
        elif key_lines:
            for line in key_lines:
                lines.append(f"        {line}")
            lines.append(f"        _packed_{prop.name}[_packed_key] = _value")
        elif value_lines:
            for line in value_lines:
                lines.append(f"        {line}")
            lines.append(f"        _packed_{prop.name}[_key] = _packed_value")
        else:
            lines.append(f"        _packed_{prop.name}[_key] = _value")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_property(prop: "Property") -> list[str] | None:
    """Generate the unpacking code for a property value."""

    lines: list[str] = []
    data_value = f"object_data.{prop.name}"

    if prop.cardinality == "scalar":
        scalar_lines = _generate_unpack_scalar(prop, data_value, f"_unpacked_{prop.name}")
        if scalar_lines:
            lines.extend(scalar_lines)
        else:
            return None
    elif prop.cardinality == "list":
        lines.extend(
            f"""\
_unpacked_{prop.name} = []
for _item in {data_value}:""".splitlines()
        )
        item_lines = _generate_unpack_scalar(prop, "_item", "_unpacked_item")
        if item_lines:
            for line in item_lines:
                lines.append(f"    {line}")
            lines.append(f"    _unpacked_{prop.name}.append(_unpacked_item)")
        else:
            lines.append(f"    _unpacked_{prop.name}.append(_item)")
    elif prop.cardinality == "map":
        lines.extend(
            f"""\
_unpacked_{prop.name} = {{}}
for _key, _value in {data_value}.items():""".splitlines()
        )
        key_lines = _generate_unpack_scalar(prop, "_key", "_unpacked_key")
        value_lines = _generate_unpack_scalar(prop, "_value", "_unpacked_value")
        if key_lines and value_lines:
            for line in key_lines + value_lines:
                lines.append(f"    {line}")
            lines.append(f"    _unpacked_{prop.name}[_unpacked_key] = _unpacked_value")
        elif key_lines:
            for line in key_lines:
                lines.append(f"    {line}")
            lines.append(f"    _unpacked_{prop.name}[_unpacked_key] = _value")
        elif value_lines:
            for line in value_lines:
                lines.append(f"    {line}")
            lines.append(f"    _unpacked_{prop.name}[_key] = _unpacked_value")
        else:
            lines.append(f"    _unpacked_{prop.name}[_key] = _value")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_scalar(
    prop: "Property | TypeAnnotation", value_expr: str, result_var: str
) -> list[str] | None:
    """Generate the packing code for a scalar value."""
    lines: list[str] = []

    def _wrap_with_null_check(code: str) -> str:
        """Wrap code with null check if property is optional."""
        if prop.is_required:
            return code
        return f"{code} if {value_expr} is not None else None"

    if prop.scalar_type == "primitive":
        if prop.primitive_type == PrimitiveType.UUID:
            lines.append(f"{result_var} = {_wrap_with_null_check(f'str({value_expr})')}")
        elif prop.primitive_type == PrimitiveType.JSON:
            lines.append(f"{result_var} = {_wrap_with_null_check(f'json.dumps({value_expr})')}")
        elif prop.primitive_type == PrimitiveType.DATETIME:
            if prop.is_required:
                lines.extend(
                    f"""\
_ts = Timestamp()
_ts.FromDatetime({value_expr})
{result_var} = _ts""".splitlines()
                )
            else:
                lines.extend(
                    f"""\
if {value_expr} is not None:
    _ts = Timestamp()
    _ts.FromDatetime({value_expr})
    {result_var} = _ts
else:
    {result_var} = None""".splitlines()
                )
        elif prop.primitive_type == PrimitiveType.DURATION:
            if prop.is_required:
                lines.extend(
                    f"""\
_dur = Duration()
_dur.FromTimedelta({value_expr})
{result_var} = _dur""".splitlines()
                )
            else:
                lines.extend(
                    f"""\
if {value_expr} is not None:
    _dur = Duration()
    _dur.FromTimedelta({value_expr})
    {result_var} = _dur
else:
    {result_var} = None""".splitlines()
                )
        else:
            return None
    elif prop.scalar_type == "enum":
        lines.append(f"{result_var} = {_wrap_with_null_check(f'{value_expr}.value')}")
    elif prop.scalar_type == "struct":
        assert prop.struct_type is not None
        struct_cls_name = prop.struct_type.bench_name
        lines.append(
            f"{result_var} = {_wrap_with_null_check(f'{struct_cls_name}.__pack_proto__({value_expr})')}"
        )
    elif prop.scalar_type == "node":
        lines.append(
            f"{result_var} = {_wrap_with_null_check(f'NodeReference.__pack_proto__({value_expr})')}"
        )
    else:
        assert_never(prop.scalar_type)

    return lines


def _generate_unpack_scalar(
    prop: "Property | TypeAnnotation", value_expr: str, result_var: str
) -> list[str] | None:
    """Generate the unpacking code for a scalar value."""
    lines: list[str] = []

    def _wrap_with_null_check(code: str) -> str:
        """Wrap code with null check if property is optional."""
        if prop.is_required:
            return code
        return f"{code} if {value_expr} is not None else None"

    if prop.scalar_type == "primitive":
        if prop.primitive_type == PrimitiveType.UUID:
            lines.append(f"{result_var} = {_wrap_with_null_check(f'UUID({value_expr})')}")
        elif prop.primitive_type == PrimitiveType.JSON:
            lines.append(f"{result_var} = {_wrap_with_null_check(f'json.loads({value_expr})')}")
        elif prop.primitive_type == PrimitiveType.DATETIME:
            lines.append(
                f"{result_var} = {_wrap_with_null_check(f'{value_expr}.ToDatetime(tzinfo=pytz.utc)')}"
            )
        elif prop.primitive_type == PrimitiveType.DURATION:
            lines.append(f"{result_var} = {_wrap_with_null_check(f'{value_expr}.ToTimedelta()')}")
        else:
            return None
    elif prop.scalar_type == "enum":
        assert prop.enum_type is not None
        enum_type_name = prop.enum_type.bench_name
        lines.append(f"{result_var} = {_wrap_with_null_check(f'{enum_type_name}({value_expr})')}")
    elif prop.scalar_type == "struct":
        assert prop.struct_type is not None
        struct_cls_name = prop.struct_type.bench_name
        lines.append(
            f"{result_var} = {_wrap_with_null_check(f'{struct_cls_name}.__unpack_proto__({value_expr})')}"
        )
    elif prop.scalar_type == "node":
        lines.append(
            f"{result_var} = {_wrap_with_null_check(f'NodeReference.__unpack_proto__({value_expr})')}"
        )
    else:
        assert_never(prop.scalar_type)

    return lines


def wrap_some_node(node: AnyNodeData) -> pb2.SomeNodeData:
    """Wraps a concrete node type into a generic node message."""
    wrapper = pb2.SomeNodeData()
    field_name = to_casing(cast(str, NodeType(node.metatype).name), Casing.SNAKE)
    getattr(wrapper, field_name).CopyFrom(node)
    return wrapper


def wrap_some_node_maybe(node: AnyNodeData | None) -> pb2.SomeNodeData | None:
    if node is None:
        return None
    else:
        return wrap_some_node(node)


def unwrap_some_node(node: pb2.SomeNodeData) -> AnyNodeData:
    """Unwraps a generic node type into a concrete node type."""
    node_key = node.WhichOneof("node")
    assert node_key is not None, f"node not set in {node!r}"
    wrapped_node = getattr(node, node_key)
    assert wrapped_node is not None, f"node not set in {node!r}"
    return wrapped_node


def pack_rpc_headers(metadata: RpcMetadata) -> dict[str, str]:
    # flat encoding with prefix, messages as base64 :RpcMetadataEncoding
    packed = {
        "2": str(int(metadata.client_type)) if metadata.client_type is not None else None,
        "3": metadata.client_id or None,
        "4": metadata.client_nonce or None,
        "5": metadata.client_access_token or None,
    }
    packed_badges = [
        {
            "2": badge.id,
            "3": badge.key or None,
            "4": badge.password or None,
        }
        for badge in metadata.badges
    ]
    if packed_badges:
        packed["6"] = b64encode(json.dumps(packed_badges).encode("utf-8")).decode("utf-8")
    return {"x-bench-" + k: v for k, v in packed.items() if v is not None}


def unpack_rpc_headers(headers: Mapping) -> RpcMetadata:
    # flat encoding with prefixy, messages as base64 :RpcMetadataEncoding
    metadata = RpcMetadata()
    if headers.get("x-bench-2"):
        metadata.client_type = cast(pb2.ClientType, int(headers["x-bench-2"]))
    if headers.get("x-bench-3"):
        metadata.client_id = headers.get("x-bench-3")  # type: ignore
    if headers.get("x-bench-4"):
        metadata.client_nonce = headers.get("x-bench-4")  # type: ignore
    if headers.get("x-bench-5"):
        metadata.client_access_token = headers.get("x-bench-5")  # type: ignore
    if headers.get("6"):
        unpacked_badges = json.loads(b64decode(headers.get("x-bench-6")).decode("utf-8"))  # type: ignore
        for unpacked_badge in unpacked_badges:
            metadata_badge = metadata.badges.add()
            metadata_badge.id = unpacked_badge.get("2")
            metadata_badge.key = unpacked_badge.get("3")
            metadata_badge.password = unpacked_badge.get("4")
    return metadata
