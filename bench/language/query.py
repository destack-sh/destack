#
# Queries
#
import abc
from typing import (
    TypeVar,
    Union,
    Optional,
    TYPE_CHECKING,
    Any,
    Generic,
    Self,
    NamedTuple,
    Collection,
    cast,
    ClassVar,
)

from asgiref.sync import async_to_sync
from opensearchpy import AsyncOpenSearch
import psycopg

from bench.language.const import (
    NodeType,
    StructType,
    BenchError,
    AggregationOp,
    StoreEngineType,
    active_tx,
)
from bench.language.graph import NodeDataGraph
from bench.language.node import node, Node, p_parent, p_regular, NODE_CLASS_BY_TYPE
from bench.proto.wire import (
    AnyNodeData,
    BenchHostStub,
    SupervisorStub,
    NodeReferenceData,
    AggregationData,
    EditData,
)
from bench.utils.func import _auto_async_to_sync

if TYPE_CHECKING:
    from bench.language import Expression, Block, ReadOptions

NodeT = TypeVar("NodeT", bound=Node)
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
FieldOrProperty = Union["Field", "Property", Any]
NodeTypeOrClass = Union[NodeType, type[Node]]


@node(NodeType.QUERY)
class Query(Node):
    """A stored query."""

    parent: Union["Block"] = p_parent(4, NodeType.BLOCK)
    name: str | None = p_regular(30, default=None)
    order_key: str | None = p_regular(31, default=None)
    node_type: NodeType = p_regular(32)
    base: Optional["Block"] = p_regular(
        33, array=False, require=False, default=None, references=NodeType.BLOCK
    )
    filter: Optional["Expression"] = p_regular(34, default=None, struct=StructType.EXPRESSION)
    sort: Optional[list["Expression"]] = p_regular(
        35, default=None, array=True, struct=StructType.EXPRESSION
    )

    def __content_str__(self):
        return f"{self.node_type}[{self.filter}, {self.sort or '<default sort>'}]"

    @property
    def node_cls(self) -> type[Node]:
        return NODE_CLASS_BY_TYPE[self.node_type]

    # ... ReadQueryBase methods


class QueryError(BenchError, ValueError):
    def __init__(self, query: "QueryBase", cause: Exception | None = None):
        super().__init__(repr(query))
        self.query = query
        self.cause = cause


class NodeNotFoundError(QueryError):
    pass


class MultipleNodesFoundError(QueryError):
    pass


class StoreEngineError(BenchError, ValueError):
    def __init__(
        self,
        engine: Union["StoreEngine", StoreEngineType],
        query: "QueryBase" = None,
        expr: Union["Expression", list["Expression"]] = None,
        reason: str = None,
    ):
        super().__init__(
            f"{engine} cannot {query or expr or 'do this'}: {reason or '<unknown error>'}"
        )
        self.engine = engine
        self.query = query
        self.expr = expr
        self.reason = reason


class StoreEngineIncapableError(StoreEngineError):
    pass


class ReadQueryBase(abc.ABC, Generic[NodeT, NodeDataT]):
    """Read the nodes matching a query. :ReadQueryBase"""

    #
    # Builder
    #

    def filter(self, filter: "Expression" = None, **kwargs) -> "QueryBase[NodeT, NodeDataT]":
        raise NotImplementedError

    def sort(
        self, sort: Union[list[Union["Expression", str]], str, "Expression"] = None, *args: str
    ) -> "QueryBase[NodeT, NodeDataT]":
        raise NotImplementedError

    def first(self, count: int) -> "QueryBase[NodeT, NodeDataT]":
        """Returns the first N results."""
        raise NotImplementedError

    def skip(self, count: int) -> "QueryBase[NodeT, NodeDataT]":
        """Skips the first N results."""
        raise NotImplementedError

    def after(self, cursor: str) -> "QueryBase[NodeT, NodeDataT]":
        """Paginate using an opaque cursor."""
        raise NotImplementedError

    def include(self, *properties: FieldOrProperty) -> "QueryBase[NodeT, NodeDataT]":
        """Includes given default-excluded properties in the results."""
        raise NotImplementedError

    def exclude(self, *properties: FieldOrProperty) -> "QueryBase[NodeT, NodeDataT]":
        """Excludes given default-included properties from the results."""
        raise NotImplementedError

    def related(self, *properties: FieldOrProperty) -> "QueryBase[NodeT, NodeDataT]":
        """Joins the given related properties in the results."""
        raise NotImplementedError

    def ancestors(self, *node_types: NodeTypeOrClass) -> "QueryBase[NodeT, NodeDataT]":
        """Joins the given ancestors in the results."""
        raise NotImplementedError

    def descendants(self, *node_types: NodeTypeOrClass) -> "QueryBase[NodeT, NodeDataT]":
        """Joins the given descendants in the results."""
        raise NotImplementedError

    #
    # Fetch
    #

    async def __aiter__(self):
        raise NotImplementedError

    def tolist(self) -> list[NodeT]:
        raise NotImplementedError

    def __iter__(self):
        raise NotImplementedError

    def __len__(self):
        raise NotImplementedError

    def __getitem__(self, item: slice | int) -> Union[Self, NodeT]:
        raise NotImplementedError

    def get(self, filter: "Expression" = None, **kwargs) -> NodeT:
        raise NotImplementedError

    def exists(self, filter: "Expression" = None, **kwargs) -> bool:
        raise NotImplementedError

    def count(self, filter: "Expression" = None, **kwargs) -> int:
        raise NotImplementedError


