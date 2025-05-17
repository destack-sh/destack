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

from bench.language.registry import DESCENDANT_NODE_TYPES, NODE_CLASS_BY_TYPE

from .const import NodeType, QueryType, SortType, active_session

if TYPE_CHECKING:
    from bench.language import (
        BenchNode,
        Expression,
        Field,
        LegacyQuery,
        Node,
        NodeGraph,
        NodeReference,
        PackageNode,
        Property,
    )
    from bench.pb2 import AnyNodeData

FieldOrProperty = Union["Field", "Property"]
NodeTypeOrClass = Union[NodeType, type["Node"]]


def attach_node[N: "Node"](
    node: N, parent: "Node", graph: "NodeGraph", move: bool = False, create: bool = True
) -> N:
    """(Re)attaches a Node to a new parent."""

    old_parent = node.parent

    # check node
    if old_parent is not None and old_parent != parent:
        # move
        if not move:
            raise ValueError(f"cannot attach {node!r} to {parent!r}: attached to {node.parent!r}")

        # check package/bench
        if node.__is_in_package__:  # must be in same package
            # NOTE :Incomplete: support cross-package moves
            #  (would have to move descendants and update their .package_ptr?)
            pkg = cast("PackageNode", node).package
            assert cast("PackageNode", parent).package == pkg, f"cannot move {node!r} to {parent!r}"
        elif node.__is_in_bench__:  # must be in same bench
            bench = cast("BenchNode", node).bench
            assert cast("BenchNode", parent).bench == bench, f"cannot move {node!r} to {parent!r}"
    else:
        # create
        move = False  # not actually a move

    # check for circular ancestry
    seen: list[Node] = [node]
    n = parent
    while n is not None:
        if n in seen:
            raise ValueError(f"circular ancestry: {node!r} -> {n!r} -> {node!r}")
        seen.append(n)
        n = n.parent
    node.parent = parent

    # move node (and descendants) to this parent's graph
    if node._graph is not graph:
        from bench.language.connection import uncapture

        old_graph = node._graph
        if not graph.supergraph.has(node._graph.supergraph):
            raise ValueError(
                f"{node!r} not in same supergraph as {parent!r} ({node._graph.supergraph!r} != {graph.supergraph!r})"
            )
        moved = node._move_to_graph(graph)
        if len(old_graph) == 0:
            # clean up old graph
            if old_graph in graph.supergraph._graphs:
                graph.supergraph.remove_graph(old_graph)
            uncapture(old_graph)
    else:
        moved = (node,)  # already in the graph
        graph.update(node)

    # actually move/create in session
    if parent._session is not None:
        if move:
            assert old_parent is not None
            parent._session._move(node, old_parent=old_parent, new_parent=parent)
        elif parent.is_attached:
            # 'create' node in session if it's attached
            if create:
                for n in moved:
                    parent._session._create(n)
            parent._session._track_many(*moved)

    # move inline node and its definition together
    if move:
        from bench.language import Block, PageNode

        if isinstance(node, Block):
            if (
                isinstance(inner_node := node.node, PageNode)
                and inner_node.definition_id == node.id
                and inner_node.parent_id != parent.id
            ):
                attach_node(inner_node, parent, graph, move=True)
        elif isinstance(node, PageNode):
            if (block := node.definition) is not None and block.parent_id != parent.id:
                attach_node(block, parent, graph, move=True)

    return node


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

    def _get_child_graph(self, parent: "Node") -> "NodeGraph":
        """Get the graph for a child node (isolate if needed)."""
        if self._child_node_type in parent._graph.node_types:
            return parent._graph
        else:
            from bench.language.connection import capture

            from .graph import NodeGraph

            # make new graph for child node :IsolatedGraph
            graph = NodeGraph(
                scope=parent._graph.scope,
                node_types=(self._child_node_type, *DESCENDANT_NODE_TYPES[self._child_node_type]),
                supergraph=parent._supergraph,
            )
            graph.supergraph.add_graph(graph)
            capture(graph)
            return graph

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

