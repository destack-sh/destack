import enum
from collections import OrderedDict
from copy import copy
from itertools import chain
from typing import Any, Collection, TypeVar, Union, cast
from uuid import UUID

import betterproto
import structlog
from betterproto.lib.google.protobuf import Struct as BetterprotoStruct

from bench.language.const import BenchType, NodeType
from bench.language.graph import NodeDataGraph
from bench.language.node import (
    BENCH_CLASS_BY_TYPE,
    METATYPE_PROPERTY,
    NODE_CLASS_BY_TYPE,
    Node,
    NodeGraph,
    NodeStatus,
    Property,
    ScopeNode,
    Struct,
    on_notice_raise,
)
from bench.language.notice import NoticeHandler
from bench.language.session import Session
from bench.proto import wire
from bench.proto.wire import AnyNodeData, AnyStructData, NodeReferenceData
from bench.sql.core import PrimitiveType
from bench.utils.casing import Casing, to_casing
from bench.utils.func import IdEnum, to_uuid

logger = structlog.get_logger(__name__)

PROTO_CLASS_BY_TYPE: dict[BenchType, type[Union[AnyNodeData, AnyStructData]]] = {
    _type: getattr(wire, _type.bench_name + "Data")
    for _type in BenchType
    if hasattr(wire, _type.bench_name + "Data")  # may just be creating it
}
BENCH_TYPE_BY_PROTO_CLASS: dict[type[Union[AnyNodeData, AnyStructData]], BenchType] = {
    cls: bench_type for bench_type, cls in PROTO_CLASS_BY_TYPE.items()
}
BENCH_CLASS_BY_PROTO_CLASS: dict[
    type[Union[AnyNodeData, AnyStructData]], type[Union[Node, Struct]]
] = {cls: BENCH_CLASS_BY_TYPE[bench_type] for cls, bench_type in BENCH_TYPE_BY_PROTO_CLASS.items()}

NodeT = TypeVar("NodeT", bound=Node)
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
StructT = TypeVar("StructT", bound=Struct)
StructDataT = TypeVar("StructDataT", bound=AnyStructData)


def copy_struct_data(data: StructDataT) -> StructDataT:
    """Deepcopy a struct data object."""
    data_cls = PROTO_CLASS_BY_TYPE[data.metatype.name]
    bench_cls = BENCH_CLASS_BY_TYPE[data.metatype.name]
    data_kwargs = {}
    try:
        for prop in bench_cls.__wired_properties__.values():
            if prop.is_computed and prop.id != METATYPE_PROPERTY.id:
                continue
            value = getattr(data, prop.name)
            if value is None or value == "" and not prop.is_required:
                data_kwargs[prop.name] = None
            elif prop.is_array:
                if prop.is_struct:
                    data_kwargs[prop.name] = [copy_struct_data(v) for v in value]
                else:
                    data_kwargs[prop.name] = list(value)
            elif prop.is_struct:
                data_kwargs[prop.name] = copy_struct_data(value)
            elif prop.primitive_type == PrimitiveType.JSON:
                data_kwargs[prop.name] = copy(value)
            else:
                data_kwargs[prop.name] = value
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not copy {data.metatype.name}: {data!r}") from e
    return data_cls(**data_kwargs)


def pack_json_value(value: dict) -> BetterprotoStruct:
    return BetterprotoStruct.from_dict(value)


def unpack_json_value(value: BetterprotoStruct) -> dict:
    return value.to_dict()


def pack_enum(enum_cls: type[enum.Enum], value: Any) -> Any:
    assert issubclass(enum_cls, IdEnum), f"{enum_cls} is not an IdEnum"
    assert isinstance(value, int), f"{value} is not an int"
    return value


def unpack_enum(enum_cls: type[enum.Enum], value: Any) -> Any:
    assert issubclass(enum_cls, IdEnum), f"{enum_cls} is not an IdEnum"
    return enum_cls(value)


