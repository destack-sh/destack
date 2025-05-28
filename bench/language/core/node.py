from datetime import datetime, timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Optional,
    Self,
    Sequence,
    cast,
    dataclass_transform,
    override,
)

import structlog
from bitarray import bitarray
from fastuuid import UUID
from opentelemetry import trace

from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.pb2 import AnyNodeData
from bench.utils.func import get_superclasses

from .const import (
    UNSET,
    NodeArea,
    NodeType,
    TraitType,
    active_session,
)
from .graph import Graph
from .object import BuiltinObject, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    Property,
    _resolve_trait_type,
    property_,
    property_parent_,
    property_runtime_,
)
from .trait import IndexIn, IsSubject

if TYPE_CHECKING:
    from bench.language import (
        Aggregation,
        AggregationType,
        Condition,
        Expression,
        ExpressionIn,
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


_object_set = object.__setattr__


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
    __area__: ClassVar[NodeArea]
    __indexes__: ClassVar[tuple[IndexIn, ...]] = ()

    __root_type__: ClassVar[NodeType | None] = None
    __parent_types__: ClassVar[tuple[NodeType, ...]] = ()
    __parent_property__: ClassVar[Property] = UNSET
    __child_types__: ClassVar[tuple[NodeType, ...]] = ()

    # 1-9: node identity
    # Node.metatype: 1
    id: UUID = property_(2, is_managed=True, is_eq=False, can_write="system")
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
    created_at: datetime = property_(10, is_managed=True, is_eq=False, can_write="system")
    created_by: Optional[IsSubject] = property_(  # type: ignore (pyright is wrong, Subject is a type)
        11,
        default=None,
        is_managed=True,
        is_eq=False,
        node_bench_from="self",
        node_exclude=("ck", "definition_id"),
        can_write="system",
    )
    updated_at: datetime = property_(12, is_managed=True, is_eq=False, can_write="system")
    updated_by: Optional[IsSubject] = property_(  # type: ignore (see above)
        13,
        default=None,
        is_managed=True,
        is_eq=False,
        node_bench_from="self",
        node_exclude=("ck", "definition_id"),
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
    _supergraph: "Supergraph" = property_runtime_()
    _graph: "Graph" = property_runtime_(default=None)
    _hash: int = property_runtime_(default=None)
    _ref: "Optional[NodeReference]" = property_runtime_(default=None)
    _is_new: bool = property_runtime_(default=False)
    _dirty: bitarray | None = property_runtime_(default=None)

    @property
    def ck(self):
        return self.id

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
        raise NotImplementedError  # generate

    def __eq__(self, other: Any):
        """Equals the Node's identity."""
        return type(self) is type(other) and (self.id == other.id)

    def hash(self):
        """Hash the Node's identity."""
        return self._hash

    __hash__ = hash  # type: ignore

    def _do_set(self, key: str, value: Any):
        """Set a property on this Node."""
        prop = self.__tracked_properties__.get(key)
        if prop is not None and not self._is_new:
            if self._dirty is None:
                self._dirty = bitarray(self.__max_property_ord__)
            self._dirty[prop.ord] = 1  # type: ignore
            self._session.dirty[self.id] = self
        _object_set(self, key, value)

    if not TYPE_CHECKING:
        __setattr__ = _do_set

    @property
    def path(self) -> str:
        raise NotImplementedError  # generated

    def __to_ref__(self) -> "NodeReference":
        """Gets a reference to this node. May be rich in subclasses."""
        raise NotImplementedError  # generated

    def to_ref(self) -> "NodeReference":
        """Gets a reference to this node. May be rich in subclasses."""
        if self._ref is None:
            self._ref = self.__to_ref__()
        return self._ref

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
        raise NotImplementedError

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
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["Join"] = None,
        **subqueries: "Query",
    ) -> "Query[Self]":
        from .query import Query, QueryType, relation_ref, to_subqueries

        return Query(
            type=QueryType.GET,
            relation=relation_ref(cls.metatype),
            name=name or cls.metatype.bench_name,
            join=join,
            where=where,
            subqueries=to_subqueries(subqueries),
        )

    @classmethod
    def search(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["Join"] = None,
        having: Optional["Condition"] = None,
        sort: Optional[list["Sort"]] = None,
        group_by: Optional[list["Expression"]] = None,
        aggregation: Optional["Aggregation"] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        count: bool = False,
        **subqueries: "Query",
    ) -> "Query[Self]":
        from .query import Query, QueryType, relation_ref, to_subqueries

        return Query(
            type=QueryType.SEARCH,
            relation=relation_ref(cls.metatype),
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
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["Join"] = None,
        group_by: Optional[list["Expression"]] = None,
        aggregation: Optional["Aggregation"] = None,
        sort: Optional[list["Sort"]] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        count: bool = False,
    ) -> "Query[Self]":
        from .query import Query, QueryType, relation_ref

        return Query(
            type=QueryType.AGGREGATE,
            relation=relation_ref(cls.metatype),
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

    @classmethod
    def count(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["Join"] = None,
        group_by: Optional[list["Expression"]] = None,
        sort: Optional[list["Sort"]] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        count: bool = False,
    ) -> "Query[Self]":
        from .query import Aggregation, AggregationType, Query, QueryType, relation_ref

        return Query(
            type=QueryType.AGGREGATE,
            relation=relation_ref(cls.metatype),
            name=name or cls.metatype.bench_name,
            join=join,
            where=where,
            group_by=group_by or [],
            aggregation=Aggregation(type=AggregationType.COUNT),
            sort=sort or [],
            limit=limit,
            offset=offset,
            count=count,
        )

    @classmethod
    def scalar(
        cls: type["Self"],
        type: "AggregationType",
        expression: "ExpressionIn",
        *,
        name: str | None = None,
        join: Optional["Join"] = None,
        where: Optional["Condition"] = None,
        group_by: Optional[list["Expression"]] = None,
        sort: Optional[list["Sort"]] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        count: bool = False,
    ) -> "Query[Self]":
        from .query import Aggregation, Query, QueryType, relation_ref
        from .query import expression as to_expression

        return Query(
            type=QueryType.AGGREGATE,
            relation=relation_ref(cls.metatype),
            name=name or cls.metatype.bench_name,
            join=join,
            where=where,
            group_by=group_by or [],
            aggregation=Aggregation(type=type, expression=to_expression(expression)),
            sort=sort or [],
            limit=limit,
            offset=offset,
            count=count,
        )
