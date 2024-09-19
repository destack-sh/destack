import json
from base64 import b64decode, b64encode
from itertools import chain
from typing import Any, Collection, Mapping, Union, cast
from uuid import UUID

import structlog
from google.protobuf.duration_pb2 import Duration
from google.protobuf.json_format import MessageToDict
from google.protobuf.message import Message as ProtoMessage
from google.protobuf.struct_pb2 import Struct as ProtoStruct
from google.protobuf.timestamp_pb2 import Timestamp
from opentelemetry import trace

from bench.language.connection import Connection
from bench.language.const import NodeType, ObjectType, PrimitiveType
from bench.language.graph import NULL_SUPERGRAPH, NodeDataGraph, NodeSuperGraph
from bench.language.node import BuiltinObject, Node, NodeGraph, NodeReference
from bench.language.property import Property
from bench.language.session import Session
from bench.language.setup import OBJECT_CLASS_BY_TYPE
from bench.language.validation import on_invalid_raise
from bench.proto import wire
from bench.proto.wire import AnyNodeData, AnyStructData, NodeReferenceData, RpcMetadata
from bench.proto.wire.lang_pb2 import FileReferenceData, SecretReferenceData
from bench.utils.func import IdEnum, IdEnumOrUnion, to_uuid
from bench.utils.string import Casing, to_casing

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


PROTO_CLASS_BY_TYPE: dict[ObjectType, type[Union[AnyNodeData, AnyStructData]]] = {
    object_type: getattr(wire, object_type.bench_name + "Data")
    for object_type in ObjectType  # type: ignore
    if hasattr(wire, object_type.bench_name + "Data")  # may just be creating a new class
}
OBJECT_TYPE_BY_PROTO_CLASS: dict[type[Union[AnyNodeData, AnyStructData]], ObjectType] = {
    cls: object_type for object_type, cls in PROTO_CLASS_BY_TYPE.items()
}
BENCH_CLASS_BY_PROTO_CLASS: dict[type[Union[AnyNodeData, AnyStructData]], type[BuiltinObject]] = {
    cls: OBJECT_CLASS_BY_TYPE[object_type]
    for cls, object_type in OBJECT_TYPE_BY_PROTO_CLASS.items()
}


def copy_struct[T: AnyStructData | AnyNodeData](data: T) -> T:
    """Deepcopy a struct data object."""
    copy = type(data)(metatype=data.metatype)
    copy.CopyFrom(data)  # type: ignore
    return copy


def pack_proto_json(value: dict[str, Any]) -> ProtoStruct:
    struct = ProtoStruct()
    struct.update(value)
    return struct


def unpack_proto_json(value: ProtoStruct) -> dict[str, Any]:
    json = MessageToDict(value)
    return json


def pack_enum[EnumT: IdEnumOrUnion](enum_cls: type[EnumT], value: EnumT) -> Any:
    assert isinstance(enum_cls, type), f"{enum_cls} is not a type"
    assert issubclass(enum_cls, IdEnum), f"{enum_cls} is not an IdEnum"
    assert isinstance(value, int), f"{value} is not an int"
    return value


def unpack_enum[EnumT: IdEnumOrUnion](enum_cls: type[EnumT], value: Any) -> EnumT:
    assert isinstance(enum_cls, type), f"{enum_cls} is not a type"
    assert issubclass(enum_cls, IdEnum), f"{enum_cls} is not an IdEnum"
    return enum_cls(value)


#
# NOTE :Performance: we could generate static pack/unpack functions for each object type
#  (all this non-linear dynamic dispatch is not very fast)
#