def _pack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_array and not ignore_array:
        return [_pack_struct_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        return pack_struct(value)
    elif prop.is_enum:
        return pack_enum(prop.py_type_stripped, value)
    elif prop.reference_kind is not None:
        return NodeReferenceData(
            metatype=wire.BenchType.NODE_REFERENCE,
            type=pack_enum(BenchType, value.type),
            id=str(value.id),
            ck=str(value.ck) if value.ck is not None else None,
        )
    elif prop.primitive_type == PrimitiveType.UUID:
        return str(value)  # uuids are wired as strings
    elif prop.primitive_type == PrimitiveType.JSON:
        return BetterprotoStruct.from_dict(value)
    else:
        return value


def _unpack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    from bench.language.expression import NodeReference

    try:
        if value is None:
            return None
        elif prop.is_array and not ignore_array:
            return [_unpack_struct_prop(prop, v, ignore_array=True) for v in value]
        elif prop.is_struct:
            return unpack_struct(value)
        elif prop.is_enum:
            return unpack_enum(prop.py_type_stripped, value)
        elif prop.reference_kind is not None:
            return NodeReference(
                type=unpack_enum(NodeType, value.type),
                id=to_uuid(value.id),
                ck=to_uuid(value.ck) if value.ck else None,
            )
        elif prop.primitive_type == PrimitiveType.UUID:
            return to_uuid(value)  # uuids are wired as strings
        elif prop.primitive_type == PrimitiveType.JSON:
            return value.to_dict()
        else:
            return value
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack value: {value!r} for {prop!r}") from e


def pack_struct(struct: StructT) -> StructDataT:
    """Pack a struct and any contained structs."""
    data_cls = PROTO_CLASS_BY_TYPE[struct.metatype]
    metatype = pack_enum(BenchType, struct.metatype)
    data = data_cls(metatype=metatype)
    try:
        for prop in struct.__wired_properties__.values():
            value = getattr(struct, prop.name)
            value = _pack_struct_prop(prop, value, ignore_array=False)
            setattr(data, prop.name, value)
        return data
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not pack {struct.metatype.name}: {struct!r}") from e


def pack_struct_maybe(struct: StructT | None) -> StructDataT | None:
    if struct is None:
        return None
    return pack_struct(struct)


def unpack_struct(struct_data: StructDataT) -> StructT:
    """Unpack a struct and any contained structs."""
    struct_cls = BENCH_CLASS_BY_TYPE[BenchType(struct_data.metatype)]
    struct_kwargs = {}
    try:
        for prop in struct_cls.__wired_properties__.values():
            if prop.is_computed:
                continue
            value = getattr(struct_data, prop.name)
            struct_kwargs[prop.name] = _unpack_struct_prop(prop, value, ignore_array=False)
        return struct_cls(**struct_kwargs, _status=NodeStatus.SOURCE)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack {struct_data.metatype.name}: {struct_data!r}") from e


def unpack_struct_maybe(struct_data: StructDataT | None) -> StructT | None:
    if struct_data is None:
        return None
    return unpack_struct(struct_data)


def unpack_struct_interp(
    struct_data: StructDataT,
    scope: ScopeNode | None = None,
    on_notice: NoticeHandler = on_notice_raise,
) -> StructT:
    struct = unpack_struct(struct_data)
    struct._interp_rec(scope=scope, on_notice=on_notice)
    return struct


def unpack_struct_interp_maybe(
    struct_data: StructDataT | None,
    scope: ScopeNode | None = None,
    on_notice: NoticeHandler = on_notice_raise,
) -> StructT | None:
    if struct_data is None:
        return None
    return unpack_struct_interp(struct_data, scope=scope, on_notice=on_notice)


def pack_node(node: NodeT) -> NodeDataT:
    return cast(AnyNodeData, pack_struct(node))


def pack_node_maybe(node: NodeT | None) -> NodeDataT | None:
    if node is None:
        return None
    return pack_node(node)


def unpack_node(
    node_data: NodeDataT, parent: Node | None = None, session: Session | None = None
) -> NodeT:
    node_cls = NODE_CLASS_BY_TYPE[NodeType(node_data.metatype)]
    node_kwargs = {}
    try:
        for prop in node_cls.__wired_properties__.values():
            if not prop.is_runtime or prop.is_computed:
                continue
            value = getattr(node_data, prop.name)
            node_kwargs[prop.name] = _unpack_struct_prop(prop, value, ignore_array=False)
        return node_cls(**node_kwargs, parent=parent, _session=session, _status=NodeStatus.SOURCE)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack {node_data.metatype.name}: {node_data!r}") from e


def pack_node_inline(
    root: Node, exclude: set[NodeType] = None
) -> tuple[NodeDataT, list[NodeDataT]]:
    """Pack a node and all its inline descendants"""
    exclude = exclude or ()
    packed_by_id: dict[UUID, AnyNodeData] = OrderedDict()

    to_pack = root._root_graph.collect_descendants(root, recursive=True)
    packed_by_id[root.id] = pack_node(root)
    for node in to_pack:
        if node.metatype in exclude:
            continue
        packed_by_id[node.ck] = pack_node(node)

    return packed_by_id[root.id], list(packed_by_id.values())


def unpack_nodes_inline(
    source_graph: NodeDataGraph,
    parent: Node | None,
    session: Session | None = None,
    exclude: set[NodeType] = None,
    roots: Collection[NodeDataT] = None,
) -> tuple[Node, ...] | list[Node]:
    """Unpack nodes and their descendants. Returns the actual roots (or passed ones)."""

    exclude = exclude or tuple()
    unpacked_roots: list[Node] = []
    source_roots = source_graph.find_roots()
    for root_data in source_roots:
        unpacked_graph = NodeGraph()
        # unpack all nodes top down (breadth first)
        for node_data in chain((root_data,), source_graph.iter_descendants(root_data)):
            if node_data.metatype in exclude:
                continue

            node_parent_id: UUID | None = (
                to_uuid(node_data.parent_ptr.id) if node_data.parent_ptr is not None else None
            )
            if node_parent_id is None or parent is not None and node_parent_id == parent.id:
                node_parent = parent
            else:
                node_parent = unpacked_graph.get(node_parent_id)
                if node_parent is None:
                    raise ValueError(f"parent {node_parent_id} not found in {unpacked_graph!r}")
            node = unpack_node(node_data, node_parent, session=session)

            # keep parent instance if it was passed (update in place)
            if parent is not None and node.id == parent.id:
                for prop in parent.__properties__.values():
                    if not prop.is_runtime_only and not prop.is_graph_reference:
                        setattr(parent, prop.name, getattr(node, prop.name))
                node = parent

            unpacked_graph.add(node)

        # index & recover node lists
        root = unpacked_graph.find_root()
        if root is None:
            raise ValueError(f"no root found in {unpacked_graph!r}")
        if isinstance(root, ScopeNode):
            root._root_graph.set(unpacked_graph.nodes)
        for node in unpacked_graph.nodes_by_id.values():
            # status is auto-set to interpreted if a session is active, but that's wrong here
            node._status = NodeStatus.SOURCE
        unpacked_roots.append(root)

    if roots:
        # recover roots if specified (may not be actual roots)
        recovered_roots = []
        for root in roots:
            for found_root in unpacked_roots:  # somewhat inefficient...
                recovered = found_root._root_graph.get(to_uuid(root.id))
                if recovered is not None:
                    recovered_roots.append(recovered)
                    break
        return recovered_roots
    else:
        return unpacked_roots


def unpack_node_inline(
    source_graph: NodeDataGraph,
    parent: Node | None,
    session: Session | None = None,
    exclude: set[NodeType] = None,
    root: NodeDataT = None,
) -> Node:
    """Unpack a node and all its inline descendants"""
    roots = unpack_nodes_inline(
        source_graph, parent, session, exclude, roots=[root] if root is not None else None
    )
    if len(roots) != 1:
        raise ValueError(f"expected 1 root, got {len(roots)}")
    return roots[0]


def wrap_some_node(node: AnyNodeData) -> wire.SomeNodeData:
    """Wraps a concrete node type into a generic node message."""
    wrapper = wire.SomeNodeData()
    field_name = to_casing(node.metatype.name, Casing.SNAKE)
    setattr(wrapper, field_name, node)
    return wrapper


def unwrap_some_node(node: wire.SomeNodeData) -> NodeDataT:
    """Unwraps a generic node type into a concrete node type."""
    _, wrapped_node = betterproto.which_one_of(node, "node")
    assert wrapped_node is not None, f"node not set in {node!r}"
    return wrapped_node
