import enum
from collections import OrderedDict
from copy import copy
from itertools import chain
from typing import Any, Collection, TypeVar, Union, cast
from uuid import UUID

import betterproto
import structlog
from betterproto.lib.google.protobuf import Struct as BetterprotoStruct

from bench.language import Property
from bench.language.const import NodeType, ObjectType
from bench.language.graph import NodeDataGraph
from bench.language.node import NODE_CLASS_BY_TYPE, InterpStatus, Node, NodeGraph, Struct
from bench.language.notice import NoticeHandler, on_notice_ignore, on_warning_raise
from bench.language.property import METATYPE_PROPERTY
from bench.language.session import Session
from bench.language.setup import BENCH_CLASS_BY_TYPE
from bench.language.validation import on_invalid_raise
from bench.proto import wire
from bench.proto.wire import AnyNodeData, AnyStructData, NodeReferenceData
from bench.sql.core import PrimitiveType
from bench.utils.casing import Casing, to_casing
from bench.utils.func import IdEnum, to_uuid

logger = structlog.get_logger(__name__)

PROTO_CLASS_BY_TYPE: dict[ObjectType, type[Union[AnyNodeData, AnyStructData]]] = {
    _type: getattr(wire, _type.bench_name + "Data")
    for _type in ObjectType
    if hasattr(wire, _type.bench_name + "Data")  # may just be creating it
}
OBJECT_TYPE_BY_PROTO_CLASS: dict[type[Union[AnyNodeData, AnyStructData]], ObjectType] = {
    cls: object_type for object_type, cls in PROTO_CLASS_BY_TYPE.items()
}
BENCH_CLASS_BY_PROTO_CLASS: dict[
    type[Union[AnyNodeData, AnyStructData]], type[Union[Node, Struct]]
] = {
    cls: BENCH_CLASS_BY_TYPE[object_type] for cls, object_type in OBJECT_TYPE_BY_PROTO_CLASS.items()
}

NodeT = TypeVar("NodeT", bound=Node)
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
StructT = TypeVar("StructT", bound=Struct)
StructDataT = TypeVar("StructDataT", bound=Union[AnyStructData, AnyNodeData])


