import abc
from typing import (
    TYPE_CHECKING,
    Collection,
    Optional,
    Sequence,
    Union,
    cast,
    overload,
    override,
)

from bench.language.registry import NODE_CLASS_BY_TYPE

from .const import NodeType, QueryType, SortType, active_session

if TYPE_CHECKING:
    from bench.language import (
        BenchNode,
        Expression,
        Field,
        Graph,
        LegacyQuery,
        Node,
        NodeReference,
        Property,
    )
    from bench.pb2 import AnyNodeData

FieldOrProperty = Union["Field", "Property"]
NodeTypeOrClass = Union[NodeType, type["Node"]]


def attach_node[N: "Node"](
    node: N, parent: "Node", graph: "Graph", move: bool = False, create: bool = True
) -> N:
    """(Re)attaches a Node to a new parent."""

    raise NotImplementedError


class NodeList[V: Node](abc.ABC):
    """
    A list of Node 'children' for a parent's child property.
    The children may not be descendants of the parent (e.g. for Records in Tables).
    """

    __slots__ = (
        "_child_node_cls",
        "_child_node_type",
        "_node",
    )

    def __init__(self, node: "Node", child_node_type: NodeType):
        self._node = node
        self._child_node_type = child_node_type
        self._child_node_cls = cast(type[V], NODE_CLASS_BY_TYPE[self._child_node_type])

    def __repr__(self):
        return f"<{self.__class__.__name__} {self._node.absolute_path}: {self}>"

    def _get_parent(self) -> "Node":
        """Get the effective parent of a child node."""
        return self._node

    def _get_child_graph(self, parent: "Node") -> "Graph":
        """Get the graph for a child node (isolate if needed)."""
        raise NotImplementedError

    def create(self, **kwargs) -> V:
        """Creates a new node in the list."""
        parent = self._get_parent()
        graph = self._get_child_graph(parent)
        node = self._child_node_cls(**kwargs, parent=parent, _graph=graph)
        self.add_child(node)
        return node

    def add_child(self, node: V, move: bool = False) -> V:
        """Attaches a child node to a parent through a list. If move, may be re-attached."""
        parent = self._get_parent()
        graph = self._get_child_graph(parent)
        attach_node(node, parent=parent, graph=graph, move=move)
        return node

    def add_children(self, *nodes: V, move: bool = False):
        """Attaches a list of child nodes to a parent. See append."""
        parent = self._get_parent()
        graph = self._get_child_graph(parent)
        for node in nodes:
            attach_node(node, parent=parent, graph=graph, move=move)

    def remove(self, node: V):
        """Removes a child node from a parent. See append for reverse."""
        if self._node._session is not None:
            self._node._session._delete(node)
        self._node._graph.remove(node)
        node.parent = None

    @abc.abstractmethod
    def clear(self):
        """Removes all child nodes from a parent. See append for reverse."""
        raise NotImplementedError

    def set(self, nodes: Collection[V]):
        """Replaces all child nodes of a parent."""
        self.clear()
        self.add_children(*nodes)


class RemoteNodeList[V: Node, VD: AnyNodeData](NodeList[V]):
    """
    A NodeList backed by a Connection to a (remote) graph.
    """

    def __str__(self):
        return "<remote>"

    @override
    def _get_parent(self) -> "Node":
        """Get the effective parent of a child node."""
        if (
            len(self._child_node_cls.__parent_types__) > 0
            and self._child_node_cls.__parent_types__[0] == NodeType.BENCH
        ):
            bench = None
            if self._node.__is_in_bench__:
                bench = cast("BenchNode", self._node).bench
            if bench is None:
                bench = active_session().bench
            assert bench is not None, f"no bench for {self!r}"
            return bench
        else:
            return self._node

    def clear(self):
        raise RuntimeError(f"cannot clear {self!r}")

    @override
    def create(self, **kwargs) -> V:
        parent = self._get_parent()
        graph = self._get_child_graph(parent)
        node = self._child_node_cls(**kwargs, parent=parent)
        attach_node(node, parent=parent, graph=graph, move=False)
        return node

    @override
    def add_child(self, node: V, move: bool = False) -> V:
        parent = self._get_parent()
        graph = self._get_child_graph(parent)
        attach_node(node, parent=parent, graph=graph, move=move)
        return node

    @override
    def add_children(self, *nodes: V, move: bool = False) -> None:
        parent = self._get_parent()
        graph = self._get_child_graph(parent)
        for node in nodes:
            attach_node(node, parent=parent, graph=graph, move=move)

    #
    # Querying
    #

    def _query(self) -> "LegacyQuery[V, VD]":
        from bench.language import Expression, Field, LegacyQuery, SelectOptions, Table

        assert isinstance(self._node, Table), f"cannot query from: {self._node!r}"
        created_at = self._child_node_cls.get_property("created_at")
        query = LegacyQuery(
            type=QueryType.SEARCH,
            node_type=self._child_node_type,
            base_type=self._node,
            # sort by created_at by default (earliest first)
            sort=[Expression(type=SortType.ASCENDING, property=created_at)],
            # select all fields by default
            select=SelectOptions(select_fields=list(self._node.get_children(Field))),
        )
        return query

    def where(self, filter: Optional["Expression"] = None, **kwargs) -> "LegacyQuery[V, VD]":
        return self._query().where(filter, **kwargs)

    def order_by(
        self, sort: "Optional[Expression] | str | Field | Property" = None, *args: str
    ) -> "LegacyQuery[V, VD]":
        return self._query().order_by(sort, *args)

    def include(self, *properties: "Property") -> "LegacyQuery[V, VD]":
        return self._query().include(*properties)

    def select(self, *keys: FieldOrProperty) -> "LegacyQuery[V, VD]":
        return self._query().select(*keys)

    def select_all(self) -> "LegacyQuery[V, VD]":
        return self._query().select_all()

    def deselect(self, *properties: "Property") -> "LegacyQuery[V, VD]":
        return self._query().deselect(*properties)

    def include_ancestors(self, *node_types: NodeTypeOrClass) -> "LegacyQuery[V, VD]":
        return self._query().include_ancestors(*node_types)

    def include_descendants(self, *node_types: NodeTypeOrClass) -> "LegacyQuery[V, VD]":
        return self._query().include_descendants(*node_types)

    @overload
    async def get(
        self,
        filter: Optional["Expression | NodeReference | None"] = None,
        live: bool = False,
        **kwargs,
    ) -> V: ...
    @overload
    async def get(
        self, filter: Sequence["NodeReference"], live: bool = False, **kwargs
    ) -> list[V]: ...
    async def get(
        self,
        filter: Optional["Expression | NodeReference | Sequence[NodeReference] | None"] = None,
        live: bool = False,
        **kwargs,
    ) -> V | list[V]:
        return await self._query().get(filter, live=live, **kwargs)

    async def search(self, filter: Optional["Expression"] = None, **kwargs) -> list[V]:
        return await self._query().search(filter, **kwargs)

    def first(self, count: int) -> "LegacyQuery[V, VD]":
        return self._query().first(count)
