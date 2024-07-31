from dataclasses import dataclass
from typing import TYPE_CHECKING, Collection, cast
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.block import Block
from bench.language.const import NODE_TYPES_SET, NodeType, ReferenceKind, TypeKind
from bench.language.field import TypeInfoBase
from bench.language.node import BuiltinObject, Node, SomeNodeReference, SourceNode, Struct
from bench.language.value import SomeValue, ValueObject, _do_get_value_runtime

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class ProjectOptions:
    max_depth: int = 10
    node_types: Collection[NodeType] = NODE_TYPES_SET
    # child ndoes to 'consider' implicitly references by parents :FoldedNodes
    folded_child_types: Collection[NodeType] = (NodeType.FIELD, NodeType.TRIGGER, NodeType.QUERY)


class Projection:
    """
    A projection into the Bench graph, collecting referenced nodes with some settings.
    Should be a proper Struct at some point (so we can inspect it in the editor).
    NOTE :Incomplete: support loading "missing" (unloaded but referenced) nodes on demand
    """

    __slots__ = (
        "_depth",
        "_depth_by_node_id",
        "_missing_nodes_by_id",
        "_nodes_by_container_id",
        "_nodes_by_depth",
        "_nodes_by_id",
        "_nodes_to_collect",
        "_options",
    )

    def __init__(self, options: ProjectOptions):
        self._options: ProjectOptions = options
        self._nodes_by_id: dict[UUID, Node] = {}
        self._missing_nodes_by_id: dict[UUID, SomeNodeReference] = {}
        self._depth_by_node_id: dict[UUID, int] = {}
        self._nodes_by_depth: dict[int, list[Node]] = {}
        self._nodes_to_collect: list[Node] = []

    def __str__(self) -> str:
        str_parts = [
            f"nodes={len(self._nodes_by_id)}",
            f"depths={'|'.join([f'{d}:{len(n)}' for d, n in self._nodes_by_depth.items()] or ['<empty>'])}",
            f"missing={len(self._missing_nodes_by_id)}",
        ]
        return ", ".join(str_parts)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"

    def __len__(self) -> int:
        return len(self._nodes_by_id)

    def __contains__(self, node: Node) -> bool:
        return node.id in self._nodes_by_id

    has = __contains__

    def has_missing(self, node: Node | SomeNodeReference) -> bool:
        return node.id in self._missing_nodes_by_id

    def _visit_node(self, node: Node) -> bool:
        """Adds a node to the result set."""
        if node.id not in self._nodes_by_id and node.metatype in self._options.node_types:
            self._nodes_by_id[node.id] = node
            self._nodes_to_collect.append(node)
            return True
        else:
            return False

    def _visit_missing_node(self, node: SomeNodeReference) -> bool:
        """Adds a node reference to the missing set."""
        assert node.id is not None, f"missing id for {node!r}"
        if node.id not in self._missing_nodes_by_id and node.type in self._options.node_types:
            self._missing_nodes_by_id[node.id] = node
            return True
        else:
            return False

    def _collect_node(self, node: Node):
        """Collects a node (recursively)."""
        self._collect_builtin_object_scalar(node)
        self._collect_node_folded_children(node)

    def _collect_node_folded_children(self, node: Node):
        """Collects the folded children of a node (recursively)."""
        for prop in node.__node_reference_properties__.values():
            if (
                prop.reference_kind != ReferenceKind.NODE_CHILDREN
                or not prop.reference_nodes
                or prop.reference_nodes[0] not in self._options.folded_child_types
            ):
                continue
            children = getattr(node, prop.name)
            if children is None:
                continue
            elif not prop.is_list:
                self._visit_node(cast(Node, children))
            else:
                for child in cast(list, children):
                    self._visit_node(cast(Node, child))

    # NOTE :Incomplete: account for missing nodes in builtin object / value object node lookups
    #  (see ValueObject._do_get, _object_node_ref and :RichReference)

    def _collect_builtin_object_scalar(self, obj: BuiltinObject):
        """Collects a builtin object (recursively)."""
        # visit referenced nodes
        for prop in obj.__node_reference_properties__.values():
            if prop.id is None or prop.id < 30 or prop.reference_kind != ReferenceKind.NODE_REGULAR:
                continue
            wired_prop = prop.reference_wired_ptr
            assert wired_prop is not None, f"no wired prop for {prop!r}"
            wired_prop_value = getattr(obj, wired_prop.name)
            if wired_prop_value is None:
                continue
            elif prop.is_list:
                for wired_ptr in cast(list, wired_prop_value):
                    prop_value = obj._supergraph.get(wired_ptr)
                    if prop_value is not None:
                        self._visit_node(cast(Node, prop_value))
                    else:
                        self._visit_missing_node(wired_ptr)
            else:
                prop_value = obj._supergraph.get(wired_prop_value)
                if prop_value is not None:
                    self._visit_node(cast(Node, prop_value))
                else:
                    self._visit_missing_node(wired_prop_value)

        # visit inner structs
        for prop in obj.__struct_properties__.values():
            struct: Struct | list[Struct] | None = getattr(obj, prop.name)
            if struct is None:
                continue
            elif not prop.is_list:
                self._collect_builtin_object_scalar(cast(Struct, struct))
            elif len(cast(list, struct)) > 0:
                for item in cast(list, struct):
                    self._collect_builtin_object_scalar(item)

        # visit values
        for prop in obj.__value_runtime_properties__.values():
            value_type, wired_prop_value = _do_get_value_runtime(obj, prop)
            if value_type is not None:
                self._collect_value(wired_prop_value, value_type)

    def _collect_value_object_scalar(self, obj: ValueObject):
        """Collects a ValueObject (recursively)."""
        if obj._value is None:
            return
        # visit fields / inner objects
        for field in obj._type._base_fields:
            wired_field_value = obj._value.get(field.storage_key)
            if wired_field_value is None:
                continue
            field_type = field._to_resolved()
            self._collect_value(wired_field_value, field_type)

    def _collect_value(self, value: SomeValue, typ: TypeInfoBase):
        """Collects a specific value (recursively)."""
        if value is None:
            return
        typ = typ._to_resolved()
        if typ.kind == TypeKind.OBJECT:
            if typ.is_list:
                for item in cast(list, value):
                    self._collect_value_object_scalar(cast(ValueObject, item))
            else:
                self._collect_value_object_scalar(cast(ValueObject, value))
        elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
            if typ.is_list:
                for wired_ptr in cast(list, value):
                    field_value = typ._supergraph.get(wired_ptr)
                    if field_value is not None:
                        self._visit_node(cast(Node, field_value))
                    else:
                        self._visit_missing_node(wired_ptr)
            else:
                field_value = typ._supergraph.get(cast(SomeNodeReference, value))
                if field_value is not None:
                    self._visit_node(cast(Node, field_value))
                else:
                    self._visit_missing_node(cast(SomeNodeReference, value))

    @tracer.start_as_current_span("projection.project")
    def project(self, *objs: BuiltinObject | ValueObject | None):
        """Collect all referenced nodes from the given objects."""
        depth = 0

        # collect nodes from initial values
        for obj in objs:
            if isinstance(obj, ValueObject):
                self._collect_value_object_scalar(obj)
            elif isinstance(obj, Node):
                self._visit_node(obj)
            elif obj is not None:
                self._collect_builtin_object_scalar(obj)

        # keep collecting nodes until we run out or hit the depth limit
        while depth < self._options.max_depth and self._nodes_to_collect:
            nodes_at_layer = self._nodes_to_collect
            # index these nodes
            if depth not in self._nodes_by_depth:
                self._nodes_by_depth[depth] = []
            for node in nodes_at_layer:
                self._depth_by_node_id[node.id] = depth
                self._nodes_by_depth[depth].append(node)
            # recurse
            self._nodes_to_collect = []
            depth += 1
            for node in nodes_at_layer:
                self._collect_node(node)

    def get_containing_pages(self) -> list[Block]:
        """Gets the pages containing the collected source nodes."""
        return find_containing_pages(*self._nodes_by_id.values())

    def get_missing_nodes(self) -> list[SomeNodeReference]:
        """Gets the nodes that were referenced but not found."""
        return list(self._missing_nodes_by_id.values())


def find_containing_pages(*nodes: Node) -> list[Block]:
    """Finds the containing pages for the given nodes"""
    pages_by_id: dict[UUID, Block] = {}
    for node in nodes:
        if not isinstance(node, SourceNode):
            continue
        parent = node.parent
        while parent is not None:
            if isinstance(parent, Block) and parent.is_page:
                pages_by_id[parent.id] = parent
                break
            parent = parent.parent
    return list(pages_by_id.values())


def project(*objs: BuiltinObject | ValueObject | None, options: ProjectOptions) -> Projection:
    """Project the nodes, objects and values referenced by the given objects."""
    projection: Projection = Projection(options)
    projection.project(*objs)
    return projection
