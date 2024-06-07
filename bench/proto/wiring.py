from collections import OrderedDict
from copy import copy
from itertools import chain
from typing import Any, Collection, Union, cast
from uuid import UUID

import betterproto
import structlog
from betterproto.lib.google.protobuf import Struct as ProtoStruct
from opentelemetry import trace

from bench.language.const import UNSET, NodeType, ObjectType
from bench.language.graph import NodeDataGraph
from bench.language.node import BuiltinObject, Node, NodeGraph, ReadInfo
from bench.language.property import Property
from bench.language.session import Session
from bench.language.setup import OBJECT_CLASS_BY_TYPE
from bench.language.validation import on_invalid_raise
from bench.proto import wire
from bench.proto.wire import AnyNodeData, AnyStructData, NodeReferenceData
from bench.sql.core import PrimitiveType
from bench.utils.casing import Casing, to_casing
from bench.utils.func import IdEnum, IdEnumOrUnion, to_uuid

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

PROTO_CLASS_BY_TYPE: dict[ObjectType, type[Union[AnyNodeData, AnyStructData]]] = {
    _type: getattr(wire, _type.bench_name + "Data")
    for _type in ObjectType
    if hasattr(wire, _type.bench_name + "Data")  # may just be creating a new class
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
    bench_cls = OBJECT_CLASS_BY_TYPE[cast(ObjectType, data.metatype)]
    data_copy = type(data)(metatype=data.metatype)  # type: ignore
    try:
        for prop in bench_cls.__wired_properties__.values():
            if not hasattr(data, prop.name):
                continue
            value = getattr(data, prop.name)
            value = copy_struct_prop(prop, value)
            setattr(data_copy, prop.name, value)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not copy {data.metatype.name}: {data!r}") from e
    return data_copy  # type: ignore


def copy_struct_prop(prop: Property, value: Any) -> Any:
    """Deepcopy a single struct property."""
    if value is None or (value == "" and not prop.is_required):
        return None
    elif prop.is_list:
        if prop.is_struct:
            return [copy_struct(cast(Any, v)) for v in value]
        else:
            return list(value)
    elif prop.is_struct:
        return copy_struct(cast(Any, value))
    elif prop.primitive_type == PrimitiveType.JSON:
        return copy(value)
    else:
        return value


def pack_proto_json(value: dict[str, Any]) -> ProtoStruct:
    return ProtoStruct.from_dict(value)


def unpack_proto_json(value: ProtoStruct) -> dict[str, Any]:
    return value.to_dict()


def pack_enum[EnumT: IdEnumOrUnion](enum_cls: type[EnumT], value: EnumT) -> Any:
    assert isinstance(enum_cls, type), f"{enum_cls} is not a type"
    assert issubclass(enum_cls, IdEnum), f"{enum_cls} is not an IdEnum"
    assert isinstance(value, int), f"{value} is not an int"
    return value


def unpack_enum[EnumT: IdEnumOrUnion](enum_cls: type[EnumT], value: Any) -> EnumT:
    assert isinstance(enum_cls, type), f"{enum_cls} is not a type"
    assert issubclass(enum_cls, IdEnum), f"{enum_cls} is not an IdEnum"
    return enum_cls(value)


def pack_object_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_list and not ignore_array:
        return [pack_object_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        return pack_object(value)
    elif prop.is_enum:
        return pack_enum(prop.py_type_stripped, value)
    elif prop.reference_kind is not None and not prop.reference_kind.is_struct_tree:
        value_id = str(value.id)
        return NodeReferenceData(
            metatype=wire.ObjectType.NODE_REFERENCE,
            type=pack_enum(NodeType, value.type),
            id=value_id,
            ck=str(value.ck) if value.ck is not None else value_id,
        )
    elif prop.primitive_type == PrimitiveType.UUID:
        return str(value)  # uuids are wired as strings
    elif prop.primitive_type == PrimitiveType.JSON:
        return ProtoStruct.from_dict(value)
    else:
        return value


def unpack_object_prop(prop: Property, value: Any, ignore_array: bool = False) -> Any:
    from bench.language.expression import NodeReference

    try:
        if value is None:
            return None
        elif prop.is_list and not ignore_array:
            return [unpack_object_prop(prop, v, ignore_array=True) for v in value]
        elif prop.is_struct:
            return unpack_object(value)
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
            return value.to_dict()
        else:
            return value
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack value: {value!r} for {prop!r}") from e


def pack_object[T: AnyStructData | AnyNodeData](
    obj: BuiltinObject, expect: type[T] | None = None
) -> T:
    """Pack a struct and any contained structs."""
    data_cls = PROTO_CLASS_BY_TYPE[obj.metatype]
    metatype = pack_enum(ObjectType, obj.metatype)  # type: ignore
    if expect is not None and not issubclass(data_cls, expect):
        raise RuntimeError(f"expected {expect.__name__} but got {data_cls}")
    data = data_cls(metatype=metatype)  # type: ignore
    try:
        for prop in obj.__wired_properties__.values():
            value = getattr(obj, prop.name)
            value = pack_object_prop(prop, value, ignore_array=False)
            setattr(data, prop.name, value)
        return cast(T, data)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not pack {obj.metatype.name}: {obj!r}") from e


def pack_object_maybe[T: AnyStructData | AnyNodeData](
    obj: BuiltinObject | None, expect: type[T]
) -> T | None:
    if obj is None:
        return None
    else:
        return pack_object(obj, expect)


def unpack_object[T: BuiltinObject](
    obj_data: AnyStructData | AnyNodeData,
    *,
    parent: Node | None = None,
    expect: type[T] | None = None,
    session: Session | None = None,
) -> T:
    """Unpack a struct and any contained structs."""
    object_cls = OBJECT_CLASS_BY_TYPE[ObjectType(obj_data.metatype)]  # type: ignore
    if expect and not issubclass(object_cls, expect):
        raise RuntimeError(f"expected {expect} but got {object_cls}")
    object_kwargs = {}
    try:
        for prop in object_cls.__wired_properties__.values():
            if not prop.is_runtime or prop.is_computed:
                continue
            value = getattr(obj_data, prop.name)
            object_kwargs[prop.name] = unpack_object_prop(prop, value, ignore_array=False)
        if parent is not None:
            object_kwargs["parent"] = parent
        if issubclass(object_cls, Node):
            object_kwargs["_session"] = UNSET
        obj = object_cls(**object_kwargs)
        if session is not None and isinstance(obj, Node):
            obj._resolve_references(obj)
            obj._track_self(session)
        return cast(T, obj)
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not unpack {obj_data.metatype.name}: {obj_data!r}") from e


def unpack_object_interp[T: BuiltinObject](
    obj_data: AnyStructData | AnyNodeData,
    parent: Node | None = None,
    scope: Node | None = None,
    expect: type[T] | None = None,
    session: Session | None = None,
) -> T:
    """Unpack, interpret and validate a Struct."""
    obj = unpack_object(obj_data, parent=parent, expect=expect, session=session)
    obj._interp_rec(scope=scope)
    obj._validate_rec(properties=(), invalid=on_invalid_raise)
    return obj


def unpack_object_interp_maybe[T: BuiltinObject](
    obj_data: AnyStructData | AnyNodeData | None,
    scope: Node | None = None,
    expect: type[T] | None = None,
    session: Session | None = None,
) -> T | None:
    if obj_data is None:
        return None
    else:
        return unpack_object_interp(obj_data, scope=scope, expect=expect, session=session)


@tracer.start_as_current_span("wiring.pack_node_graph")
def pack_node_graph(
    root: Node, exclude: set[NodeType] | tuple[NodeType, ...] = ()
) -> tuple[AnyNodeData, list[AnyNodeData]]:
    """Pack a node and all its descendants"""
    exclude = exclude or ()
    packed_by_id: dict[UUID, AnyNodeData] = OrderedDict()

    to_pack = root._graph.collect_descendants(root, recursive=True)
    packed_by_id[root.id] = pack_object(root)
    for node in to_pack:
        if node.metatype in exclude:
            continue
        packed_by_id[node.ck] = pack_object(node)

    return packed_by_id[root.id], list(packed_by_id.values())


@tracer.start_as_current_span("wiring.unpack_node_graph")
def unpack_node_graph(
    data_graph: NodeDataGraph,
    parent: Node | None = None,
    session: Session | None = None,
    exclude: set[NodeType] | tuple[NodeType, ...] | None = (),
    read: ReadInfo | None = None,
) -> NodeGraph["Node"]:
    """Unpacks the node data(s) into a node graph."""
    trace.get_current_span().set_attribute("nodes", len(data_graph))

    exclude = exclude or ()
    parent_id = parent.id if parent is not None else None
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
                # NOTE :Architecture: enable loading nodes without ancestors :LoadOrphanNode?
                #  (this errors here, but sometimes we just want a node without ancestors)
                # if node_parent is None:
                #     raise ValueError(f"parent {node_parent_id} not found in {unpacked_graph!r}")
            node = unpack_object(node_data, parent=node_parent)
            node._read = read

            # keep parent instance if it was passed (update it in place)
            if node.id == parent_id:
                for prop in (cast(Node, parent)).__properties__.values():
                    if not prop.is_ephemeral and not prop.is_tree_reference:
                        setattr(parent, prop.name, getattr(node, prop.name))
                node = parent

            unpacked_graph.add(node)

    # update parent references
    if parent is not None:
        parent._resolve_references(parent)

    # resolve references
    for source_root in source_roots:
        root = unpacked_graph.get(UUID(source_root.id))
        if root is None:
            raise ValueError(f"root {source_root!r} root found in unpacked {unpacked_graph!r}")
        root._graph.set(unpacked_graph.nodes)
        for node in unpacked_graph.nodes_by_id.values():
            node._resolve_references(node)
            if session is not None:
                node._track_self(session)
        unpacked_roots.append(root)

    return unpacked_graph


def unpack_node_roots(
    data_graph: NodeDataGraph,
    parent: Node | None = None,
    session: Session | None = None,
    exclude: set[NodeType] | None = None,
    roots: Collection[NodeReferenceData] | None = None,
    read: ReadInfo | None = None,
) -> tuple[tuple[Node, ...], NodeGraph]:
    """Unpack nodes and their descendants. Returns the actual roots (or passed ones)."""

    node_graph = unpack_node_graph(data_graph, parent, session, exclude=exclude, read=read)

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
    field_name = to_casing(cast(str, node.metatype.name), Casing.SNAKE)
    setattr(wrapper, field_name, node)
    return wrapper


def unwrap_some_node(node: wire.SomeNodeData) -> AnyNodeData:
    """Unwraps a generic node type into a concrete node type."""
    _, wrapped_node = betterproto.which_one_of(node, "node")
    assert wrapped_node is not None, f"node not set in {node!r}"
    return wrapped_node
