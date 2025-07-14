import abc
from collections.abc import Collection, Sequence
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    assert_never,
    final,
    override,
)

import structlog
from opentelemetry import trace

from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    NODE_TYPE_BY_CLASS,
    NODE_TYPES_BY_TRAIT_TYPE,
    TRAIT_TYPE_BY_CLASS,
)
from destack.utils.fractional import INTEGER_ZERO
from destack.utils.uuid import UUID

from ..builtin import EMPTY_LIST, UNSET, NodeType, TraitType

if TYPE_CHECKING:
    from destack.language import Entity, Event, Node, Session

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)
type_ = type


class Graph[N: Node](abc.ABC):
    """
    A Graph is a collection of Nodes.
    """

    __slots__ = ("supergraph",)

    def __init__(self, supergraph: "Supergraph"):
        self.supergraph = supergraph

    @final
    def __str__(self):
        return f"{len(self.nodes)} nodes"

    @final
    def __repr__(self):
        return f"<Graph {self}>"

    @property
    @abc.abstractmethod
    def nodes(self) -> Collection[N]:
        """All nodes in the graph"""
        raise NotImplementedError

    def __len__(self):
        """Number of nodes in the graph"""
        raise NotImplementedError

    @abc.abstractmethod
    def get(self, id: UUID) -> Optional[N]:
        """Gets a Node by id"""
        raise NotImplementedError

    @final
    def get_or_error(self, id: UUID) -> N:
        """Gets a Node by id, raising an error if not found"""
        node = self.get(id)
        if node is None:
            raise KeyError(f"node {id!r} not found in {self!r}")
        return node

    @abc.abstractmethod
    def has(self, id: UUID) -> bool:
        """Check if a Node exists in this Graph."""
        raise NotImplementedError

    @abc.abstractmethod
    def clear(self):
        """Clear the Graph."""
        raise NotImplementedError

    @abc.abstractmethod
    def add(self, node: N):
        """Add a new Node to the graph (must not exist, excluding descendants)."""
        raise NotImplementedError

    @abc.abstractmethod
    def remove(self, node: N):
        """Remove a Node from the graph (must exist, excluding descendants)."""
        raise NotImplementedError

    @abc.abstractmethod
    def get_roots[M: Entity = Entity](
        self, node_type: NodeType | TraitType | type[M] | None = None
    ) -> Sequence[M]:
        """Find root Nodes in the graph."""
        raise NotImplementedError

    @abc.abstractmethod
    def get_leaves[M: Entity = Entity](
        self, node_type: NodeType | TraitType | type[M] | None = None, node: "Entity | None" = None
    ) -> Sequence[M]:
        """Find leaf Nodes in the graph."""
        raise NotImplementedError

    @abc.abstractmethod
    def get_children[M: Entity = Entity](
        self,
        node: "Entity",
        type: NodeType | TraitType | type[M] | None = None,
    ) -> Sequence[M]:
        """
        Collect child Nodes (one level down).
        If the Nodes are IsOrdered, their order is preserved.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def get_ancestors[M: Entity = Entity](
        self, node: "Entity", type: NodeType | TraitType | type[M] | None = None
    ) -> Sequence[M]:
        """Gets the ancestors of this Node (recursively up)."""
        raise NotImplementedError

    @abc.abstractmethod
    def get_descendants[M: Entity = Entity](
        self,
        node: "Entity",
        type: NodeType | TraitType | type[M] | None = None,
    ) -> Sequence[M]:
        """
        Collect descendant Nodes (recursively down).
        If a type is specified, only Nodes of that type are collected.
        (Descendants are not collected unless all their ancestors are included).
        Nodes are BFS but IsOrdered is ignored.
        """
        raise NotImplementedError


class EntitySingletonGraph(Graph["Entity"]):
    """
    A Graph that contains only a single Entity.
    """

    __slots__ = ("node", "supergraph")

    def __init__(self, supergraph: "Supergraph", node: "Entity"):
        self.supergraph = supergraph
        self.node = node

    @property
    @override
    def nodes(self) -> Collection["Entity"]:
        return (self.node,)

    @override
    def __len__(self):
        return 1

    @override
    def get(self, id: UUID) -> Optional["Entity"]:
        return self.node if self.node.id == id else None

    @override
    def has(self, id: UUID) -> bool:
        return self.node.id == id

    @override
    def clear(self):
        raise ValueError(f"cannot clear {self!r}")

    @override
    def add(self, node: "Entity"):
        raise ValueError(f"cannot add {node!r} to {self!r}")

    @override
    def remove(self, node: "Entity"):
        raise ValueError(f"cannot remove {node!r} from {self!r}")

    @override
    def get_roots[M: "Entity" = "Entity"](
        self, node_type: NodeType | TraitType | type[M] | None = None
    ) -> Sequence[M]:
        return ()

    @override
    def get_leaves[M: "Entity" = "Entity"](
        self, node_type: NodeType | TraitType | type[M] | None = None, node: "Entity | None" = None
    ) -> Sequence[M]:
        return ()

    @override
    def get_children[M: "Entity" = "Entity"](
        self,
        node: "Entity",
        type: NodeType | TraitType | type[M] | None = None,
    ) -> Sequence[M]:
        return ()

    @override
    def get_ancestors[M: "Entity" = "Entity"](
        self, node: "Entity", type: NodeType | TraitType | type[M] | None = None
    ) -> Sequence[M]:
        return ()

    @override
    def get_descendants[M: "Entity" = "Entity"](
        self,
        node: "Entity",
        type: NodeType | TraitType | type[M] | None = None,
    ) -> Sequence[M]:
        return ()


class EntityGraph(Graph["Entity"]):
    """
    A Graph with an arbitrary, hierarchical set of Entities.
    """

    __slots__ = ("nodes_by_id", "nodes_by_parent", "supergraph")

    def __init__(self, supergraph: "Supergraph"):
        self.supergraph = supergraph
        self.nodes_by_id: dict[UUID, Entity] = {}
        self.nodes_by_parent: dict[UUID, dict[NodeType, list[Entity]]] = {}

    @property
    @override
    def nodes(self) -> Collection["Entity"]:
        return self.nodes_by_id.values()

    @override
    def __len__(self):
        return len(self.nodes_by_id)

    @override
    def get(self, id: UUID) -> Optional["Entity"]:
        return self.nodes_by_id.get(id)

    @override
    def has(self, id: UUID) -> bool:
        return id in self.nodes_by_id

    @override
    def clear(self):
        # supergraph
        for node in self.nodes:
            if self.supergraph._cached_nodes_by_id.get(node.id) is node:
                self.supergraph._cached_nodes_by_id.pop(node.id)
        # nodes
        self.nodes_by_id.clear()
        self.nodes_by_parent.clear()

    @override
    def add(self, node: "Entity"):
        if (existing := self.nodes_by_id.get(node.id)) is not None:
            raise ValueError(f"node {node!r} already in {self!r}: {existing!r}")
        # node
        self.nodes_by_id[node.id] = node
        # parent
        if (parent_ptr := node.parent_ptr) is not None:
            if parent_ptr.id not in self.nodes_by_parent:
                self.nodes_by_parent[parent_ptr.id] = {}
            child_node_type = node.metatype
            if child_node_type not in self.nodes_by_parent[parent_ptr.id]:
                self.nodes_by_parent[parent_ptr.id][child_node_type] = []
            self.nodes_by_parent[parent_ptr.id][child_node_type].append(node)
        # supergraph
        if (
            cached := self.supergraph._cached_nodes_by_id.get(node.id)
        ) is None or cached is _MISSING:
            self.supergraph._cached_nodes_by_id[node.id] = node

    @override
    def remove(self, node: "Entity"):
        # supergraph
        if self.supergraph._cached_nodes_by_id.get(node.id) is node:
            self.supergraph._cached_nodes_by_id.pop(node.id)
        # parent
        if (parent_ptr := node.parent_ptr) is not None:
            if parent_ptr.id not in self.nodes_by_parent:
                self.nodes_by_parent[parent_ptr.id] = {}
            child_node_type = node.metatype
            if child_node_type not in self.nodes_by_parent[parent_ptr.id]:
                self.nodes_by_parent[parent_ptr.id][child_node_type] = []
            self.nodes_by_parent[parent_ptr.id][child_node_type].remove(node)
            if not self.nodes_by_parent[parent_ptr.id][child_node_type]:
                self.nodes_by_parent[parent_ptr.id].pop(child_node_type)
                if not self.nodes_by_parent[parent_ptr.id]:
                    self.nodes_by_parent.pop(parent_ptr.id)
        # node
        self.nodes_by_id.pop(node.id)

    @override
    def get_roots[M: "Entity" = "Entity"](
        self, node_type: NodeType | TraitType | type[M] | None = None
    ) -> Sequence[M]:
        node_types = expand_node_types(node_type)
        if node_types is None:
            roots = tuple(
                node
                for node in self.nodes
                if node.parent_ptr is None or node.parent_ptr.id not in self.nodes_by_id
            )
        else:
            roots = tuple(
                node
                for node in self.nodes
                if (node.parent_ptr is None or node.parent_ptr.id not in self.nodes_by_id)
                and node.metatype in node_types
            )
        return roots  # type: ignore (must be right type)

    @override
    def get_leaves[M: "Entity" = "Entity"](
        self, node_type: NodeType | TraitType | type[M] | None = None, node: "Entity | None" = None
    ) -> Sequence[M]:
        if node is None:
            node_types = expand_node_types(node_type)
            if node_types is None:
                leaves = tuple(node for node in self.nodes if not self.nodes_by_parent.get(node.id))
            else:
                leaves = tuple(
                    node
                    for node in self.nodes
                    if not self.nodes_by_parent.get(node.id) and node.metatype in node_types
                )
        else:
            descendants = self.get_descendants(node, node_type)
            leaves = tuple(node for node in descendants if not self.nodes_by_parent.get(node.id))
        return leaves  # type: ignore (must be right type)

    @override
    def get_children[M: "Entity" = "Entity"](
        self,
        node: "Node",
        type: NodeType | TraitType | type[M] | None = None,
    ) -> Sequence[M]:
        # bail if no children
        if not self.nodes_by_parent:
            return ()
        children_by_type = self.nodes_by_parent.get(node.id)
        if not children_by_type:
            return ()

        if type is None:
            # collect children across all types
            children: list = []
            is_ordered = False
            for children_of_type in children_by_type.values():
                node_cls = type_(children_of_type[0])
                if TraitType.ORDERED in node_cls.__traits__:
                    is_ordered = True
                children.extend(children_of_type)
            if is_ordered:
                children.sort(key=lambda n: getattr(n, "order_key", INTEGER_ZERO))
            return children
        else:
            # turn into type
            node_types = expand_node_types(type, expand_inheritance=True)
            if not node_types:
                return ()
            node_cls = NODE_CLASS_BY_TYPE[node_types[0]]

            # collect
            if len(node_types) == 1:
                # collect for single node type
                children: list = children_by_type.get(node_types[0], EMPTY_LIST)
                if children and TraitType.ORDERED in node_cls.__traits__:
                    children.sort(key=lambda n: getattr(n, "order_key", INTEGER_ZERO))
                return children
            else:
                # collect for trait (multiple node types)
                children: list = []
                for node_type in node_types:
                    children.extend(children_by_type.get(node_type, EMPTY_LIST))
                if children and TraitType.ORDERED in node_cls.__traits__:
                    children.sort(key=lambda n: getattr(n, "order_key", INTEGER_ZERO))
                return children

    @override
    def get_ancestors[M: "Entity" = "Entity"](
        self, node: "Entity", type: NodeType | TraitType | type[M] | None = None
    ) -> Sequence[M]:
        ancestors: list[Entity] = []
        current = node.parent_ptr
        node_types = expand_node_types(type, expand_inheritance=True)

        # traverse up the parent chain
        while current is not None:
            parent_node = self.get(current.id)
            if parent_node is None:
                break
            if node_types is None or parent_node.metatype in node_types:
                ancestors.append(parent_node)
            current = parent_node.parent_ptr

        return ancestors  # type: ignore (must be right type)

    @override
    def get_descendants[M: "Entity" = "Entity"](
        self,
        node: "Entity",
        type: NodeType | TraitType | type[M] | None = None,
    ) -> Sequence[M]:
        if not self.nodes_by_parent:
            return ()

        queue: list[Entity] = [node]
        descendants: list[Entity] = []
        node_types = expand_node_types(type, expand_inheritance=True)

        # BFS
        while queue:
            current = queue.pop(0)
            children_by_type = self.nodes_by_parent.get(current.id)
            if not children_by_type:
                continue
            for children_of_type in children_by_type.values():
                queue.extend(children_of_type)

            if not node_types:
                for children_of_type in children_by_type.values():
                    descendants.extend(children_of_type)
            else:
                for node_t in node_types:
                    descendants.extend(children_by_type.get(node_t, ()))

        return descendants  # type: ignore (must be right type)


class EventGraph(Graph["Event"]):
    """A Graph with a flat set of Events."""

    def __init__(self, supergraph: "Supergraph"):
        self.supergraph = supergraph
        self.nodes_by_id: dict[UUID, Event] = {}

    @property
    @override
    def nodes(self) -> Collection["Event"]:
        return self.nodes_by_id.values()

    @override
    def __len__(self):
        return len(self.nodes_by_id)

    @override
    def get(self, id: UUID) -> Optional["Event"]:
        return self.nodes_by_id.get(id)

    @override
    def has(self, id: UUID) -> bool:
        return id in self.nodes_by_id

    @override
    def clear(self):
        self.nodes_by_id.clear()

    @override
    def add(self, node: "Event"):
        if (existing := self.nodes_by_id.get(node.id)) is not None:
            raise ValueError(f"node {node!r} already in {self!r}: {existing!r}")
        self.nodes_by_id[node.id] = node

    @override
    def remove(self, node: "Event"):
        self.nodes_by_id.pop(node.id)

    @override
    def get_roots[M: "Entity" = "Entity"](
        self, node_type: NodeType | TraitType | type[M] | None = None
    ) -> Sequence[M]:
        return ()

    @override
    def get_leaves[M: "Entity" = "Entity"](
        self, node_type: NodeType | TraitType | type[M] | None = None, node: "Entity | None" = None
    ) -> Sequence[M]:
        return ()

    @override
    def get_children[M: "Entity" = "Entity"](
        self,
        node: "Entity",
        type: NodeType | TraitType | type[M] | None = None,
    ) -> Sequence[M]:
        return ()

    @override
    def get_ancestors[M: "Entity" = "Entity"](
        self, node: "Entity", type: NodeType | TraitType | type[M] | None = None
    ) -> Sequence[M]:
        return ()

    @override
    def get_descendants[M: "Entity" = "Entity"](
        self,
        node: "Entity",
        type: NodeType | TraitType | type[M] | None = None,
    ) -> Sequence[M]:
        return ()


class NullGraph(Graph):
    """An always empty Graph."""

    def __init__(self, supergraph: "Supergraph"):
        self.supergraph = supergraph

    @property
    @override
    def nodes(self) -> Collection["Node"]:
        return ()

    @override
    def __len__(self):
        return 0

    @override
    def get(self, id: UUID) -> Optional["Node"]:
        return None

    @override
    def has(self, id: UUID) -> bool:
        return False

    @override
    def clear(self):
        pass

    @override
    def add(self, node: "Node"):
        pass

    @override
    def get_roots[N: "Entity" = "Entity"](
        self, node_type: NodeType | TraitType | type[N] | None = None
    ) -> Sequence[N]:
        return ()

    @override
    def remove(self, node: "Node"):
        pass

    @override
    def get_leaves[N: "Entity" = "Entity"](
        self, node_type: NodeType | TraitType | type[N] | None = None, node: "Node | None" = None
    ) -> Sequence[N]:
        return ()

    @override
    def get_children[N: "Entity" = "Entity"](
        self,
        node: "Node",
        type: NodeType | TraitType | type[N] | None = None,
    ) -> Sequence[N]:
        return ()

    @override
    def get_ancestors[N: "Entity" = "Entity"](
        self, node: "Node", type: NodeType | TraitType | type[N] | None = None
    ) -> Sequence[N]:
        return ()

    @override
    def get_descendants[N: "Entity" = "Entity"](
        self,
        node: "Node",
        type: NodeType | TraitType | type[N] | None = None,
    ) -> Sequence[N]:
        return ()


_MISSING = object()


class Supergraph:
    """
    A collection of Graphs in a Session.
    Instances of a Node may be in multiple Graphs (but any specific Node is only in one Graph).
    """

    __slots__ = ("_cached_nodes_by_id", "graphs", "session")

    def __init__(self, session: "Session"):
        self.session = session
        self.graphs: list[Graph[Any]] = []
        self._cached_nodes_by_id: dict[UUID, Node | object] = {}  # cache

    def __str__(self):
        return f"{len(self.graphs)} graphs"

    def __repr__(self):
        return f"<Supergraph {self}>"

    def create_entity_singleton_graph(self, node: "Entity") -> EntitySingletonGraph:
        """Create a new EntitySingletonGraph and add it to this Supergraph."""
        new_graph = EntitySingletonGraph(self, node)
        self.add_graph(new_graph)
        return new_graph

    def create_entity_graph(self) -> EntityGraph:
        """Create a new EntityGraph and add it to this Supergraph."""
        new_graph = EntityGraph(self)
        self.add_graph(new_graph)
        return new_graph

    def create_event_graph(self) -> EventGraph:
        """Create a new EventGraph and add it to this Supergraph."""
        new_graph = EventGraph(self)
        self.add_graph(new_graph)
        return new_graph

    def promote_to_polygraph(self, graph: EntitySingletonGraph) -> EntityGraph:
        """Promote an EntitySingletonGraph to an EntityGraph in one operation."""
        new_graph = self.create_entity_graph()
        new_graph.add(graph.node)
        self.graphs.remove(graph)
        self.graphs.append(new_graph)
        return new_graph

    def add_graph(self, graph: Graph[Any]):
        """Add a Graph to this Supergraph."""
        self.graphs.append(graph)
        for node in graph.nodes:
            if (cached := self._cached_nodes_by_id.get(node.id)) is None or cached is _MISSING:
                self._cached_nodes_by_id[node.id] = node

    def remove_graph(self, graph: Graph[Any]):
        """Remove a Graph from this Supergraph."""
        self.graphs.remove(graph)
        for node in graph.nodes:
            if self._cached_nodes_by_id.get(node.id) is node:
                self._cached_nodes_by_id.pop(node.id)

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


def expand_node_traits(types: Collection[NodeType | TraitType]) -> tuple[NodeType, ...]:
    """
    Expand a collection of NodeTypes and Traits into a flat collection of NodeTypes.
    """
    node_types: set[NodeType] = set()
    for typ in types:
        if isinstance(typ, NodeType):
            node_types.add(typ)
        elif isinstance(typ, TraitType):
            node_types.update(NODE_TYPES_BY_TRAIT_TYPE.get(typ, ()))
        else:
            assert_never(typ)
    return tuple(node_types)


def expand_node_inheritance(types: Collection[NodeType | TraitType]) -> tuple[NodeType, ...]:
    """
    Expand a collection of NodeTypes and Traits into a flat collection of NodeTypes.
    """
    node_types: set[NodeType] = set()
    for typ in types:
        if isinstance(typ, NodeType):
            node_cls = NODE_CLASS_BY_TYPE[typ]
            node_types.update(node_cls.__inherited_by__)
            if not node_cls.__is_abstract__:
                node_types.add(typ)
        elif isinstance(typ, TraitType):
            node_types.update(NODE_TYPES_BY_TRAIT_TYPE.get(typ, ()))
        else:
            assert_never(typ)
    return tuple(node_types)


def expand_node_types(
    node_type: "NodeType | TraitType | Collection[NodeType | TraitType] | type[Node] | None",
    expand_inheritance: bool = True,
) -> Sequence["NodeType"]:
    """Resolve the NodeTypes for a NodeType, TraitType, or Node class."""
    if node_type is None:
        return ()

    node_types: Sequence[NodeType] = []
    if isinstance(node_type, type):
        if node_t := NODE_TYPE_BY_CLASS.get(node_type):
            node_types.append(node_t)
        else:
            node_types.extend(NODE_TYPES_BY_TRAIT_TYPE[TRAIT_TYPE_BY_CLASS[node_type]])  # type: ignore
    elif isinstance(node_type, Collection):
        node_types = []
        for t in node_type:
            if isinstance(t, NodeType):
                node_types.append(t)
            else:
                node_types.extend(NODE_TYPES_BY_TRAIT_TYPE[t])
    else:
        if isinstance(node_type, NodeType):
            node_types.append(node_type)
        else:
            node_types.extend(NODE_TYPES_BY_TRAIT_TYPE[node_type])

    if expand_inheritance:
        node_types = expand_node_inheritance(node_types)

    return node_types
