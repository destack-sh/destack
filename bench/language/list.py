import abc
from typing import (
    TYPE_CHECKING,
    Collection,
    Generic,
    Iterator,
    Optional,
    Sequence,
    TypeVar,
    Union,
    cast,
    overload,
    override,
)
from uuid import UUID

from more_itertools import first

from bench.language.const import NodeType, QueryType
from bench.language.graph import generate_node_name
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.language.validation import on_invalid_raise
from bench.utils.fractional import get_key_bounds, get_order_key

if TYPE_CHECKING:
    from bench.language import (
        CustomObject,
        Expression,
        Field,
        Node,
        NodeReference,
        Property,
        QueryBuilder,
        Struct,
    )
    from bench.proto.wire import AnyNodeData

FieldOrProperty = Union["Field", "Property"]
NodeTypeOrClass = Union[NodeType, type["Node"]]


class NodeList[V: Node](abc.ABC):
    """
    A list of node children for a parent's child property.
    """

    __slots__ = ("_child_node_cls", "_child_node_type", "_node", "_property")

    def __init__(self, node: "Node", property: "Property"):
        self._node = node
        self._property = property
        assert (
            property.reference_nodes
            and property.reference_nodes != "any"
            and len(property.reference_nodes) == 1
        ), f"cannot have many child types: {property!r}"
        self._child_node_type: NodeType = property.reference_nodes[0]
        self._child_node_cls = cast(type[V], NODE_CLASS_BY_TYPE[self._child_node_type])

    def __repr__(self):
        return (
            f"<{self.__class__.__name__} {self._node.absolute_path}.{self._property.name}: {self}>"
        )

    def create(self, **kwargs) -> V:
        """Creates a new node in the list."""
        node = self._child_node_cls(**kwargs, parent=self._node)
        self.append(node)
        return node

    def append(self, node: V) -> V:
        """Attaches a child node to a parent through a list."""
        if node.parent is not None:
            if node.parent is not self._node:
                raise ValueError(f"cannot attach {node!r} to {self!r}: attached to {node.parent!r}")
        else:
            node.parent = self._node

        # validate
        if self._node._session is not None:
            node._validate_self((), invalid=on_invalid_raise)

        # add node (and descendants) to this parent's graph
        new_graph = self._node._graph
        if node._graph is not new_graph:
            old_graph = node._graph
            assert new_graph.supergraph.has(
                node._graph.supergraph
            ), f"{node!r} not in same supergraph as {self!r} ({node._graph.supergraph!r} != {new_graph.supergraph!r})"
            added = node._graph.get_descendants(node, recursive=True)
            added = (node, *added)
            for n in added:
                new_graph.add(n)
                n._graph = new_graph
            new_graph.supergraph.remove_graph(old_graph)  # must be in same supergraph
        else:
            added = (node,)  # already in the graph

        # 'create' node in session if it's attached
        if self._node._session and self._node._is_attached:
            self._node._session._create(*added)
            self._node._session.track_many(*added)

        return node

    def extend(self, *nodes: V):
        """Attaches a list of child nodes to a parent. See append."""
        for node in nodes:
            self.append(node)

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
        self.extend(*nodes)


class LocalNodeList[V: Node](NodeList[V], Sequence[V]):
    """A NodeList backed by a local graph."""

    def __str__(self):
        return str(self.nodes)

    @override
    def create(self, **kwargs) -> V:
        if "name" in self._child_node_cls.__properties__ and "name" not in kwargs:
            # auto-generate name if required and not given :AutoNaming
            kwargs["name"] = generate_node_name(self._child_node_type, kwargs.get("type"), self)
        return super().create(**kwargs)

    @override
    def append(self, node: V, after: V | None = None, before: V | None = None) -> V:
        super().append(node)
        # assign order key to ordered nodes
        if hasattr(node, "order_key"):
            ok = get_order_key(*get_key_bounds(self.nodes, after, before))
            setattr(node, "order_key", ok)
        return node

    @override
    def extend(self, *nodes: V, after: V | None = None, before: V | None = None) -> None:  # type: ignore
        if not nodes:
            return
        elif after is not None:
            for node in nodes:
                self.append(node, after=after)
                after = node
        elif before is not None:
            self.append(nodes[0], before=before)
            after = nodes[0]
            for node in nodes[1:]:
                self.append(node, after=after)
                after = node
        else:
            for node in nodes:
                self.append(node)

    def clear(self):
        if not self.nodes:
            return
        removed = tuple(self.nodes)
        for n in removed:
            self.remove(n)

    #
    # Querying
    #

    @property
    def nodes(self) -> tuple[V, ...] | list[V]:
        """Access the computed nodes"""
        descendants = self._node._graph.get_descendants(
            node=self._node, node_type=self._child_node_type, recursive=False
        )
        if (
            len(descendants) > 1
            and "order_key" in NODE_CLASS_BY_TYPE[self._child_node_type].__properties__
        ):
            descendants.sort(key=lambda n: n.order_key)  # type: ignore
        return cast(list[V], descendants)

    def tolist(self) -> list[V]:
        return list(self)

    def get(self, key: UUID | str | int) -> V | None:
        if isinstance(key, UUID):
            return cast(V, self._node._graph.get(key))
        elif isinstance(key, str):
            return first(
                (n for n in self.nodes if n.code_name == key or getattr(n, "name", None) == key),
                None,
            )
        else:
            return self.nodes[key]

    @overload
    def __getitem__(self, item: str) -> Optional[V]: ...
    @overload
    def __getitem__(self, item: UUID) -> Optional[V]: ...
    @overload
    def __getitem__(self, item: int) -> V: ...
    @overload
    def __getitem__(self, item: slice) -> list[V]: ...
    def __getitem__(self, item: Union[str, UUID, int, slice]):  # type: ignore
        """Gets a node by index or name."""
        if isinstance(item, (str, UUID)):
            return self.get(item)
        else:
            return self.nodes[item]

    def __getattr__(self, item: str) -> V:
        """Gets a node by name."""
        node = self.get(item)
        if node is None:
            raise AttributeError(f"{self!r} has no node {item!r}")
        return node

    def __bool__(self):
        return len(self.nodes) > 0

    def __contains__(self, obj: object) -> bool:
        metatype = getattr(obj, "metatype", None)
        if not metatype or metatype not in self._property.reference_nodes:
            raise TypeError(f"{self!r} cannot contain {obj!r}")
        return obj in self.nodes

    def __iter__(self) -> Iterator[V]:
        yield from self.nodes

    def __len__(self) -> int:
        return len(self.nodes)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, LocalNodeList):
            return self.nodes == other.nodes
        elif isinstance(other, list):
            return self.nodes == other
        else:
            return False


