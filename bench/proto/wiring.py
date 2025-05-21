import json
from base64 import b64decode, b64encode
from itertools import chain
from typing import TYPE_CHECKING, Any, Mapping, Union, cast

import pytz
import structlog
from fastuuid import UUID
from google.protobuf.duration_pb2 import Duration
from google.protobuf.timestamp_pb2 import Timestamp
from opentelemetry import trace

from bench import pb2
from bench.language.core import (
    BuiltinEnumOrUnion,
    BuiltinObject,
    EditType,
    Graph,
    GraphData,
    Node,
    NodeReference,
    NodeType,
    ObjectType,
    PrimitiveType,
    Property,
    Session,
    Supergraph,
    pack_proto_json,
    unpack_proto_json,
)
from bench.language.registry import BUILTIN_OBJECT_CLASS_BY_TYPE
from bench.pb2 import AnyNodeData, AnyStructData, EditData, NodeReferenceData, RpcMetadata
from bench.utils.string import Casing, to_casing

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


PROTO_CLASS_BY_TYPE: dict[ObjectType, type[Union[AnyNodeData, AnyStructData]]] = {
    object_type: getattr(pb2, object_type.bench_name + "Data")
    for object_type in ObjectType  # type: ignore
    if hasattr(pb2, object_type.bench_name + "Data")  # may just be creating a new class
}
OBJECT_TYPE_BY_PROTO_CLASS: dict[type[Union[AnyNodeData, AnyStructData]], ObjectType] = {
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
    node_type = unpack_enum(NodeType, node.metatype)
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
    edit_type = unpack_enum(EditType, edit.type)
    node_str = describe_node_ptr(edit.node_ptr)
    return f"{edit_type.name}[id={edit.id}, edited_at={edit.edited_at.ToJsonString()}, node={node_str}]"


def copy_struct[T: AnyStructData | AnyNodeData](data: T) -> T:
    """Deepcopy a struct data object."""
    copy = type(data)(metatype=data.metatype)
    copy.CopyFrom(data)  # type: ignore
    return copy


def pack_enum[EnumT: BuiltinEnumOrUnion](enum_cls: type[EnumT], value: EnumT) -> Any:
    return value


def unpack_enum[EnumT: BuiltinEnumOrUnion](enum_cls: type[EnumT], value: Any) -> EnumT:
    return enum_cls(value)


def pack_builtin_object_prop_scalar(obj: BuiltinObject, prop: Property, value: Any) -> Any:
    if value is None:
        return None
    elif prop.is_struct:
        return pack_builtin_object(value)
    elif prop.is_node_data:
        return wrap_some_node(value)
    elif prop.is_enum:
        return pack_enum(prop.py_type, value)
    elif prop.node_kind is not None and not prop.node_kind.is_struct_tree:
        value_id = str(value.id)
        return NodeReferenceData(
            metatype=pb2.ObjectType.OBJECT_TYPE_NODE_REFERENCE,
            node_type=pack_enum(NodeType, value.type),
            id=value_id,
            ck=str(value.ck) if value.ck is not None else value_id,
        )
    elif prop.primitive_type == PrimitiveType.UUID:
        return str(value)  # uuids are wired as strings
    elif prop.primitive_type == PrimitiveType.JSON:
        return pack_proto_json(value)
    elif prop.primitive_type == PrimitiveType.DATETIME:
        ts = Timestamp()
        ts.FromDatetime(value)
        return ts
    elif prop.primitive_type == PrimitiveType.DURATION:
        dur = Duration()
        dur.FromTimedelta(value)
        return dur
    else:
        return value


def unpack_builtin_object_prop_scalar(prop: Property, value: Any, *, supergraph: Supergraph) -> Any:
    try:
        if value is None:
            return None
        elif prop.is_struct:
            return unpack_builtin_object(value, supergraph=supergraph)
        elif prop.is_node_data:
            return unwrap_some_node(value)
        elif prop.is_enum:
            return unpack_enum(prop.py_type, value)
        elif prop.node_kind is not None and not prop.node_kind.is_struct_tree:
            value_id = UUID(value.id)
            return NodeReference(
                node_type=unpack_enum(NodeType, value.node_type),
                id=value_id,
                ck=UUID(value.ck) if value.ck else value_id,
            )
        elif prop.primitive_type == PrimitiveType.UUID:
            return UUID(value)  # uuids are wired as strings
        elif prop.primitive_type == PrimitiveType.JSON:
            return unpack_proto_json(value)
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return Timestamp.ToDatetime(value, tzinfo=pytz.utc)
        elif prop.primitive_type == PrimitiveType.DURATION:
            return Duration.ToTimedelta(value)
        else:
            return value
    except (AssertionError, AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(
            f"could not unpack value {type(value).__name__}: {value!r} for {prop!r}"
        ) from e


def pack_builtin_object[T: AnyStructData | AnyNodeData](
    obj: BuiltinObject, expect: type[T] | None = None, into: T | None = None
) -> T:
    """Pack a struct and any contained structs."""
    raise NotImplementedError


# nocheckin: generate proto pack/unpack for BuiltinObjects


def unpack_builtin_object[T: BuiltinObject](
    obj_data: AnyStructData | AnyNodeData,
    *,
    expect: type[T] | None = None,
    supergraph: Supergraph | None,
    session: Session | None = None,
    # for nodes
    graph: Graph | None = None,
    # NOTE: by default new Nodes add themselves to their graph, but during
    #  unpacking we almost never want this (because we manage unpacking manually).
    validate: bool = False,
    skip_add_self: bool = True,
) -> T:
    """Unpack a builtin object and any contained structs without validating."""
    raise NotImplementedError


def unpack_builtin_object_validate[T: BuiltinObject](
    obj_data: AnyStructData | AnyNodeData,
    *,
    supergraph: Supergraph | None,
    graph: Graph | None = None,
    expect: type[T] | None = None,
    session: Session | None = None,
) -> T:
    """Unpack a builtin object and validate it."""
    obj = unpack_builtin_object(
        obj_data, supergraph=supergraph, graph=graph, expect=expect, session=session
    )
    return obj


def unpack_builtin_object_validate_maybe[T: BuiltinObject](
    obj_data: AnyStructData | AnyNodeData | None,
    *,
    supergraph: Supergraph | None,
    expect: type[T] | None = None,
    session: Session | None = None,
) -> T | None:
    if obj_data is None or obj_data.metatype is None or obj_data.metatype == 0:
        return None
    else:
        return unpack_builtin_object_validate(
            obj_data,
            supergraph=supergraph,
            expect=expect,
            session=session,
        )


@tracer.start_as_current_span("wiring.unpack_graph")
def unpack_graph(
    data_graph: GraphData,
    supergraph: Supergraph,
    parent: Node | None = None,
    session: Session | None = None,
    exclude: set[NodeType] | tuple[NodeType, ...] | None = (),
) -> Graph:
    """Unpacks the node data(s) into a node graph."""
    trace.get_current_span().set_attribute("nodes", len(data_graph))

    exclude = exclude or ()
    parent_id = parent.id if parent is not None else None
    roots_data = data_graph.find_roots()
    graph = Graph(scope=data_graph.scope, node_types=data_graph.node_types, supergraph=supergraph)
    supergraph.add_graph(graph)

    for root_data in roots_data:
        # unpack all nodes top down (breadth first)
        for node_data in chain(
            (root_data,), data_graph.iter_descendants(root_data, recursive=True)
        ):
            if node_data.metatype in exclude:
                continue
            node = unpack_builtin_object(
                node_data,
                graph=graph,
                supergraph=supergraph,
                expect=Node,
                session=session,
            )
            graph.add(node)

            # keep parent instance if it was passed (update it in place)
            if node.id == parent_id:
                for prop in (cast(Node, parent)).__properties__.values():
                    if prop.is_wired and not prop.is_tree_reference:
                        setattr(parent, prop.name, getattr(node, prop.name))
                node = cast(Node, parent)

    return graph


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
