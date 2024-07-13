from dataclasses import dataclass
from typing import TYPE_CHECKING, Collection, cast
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.const import NODE_TYPES_SET, NodeType, ReferenceKind, TypeKind
from bench.language.node import BuiltinObject, InlineStruct, Node, NodeReferenceBase
from bench.language.value import ValueObject

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class ProjectOptions:
    max_depth: int = 5
    node_types: Collection[NodeType] = NODE_TYPES_SET


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
        "_missing_nodes_by_type",
        "_nodes_by_container_id",
        "_nodes_by_id",
        "_nodes_to_collect",
        "_options",
    )

    def __init__(self, options: ProjectOptions):
        self._options: ProjectOptions = options
        self._nodes_by_id: dict[UUID, Node] = {}
        self._missing_nodes_by_id: dict[UUID, NodeReferenceBase] = {}
        self._missing_nodes_by_type: dict[NodeType, list[NodeReferenceBase]] = {}
        self._depth_by_node_id: dict[UUID, int] = {}
        self._nodes_to_collect: list[Node] = []

    def __str__(self) -> str:
        str_parts = [
            f"nodes={len(self._nodes_by_id)}",
            f"missing={len(self._missing_nodes_by_id)}",
            f"depth={self._depth}",
        ]
        if self._nodes_to_collect:
            str_parts.append(f"to_collect={len(self._nodes_to_collect)}")
        return ", ".join(str_parts)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"

    def _visit_node(self, node: Node) -> bool:
        if node.id not in self._nodes_by_id and node.metatype in self._options.node_types:
            self._nodes_by_id[node.id] = node
            self._nodes_to_collect.append(node)
            return True
        else:
            return False

    def _collect_node(self, node: Node):
        self._collect_builtin_object_scalar(node)
        self._collect_node_children(node)

    def _collect_node_children(self, node: Node):
        # visit all node children
        for prop in node.__node_reference_properties__.values():
            if prop.reference_kind != ReferenceKind.NODE_CHILDREN:
                continue
            children = getattr(node, prop.name)
            if children is None:
                continue
            elif not prop.is_list:
                self._visit_node(cast(Node, children))
            elif len(cast(list, children)) > 0:
                for child in cast(list, children):
                    self._visit_node(cast(Node, child))

    # NOTE :Incomplete: account for missing nodes in builtin object / value object node lookups
    #  (see ValueObject._do_get, _object_node_ref and :RichReference)

    def _collect_builtin_object_scalar(self, obj: BuiltinObject):
        # visit referenced nodes
        for prop in obj.__node_reference_properties__.values():
            if prop.reference_kind != ReferenceKind.NODE_REGULAR:
                continue
            prop_value = getattr(obj, prop.name)
            if prop_value is None:
                continue
            elif prop.is_list:
                for item in cast(list, prop_value):
                    self._visit_node(cast(Node, item))
            else:
                self._visit_node(cast(Node, prop_value))

        # visit inner structs
        for prop in obj.__struct_properties__.values():
            struct: InlineStruct | list[InlineStruct] | None = getattr(obj, prop.name)
            if struct is None:
                continue
            elif not prop.is_list:
                self._collect_builtin_object_scalar(cast(InlineStruct, struct))
            elif len(cast(list, struct)) > 0:
                for item in cast(list, struct):
                    self._collect_builtin_object_scalar(item)

    def _collect_value_object_scalar(self, obj: ValueObject):
        # visit inner objects and referenced nodes
        if obj._value is None:
            return
        for field in obj._type._base_fields:
            field_value_packed = obj._value.get(field.storage_key)
            if not field_value_packed:
                continue
            field_value = obj._do_get(field)
            if field_value is None:
                continue
            elif field.kind == TypeKind.OBJECT:
                if field.is_list:
                    for item in cast(list, field_value):
                        self._collect_value_object_scalar(cast(ValueObject, item))
                else:
                    self._collect_value_object_scalar(cast(ValueObject, field_value))
            elif field.kind == TypeKind.NODE or field.kind == TypeKind.BASED_NODE:
                if field.is_list:
                    for item in cast(list, field_value):
                        self._visit_node(cast(Node, item))
                else:
                    self._visit_node(cast(Node, field_value))

    def project(self, *objs: BuiltinObject | ValueObject | None):
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
            for node in nodes_at_layer:
                self._depth_by_node_id[node.id] = depth
            self._nodes_to_collect = []
            depth += 1
            for node in nodes_at_layer:
                self._collect_node(node)


@tracer.start_as_current_span("projection.project")
def project(*objs: BuiltinObject | ValueObject | None, options: ProjectOptions) -> Projection:
    """Project the nodes, objects and values referenced by the given objects."""
    projection: Projection = Projection(options)
    projection.project(*objs)
    return projection