class RemoteNodeList[V: Node, VD: AnyNodeData](NodeList[V]):
    """A NodeList backed by a connection to a (remote) graph."""

    def clear(self):
        raise RuntimeError(f"cannot clear {self!r}")

    @override
    def create(self, **kwargs) -> V:
        if "block" in self._child_node_cls.__properties__ and "block" not in kwargs:
            kwargs["block"] = self._node
        return super().create(**kwargs)

    @override
    def append(self, node: V) -> V:
        node = super().append(node)
        # automatically associate the node with the block
        if (
            "block" in self._child_node_cls.__properties__
            and getattr(node, "block") is not self._node
        ):
            node._do_set("block", self._node)
        return node

    #
    # Querying
    #

    def _query(self) -> "QueryBuilder[V, VD]":
        from bench.language import Block, QueryBuilder, SelectOptions

        assert isinstance(self._node, Block), f"can only query from a block: {self._node!r}"
        query = QueryBuilder(
            type=QueryType.SEARCH,
            node_type=self._child_node_type,
            block=self._node,
            # select all fields by default
            select=SelectOptions(select_fields=list(self._node.fields)),
        )
        return query

    def where(self, filter: Optional["Expression"] = None, **kwargs) -> "QueryBuilder[V, VD]":
        return self._query().where(filter, **kwargs)

    def order_by(
        self, sort: "Optional[Expression] | str | Field | Property" = None, *args: str
    ) -> "QueryBuilder[V, VD]":
        return self._query().order_by(sort, *args)

    def include(self, *properties: "Property") -> "QueryBuilder[V, VD]":
        return self._query().include(*properties)

    def select(self, *keys: FieldOrProperty) -> "QueryBuilder[V, VD]":
        return self._query().select(*keys)

    def select_all(self) -> "QueryBuilder[V, VD]":
        return self._query().select_all()

    def exclude(self, *properties: "Property") -> "QueryBuilder[V, VD]":
        return self._query().exclude(*properties)

    def include_ancestors(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[V, VD]":
        return self._query().include_ancestors(*node_types)

    def include_descendants(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[V, VD]":
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

    def first(self, count: int) -> "QueryBuilder[V, VD]":
        return self._query().first(count)

    async def count(self, filter: Optional["Expression"] = None, **kwargs) -> int:
        return await self._query().count(filter, **kwargs)

    async def exists(self, filter: Optional["Expression"] = None, **kwargs) -> bool:
        return await self._query().exists(filter, **kwargs)


ValueParentT = TypeVar("ValueParentT", bound=Union["CustomObject", "Struct", "Node"])
ValueT = TypeVar("ValueT", bound=Union["CustomObject", "Struct", "Property"])
ValueParentKey = Union["Property", "Field"]


class ValueList(list, Generic[ValueParentT]):
    """
    A list of Values or Value-like objects (with local identity, so can't be inlined).
    Unlike a NodeList, value lists are actual lists and not computed on access.
    NOTE :Cleanup: shouldn't ValueList be in value.py?
    """

    def __init__(
        self,
        parent: ValueParentT,
        parent_key: ValueParentKey,
        *args,
        **kwargs,
    ):  # type: ignore
        from bench.language.node import Property

        super().__init__(*args, **kwargs)
        self.parent = parent
        self.parent_key = parent_key
        self.is_property_reference = (
            isinstance(parent_key, Property) and parent_key.is_property_reference
        )

    def append(self, item: ValueT, after: ValueT | None = None, before: ValueT | None = None):  # type: ignore
        if not self.is_property_reference:
            item = item._move_to(self.parent, self.parent_key)  # type: ignore
        super().append(item)
        return item

    def extend(self, items: Collection[ValueT]):  # type: ignore
        super().extend(items)
        if not self.is_property_reference:
            values = cast(list[Union["CustomObject", "Struct"]], items)
            if any(item.parent is not None for item in values):
                values = [e._copy_to(self.parent, self.parent_key) for e in items]  # type: ignore
            else:
                for item in values:
                    item.parent = self.parent

    def clear(self):
        super().clear()

    @staticmethod
    def _move_list(
        values: Collection[ValueT],
        parent: ValueParentT,
        parent_key: ValueParentKey,
        ancestor_prop: Optional["Property"] = None,
    ):
        """Moves or copies the values in the list to the given parent."""
        if any(v.parent is not None for v in cast(list[Union["CustomObject", "Struct"]], values)):
            values = [v._copy_to(parent, parent_key) for v in values]  # type: ignore
        return ValueList(parent, parent_key, ancestor_prop, values)