def pack_object_prop_scalar(prop: Property, value: Any, into: Any | None = None) -> Any:
    if value is None:
        return None
    elif prop.is_struct:
        return pack_object(value)
    elif prop.reference_is_node_data:
        return wrap_some_node(value)
    elif prop.is_enum:
        return pack_enum(prop.py_type_stripped, value)
    elif prop.reference_kind is not None and not prop.reference_kind.is_struct_tree:
        value_id = str(value.id)
        return NodeReferenceData(
            metatype=wire.ObjectType.OBJECT_TYPE_NODE_REFERENCE,
            type=pack_enum(NodeType, value.type),
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
    elif prop.primitive_type == PrimitiveType.INTERVAL:
        dur = Duration()
        dur.FromTimedelta(value)
        return dur
    else:
        return value


def pack_object_prop(prop: Property, value: Any) -> Any:
    if value is None:
        return None
    elif not prop.is_list:
        return pack_object_prop_scalar(prop, value)
    else:
        return [pack_object_prop_scalar(prop, v) for v in value]


def unpack_object_prop_scalar(prop: Property, value: Any, *, supergraph: NodeSuperGraph) -> Any:
    try:
        if value is None:
            return None
        elif prop.is_struct:
            return unpack_object(value, supergraph=supergraph)
        elif prop.reference_is_node_data:
            return unwrap_some_node(value)
        elif prop.is_enum:
            return unpack_enum(prop.py_type_stripped, value)
        elif prop.reference_kind is not None and not prop.reference_kind.is_struct_tree:
            value_id = UUID(value.id)
            return NodeReference(
                type=unpack_enum(NodeType, value.type),
                id=value_id,
                ck=UUID(value.ck) if value.ck else value_id,
            )
        elif prop.primitive_type == PrimitiveType.UUID:
            return UUID(value)  # uuids are wired as strings
        elif prop.primitive_type == PrimitiveType.JSON:
            return unpack_proto_json(value)
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return Timestamp.ToDatetime(value)
        elif prop.primitive_type == PrimitiveType.INTERVAL:
            return Duration.ToTimedelta(value)
        else:
            return value
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack value: {value!r} for {prop!r}") from e


def unpack_object_prop(prop: Property, value: Any, *, supergraph: NodeSuperGraph) -> Any:
    if value is None:
        return None
    elif not prop.is_list:
        return unpack_object_prop_scalar(prop, value, supergraph=supergraph)
    else:
        return [unpack_object_prop_scalar(prop, v, supergraph=supergraph) for v in value]


def get_rich_reference_prop_name(prop: Property, value: Any) -> str:
    """Gets the name of the rich reference property for the given property. :RichReferences"""
    if isinstance(value, FileReferenceData):
        return f"{prop.name}_file"
    elif isinstance(value, SecretReferenceData):
        return f"{prop.name}_secret"
    elif isinstance(value, NodeReferenceData):
        return f"{prop.name}_node"
    else:
        raise ValueError(f"unexpected rich reference value: {value!r}")


def pack_and_set_object_prop(obj_data: AnyStructData | AnyNodeData, prop: Property, value: Any):
    """Pack and set the given property on the given data object."""
    if not prop.is_list:  # scalar
        packed_value = pack_object_prop_scalar(prop, value)
        if isinstance(packed_value, ProtoMessage):  # message field
            if prop.reference_is_rich:  # one of field, set appropriate one
                prop_name = get_rich_reference_prop_name(prop, packed_value)
                getattr(obj_data, prop_name).CopyFrom(packed_value)
            else:  # regular message field
                getattr(obj_data, prop.name).CopyFrom(packed_value)
        elif prop.is_struct:  # empty message field
            assert value is None, f"unexpected non-proto struct value for {prop!r}: {value!r}"
            obj_data.ClearField(prop.name)
        elif value is None:
            obj_data.ClearField(prop.name)
        else:  # primitive field
            setattr(obj_data, prop.name, packed_value)
    else:  # list
        assert prop.is_struct or len(value) == 0, f"cannot set non-struct {prop!r}: {value!r}"
        packed_value = getattr(obj_data, prop.name)
        for item in value:
            packed_item = packed_value.add()
            _ = pack_object_prop_scalar(prop, item, into=packed_item)


def set_object_prop(obj_data: AnyStructData | AnyNodeData, prop: Property, value: Any):
    """Set the packed property on the given data object."""
    if not prop.is_list:  # scalar
        if isinstance(value, ProtoMessage):  # message field
            if prop.reference_is_rich:  # one of field, set appropriate one
                prop_name = get_rich_reference_prop_name(prop, value)
                getattr(obj_data, prop_name).CopyFrom(value)
            else:  # regular message field
                getattr(obj_data, prop.name).CopyFrom(value)
        elif prop.is_struct:  # empty message field
            assert value is None, f"unexpected non-proto struct value for {prop!r}: {value!r}"
            obj_data.ClearField(prop.name)
        elif value is None:
            obj_data.ClearField(prop.name)
        else:  # primitive field
            setattr(obj_data, prop.name, value)
    else:  # list
        assert prop.is_struct or len(value) == 0, f"cannot set non-struct {prop!r}: {value!r}"
        getattr(obj_data, prop.name).clear()
        getattr(obj_data, prop.name).extend(value)


def pack_object[T: AnyStructData | AnyNodeData](
    obj: BuiltinObject, expect: type[T] | None = None, into: T | None = None
) -> T:
    """Pack a struct and any contained structs."""
    data_cls = PROTO_CLASS_BY_TYPE[obj.metatype]
    metatype = pack_enum(ObjectType, obj.metatype)  # type: ignore
    if expect is not None and not issubclass(data_cls, expect):
        raise RuntimeError(f"expected {expect.__name__} but got {data_cls}")
    obj_data = into if into is not None else data_cls(metatype=metatype)  # type: ignore
    try:
        for prop in obj.__wired_properties__.values():
            value = getattr(obj, prop.name)
            if value is None:
                continue
            pack_and_set_object_prop(obj_data, prop, value)
        return cast(T, obj_data)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not pack {obj.metatype.name}: {obj!r}") from e


def pack_object_maybe[T: AnyStructData | AnyNodeData](
    obj: BuiltinObject | None, expect: type[T] | None = None
) -> T | None:
    if obj is None:
        return None
    else:
        return pack_object(obj, expect)


def unpack_object[T: BuiltinObject](
    obj_data: AnyStructData | AnyNodeData,
    *,
    expect: type[T] | None = None,
    supergraph: NodeSuperGraph | None,
    session: Session | None = None,
    connection: Connection | None = None,
    # for nodes
    graph: NodeGraph | None = None,
    parent: Node | None = None,
    # NOTE: by default new Nodes add themselves to their graph, but during
    #  unpacking we almost never want this (because we manage unpacking manually).
    skip_add_self: bool = True,
) -> T:
    """Unpack a builtin object and any contained structs without validating."""
    supergraph = supergraph or NULL_SUPERGRAPH
    assert obj_data.metatype is not None, f"missing metatype for {obj_data!r}"
    assert obj_data.metatype != 0, f"missing metatype for {obj_data!r}"
    object_cls = OBJECT_CLASS_BY_TYPE[obj_data.metatype]  # type: ignore
    if expect and not issubclass(object_cls, expect):
        raise RuntimeError(f"expected {expect} but got {object_cls}")
    object_kwargs = {}
    try:
        for prop in object_cls.__wired_properties__.values():
            if not prop.is_runtime or prop.is_computed:
                continue
            if prop.is_optional_scalar and not obj_data.HasField(prop.name):
                continue
            value = getattr(obj_data, prop.name)
            object_kwargs[prop.name] = unpack_object_prop(prop, value, supergraph=supergraph)
        object_kwargs["_supergraph"] = supergraph
        if session is not None:
            object_kwargs["_session"] = session
        if issubclass(object_cls, Node):
            if parent is not None:
                object_kwargs["_parent"] = parent
            if graph is not None:
                object_kwargs["_graph"] = graph
            if connection is not None:
                object_kwargs["_connection"] = connection
            object_kwargs["_skip_add_self"] = skip_add_self
        obj = object_cls(**object_kwargs)
        if session is not None and isinstance(obj, Node):
            obj._track_self(session)
        return cast(T, obj)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack {type(obj_data).__name__}: {obj_data!r}") from e


def unpack_object_validate[T: BuiltinObject](
    obj_data: AnyStructData | AnyNodeData,
    *,
    supergraph: NodeSuperGraph | None,
    graph: NodeGraph | None = None,
    parent: Node | None = None,
    expect: type[T] | None = None,
    session: Session | None = None,
) -> T:
    """Unpack a builtin object and validate it."""
    obj = unpack_object(
        obj_data, supergraph=supergraph, graph=graph, parent=parent, expect=expect, session=session
    )
    obj._validate_rec(invalid=on_invalid_raise)
    return obj


def unpack_object_validate_maybe[T: BuiltinObject](
    obj_data: AnyStructData | AnyNodeData | None,
    *,
    supergraph: NodeSuperGraph | None,
    expect: type[T] | None = None,
    session: Session | None = None,
) -> T | None:
    if obj_data is None:
        return None
    else:
        return unpack_object_validate(
            obj_data,
            supergraph=supergraph,
            expect=expect,
            session=session,
        )


@tracer.start_as_current_span("wiring.unpack_node_graph")
def unpack_node_graph(
    data_graph: NodeDataGraph,
    supergraph: NodeSuperGraph,
    parent: Node | None = None,
    session: Session | None = None,
    exclude: set[NodeType] | tuple[NodeType, ...] | None = (),
    connection: Connection | None = None,
) -> NodeGraph:
    """Unpacks the node data(s) into a node graph."""
    trace.get_current_span().set_attribute("nodes", len(data_graph))

    exclude = exclude or ()
    parent_id = parent.id if parent is not None else None
    roots_data = data_graph.find_roots()
    graph = NodeGraph(
        scope=data_graph.scope, node_types=data_graph.node_types, supergraph=supergraph
    )
    supergraph.add_graph(graph)

    for root_data in roots_data:
        # unpack all nodes top down (breadth first)
        for node_data in chain(
            (root_data,), data_graph.iter_descendants(root_data, recursive=True)
        ):
            if node_data.metatype in exclude:
                continue
            node_parent_id: UUID | None = (
                to_uuid(node_data.parent_ptr.id) if node_data.parent_ptr is not None else None
            )
            if node_parent_id is None or node_parent_id == parent_id:
                node_parent = parent
            else:
                node_parent = graph.get(node_parent_id)
                # NOTE :Architecture: enable loading nodes without ancestors :LoadOrphanNode?
                #  (this errors here, but sometimes we just want a node without ancestors)
                # if node_parent is None:
                #     raise ValueError(f"parent {node_parent_id} not found in {unpacked_graph!r}")
            node = unpack_object(
                node_data,
                graph=graph,
                supergraph=supergraph,
                connection=connection,
                parent=node_parent,
                expect=Node,
            )
            graph.add(node)

            # keep parent instance if it was passed (update it in place)
            if node.id == parent_id:
                for prop in (cast(Node, parent)).__properties__.values():
                    if not prop.is_ephemeral and not prop.is_tree_reference:
                        setattr(parent, prop.name, getattr(node, prop.name))
                node = cast(Node, parent)

    # track in session
    if session is not None:
        for node in graph._nodes_by_id.values():
            node._track_self(session)

    return graph


def unpack_node_roots(
    data_graph: NodeDataGraph,
    supergraph: NodeSuperGraph,
    parent: Node | None = None,
    session: Session | None = None,
    exclude: set[NodeType] | None = None,
    roots: Collection[NodeReferenceData] | None = None,
    connection: Connection | None = None,
) -> tuple[tuple[Node, ...], NodeGraph]:
    """Unpack nodes and their descendants. Returns the actual roots (or passed ones)."""

    node_graph = unpack_node_graph(
        data_graph=data_graph,
        supergraph=supergraph,
        parent=parent,
        session=session,
        exclude=exclude,
        connection=connection,
    )

    if roots:
        # recover roots if specified (may not be actual roots)
        recovered_roots = []
        for root in roots:
            unpacked_root = node_graph.get(UUID(root.id))
            if unpacked_root is not None:
                recovered_roots.append(unpacked_root)
        return tuple(recovered_roots), node_graph
    else:
        return node_graph.find_roots(), node_graph


def wrap_some_node(node: AnyNodeData) -> wire.SomeNodeData:
    """Wraps a concrete node type into a generic node message."""
    wrapper = wire.SomeNodeData()
    field_name = to_casing(cast(str, NodeType(node.metatype).name), Casing.SNAKE)
    getattr(wrapper, field_name).CopyFrom(node)
    return wrapper


def wrap_some_node_maybe(node: AnyNodeData | None) -> wire.SomeNodeData | None:
    if node is None:
        return None
    else:
        return wrap_some_node(node)


def unwrap_some_node(node: wire.SomeNodeData) -> AnyNodeData:
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
        "3": metadata.client_id,
        "4": metadata.client_nonce,
        "5": metadata.client_access_token,
    }
    packed_badges = [
        {
            "2": badge.id,
            "3": badge.key,
            "4": badge.password,
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
        metadata.client_type = wire.ClientType(int(headers["x-bench-2"]))
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
