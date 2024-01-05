import enum
from collections import OrderedDict
from copy import copy
from dataclasses import dataclass
from typing import Any, Mapping, Self, Union
from uuid import UUID

import betterproto
import structlog
from betterproto.lib.google.protobuf import Struct as BetterprotoStruct

from bench.language.const import BenchType, NodeType, StructType
from bench.language.module import (
    BENCH_CLASS_BY_TYPE,
    METATYPE_PROPERTY,
    NODE_CLASS_BY_NODE_TYPE,
    STRUCT_CLASS_BY_STRUCT_TYPE,
    Node,
    NodeStatus,
    NodeTree,
    Property,
    ScopeNode,
    Struct,
)
from bench.language.session import Session
from bench.proto import wire
from bench.sql.core import ColumnType
from bench.utils.func import to_uuid
from bench.utils.utils import hybridmethod, to_snake_case

logger = structlog.get_logger(__name__)
# could auto-gen the Any types?
AnyNodeData = Union[wire.ModuleData, wire.FileData, wire.StatementData, wire.FieldData]
AnyStructData = Union[wire.WorkerImageData, wire.ExpressionData]


# monkey-patch betterproto 'Struct' to fix from_dict/to_dict
#  pulls ahead changes from https://github.com/danielgtaylor/python-betterproto/pull/551
#  see https://github.com/danielgtaylor/python-betterproto/issues/332


@dataclass(eq=False, repr=False)
class PatchedStruct(BetterprotoStruct):
    @hybridmethod
    def from_dict(cls: type[Self], value: Mapping[str, Any]) -> Self:  # noqa
        self = cls()
        return self.from_dict(value)

    @from_dict.instancemethod
    def from_dict(self, value: Mapping[str, Any]) -> Self:
        fields = {**value}
        for k in fields:
            if hasattr(fields[k], "from_dict"):
                fields[k] = fields[k].from_dict()

        self.fields = fields
        return self

    def to_dict(
        self,
        casing: betterproto.Casing = betterproto.Casing.CAMEL,
        include_default_values: bool = False,
    ) -> dict[str, Any]:
        output = {**self.fields}
        for k in self.fields:
            if hasattr(self.fields[k], "to_dict"):
                output[k] = self.fields[k].to_dict(casing, include_default_values)
        return output


# ensure 'Value' is in namespace the first time a Struct-like class is created
# if we don't do this here calls will fail mysteriously later
from betterproto.lib.google.protobuf import Value  # noqa

PatchedStruct()

BetterprotoStruct.from_dict = PatchedStruct.from_dict
BetterprotoStruct.to_dict = PatchedStruct.to_dict

PROTO_CLASS_BY_TYPE: dict[BenchType, type[Union[AnyNodeData, AnyStructData]]] = {
    _type: getattr(wire, _type.camel_name + "Data")
    for _type in BenchType
    if hasattr(wire, _type.camel_name + "Data")  # may just be creating it
}


def copy_struct_data(data: AnyStructData) -> AnyStructData:
    """Deepcopy a struct data object."""
    data_cls = PROTO_CLASS_BY_TYPE[data.metatype.name]
    bench_cls = BENCH_CLASS_BY_TYPE[data.metatype.name]
    data_kwargs = {}
    try:
        for prop in bench_cls.__stored_properties__.values():
            if prop.is_computed and prop.id != METATYPE_PROPERTY.id and prop.name != "parent_id":
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
            elif prop.column_type == ColumnType.JSON:
                data_kwargs[prop.name] = copy(value)
            else:
                data_kwargs[prop.name] = value
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not copy {data.metatype.name}: {data!r}") from e
    return data_cls(**data_kwargs)


def pack_jsonable(value: dict) -> BetterprotoStruct:
    return BetterprotoStruct.from_dict(value)


def unpack_jsonable(value: BetterprotoStruct) -> dict:
    return value.to_dict()


