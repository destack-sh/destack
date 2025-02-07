from dataclasses import dataclass
from typing import TYPE_CHECKING, Collection, cast
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import LocalNodeList, NodeSuperGraph
from bench.language.core import (
    NODE_TYPES_SET,
    BuiltinObject,
    CustomObject,
    Node,
    NodeReference,
    NodeType,
    Property,
    ReferenceKind,
    SomeValue,
    Struct,
    TypeBase,
    TypeKind,
    get_custom_object_properties,
)

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class ProjectOptions:
    node_types: Collection[NodeType] = NODE_TYPES_SET
    inline_node_types: Collection[NodeType] = (NodeType.FIELD, NodeType.TRIGGER)


class Projection:
    """A projection into the Bench graph, collecting referenced Nodes."""

    __slots__ = (
        "depth_by_node_id",
        "missing_nodes_by_id",
        "nodes_by_depth",
        "nodes_by_id",
        "options",
        "supergraph",
    )

    def __init__(self, supergraph: NodeSuperGraph, options: ProjectOptions):
        self.supergraph: NodeSuperGraph = supergraph
        self.options: ProjectOptions = options
        self.nodes_by_id: dict[UUID, Node] = {}
        self.missing_nodes_by_id: dict[UUID, NodeReference] = {}
        self.depth_by_node_id: dict[UUID, int] = {}
        self.nodes_by_depth: dict[int, list[Node]] = {}

    def __str__(self) -> str:
        str_parts = [f"nodes={len(self.nodes_by_id)}", f"missing={len(self.missing_nodes_by_id)}"]
        return ", ".join(str_parts)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"

    def __len__(self) -> int:
        return len(self.nodes_by_id)

    def __contains__(self, node: Node | NodeReference) -> bool:
        return node.id in self.nodes_by_id

    @property
    def nodes(self):
        return self.nodes_by_id.values()

    def add_node(self, node: Node | NodeReference, depth: int) -> bool:
        """Adds a node to the result set."""
        if isinstance(node, Node):
            if node.id not in self.nodes_by_id and node.metatype in self.options.node_types:
                self.nodes_by_id[node.id] = node
                self.depth_by_node_id[node.id] = depth
                if depth not in self.nodes_by_depth:
                    self.nodes_by_depth[depth] = []
                self.nodes_by_depth[depth].append(node)
                return True
            else:
                return False
        else:
            assert node.id is not None, f"missing id for {node!r}"
            if (
                node.id not in self.missing_nodes_by_id
                and node.node_type in self.options.node_types
            ):
                self.missing_nodes_by_id[node.id] = node
                return True
            else:
                return False

    def collect_node(self, node: Node, depth: int):
        """Collect a Node."""
        self.collect_builtin_object(node, depth)
        # inline children
        for prop in node.__node_child_properties__.values():
            if (
                not prop.reference_nodes
                or prop.reference_nodes[0] not in self.options.inline_node_types
            ):
                continue
            children = getattr(node, prop.name)
            if type(children) is not LocalNodeList:
                continue
            elif not prop.is_list:
                self.add_node(cast(Node, children), depth)
            else:
                for child in cast(list, children):
                    self.add_node(cast(Node, child), depth)

    def collect_builtin_object(self, obj: BuiltinObject, depth: int):
        """Collect a BuiltinObject."""
        cls = obj._get_effective_cls()
        # nodes
        for prop in cls.__node_reference_properties__.values():
            if prop.id is None or prop.id < 30 or prop.reference_kind != ReferenceKind.NODE_REGULAR:
                continue
            prop = prop.reference_wired_ptr
            assert prop is not None, f"no wired prop for {prop!r}"
            prop_value = getattr(obj, prop.name)
            if prop_value is None:
                continue
            elif prop.is_list:
                for node_ptr in cast(list, prop_value):
                    node = self.supergraph.get(node_ptr)
                    self.add_node(node or node_ptr, depth)
            else:
                node = self.supergraph.get(prop_value)
                self.add_node(node or prop_value, depth)
        # structs
        for prop in cls.__struct_properties__.values():
            struct: Struct | list[Struct] | None = getattr(obj, prop.name)
            if struct is None:
                continue
            elif not prop.is_list:
                self.collect_builtin_object(cast(Struct, struct), depth)
            elif len(cast(list, struct)) > 0:
                for item in cast(list, struct):
                    self.collect_builtin_object(item, depth)
        # custom objects
        for prop in cls.__value_runtime_properties__.values():
            if type(prop.value_packed_ptr) is not Property:
                continue
            value_packed = getattr(obj, prop.value_packed_ptr.name)
            if not value_packed:
                continue
            value = getattr(obj, prop.name)
            assert type(value) is CustomObject, f"unexpected {value!r} for {prop!r}"
            self.collect_custom_object(value, depth)

    def collect_custom_object(self, obj: CustomObject, depth: int):
        """Collect a CustomObject."""
        if obj._value is None:
            return
        # properties
        for prop in get_custom_object_properties(obj._type, obj._value):
            storage_key = prop.subtype_key or prop.key
            prop_value = cast(SomeValue, obj._value.get(storage_key))
            if prop_value is None:
                continue
            self.collect_value(prop_value, prop.type_info, depth)
        # fields
        for field in obj._type._fields:
            field_value = obj._value.get(field.storage_key)
            if field_value is None:
                continue
            self.collect_value(field_value, field, depth)

    def collect_value(self, value: SomeValue, typ: TypeBase, depth: int):
        """Collect a typed value."""
        if value is None:
            return
        if typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_OBJECT:
            if typ.is_list:
                for item in cast(list, value):
                    self.collect_custom_object(cast(CustomObject, item), depth)
            else:
                self.collect_custom_object(cast(CustomObject, value), depth)
        elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
            if typ.is_list:
                for wired_ptr in cast(list, value):
                    node = self.supergraph.get(wired_ptr)
                    self.add_node(node or wired_ptr, depth)
            else:
                node = self.supergraph.get(cast(NodeReference, value))
                self.add_node(node or cast(NodeReference, value), depth)
