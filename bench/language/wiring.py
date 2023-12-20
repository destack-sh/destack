import enum
from collections import OrderedDict
from typing import Any, Union
from uuid import UUID

import betterproto
import structlog

from bench.language import Session, wire
from bench.language.const import BenchType, NodeType, StructType
from bench.language.module import (
    NODE_CLASS_BY_NODE_TYPE,
    STRUCT_CLASS_BY_STRUCT_TYPE,
    Node,
    NodeStatus,
    NodeTree,
    Property,
    ScopeNode,
    Struct,
)
from bench.sql.core import ColumnType
from bench.utils.utils import to_snake_case

logger = structlog.get_logger(__name__)
# nocheckin: auto-gen AnyNodeData/AnyStructData?
AnyNodeData = Union[wire.ModuleData, wire.FileData, wire.StatementData, wire.FieldData]
AnyStructData = Union[wire.StructType]

# :ProtoSchema
PROTO_CLASS_BY_TYPE: dict[BenchType, type[Union[AnyNodeData, AnyStructData]]] = {
    _type: getattr(wire, _type.camel_name + "Data") for _type in BenchType
}


def to_uuid(id: str | UUID | None) -> UUID | None:
    if not id:
        return None  # ignore empty strings
    if isinstance(id, str):
        try:
            return UUID(id)
        except ValueError as e:
            raise ValueError(f"invalid UUID: {id!r}") from e
    if isinstance(id, UUID):
        return id
    raise TypeError(f"unexpected id type: {id!r}")


def _pack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_array and not ignore_array:
        return [_pack_struct_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        return pack_struct(value)
    elif prop.is_enum:
        proto_enum_cls = getattr(wire, prop.py_type_raw.__name__)
        return proto_enum_cls(value)
    elif prop.store_as == ColumnType.UUID:
        return str(value)  # uuids are wired as strings
    else:
        return value


def _unpack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_array and not ignore_array:
        return [_unpack_struct_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        return unpack_struct(value)
    elif prop.is_enum:
        if isinstance(value, int):
            return prop.py_type_raw(value)
        elif value.name == "UNSPECIFIED":
            return None  # revert to default
        return prop.py_type_raw[value.name]
    elif prop.store_as == ColumnType.UUID:
        return to_uuid(value)  # uuids are wired as strings
    else:
        return value


def pack_struct(struct: Struct) -> AnyStructData:
    """Pack a struct and any contained structs."""
    data_cls = PROTO_CLASS_BY_TYPE[struct.metatype]
    data_kwargs = {}
    for prop in struct.__stored_properties__.values():
        value = getattr(struct, prop.name)
        data_kwargs[prop.name] = _pack_struct_prop(prop, value, ignore_array=False)
    return data_cls(**data_kwargs)


def unpack_struct(data: AnyStructData) -> Struct:
    """Unpack a struct and any contained structs."""
    struct_cls = STRUCT_CLASS_BY_STRUCT_TYPE[StructType(data.metatype.name)]
    struct_kwargs = {}
    for prop in data.__stored_properties__.values():
        if prop.is_computed:
            continue
        value = getattr(data, prop.name)
        struct_kwargs[prop.name] = _unpack_struct_prop(prop, value, ignore_array=False)
    return struct_cls(**struct_kwargs)


def pack_node(node: Node) -> AnyNodeData:
    node_cls = PROTO_CLASS_BY_TYPE[node.metatype]
    node_kwargs = {}
    for prop in node.__stored_properties__.values():
        value = getattr(node, prop.name)
        node_kwargs[prop.name] = _pack_struct_prop(prop, value, ignore_array=False)
    return node_cls(**node_kwargs)


def unpack_node(data: AnyNodeData, parent: Node, session: Session | None) -> Node:
    node_cls = NODE_CLASS_BY_NODE_TYPE[NodeType(data.metatype.name)]
    node_kwargs = {}
    for prop in node_cls.__stored_properties__.values():
        if prop.is_computed:
            continue
        value = getattr(data, prop.name)
        node_kwargs[prop.name] = _unpack_struct_prop(prop, value, ignore_array=False)
    return node_cls(**node_kwargs, parent=parent, _session=session)


def pack_node_inline(
    root: Node, exclude: set[NodeType] = None
) -> tuple[AnyNodeData, list[AnyNodeData]]:
    """Pack a node and all its inline descendants"""
    exclude = exclude or ()
    packed_by_id: dict[UUID, AnyNodeData] = OrderedDict()

    to_pack = root._local_root_tree.get_descendants(root.ck, include_self=True, recursive=True)
    for node in to_pack:
        if node.metatype in exclude:
            continue
        packed_by_id[node.ck] = pack_node(node)

    return packed_by_id[root.id], list(packed_by_id.values())


def unpack_node_inline(
    source_tree: NodeTree[AnyNodeData],
    parent: Node | None,
    session: Session | None = None,
    exclude: set[NodeType] = None,
) -> Node:
    exclude = exclude or tuple()
    unpacked_tree = NodeTree()

    # unpack all nodes top down (breadth first)
    for node_data in source_tree.walk_bfs():
        if node_data.metatype in exclude:
            continue

        parent_id = to_uuid(node_data.parent_id)
        if parent_id is None:
            node_parent = parent
        elif parent_id not in unpacked_tree.nodes_by_id:
            if parent is not None and parent_id == parent.id:
                node_parent = parent
            else:
                logger.warn(
                    f"node {node_data.id} parent {parent_id} not found in unpacked {unpacked_tree!r}"
                )
                continue  # can happen if there was a race condition in delete cascade and create
        else:
            node_parent = unpacked_tree.nodes_by_id[parent_id]
        node = unpack_node(node_data, node_parent, session=session)

        # keep parent instance if it was passed (update in place)
        if parent is not None and node.id == parent.id:
            for prop in parent.__properties__.values():
                if not prop.is_runtime and not prop.is_tree_relation:
                    setattr(parent, prop.name, getattr(node, prop.name))
            node = parent

        unpacked_tree.add(node)

    # index & recover node lists
    root = unpacked_tree.root
    if isinstance(root, ScopeNode):
        root._local_root_tree.set(unpacked_tree.nodes_by_ck.values())
    for node in unpacked_tree.nodes_by_id.values():
        node._status = NodeStatus.SOURCE  # status is auto-set to interpreted if a session is active
        if isinstance(node, ScopeNode):
            node._update_lists(node)

    if isinstance(root, ScopeNode):
        root._index_rec()
    elif isinstance(root, Node):
        root._index_self()
    else:
        raise ValueError(f"unexpected root {root} ({type(root)})")

    return root


def wrap_some_node(node: AnyNodeData) -> wire.SomeNodeData:
    """Wraps a concrete node type into a generic node message."""
    field_name = to_snake_case(node.metatype)
    wrapper = wire.SomeNodeData()
    setattr(wrapper, field_name, node)
    return wrapper


def unwrap_some_node(node: wire.SomeNodeData) -> AnyNodeData:
    """Unwraps a generic node type into a concrete node type."""
    _, wrapped_node = betterproto.which_one_of(node, "node")
    assert wrapped_node is not None, f"node not set in {node!r}"
    return wrapped_node
