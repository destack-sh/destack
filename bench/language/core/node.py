from datetime import datetime, timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Iterable,
    Optional,
    Self,
    Sequence,
    cast,
    dataclass_transform,
    override,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.pb2 import AnyNodeData, NodeReferenceData
from bench.utils.func import get_superclasses

from .const import (
    UNSET,
    NodeArea,
    NodeEdgeKind,
    NodeType,
    StructType,
    TraitType,
    active_session,
)
from .graph import Graph, attach_node
from .object import BuiltinObject, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    Property,
    _resolve_trait_type,
    property_,
    property_parent_,
    property_runtime_,
)
from .struct import Struct, struct_
from .trait import IndexIn, IsBased, IsBlockable, IsInBench, IsModal, IsSubject

if TYPE_CHECKING:
    from bench.language import (
        Aggregation,
        Condition,
        Expression,
        Join,
        NodeReference,
        Query,
        Session,
        Sort,
        Supergraph,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node_(
    node_type: NodeType | None,
    root_type: NodeType | None = NodeType.BENCH,
    index: tuple[IndexIn, ...] = (),
):
    """Register a class as a concrete node for the given node type."""

    # default index for nodes with parents
    if root_type:
        index = (*index, IndexIn(columns=("parent_id",), cover=("id",)))

    def decorate(cls: type["Node"]) -> type["Node"]:
        assert cls.__name__ == "Node" or issubclass(cls, Node), f"{cls.__name__} is not a Node"
        cls, _ = _process_object_cls(
            cls=cls,
            object_type=node_type,
            is_concrete=node_type is not None,
            is_node=True,
        )
        cls.__indexes__ = index

        if node_type is not None:
            cls.metatype = node_type
            # index
            NODE_CLASS_BY_TYPE[node_type] = cls
            traits = set()
            for superclass in get_superclasses(cls):
                if trait := _resolve_trait_type(superclass.__name__):
                    traits.add(trait)
            cls.__traits__ = tuple(traits)
            # area
            if TraitType.GLOBAL in traits:
                cls.__area__ = NodeArea.GLOBAL_POSTGRES
            elif TraitType.LOCAL in traits:
                cls.__area__ = NodeArea.LOCAL_POSTGRES
            else:
                cls.__area__ = NodeArea.REGIONAL_POSTGRES

        # parent/root
        parent_property = cls.__properties__.get("parent", None)
        assert parent_property is not None, f"missing parent property for {node_type}"
        cls.__parent_property__ = parent_property
        cls.__root_type__ = root_type

        return cls

    return decorate


@node_(node_type=None, root_type=None)
class Node[NodeDataT: AnyNodeData](BuiltinObject[NodeDataT]):
    """
    A Node with properties and an identity.
    Conceptually, all Nodes live together happily in a single giant supergraph.
    In practice, there are multiple stores and we load smaller subgraphs at runtime.
    """

    metatype: ClassVar[NodeType]  # type: ignore

    __is_node__: ClassVar[bool] = True
    __traits__: ClassVar[tuple[TraitType, ...]] = ()
    __id_factory__: ClassVar[Callable[[], UUID]] = UUID
    __area__: ClassVar[NodeArea]
    __indexes__: ClassVar[tuple[IndexIn, ...]] = ()

    __root_type__: ClassVar[NodeType | None] = None
    __parent_types__: ClassVar[tuple[NodeType, ...]] = ()
    __parent_property__: ClassVar[Property] = UNSET
    __child_types__: ClassVar[tuple[NodeType, ...]] = ()

    # 1-9: node identity
    # Node.metatype: 1
    id: UUID = property_(2, is_managed=True, can_write="system")
    # IsTemplatable.ck: 3
    parent: Optional["Node"] = property_parent_()  # type: ignore
    if TYPE_CHECKING:
        parent_type: NodeType | None = None
        parent_id: Optional[UUID] = None
        parent_ck: Optional[UUID] = None
        parent_ptr: Optional[NodeReference] = None
    # Node.area?
    # IsInBench.bench: 6
    # IsInPackage.package: 7

    # 10-29: node tracking
    created_at: datetime = property_(10, is_managed=True, can_write="system")
    created_by: Optional[IsSubject] = property_(  # type: ignore (pyright is wrong, Subject is a type)
        11,
        default=None,
        is_managed=True,
        node_bench_from="self",
        node_exclude=("ck", "base_id"),
        can_write="system",
    )
    updated_at: datetime = property_(12, is_managed=True, can_write="system")
    updated_by: Optional[IsSubject] = property_(  # type: ignore (see above)
        13,
        default=None,
        is_managed=True,
        node_bench_from="self",
        node_exclude=("ck", "base_id"),
        can_write="system",
    )
    if TYPE_CHECKING:
        created_by_id: Optional[UUID] = None
        created_by_type: NodeType | None = None
        created_by_ptr: Optional[NodeReference] = None
        updated_by_id: Optional[UUID] = None
        updated_by_type: NodeType | None = None
        updated_by_ptr: Optional[NodeReference] = None
    # IsArchivable.archived_at: 14
    # IsDeletable.deleted_at: 15
    # IsTemplatable.template: 16
    # IsOwnable.owned_by: 17
    # IsClaimable.claimed_by: 18
    # ...managed_by/controlled_by?
    # IsModal.mode: 20
    # IsExtensible.value: 21
    # IsOrdered.order_key: 22
    # IsRegional.region: 23

    # 30+ for general properties
    # ...

    _session: "Session" = property_runtime_()
    _hash: int = property_runtime_(default=None)
    _ref: "Optional[NodeReference]" = property_runtime_(default=None)
    _supergraph: "Supergraph" = property_runtime_()
    _graph: "Graph" = property_runtime_(default=None)
    _is_attached: bool = property_runtime_(default=False)  # :CachedAncestors
    _is_new: bool = property_runtime_(default=False)
    _dirty: int | None = property_runtime_(default=None)

    @property
    def ck(self):
        return self.id

    @property
    def is_attached(self) -> bool:
        """
        Whether this Node is attached to a root.
        TODO: generate & cache Node.is_attached/_is_attached :CachedAncestors
        """
        raise NotImplementedError
        if self.__root_type__ is None:
            return True  # always attached
        parent = self
        while parent is not None:
            if parent.metatype == self.__root_type__:
                return True
            parent = parent.parent
        return False

    def iter_descendants(self, recursive: bool = False) -> Iterable["Node"]:
        """Iterate over all descendants of this node."""
        for child_type in self.__child_types__:
            for child in self._graph.iter_descendants(self, child_type):
                yield child
                if recursive:
                    yield from child.iter_descendants(recursive=True)

    @override
    def clone(
        self,
        *,
        reset: bool = True,
        recursive: bool = True,
        detach: bool = False,
        _map: bool | dict[UUID, "Node"] = True,
        _ignore_definition: bool = False,
        _is_nested: bool = False,
        **kwargs,
    ) -> Self:
        from bench.language import Block

        # clone self
        copy_kwargs = self._clone_kwargs(reset=reset)
        if isinstance(self, IsModal):
            copy_kwargs["mode"] = self.mode  # keep mode
        copy_kwargs.update(kwargs)
        if detach and not reset:
            # put the node in a new graph to isolate (because same ids)
            copy_kwargs["_graph"] = Graph(
                scope=self._graph.scope,
                node_types=self._graph.node_types,
                supergraph=active_session().supergraph,
            )
        clone = self.__class__(**copy_kwargs, _is_new=True)

        # remember new identities
        if _map is True:
            _map = {self.id: clone}
        elif _map is not False:
            _map[self.id] = clone

        # clone blocks/definitions together
        if not _ignore_definition:
            clone_parent = clone.parent
            if clone_parent is None and not detach:
                clone_parent = self.parent
            if isinstance(self, Block):
                if isinstance(node := self.node, IsBlockable) and node.definition_id == self.id:
                    assert clone_parent is not None, f"cannot clone detached {self!r}"
                    cloned_node = node.clone(
                        reset=reset,
                        recursive=True,
                        detach=True,
                        _map=_map,
                        _is_nested=True,
                        _ignore_definition=True,
                    )
                    cast(Block, clone).node_ptr = cloned_node.to_ref()
                    cloned_node.definition_ptr = clone.to_ref()
            elif isinstance(self, IsBlockable) and (definition := self.definition) is not None:
                assert clone_parent is not None, f"cannot clone detached {self!r}"
                cloned_node = definition.clone(
                    reset=reset,
                    recursive=True,
                    detach=True,
                    _map=_map,
                    _is_nested=True,
                    _ignore_definition=True,
                )
                cast(IsBlockable, clone).definition_ptr = cloned_node.to_ref()
                cloned_node.node_ptr = clone.to_ref()

        # clone children and append to self (recursive)
        if recursive:
            for child_type in self.__child_types__:
                for child in self._graph.iter_descendants(self, child_type):
                    child_clone = child.clone(
                        reset=reset,
                        recursive=True,
                        detach=True,
                        _map=_map,
                        _is_nested=True,
                        _ignore_definition=True,  # we're the parent, so we clone both
                    )
                    attach_node(child_clone, clone, clone._graph, create=False)  # re-attach

        # map new identities (at root)
        if not _is_nested and type(_map) is dict:
            for node in _map.values():
                node.replace_references(_map, exclude=(NodeEdgeKind.NODE_PARENT,))

        # append to our parent to re-attach
        parent = self.parent
        if detach:
            clone.parent_ptr = None
        elif parent:
            parent.add_child(clone)
        return clone

    def __eq__(self, other: Any):
        """Equals the Node's identity."""
        return type(self) is type(other) and (self.id == other.id)

    def _stable_hash(self):
        """Hash the Node's identity."""
        return self._hash

    # only define __hash__ for nodes since their id is constant
    __hash__ = _stable_hash  # type: ignore

    def _do_set(self, key: str, value: Any):
        """Set a property on this Node."""
        raise NotImplementedError

    if not TYPE_CHECKING:
        __setattr__ = _do_set

    @property
    def path(self) -> str:
        raise NotImplementedError  # generated automatically

    def to_ref(self) -> "NodeReference":
        """Gets a reference to this node. May be rich in subclasses."""
        # TODO: cache Node._ref/Node._ref_data?
        raise NotImplementedError

    def _to_ref_data(self) -> "NodeReferenceData":
        """Gets a data reference to this node. May be rich in subclasses."""
        raise NotImplementedError

    def erase(self):
        """Wipe this Node from this cosmos forever."""
        self._session.erase(self)

    def move(self, to: "Node"):
        """Move this Node to a new parent."""
        to.add_child(self, move=True)

    def add_child[T: Node](self, child: T, move: bool = False) -> T:
        """Append a Node as a child of this Node."""
        raise NotImplementedError

    def add_children[T: Node](self, *children: T, move: bool = False) -> Sequence[T]:
        """Append multiple Nodes as children of this Node."""
        for child in children:
            self.add_child(child, move=move)
        return children

    def remove_child(self, child: "Node"):
        """Remove a child from this Node."""
        self._graph.remove(child)  # type: ignore
        child.parent_ptr = None

    def get_children[N: Node = Node](
        self, node_type: NodeType | type[N] | None = None
    ) -> Sequence[N]:
        """Gets the children of this Node."""
        if isinstance(node_type, type):
            node_type = node_type.metatype
        children = self._graph.get_descendants(self, node_type=node_type, recursive=False)
        return cast(Sequence[N], children)

    def get_child[N: Node = Node](self, node_type: NodeType | type[N], key: str) -> N | None:
        """Gets a specific child of this Node."""
        if isinstance(node_type, type):
            node_type = node_type.metatype
        for child in self._graph.get_descendants(self, node_type=node_type, recursive=False):
            if getattr(child, "name", None) == key:
                return cast(N, child)
        return None

    def child[N: Node = Node](self, node_type: NodeType | type[N], key: str) -> N:
        """Gets a specific child of this Node, or raises an error if not found."""
        child = self.get_child(node_type, key)
        if child is None:
            raise LookupError(f"no child {key} of {self!r}")
        return cast(N, child)

    def get_children_between[N: Node = Node](
        self, node_type: NodeType | type[N], after: N | None = None, before: N | None = None
    ) -> list[N]:
        """Get all nodes between two nodes (exclusive)."""
        found_after = after is None
        nodes = []
        for node in self.get_children(node_type):
            if after is not None and after == node:
                found_after = True
                continue
            if before is not None and before == node:
                break
            if found_after:
                nodes.append(node)
        return nodes

    def remove_children_between[N: Node = Node](
        self, node_type: NodeType | type[N], after: N | None = None, before: N | None = None
    ):
        """Removes all nodes between two nodes (exclusive)."""
        nodes_to_remove = self.get_children_between(node_type, after, before)
        for node in nodes_to_remove:
            self.remove_child(node)

    def get_descendants[N: Node = Node](
        self, node_type: NodeType | type[N] | None = None
    ) -> Sequence[N]:
        """Gets the descendants of this Node."""
        if isinstance(node_type, type):
            node_type = node_type.metatype
        descendants = self._graph.get_descendants(self, node_type=node_type, recursive=True)
        return cast(Sequence[N], descendants)

    def _move_to_graph(self, graph: Graph):
        """Moves this Node and its descendants to a new graph."""
        moved = self._graph.get_descendants(self, recursive=True)  # type: ignore
        moved = (self, *moved)
        for n in moved:
            # add/update in new graph
            if n.id in graph._nodes_by_id:
                graph.update(n)
            else:
                graph.add(n)
            # remove from old graph
            if n._graph is not graph and n.id in n._graph._nodes_by_id:
                n._graph.remove(n)
            n._graph = graph
        return moved

    def _detach_rec(self):
        """Removes this node from the graph / supergraph."""
        self._graph.remove(self)  # type: ignore ("depends on itself")

    async def wait_until(self, condition: Callable[[Self], bool], timeout: timedelta | None = None):
        """Wait until the given condition is true."""
        runtime = active_session().runtime
        await runtime.wait_for(nodes=[self], condition=lambda: condition(self), timeout=timeout)

    #
    # Querying
    #
    @classmethod
    def get(
        cls: type["Self"],
        name: str | None = None,
        join: Optional["Join"] = None,
        where: Optional["Condition"] = None,
        **subqueries: "Query",
    ) -> "Query[Self]":
        from .query import Query, QueryType, RelationReference, to_subqueries

        return Query(
            type=QueryType.GET,
            relation=RelationReference(node_type=cls.metatype),
            name=name or cls.metatype.bench_name,
            join=join,
            where=where,
            subqueries=to_subqueries(subqueries),
        )

    @classmethod
    def search(
        cls: type["Self"],
        name: str | None = None,
        join: Optional["Join"] = None,
        where: Optional["Condition"] = None,
        having: Optional["Condition"] = None,
        sort: Optional[list["Sort"]] = None,
        group_by: Optional[list["Expression"]] = None,
        aggregation: Optional["Aggregation"] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        count: bool = False,
        **subqueries: "Query",
    ) -> "Query[Self]":
        from .query import Query, QueryType, RelationReference, to_subqueries

        return Query(
            type=QueryType.SEARCH,
            relation=RelationReference(node_type=cls.metatype),
            name=name or cls.metatype.bench_name,
            join=join,
            where=where,
            having=having,
            group_by=group_by or [],
            aggregation=aggregation,
            sort=sort or [],
            limit=limit,
            offset=offset,
            count=count,
            subqueries=to_subqueries(subqueries),
        )

    @classmethod
    def aggregate(
        cls: type["Self"],
        name: str | None = None,
        join: Optional["Join"] = None,
        where: Optional["Condition"] = None,
        group_by: Optional[list["Expression"]] = None,
        aggregation: Optional["Aggregation"] = None,
        sort: Optional[list["Sort"]] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        count: bool = False,
    ) -> "Query[Self]":
        from .query import Query, QueryType, RelationReference

        return Query(
            type=QueryType.AGGREGATE,
            relation=RelationReference(node_type=cls.metatype),
            name=name or cls.metatype.bench_name,
            join=join,
            where=where,
            group_by=group_by or [],
            aggregation=aggregation,
            sort=sort or [],
            limit=limit,
            offset=offset,
            count=count,
        )


#
# Utility types
#


@struct_(StructType.NODE_REFERENCE, is_frozen=True)
class NodeReference(Struct[NodeReferenceData]):
    """
    A reference to a Node.
    """

    node_type: NodeType = property_(30, is_repr=True)
    id: UUID = property_(31, is_repr=True)
    ck: Optional[UUID] = property_(32, is_repr=True)
    bench_id: Optional[UUID] = property_(33, is_repr=True)
    base_id: Optional[UUID] = property_(34, is_repr=True)
    # area? external_id?

    @staticmethod
    def _ref_from_node(node: Node) -> "NodeReference":
        assert isinstance(node, Node), f"expected Node, got {node!r}"

        # bench
        bench_id: UUID | None = None
        if node.metatype == NodeType.BENCH:
            bench_id = node.id
        elif isinstance(node, IsInBench):
            bench_id = node.bench_id

        # base
        base_id: UUID | None = None
        if isinstance(node, IsBased):
            base_id = node.base_id

        reference = NodeReference(
            node_type=node.metatype,
            id=node.id,
            ck=node.ck,
            bench_id=bench_id,
            base_id=base_id,
        )
        return reference

    @staticmethod
    def _ref_data_from_node_data(node_data: AnyNodeData) -> "NodeReferenceData":
        raise NotImplementedError
