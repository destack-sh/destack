import enum
from collections import OrderedDict
from copy import copy
from dataclasses import dataclass
from itertools import chain
from typing import Any, Collection, Mapping, Self, Union
from uuid import UUID

import betterproto
import structlog
from betterproto.lib.google.protobuf import Struct as BetterprotoStruct

from bench.language.const import BenchType, NodeType, StructType
from bench.language.module import (
    BENCH_CLASS_BY_TYPE,
    NODE_CLASS_BY_NODE_TYPE,
    STRUCT_CLASS_BY_STRUCT_TYPE,
    TYPE_DISCRIMINATOR_PROPERTY,
    Node,
    NodeStatus,
    NodeTree,
    Property,
    ScopeNode,
    Struct,
)
from bench.language.session import Session
from bench.proto import wire
from bench.proto.core import (
    Enum,
    EnumValue,
    Field,
    FieldType,
    Message,
    Proto,
    ProtoStrEnum,
    ProtoThing,
)
from bench.sql.core import ColumnType
from bench.utils.func import to_uuid
from bench.utils.utils import hybridmethod, to_all_caps, to_snake_case

logger = structlog.get_logger(__name__)
# nocheckin: auto-gen AnyNodeData/AnyStructData?
AnyNodeData = Union[wire.ModuleData, wire.FileData, wire.StatementData, wire.FieldData]
AnyStructData = Union[wire.EnvironmentData, wire.ExpressionData]


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

# :ProtoSchema
PROTO_CLASS_BY_TYPE: dict[BenchType, type[Union[AnyNodeData, AnyStructData]]] = {
    _type: getattr(wire, _type.camel_name + "Data") for _type in BenchType
}


def copy_struct_data(data: AnyStructData) -> AnyStructData:
    """Deepcopy a struct data object."""
    data_cls = PROTO_CLASS_BY_TYPE[data.metatype.name]
    bench_cls = BENCH_CLASS_BY_TYPE[data.metatype.name]
    data_kwargs = {}
    try:
        for prop in bench_cls.__stored_properties__.values():
            if (
                prop.is_computed
                and prop.id != TYPE_DISCRIMINATOR_PROPERTY.id
                and prop.name != "parent_id"
            ):
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
            elif prop.store_as == ColumnType.JSON:
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
        if issubclass(prop.py_type_stripped, enum.IntFlag):
            return int(value)
        else:
            proto_enum_cls = getattr(wire, prop.py_type_raw.__name__)
            return proto_enum_cls[value.name]
    elif prop.store_as == ColumnType.UUID:
        return str(value)  # uuids are wired as strings
    elif prop.store_as == ColumnType.JSON:
        return BetterprotoStruct.from_dict(value)
    else:
        return value


def _unpack_struct_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    try:
        if value is None:
            return None
        elif prop.is_array and not ignore_array:
            return [_unpack_struct_prop(prop, v, ignore_array=True) for v in value]
        elif prop.is_struct:
            return unpack_struct(value)
        elif prop.is_enum:
            if issubclass(prop.py_type_stripped, int):
                return prop.py_type_raw(value)
            elif type(value) == str:
                return prop.py_type_raw(value)
            elif value.name == "UNSPECIFIED":
                return None  # revert to default
            else:
                return prop.py_type_raw[value.name]
        elif prop.store_as == ColumnType.UUID:
            return to_uuid(value)  # uuids are wired as strings
        elif prop.store_as == ColumnType.JSON:
            return value.to_dict()
        else:
            return value
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack value: {value!r} for {prop!r}") from e


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
    wrapper = wire.SomeNodeData()
    field_name = to_snake_case(node.metatype.name)
    setattr(wrapper, field_name, node)
    return wrapper


def unwrap_some_node(node: wire.SomeNodeData) -> AnyNodeData:
    """Unwraps a generic node type into a concrete node type."""
    _, wrapped_node = betterproto.which_one_of(node, "node")
    assert wrapped_node is not None, f"node not set in {node!r}"
    return wrapped_node


#
# Map Bench types to Proto types
# We map and walk at the same type for simplicity (using the cache)
#

PROTO_FIELD_TYPE_BY_COLUMN_TYPE: dict[ColumnType, FieldType] = {
    ColumnType.BOOLEAN: FieldType.BOOL,
    ColumnType.INT: FieldType.INT32,
    ColumnType.BIGINT: FieldType.INT64,
    ColumnType.FLOAT: FieldType.FLOAT,
    ColumnType.STRING: FieldType.STRING,
    ColumnType.BYTES: FieldType.BYTES,
    ColumnType.DATETIME: FieldType.TIMESTAMP,
    ColumnType.UUID: FieldType.STRING,  # see https://stackoverflow.com/q/36344826/3375858
    ColumnType.JSON: FieldType.STRUCT,
}

_BenchType = type[Union["Node", "Struct", "Property", enum.StrEnum, enum.IntFlag]]


def _bench_property_to_proto(prop: "Property", cache: dict[_BenchType, ProtoThing]) -> Field:
    assert not prop.is_runtime, f"shouldn't map runtime property: {prop!r}"
    assert isinstance(prop.id, int), f"stored properties need an id: {prop!r}"
    # store typed enum/struct references (except for int/flag enums, which proto doesn't have)
    if prop.is_struct or prop.is_enum and prop.store_as == ColumnType.STRING:
        struct_type = bench_to_proto(prop.py_type_stripped, cache)
        return Field(id=prop.id, name=prop.name, type=struct_type, repeated=prop.is_array)
    elif prop.store_as in PROTO_FIELD_TYPE_BY_COLUMN_TYPE:
        field_type = PROTO_FIELD_TYPE_BY_COLUMN_TYPE[prop.store_as]
        return Field(id=prop.id, name=prop.name, type=field_type, repeated=prop.is_array)
    else:
        raise TypeError(f"cannot map to proto type: {prop!r}")


