from collections.abc import Collection, Sequence
from typing import (
    TYPE_CHECKING,
    Optional,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language.registry import (
    NODE_CLASS_BY_TRAIT,
    NODE_CLASS_BY_TYPE,
    NODE_TRAIT_BY_CLASS,
    NODE_TYPE_BY_CLASS,
    NODE_TYPES_BY_TRAIT,
)
from bench.utils.fractional import INTEGER_MAX

from .const import EMPTY_LIST, UNSET, NodeType, TraitType

if TYPE_CHECKING:
    from bench.language import Node, QueryConnection, Session

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Graph:
    """
    A Graph is a collection of Nodes in a Session.
    Graphs may be tied to the result of a Query (via QueryConnection) or just free-floating.
    """

    __slots__ = ("connection", "nodes_by_id", "nodes_by_parent_id", "supergraph")

    def __init__(self, supergraph: "Supergraph", connection: "QueryConnection | None"):
        self.supergraph = supergraph
        self.connection = connection
        self.nodes_by_id: dict[UUID, Node] = {}
        self.nodes_by_parent_id: dict[UUID, dict[NodeType, list[Node]]] = {}

    def __str__(self):
        return f"{len(self.nodes)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self) -> Collection["Node"]:
        """All nodes in the graph"""
        return self.nodes_by_id.values()

    def __len__(self):
        """Number of nodes in the graph"""
        return len(self.nodes_by_id)

    def get(self, id: UUID) -> Optional["Node"]:
        """Gets a node by id"""
        return self.nodes_by_id.get(id)

    def get_or_error(self, id: UUID) -> "Node":
        """Gets a node by id, raising an error if not found"""
        node = self.get(id)
        if node is None:
            raise KeyError(f"node {id!r} not found in {self!r}")
        return node

    def has(self, id: UUID) -> bool:
        """Check if a Node exists in this Graph."""
        return id in self.nodes_by_id

    def clear(self):
        """Clear the Graph."""
        # supergraph
        for node in self.nodes:
            if self.supergraph._cached_nodes_by_id.get(node.id) is node:
                self.supergraph._cached_nodes_by_id.pop(node.id)
        # nodes
        self.nodes_by_id.clear()
        self.nodes_by_parent_id.clear()

    def add(self, node: "Node"):
        """Add a new Node to the graph (must not exist, excluding descendants)."""
        if (existing := self.nodes_by_id.get(node.id)) is not None:
            raise ValueError(f"node {node!r} already in {self!r}: {existing!r}")
        # node
        self.nodes_by_id[node.id] = node
        # parent
        if (parent_ptr := node.parent_ptr) is not None:
            if parent_ptr.id not in self.nodes_by_parent_id:
                self.nodes_by_parent_id[parent_ptr.id] = {}
            child_node_type = node.metatype
            if child_node_type not in self.nodes_by_parent_id[parent_ptr.id]:
                self.nodes_by_parent_id[parent_ptr.id][child_node_type] = []
            self.nodes_by_parent_id[parent_ptr.id][child_node_type].append(node)
        # supergraph
        if (
            cached := self.supergraph._cached_nodes_by_id.get(node.id)
        ) is None or cached is _MISSING:
            self.supergraph._cached_nodes_by_id[node.id] = node

    def remove(self, node: "Node"):
        """Remove a Node from the graph (must exist, excluding descendants)."""
        # supergraph
        if self.supergraph._cached_nodes_by_id.get(node.id) is node:
            self.supergraph._cached_nodes_by_id.pop(node.id)
        # parent
        if (parent_ptr := node.parent_ptr) is not None:
            if parent_ptr.id not in self.nodes_by_parent_id:
                self.nodes_by_parent_id[parent_ptr.id] = {}
            child_node_type = node.metatype
            if child_node_type not in self.nodes_by_parent_id[parent_ptr.id]:
                self.nodes_by_parent_id[parent_ptr.id][child_node_type] = []
            self.nodes_by_parent_id[parent_ptr.id][child_node_type].remove(node)
            if not self.nodes_by_parent_id[parent_ptr.id][child_node_type]:
                self.nodes_by_parent_id[parent_ptr.id].pop(child_node_type)
                if not self.nodes_by_parent_id[parent_ptr.id]:
                    self.nodes_by_parent_id.pop(parent_ptr.id)
        # node
        self.nodes_by_id.pop(node.id)

    def get_children[N: Node = Node](
        self,
        node: "Node",
        node_type: NodeType | TraitType | type[N] | None = None,
    ) -> Sequence[N]:
        """
        Collect child Nodes (one level down).
        If the Nodes are IsOrdered, their order is preserved.
        """
        # bail if no children
        if not self.nodes_by_parent_id:
            return ()
        children_by_type = self.nodes_by_parent_id.get(node.id)
        if not children_by_type:
            return ()

        if node_type is None:
            # collect children across all types
            children: list = []
            is_ordered = False
            for children_of_type in children_by_type.values():
                node_cls = type(children_of_type[0])
                if TraitType.ORDERED in node_cls.__traits__:
                    is_ordered = True
                children.extend(children_of_type)
            if is_ordered:
                children.sort(key=lambda n: getattr(n, "order_key", INTEGER_MAX))
            return children
        else:
            # turn into type
            node_cls: type[Node]
            node_types: tuple[NodeType, ...]
            if isinstance(node_type, type):
                if node_t := NODE_TYPE_BY_CLASS.get(node_type):
                    node_cls = node_type
                    node_types = (node_t,)
                else:
                    node_cls = node_type
                    node_types = NODE_TYPES_BY_TRAIT[NODE_TRAIT_BY_CLASS[node_type]]  # type: ignore
            else:
                if isinstance(node_type, NodeType):
                    node_cls = NODE_CLASS_BY_TYPE[node_type]
                    node_types = (node_type,)
                else:
                    node_cls = NODE_CLASS_BY_TRAIT[node_type]  # type: ignore
                    node_types = NODE_TYPES_BY_TRAIT[node_type]

            # collect
            if len(node_types) == 1:
                # collect for single node type
                children: list = children_by_type.get(node_types[0], EMPTY_LIST)
                if children and TraitType.ORDERED in node_cls.__traits__:
                    children.sort(key=lambda n: getattr(n, "order_key", INTEGER_MAX))
                return children
            else:
                # collect for trait (multiple node types)
                children: list = []
                for node_type in node_types:
                    children.extend(children_by_type.get(node_type, EMPTY_LIST))
                if children and TraitType.ORDERED in node_cls.__traits__:
                    children.sort(key=lambda n: getattr(n, "order_key", INTEGER_MAX))
                return children

    def get_descendants[N: Node = Node](
        self,
        node: "Node",
        node_type: NodeType | TraitType | type[N] | None = None,
    ) -> Sequence[N]:
        """
        Collect descendant Nodes (recursively down).
        If a type is specified, only Nodes of that type are collected.
        (Descendants are not collected unless all their ancestors are included).
        Nodes are BFS but IsOrdered is ignored.
        """
        if not self.nodes_by_parent_id:
            return ()

        queue: list[Node] = [node]
        descendants: list[Node] = []

        # turn into type
        node_types: tuple[NodeType, ...] | None = None
        if node_type is not None:
            if isinstance(node_type, type):
                if issubclass(node_type, Node):
                    node_types = (node_type.metatype,)
                else:
                    trait_type = NODE_TRAIT_BY_CLASS[node_type]
                    node_types = NODE_TYPES_BY_TRAIT[trait_type]
            else:
                if isinstance(node_type, NodeType):
                    node_types = (node_type,)
                else:
                    node_types = NODE_TYPES_BY_TRAIT[node_type]

        # collect
        while queue:
            current = queue.pop(0)
            children_by_type = self.nodes_by_parent_id.get(current.id)
            if not children_by_type:
                continue
            for children_of_type in children_by_type.values():
                queue.extend(children_of_type)

            # collect level
            if node_types is None:
                for children_of_type in children_by_type.values():
                    descendants.extend(children_of_type)
            else:
                for node_t in node_types:
                    descendants.extend(children_by_type.get(node_t, ()))

        return descendants  # type: ignore (must be right type@)


_MISSING = object()


class Supergraph:
    """
    A collection of Graphs in a Session.
    Instances of a Node may be in multiple Graphs (but any specific Node is only in one Graph).
    """

    __slots__ = ("_cached_nodes_by_id", "graphs", "session")

    def __init__(self, session: "Session"):
        self.session = session
        self.graphs: list[Graph] = []
        self._cached_nodes_by_id: dict[UUID, Node | object] = {}  # cache

    def __str__(self):
        return f"{len(self.graphs)} graphs"

    def __repr__(self):
        return f"<Supergraph {self}>"

    def add_graph(self, graph: Graph):
        """Add a Graph to this Supergraph."""
        self.graphs.append(graph)
        for node in graph.nodes:
            if (cached := self._cached_nodes_by_id.get(node.id)) is None or cached is _MISSING:
                self._cached_nodes_by_id[node.id] = node

    def remove_graph(self, graph: Graph):
        """Remove a Graph from this Supergraph."""
        self.graphs.remove(graph)

    def get(self, node_id: "UUID") -> Optional["Node"]:
        """Get a node by ID."""
        cached = self._cached_nodes_by_id.get(node_id, UNSET)
        if cached is UNSET:
            # look in all graphs
            for graph in self.graphs:
                if (node := graph.get(node_id)) is not None:
                    self._cached_nodes_by_id[node_id] = node
                    return node
            else:
                self._cached_nodes_by_id[node_id] = _MISSING
                return None
        elif cached is _MISSING:
            return None
        else:
            return cached  # type: ignore (must be Node)

    def get_or_error(self, node_id: "UUID") -> "Node":
        """Get a node by ID (error if not found)."""
        node = self.get(node_id)
        if node is None:
            raise KeyError(f"node {node_id!r} not found in {self!r}")
        return node