class WriteQueryBase(abc.ABC, Generic[NodeT]):
    """Modify all matching nodes."""

    def update(self, **kwargs) -> None:
        """Update the properties of all matching nodes."""
        raise NotImplementedError

    def delete(self) -> None:
        """Removes and deletes all matching nodes from the query's parent."""
        raise NotImplementedError


class QueryBase(Generic[NodeT], ReadQueryBase[NodeT, AnyNodeData], WriteQueryBase[NodeT]):
    def __init__(
        self,
        node_type: NodeType,
        base: Optional["Block"] = None,
        filter: Optional["Expression"] = None,
        sort: list["Expression"] | None = None,
        first: int | None = None,
        skip: int | None = None,
        options: Optional["ReadOptions"] = None,
    ):
        from bench.language.node import NODE_CLASS_BY_TYPE, Node

        self._node_type = node_type
        self._node_cls = NODE_CLASS_BY_TYPE[node_type] if node_type else Node
        self._base = base
        self._filter = filter
        self._sort = sort
        self._first = first
        self._skip = skip
        self._options = options

    def __str__(self):
        args_strs = []
        if self._base:
            args_strs.append(self._base.absolute_path)
        for k in ("filter", "sort", "first", "skip"):
            v = getattr(self, f"_{k}", None)
            if k == "query":
                v = f"({v})" if v is not None else None
            if v is not None:
                args_strs.append(f"{k}={v}")
        if args_strs:
            args_str = ", ".join(args_strs)
        else:
            args_str = "[*]"
        return args_str

    def __repr__(self):
        return f"<{self._node_type.bench_name}Query {self}>"

    def query(self) -> Self:
        return self

    #
    # Builder
    #

    def copy(self):
        """Clones the query (the properties are immutable)."""
        return QueryBase(
            node_type=self._node_type,
            base=self._base,
            filter=self._filter,
            sort=self._sort,
            first=self._first,
            skip=self._skip,
            options=self._options,
            # cache is not copied on purpose as it shouldn't propagate
        )

    def _copy_options(self) -> "ReadOptions":
        from bench.language.access import ReadOptions

        if self._options is None:
            return ReadOptions()
        else:
            return self._options.copy()

    def filter(self, filter: "Expression" = None, **kwargs) -> "QueryBase[NodeT]":
        """Adds a filter clause to the query."""
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        copy = self.copy()
        copy._filter = filter & self._filter if self._filter is not None else filter
        return copy

    def sort(
        self, sort: Union[list[Union["Expression", str]], str, "Expression"] = None, *args: str
    ) -> "QueryBase[NodeT]":
        """Sorts the query results by the given sort criteria."""
        from bench.language.expression import coerce_sort

        copy = self.copy()
        sort = coerce_sort(self._node_cls, sort, *args)
        copy._sort = sort
        return copy

    def first(self, count: int) -> "QueryBase[NodeT]":
        """Returns the first N results."""
        copy = self.copy()
        copy._first = count
        return copy

    def skip(self, count: int) -> "QueryBase[NodeT]":
        """Skips the first N results."""
        copy = self.copy()
        copy._skip = count
        return copy

    def include(self, *properties: FieldOrProperty) -> "QueryBase":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.include_properties.extend(properties)
        return copy

    def exclude(self, *properties: FieldOrProperty) -> "QueryBase":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.exclude_properties.extend(*properties)
        return copy

    def related(self, *properties: FieldOrProperty) -> "QueryBase":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.related_properties.extend(*properties)
        return copy

    def ancestors(self, *node_types: NodeTypeOrClass) -> "QueryBase":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.ancestor_types = tuple(
            cast(type[Node], t).metatype if isinstance(t, type) else t for t in node_types
        )
        return copy

    def descendants(self, *node_types: NodeTypeOrClass) -> "QueryBase":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.descendant_types = tuple(
            cast(type[Node], t).metatype if isinstance(t, type) else t for t in node_types
        )
        return copy

    #
    # Fetch
    #

    def __getitem__(self, item: slice) -> Union["QueryBase[NodeT]", NodeT]:
        if isinstance(item, slice):
            if item.stop is None:
                return self.skip(item.start or 0)
            elif item.start is not None:
                return self.skip(item.start).first(item.stop - item.start)
            else:
                return self.first(item.stop)
        else:
            raise TypeError(f"expected slice into {self!r}, got {type(item)}: {item}")

    @_auto_async_to_sync
    async def tolist(self) -> list[NodeT]:
        return await self.fetch()

    async def __aiter__(self):
        return iter(await self.fetch())

    def __iter__(self):
        return iter(async_to_sync(self.fetch)())

    def __len__(self):
        return self.count()

    @_auto_async_to_sync
    async def get(self, filter: "Expression" = None, **kwargs) -> NodeT:
        """Returns the unique result matching the query (errors otherwise)."""
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        results = await self.filter(filter).tolist()
        if len(results) == 1:
            return results[0]
        else:
            combined_query = self.filter(filter)
            if len(results) == 0:
                raise NodeNotFoundError(combined_query)
            else:
                raise MultipleNodesFoundError(combined_query)

    async def fetch(self) -> list[NodeT] | tuple[NodeT, ...]:
        raise NotImplementedError("nocheckin: Query.fetch")

    @_auto_async_to_sync
    async def count(self, filter: "Expression" = None, **kwargs) -> int:
        """Returns the number of results. May refine the query."""
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs, return_none_if_empty=True)
        query = self.filter(filter)
        tx = active_tx()

        raise NotImplementedError("nocheckin: Query.count")

    @_auto_async_to_sync
    async def exists(self, filter: "Expression" = None, **kwargs) -> bool:
        """Whether any results exist. May refine the query."""
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs, return_none_if_empty=True)
        query = self.filter(filter)
        tx = active_tx()
        raise NotImplementedError("nocheckin: Query.exists")

    #
    # Write
    #

    def update(self, **kwargs) -> None:
        """Update the properties of all matching nodes."""
        raise NotImplementedError

    def delete(self) -> None:
        """Removes and deletes all matching nodes from the query's parent."""
        raise NotImplementedError