def _bench_struct_to_proto(
    node: type["Struct"], cache: dict[_BenchType, ProtoThing], alias: str = None
) -> Message:
    struct = Message(name=alias or node.__name__, reserved_names=[], reserved_ids=[], fields=[])
    cache[node] = struct  # to solve recursive references
    for prop in node.__properties__.values():
        if not prop.is_stored:
            continue
        field = _bench_property_to_proto(prop, cache)
        struct.fields.append(field)
    for reserved in node.__reserved_properties__:
        if isinstance(reserved, str):
            struct.reserved_names.append(reserved)
        elif isinstance(reserved, int):
            struct.reserved_ids.append(reserved)
        else:
            raise TypeError(f"invalid reserved property: {reserved!r}")
    struct.fields.sort(key=lambda f: f.id)
    return struct


def _bench_enum_to_proto(
    bench_t: type[ProtoStrEnum] | type[enum.IntEnum] | type[enum.IntFlag],
    cache: dict[_BenchType, ProtoThing],
    alias: str = None,
) -> Enum:
    assert issubclass(
        bench_t, (ProtoStrEnum, enum.IntEnum, enum.IntFlag)
    ), f"invalid enum: {bench_t!r}"
    # TODO @Broken: assign static ids to enum values (or use int enums) for proto serialization
    enum_prefix = to_all_caps(alias or bench_t.__name__) + "_"
    if issubclass(bench_t, ProtoStrEnum):
        enum_values = [
            EnumValue(id=member.id, name=enum_prefix + member.name) for member in bench_t
        ]
    elif issubclass(bench_t, (enum.IntFlag, enum.IntEnum)):
        # use int values as ids
        enum_values = [
            EnumValue(id=name, name=enum_prefix + id_) for id_, name in bench_t.__members__.items()
        ]
    else:
        raise TypeError(f"invalid type: {bench_t!r}")
    # add unset if not already present
    if not any(v.id == 0 for v in enum_values):
        enum_values = [EnumValue(id=0, name=enum_prefix + "UNSPECIFIED"), *enum_values]
    has_duplicates = len(enum_values) != len(set(v.id for v in enum_values))
    return Enum(name=alias or bench_t.__name__, values=enum_values, allow_alias=has_duplicates)


def bench_to_proto(
    bench_t: _BenchType, cache: dict[_BenchType, ProtoThing], alias: str = None
) -> ProtoThing:
    """Maps a Bench type to a Proto type. If not yet mapped, adds it to the cache."""
    from bench.language import Node, Struct

    assert isinstance(bench_t, type), f"invalid type: {bench_t!r}"
    if bench_t in cache:
        return cache[bench_t]
    if issubclass(bench_t, (Node, Struct)):
        ret = _bench_struct_to_proto(bench_t, cache, alias=alias)
    elif issubclass(bench_t, enum.Enum):
        ret = _bench_enum_to_proto(bench_t, cache, alias=alias)
    else:
        raise TypeError(f"invalid type: {bench_t!r}")
    cache[bench_t] = ret
    return ret


def generate_proto_schema(
    bench_types: Collection[type[Union["Node", "Struct", enum.Enum]]],
    aliases: dict[type[Union["Node", "Struct", enum.Enum]], str],
    unions: dict[str, tuple[str, list[type[Union["Node", "Struct", enum.Enum]]]]],
    extras: list[Enum | Message],
    message_postfix: str = "",
) -> Proto:
    """Maps a collection of Bench types to a Proto schema :ProtoSchema."""
    from bench.language import Node, Struct

    proto_types_cache: dict[type[_BenchType], ProtoThing] = {}
    for thing in bench_types:
        _ = bench_to_proto(thing, proto_types_cache, alias=aliases.get(thing))

    # collect proto types
    collected_enums: list[type[enum.Enum]] = [t for t in bench_types if issubclass(t, enum.Enum)]
    collected_structs: list[type["Struct"]] = [
        t for t in bench_types if issubclass(t, Struct) and not issubclass(t, Node)
    ]
    collected_nodes: list[type["Node"]] = [t for t in bench_types if issubclass(t, Node)]
    collected_enums.sort(key=lambda t: t.__name__)
    collected_structs.sort(key=lambda t: t.__name__)
    collected_nodes.sort(key=lambda t: t.__name__)
    proto_types: list[Enum | Message] = [
        proto_types_cache[t] for t in chain(collected_enums, collected_structs, collected_nodes)
    ]

    # add custom union types
    for union_name, (wrapper_field_name, unioned_types) in unions.items():
        sub_fields = [
            Field(
                id=i + 1, name=to_snake_case(t.__name__), type=bench_to_proto(t, proto_types_cache)
            )
            for i, t in enumerate(unioned_types)
        ]
        wrapper_field = Field(
            id=None, name=wrapper_field_name, type=FieldType.ONE_OF, sub_fields=sub_fields
        )
        wrapper_message = Message(
            name=union_name, reserved_names=[], reserved_ids=[], fields=[wrapper_field]
        )
        proto_types.append(wrapper_message)
    # and other extra types
    proto_types.extend(extras)

    if message_postfix:  # apply postfix to messages
        for proto_type in proto_types:
            if isinstance(proto_type, Message):
                proto_type.name += message_postfix

    return Proto.from_types("bench", proto_types)