def _pack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_array and not ignore_array:
        return [_pack_struct_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        return pack_struct(value)
    elif prop.is_enum:
        return pack_enum(prop.py_type_stripped, value)
    elif prop.column_type == ColumnType.UUID:
        return str(value)  # uuids are wired as strings
    elif prop.column_type == ColumnType.JSON:
        return BetterprotoStruct.from_dict(value)
    else:
        return value


def pack_enum(enum_cls: type[enum.Enum], value: Any) -> Any:
    if issubclass(enum_cls, enum.IntFlag):
        return int(value)
    else:
        proto_enum_cls = getattr(wire, enum_cls.__name__)
        return proto_enum_cls[value.name]


def _unpack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    try:
        if value is None:
            return None
        elif prop.is_array and not ignore_array:
            return [_unpack_struct_prop(prop, v, ignore_array=True) for v in value]
        elif prop.is_struct:
            return unpack_struct(value)
        elif prop.is_enum:
            return unpack_enum(prop.py_type_stripped, value)
        elif prop.column_type == ColumnType.UUID:
            return to_uuid(value)  # uuids are wired as strings
        elif prop.column_type == ColumnType.JSON:
            return value.to_dict()
        else:
            return value
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack value: {value!r} for {prop!r}") from e


def unpack_enum(enum_cls: type[enum.Enum], value: Any) -> Any:
    if issubclass(enum_cls, int):
        return enum_cls(value)
    elif type(value) == str:  # noqa
        return enum_cls(value)
    elif value.name == "UNSPECIFIED":
        return None  # revert to default
    else:
        return enum_cls[value.name]


def pack_struct(struct: Struct) -> AnyStructData:
    """Pack a struct and any contained structs."""
    data_cls = PROTO_CLASS_BY_TYPE[struct.metatype]
    data_kwargs = {}
    try:
        for prop in struct.__stored_properties__.values():
            value = getattr(struct, prop.name)
            data_kwargs[prop.name] = _pack_struct_prop(prop, value, ignore_array=False)
        return data_cls(**data_kwargs)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not pack {struct.metatype.name}: {struct!r}") from e


def unpack_struct(struct_data: AnyStructData) -> Struct:
    """Unpack a struct and any contained structs."""
    struct_cls = STRUCT_CLASS_BY_STRUCT_TYPE[StructType(struct_data.metatype.name)]
    struct_kwargs = {}
    try:
        for prop in struct_cls.__stored_properties__.values():
            if prop.is_computed:
                continue
            value = getattr(struct_data, prop.name)
            struct_kwargs[prop.name] = _unpack_struct_prop(prop, value, ignore_array=False)
        return struct_cls(**struct_kwargs)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack {struct_data.metatype.name}: {struct_data!r}") from e


def pack_node(node: Node) -> AnyNodeData:
    return pack_struct(node)


def unpack_node(node_data: AnyNodeData, parent: Node, session: Session | None) -> Node:
    node_cls = NODE_CLASS_BY_NODE_TYPE[NodeType(node_data.metatype.name)]
    node_kwargs = {}
    try:
        for prop in node_cls.__stored_properties__.values():
            if prop.is_computed:
                continue
            value = getattr(node_data, prop.name)
            node_kwargs[prop.name] = _unpack_struct_prop(prop, value, ignore_array=False)
        return node_cls(**node_kwargs, parent=parent, _session=session)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack {node_data.metatype.name}: {node_data!r}") from e


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
    my_root: UUID | None = None,
) -> Node:
    """Unpack a node and all its inline descendants"""
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
                if not prop.is_runtime_only and not prop.is_tree_relation:
                    setattr(parent, prop.name, getattr(node, prop.name))
            node = parent

        unpacked_tree.add(node)

    # index & recover node lists
    real_root = unpacked_tree.root
    if isinstance(real_root, ScopeNode):
        real_root._local_root_tree.set(unpacked_tree.nodes_by_ck.values())
    for node in unpacked_tree.nodes_by_id.values():
        node._status = NodeStatus.SOURCE  # status is auto-set to interpreted if a session is active
        if isinstance(node, ScopeNode):
            node._update_lists(node)
    if isinstance(real_root, ScopeNode):
        real_root._index_rec()
    elif isinstance(real_root, Node):
        real_root._index_self()
    else:
        raise ValueError(f"unexpected root {real_root} ({type(real_root)})")

    if my_root:
        return real_root.lookup(my_root)
    return real_root


def wrap_some_node(node: AnyNodeData) -> wire.SomeNodeData:
    """Wraps a concrete node type into a generic node message."""
    wrapper = wire.SomeNodeData()
    field_name = to_snake_case(node.metatype.name)
    setattr(wrapper, field_name, node)
    return wrapper


def unwrap_some_node(node: wire.SomeNodeData) -> AnyNodeData:
    """Unwraps a generic node type into a concrete node type."""
    _, wrapped_node = betterproto.which_one_of(node, "node")
    assert wrapped_node is not None, f"node not set in {node!r}"
    return wrapped_node