def copy_data(data: StructDataT) -> StructDataT:
    """Deepcopy a struct data object."""
    data_cls = PROTO_CLASS_BY_TYPE[data.metatype]
    bench_cls = BENCH_CLASS_BY_TYPE[data.metatype]
    data_kwargs = {}
    try:
        for prop in bench_cls.__wired_properties__.values():
            if prop.is_computed and prop.id != METATYPE_PROPERTY.id:
                continue
            value = getattr(data, prop.name)
            if value is None or value == "" and not prop.is_required:
                data_kwargs[prop.name] = None
            elif prop.is_list:
                if prop.is_struct:
                    data_kwargs[prop.name] = [copy_data(v) for v in value]
                else:
                    data_kwargs[prop.name] = list(value)
            elif prop.is_struct:
                data_kwargs[prop.name] = copy_data(value)
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
    elif prop.is_list and not ignore_array:
        return [_pack_struct_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        return pack_struct(value)
    elif prop.is_enum:
        return pack_enum(prop.py_type_stripped, value)
    elif prop.reference_kind is not None and not prop.reference_kind.is_struct_tree:
        # struct references are just integers
        return NodeReferenceData(
            metatype=wire.ObjectType.NODE_REFERENCE,
            type=pack_enum(ObjectType, value.type),
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
        elif prop.is_list and not ignore_array:
            return [_unpack_struct_prop(prop, v, ignore_array=True) for v in value]
        elif prop.is_struct:
            return unpack_struct(value)
        elif prop.is_enum:
            return unpack_enum(prop.py_type_stripped, value)
        elif prop.reference_kind is not None and not prop.reference_kind.is_struct_tree:
            # struct references are just integers
            return NodeReference(
                type=unpack_enum(NodeType, value.type),
                id=UUID(value.id),
                ck=UUID(value.ck) if value.ck else None,
            )
        elif prop.primitive_type == PrimitiveType.UUID:
            return UUID(value)  # uuids are wired as strings
        elif prop.primitive_type == PrimitiveType.JSON:
            return value.to_dict()
        else:
            return value
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack value: {value!r} for {prop!r}") from e


def pack_struct(struct: StructT) -> StructDataT:
    """Pack a struct and any contained structs."""
    data_cls = PROTO_CLASS_BY_TYPE[struct.metatype]
    metatype = pack_enum(ObjectType, struct.metatype)
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
    struct_cls = BENCH_CLASS_BY_TYPE[ObjectType(struct_data.metatype)]
    struct_kwargs = {}
    try:
        for prop in struct_cls.__wired_properties__.values():
            if prop.is_computed:
                continue
            value = getattr(struct_data, prop.name)
            struct_kwargs[prop.name] = _unpack_struct_prop(prop, value, ignore_array=False)
        return struct_cls(**struct_kwargs, _status=InterpStatus.SOURCE)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack {struct_data.metatype.name}: {struct_data!r}") from e


def unpack_struct_maybe(struct_data: StructDataT | None) -> StructT | None:
    if struct_data is None:
        return None
    return unpack_struct(struct_data)


def unpack_struct_interp(
    struct_data: StructDataT,
    scope: Node | None = None,
    on_notice: NoticeHandler = on_warning_raise,
) -> StructT:
    """Unpack, interpret and validate a Struct."""
    struct = unpack_struct(struct_data)
    struct._interp_rec(scope=scope, on_notice=on_notice)
    struct._validate_rec(properties=(), on_invalid=on_invalid_raise)
    return struct


def unpack_struct_interp_maybe(
    struct_data: StructDataT | None,
    scope: Node | None = None,
    on_notice: NoticeHandler = on_warning_raise,
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
        return node_cls(**node_kwargs, parent=parent, _session=session, _status=InterpStatus.SOURCE)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack {node_data.metatype.name}: {node_data!r}") from e


def pack_node_graph(root: Node, exclude: set[NodeType] = None) -> tuple[NodeDataT, list[NodeDataT]]:
    """Pack a node and all its descendants"""
    exclude = exclude or ()
    packed_by_id: dict[UUID, AnyNodeData] = OrderedDict()

    to_pack = root._root_graph.collect_descendants(root, recursive=True)
    packed_by_id[root.id] = pack_node(root)
    for node in to_pack:
        if node.metatype in exclude:
            continue
        packed_by_id[node.ck] = pack_node(node)

    return packed_by_id[root.id], list(packed_by_id.values())


def unpack_node_graph(
    data_graph: NodeDataGraph,
    parent: Node | None = None,
    session: Session | None = None,
    exclude: set[NodeType] = None,
) -> NodeGraph:
    """Unpacks the node data(s) into a node graph."""

    parent_id = parent.id if parent is not None else None
    exclude = exclude or tuple()
    unpacked_roots: list[Node] = []
    source_roots = data_graph.find_roots()
    unpacked_graph = NodeGraph()

    for root_data in source_roots:
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
                node_parent = unpacked_graph.get(node_parent_id)
                # TODO @Architecture: enable loading nodes without ancestors :LoadOrphanNode
                #  (this errors as below, but sometimes we just want a node without ancestors)
                # if node_parent is None:
                #     raise ValueError(f"parent {node_parent_id} not found in {unpacked_graph!r}")
            node = unpack_node(node_data, node_parent, session=session)

            # keep parent instance if it was passed (update in place)
            if node.id == parent_id:
                for prop in parent.__properties__.values():
                    if not prop.is_ephemeral and not prop.is_tree_reference:
                        setattr(parent, prop.name, getattr(node, prop.name))
                node = parent

            unpacked_graph.add(node)

    # index & recover node lists
    for source_root in source_roots:
        root = unpacked_graph.get(to_uuid(source_root.id))
        if root is None:
            raise ValueError(f"root {source_root!r} root found in unpacked {unpacked_graph!r}")
        root._graph.set(unpacked_graph.nodes)
        root._data_graph = data_graph
        for node in unpacked_graph.nodes_by_id.values():
            # status is auto-set to interpreted if a session is active, but that's wrong here
            node._status = InterpStatus.SOURCE
            # TODO :Cleanup: it feels weird to manually interp *and* track in unpack?
            #  (we want to resolve node references and such)
            node._interp_self(node, on_notice=on_notice_ignore)
            if session is not None:
                node._track_self(session)
        unpacked_roots.append(root)

    return unpacked_graph


def unpack_roots(
    data_graph: NodeDataGraph,
    parent: Node | None = None,
    session: Session | None = None,
    exclude: set[NodeType] = None,
    roots: Collection[NodeReferenceData] = None,
) -> tuple[Node, ...] | list[Node]:
    """Unpack nodes and their descendants. Returns the actual roots (or passed ones)."""

    node_graph = unpack_node_graph(data_graph, parent, session, exclude=exclude)

    if roots:
        # recover roots if specified (may not be actual roots)
        recovered_roots = []
        for root in roots:
            unpacked_root = node_graph.get(to_uuid(root.id))
            if unpacked_root is not None:
                recovered_roots.append(unpacked_root)
        return recovered_roots
    else:
        return node_graph.find_roots()


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
