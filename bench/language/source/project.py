from dataclasses import dataclass
from typing import TYPE_CHECKING, Collection, Sequence, assert_never, cast
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.core import (
    NODE_TYPES_SET,
    BuiltinObject,
    CustomObject,
    Node,
    NodeReference,
    NodeType,
    ReferenceKind,
    SomeValue,
    SourceNode,
    Struct,
    TypeKind,
)
from bench.language.core.value import _do_get_value_runtime

from .block import Block
from .field import TypeBase

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class ProjectOptions:
    max_depth: int = 10
    inline_pages: bool = False  # whether to include all source nodes of contained pages
    node_types: Collection[NodeType] = NODE_TYPES_SET
    # child ndoes to 'consider' implicitly references by parents :FoldedNodes
    inline_node_types: Collection[NodeType] = (NodeType.FIELD,)
    inline_page_node_types: Collection[NodeType] = (NodeType.BLOCK,)


class Projection:
    """
    A projection into the Bench graph, collecting referenced nodes with some settings.
    Should be a proper Struct at some point (so we can inspect it in the editor).
    NOTE :Incomplete: support loading "missing" (unloaded but referenced) nodes on demand
    """

    def __init__(self, options: ProjectOptions):
        self.options: ProjectOptions = options
        self._nodes_by_id: dict[UUID, Node] = {}
        self._remote_nodes_by_id: dict[UUID, NodeReference] = {}
        self._depth_by_node_id: dict[UUID, int] = {}
        self._nodes_by_depth: dict[int, list[Node]] = {}
        self._nodes_to_collect: list[Node] = []

    def __str__(self) -> str:
        str_parts = [
            f"nodes={len(self._nodes_by_id)}",
            f"depths={'|'.join([f'{d}:{len(n)}' for d, n in self._nodes_by_depth.items()] or ['<empty>'])}",
            f"missing={len(self._remote_nodes_by_id)}",
        ]
        return ", ".join(str_parts)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"

    def __len__(self) -> int:
        return len(self._nodes_by_id)

    def __contains__(self, node: Node) -> bool:
        return node.id in self._nodes_by_id

    has = __contains__

    def has_remote(self, node: Node | NodeReference) -> bool:
        return node.id in self._remote_nodes_by_id

    def _visit_node(self, node: Node) -> bool:
        """Adds a node to the result set."""
        if node.id not in self._nodes_by_id and node.metatype in self.options.node_types:
            self._nodes_by_id[node.id] = node
            self._nodes_to_collect.append(node)
            return True
        else:
            return False

    def _visit_node_ref(self, node: NodeReference) -> bool:
        """Adds a node reference to the remote set."""
        assert node.id is not None, f"missing id for {node!r}"
        if node.id not in self._remote_nodes_by_id and node.node_type in self.options.node_types:
            self._remote_nodes_by_id[node.id] = node
            return True
        else:
            return False

    def _collect_node(self, node: Node):
        """Collects a node (recursively)."""
        self._collect_builtin_object_scalar(node)
        self._collect_node_children(node, self.options.inline_node_types)

    def _collect_node_children(self, node: Node, node_types: Collection[NodeType]):
        """Collects children of a node (recursively)."""
        for prop in node.__node_reference_properties__.values():
            if (
                prop.reference_kind != ReferenceKind.NODE_CHILDREN
                or not prop.reference_nodes
                or prop.reference_nodes[0] not in node_types
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

    # NOTE :Incomplete: account for missing nodes in builtin object / custom object node lookups
    #  (see CustomObject._do_get, _object_node_ref and :RichReference)

    def _collect_builtin_object_scalar(self, obj: BuiltinObject):
        """Collects a builtin object (recursively)."""
        cls = obj._get_effective_cls()
        # visit referenced nodes
        for prop in cls.__node_reference_properties__.values():
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
                        self._visit_node_ref(wired_ptr)
            else:
                prop_value = obj._supergraph.get(wired_prop_value)
                if prop_value is not None:
                    self._visit_node(cast(Node, prop_value))
                else:
                    self._visit_node_ref(wired_prop_value)

        # visit inner structs
        for prop in cls.__struct_properties__.values():
            struct: Struct | list[Struct] | None = getattr(obj, prop.name)
            if struct is None:
                continue
            elif not prop.is_list:
                self._collect_builtin_object_scalar(cast(Struct, struct))
            elif len(cast(list, struct)) > 0:
                for item in cast(list, struct):
                    self._collect_builtin_object_scalar(item)

        # visit values
        for prop in cls.__value_runtime_properties__.values():
            value_type, wired_prop_value = _do_get_value_runtime(cast("Struct | Node", obj), prop)
            if value_type is not None:
                self._collect_value(wired_prop_value, value_type)

    def _collect_custom_object_scalar(self, obj: CustomObject):
        """Collects a CustomObject (recursively)."""
        if obj._value is None:
            return
        # visit fields / inner objects
        for field in obj._type._base_fields:
            wired_field_value = obj._value.get(field.storage_key)
            if wired_field_value is None:
                continue
            self._collect_value(wired_field_value, field)

    def _collect_value(self, value: SomeValue, typ: TypeBase):
        """Collects a specific value (recursively)."""
        if value is None:
            return
        if typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_OBJECT:
            if typ.is_list:
                for item in cast(list, value):
                    self._collect_custom_object_scalar(cast(CustomObject, item))
            else:
                self._collect_custom_object_scalar(cast(CustomObject, value))
        elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
            if typ.is_list:
                for wired_ptr in cast(list, value):
                    field_value = typ._supergraph.get(wired_ptr)
                    if field_value is not None:
                        self._visit_node(cast(Node, field_value))
                    else:
                        self._visit_node_ref(wired_ptr)
            else:
                field_value = typ._supergraph.get(cast(NodeReference, value))
                if field_value is not None:
                    self._visit_node(cast(Node, field_value))
                else:
                    self._visit_node_ref(cast(NodeReference, value))

    def _do_project(self, depth: int, max_depth: int):
        """Projects the current nodes to the given depth (or until we run out)."""
        while depth < max_depth and self._nodes_to_collect:
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
        return depth

    @tracer.start_as_current_span("projection.project")
    def project(
        self,
        *objs: BuiltinObject | CustomObject | None,
        max_depth: int | None = None,
        inline_pages: bool | None = None,
    ) -> Sequence[Node]:
        """
        Collect all referenced nodes from the given objects.
        If inlining pages, we do a second pass where we collect all source children of contained pages.
        Return the new nodes collected.
        """

        max_depth = self.options.max_depth if max_depth is None else max_depth
        inline_pages = self.options.inline_pages if inline_pages is None else inline_pages
        old_nodes_by_id = {**self._nodes_by_id}

        # collect nodes from initial values
        for obj in objs:
            if obj is None:
                continue  # convenient when passing multiple objects
            elif isinstance(obj, CustomObject):
                self._collect_custom_object_scalar(obj)
            elif isinstance(obj, Node):
                self._visit_node(obj)
            elif isinstance(obj, BuiltinObject):
                self._collect_builtin_object_scalar(obj)
            else:
                assert_never(obj)

        # first pass
        depth = self._do_project(0, max_depth)

        # second pass if inlining pages
        if inline_pages:
            new_nodes_by_id = {
                k: v for k, v in self._nodes_by_id.items() if k not in old_nodes_by_id
            }
            containing_pages = find_containing_pages(*new_nodes_by_id.values())
            for page in containing_pages:
                self._collect_node(page)
                self._collect_node_children(page, self.options.inline_page_node_types)
            # and project that
            self._do_project(depth, depth + 1)
        return tuple(self._nodes_by_id[id] for id in self._nodes_by_id if id not in old_nodes_by_id)

    def get_containing_pages(self) -> list[Block]:
        """Gets the pages containing the collected source nodes."""
        return find_containing_pages(*self._nodes_by_id.values())

    def get_remote_nodes(self) -> list[NodeReference]:
        """Gets the nodes that were referenced but not found."""
        return list(self._remote_nodes_by_id.values())

    def get_nodes_like[T: Node | NodeReference](self, *node_classes: type[T]) -> list[T]:
        """Gets the nodes of the given type (including missing nodes)."""
        nodes = [n for n in self._nodes_by_id.values() if isinstance(n, node_classes)] + [
            n for n in self._remote_nodes_by_id.values() if isinstance(n, node_classes)
        ]
        return nodes


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


def project(*objs: BuiltinObject | CustomObject | None, options: ProjectOptions) -> Projection:
    """
    Project the nodes, objects and values referenced by the given objects recursively.
    If inlining pages, also include all referenced pages (non-recursively).
    """
    projection: Projection = Projection(options)
    projection.project(*objs)
    return projection
