import abc
from collections.abc import Collection, Sequence
from typing import (
    TYPE_CHECKING,
    Optional,
    assert_never,
    final,
)

import structlog
from opentelemetry import trace

from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    NODE_TYPE_BY_CLASS,
    NODE_TYPES_BY_TRAIT_TYPE,
    TRAIT_TYPE_BY_CLASS,
)
from destack.utils.uuid import UUID

from ..builtin import NodeType, TraitType

if TYPE_CHECKING:
    from destack.language import Entity, Node

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)
type_ = type


class Graph[N: Node](abc.ABC):
    """
    A Graph is a collection of Entities.
    """

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