class FetchResult(NamedTuple):
    nodes: list[NodeDataT] | tuple[NodeDataT, ...]
    roots: list[NodeReferenceData] | tuple[NodeReferenceData, ...]
    cursors: list[str] | tuple[str, ...]
    start_cursor: str | None
    total: int | None = None


class AggregateResult(NamedTuple):
    aggregation: AggregationData


class StoreEngine(abc.ABC, Generic[NodeT, NodeDataT]):
    """A store engine backing specific types of queries for a session-like scope."""

    type: ClassVar[StoreEngineType]

    def __repr__(self):
        self_str = str(self)
        if self_str:
            return f"<{self.__class__.__name__} {self_str} ({self.type.bench_name})>"
        else:
            return f"<{self.__class__.__name__} ({self.type.bench_name})>"

    async def open(self):
        pass

    async def close(self):
        pass

    #
    # Read
    #

    async def fetch(self, query: "QueryBase[NodeT, NodeDataT]", count: bool) -> FetchResult:
        raise StoreEngineIncapableError(self, query, reason="fetch unsupported")

    async def aggregate(
        self, query: "QueryBase[NodeT, NodeDataT]", aggregation: "Expression"
    ) -> AggregateResult:
        raise StoreEngineIncapableError(self, query, reason="exists unsupported")

    #
    # Write
    #

    async def update(self, query: "QueryBase[NodeT, NodeDataT]", **kwargs) -> list[EditData]:
        raise StoreEngineIncapableError(self, query, reason="update unsupported")

    async def delete(self, query: "QueryBase[NodeT, NodeDataT]") -> list[EditData]:
        raise StoreEngineIncapableError(self, query, reason="delete unsupported")

    #
    # Transactions
    #

    async def flush(self, edits: Collection[EditData]) -> None:
        raise StoreEngineIncapableError(self, reason="flush unsupported")

    async def commit(self, edits: Collection[EditData]) -> None:
        raise StoreEngineIncapableError(self, reason="commit unsupported")

    async def rollback(self, edits: Collection[EditData]) -> None:
        raise StoreEngineIncapableError(self, reason="rollback unsupported")


class RemoteStoreEngine(StoreEngine[NodeT, NodeDataT]):
    type = StoreEngineType.REMOTE

    def __init__(self, remote: Union["BenchHostStub", "SupervisorStub"]):
        self._remote = remote

    def __str__(self):
        return f"remote={self._remote}"

    async def fetch(self, query: "QueryBase[NodeT, NodeDataT]", count: bool) -> FetchResult:
        from bench.proto import wire, wiring

        request = wire.SearchNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_struct_maybe(query._filter),
            sort=[wiring.pack_struct(s) for s in query._sort] if query._sort else None,
            limit=query._first,
            options=wiring.pack_struct_maybe(query._options),
            count=count,
        )
        response = await self._remote.search_nodes(request)
        return FetchResult(
            nodes=[wiring.unwrap_some_node(n) for n in response.nodes],
            roots=response.rootsy,
            cursors=response.cursors,
            start_cursor=response.start_cursor,
        )

    async def aggregate(
        self, query: "QueryBase[NodeT, NodeDataT]", aggregation: "Expression"
    ) -> AggregateResult:
        from bench.proto import wire, wiring

        request = wire.AggregateNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_struct_maybe(query._filter),
            limit=query._first,
            aggregation=aggregation._to_data(),
        )
        response = await self._remote.aggregate_nodes(request)
        return AggregateResult(response.aggregation)


class InMemoryStoreEngine(StoreEngine):
    type = StoreEngineType.INMEMORY

    def __init__(self, graph: NodeDataGraph):
        self._graph = graph

    def __str__(self):
        return f"graph={self._graph}"


class PostgresStoreEngine(StoreEngine):
    type = StoreEngineType.POSTGRES

    def __init__(self, connection: psycopg.AsyncConnection):
        self._conn = connection
        self._cur: psycopg.AsyncCursor | None = None

    def __str__(self):
        return f"connection={self._conn}"

    async def open(self):
        self._cur = self._conn.cursor()

    async def close(self):
        await self._cur.close()

    async def fetch(self, query: "QueryBase[NodeT, NodeDataT]", count: bool) -> FetchResult:
        from bench.sql.engine import pg_search_nodes_data_graph, pg_count, compile_pg_conditional
        from bench.language import NodeReference

        roots, graph = await pg_search_nodes_data_graph(
            cur=self._cur,
            node_type=query._node_type,
            options=query._options,
            filter=query._filter,
            sort=query._sort,
            first=query._first,
            skip=query._skip,
        )
        if count:
            total = await pg_count(
                cur=self._cur,
                table=query._node_cls.__table__,
                where=compile_pg_conditional(query._node_cls, query._filter),
            )
        else:
            total = None
        return FetchResult(
            roots=[NodeReference.from_node_data(r) for r in roots.nodes],
            nodes=graph.nodes,
            cursors=roots.cursors,
            start_cursor=roots.start_cursor,
            total=total,
        )

    async def aggregate(
        self, query: "QueryBase[NodeT, NodeDataT]", aggregation: "Expression"
    ) -> AggregateResult:
        from bench.sql.engine import compile_pg_conditional, pg_exists, pg_count

        if aggregation.op == AggregationOp.EXISTS:
            filter = compile_pg_conditional(query._node_cls, query._filter)
            exists = await pg_exists(self._cur, query._node_cls.__table__, filter)
            return AggregateResult(AggregationData(exists=exists))
        elif aggregation.op == AggregationOp.COUNT:
            filter = compile_pg_conditional(query._node_cls, query._filter)
            count = await pg_count(self._cur, query._node_cls.__table__, filter)
            return AggregateResult(AggregationData(count=count))
        else:
            raise StoreEngineIncapableError(self, query, expr=aggregation, reason="unsupported")


class OpensearchStoreEngine(StoreEngine):
    type = StoreEngineType.OPENSEARCH

    def __init__(self, client: AsyncOpenSearch):
        self._client = client

    def __str__(self):
        return f"client={self._client}"
