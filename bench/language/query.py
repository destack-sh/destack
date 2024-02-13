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
    Callable,
)
from uuid import UUID

from asgiref.sync import async_to_sync
import psycopg

from bench.language.const import (
    NodeType,
    StructType,
    BenchError,
    AggregationOp,
    StoreEngineType,
    active_tx,
    ExpressionKind,
)
from bench.language.graph import NodeDataGraph, NodeGraph
from bench.language.node import node, Node, p_parent, p_regular, NODE_CLASS_BY_TYPE
from bench.proto.wire import (
    AnyNodeData,
    NodeReferenceData,
    AggregationData,
    EditData,
    GraphIoStub,
    GraphScope,
)
from bench.utils.func import _auto_async_to_sync, bytetuple

if TYPE_CHECKING:
    from bench.language import Expression, Block, ReadOptions, Session, Store
    from bench.sql.client import _PgStoreConnection

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

    def query(self) -> "QueryBuilder":
        return QueryBuilder(
            node_type=self.node_type,
            base=self.base,
            filter=self.filter,
            sort=self.sort,
            first=None,
            skip=None,
            aggregation=None,
            options=None,
        )

    # ... ReadQueryBase methods


class QueryError(BenchError, ValueError):
    def __init__(self, query: "QueryBuilder", cause: Exception | None = None):
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
        engine: Union["StoreEngine", "StoreConnection", StoreEngineType],
        query: "QueryBuilder" = None,
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


class MakeQueryBase(abc.ABC, Generic[NodeT, NodeDataT]):
    """Build or modify a query. :ReadQueryBase"""

    def filter(self, filter: "Expression" = None, **kwargs) -> "QueryBuilder[NodeT, NodeDataT]":
        raise NotImplementedError

    def sort(
        self, sort: Union[list[Union["Expression", str]], str, "Expression"] = None, *args: str
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        raise NotImplementedError

    def first(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Returns the first N results."""
        raise NotImplementedError

    def skip(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Skips the first N results."""
        raise NotImplementedError

    def after(self, cursor: str) -> "QueryBuilder[NodeT, NodeDataT]":
        """Paginate using an opaque cursor."""
        raise NotImplementedError

    def include(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Includes given default-excluded properties in the results."""
        raise NotImplementedError

    def exclude(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Excludes given default-included properties from the results."""
        raise NotImplementedError

    def related(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Joins the given related properties in the results."""
        raise NotImplementedError

    def ancestors(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        """Joins the given ancestors in the results."""
        raise NotImplementedError

    def descendants(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        """Joins the given descendants in the results."""
        raise NotImplementedError


class ReadQueryBase(abc.ABC, Generic[NodeT, NodeDataT]):
    """Fetch the nodes matching a query. :ReadQueryBase"""

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


class WriteQueryBase(abc.ABC, Generic[NodeT, NodeDataT]):
    """Modify all matching nodes."""

    def update(self, **kwargs) -> None:
        """Update the properties of all matching nodes."""
        raise NotImplementedError

    def delete(self) -> None:
        """Removes and deletes all matching nodes from the query's parent."""
        raise NotImplementedError


class QueryBuilder(
    Generic[NodeT, NodeDataT],
    MakeQueryBase[NodeT, NodeDataT],
    ReadQueryBase[NodeT, NodeDataT],
    WriteQueryBase[NodeT, NodeDataT],
):
    def __init__(
        self,
        node_type: NodeType,
        base: Optional["Block"] = None,
        filter: Optional["Expression"] = None,
        sort: list["Expression"] | None = None,
        first: int | None = None,
        skip: int | None = None,
        after: str | None = None,
        aggregation: Optional["Expression"] = None,
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
        self._after = after
        self._aggregation = aggregation
        self._options = options

    def __str__(self):
        args_strs = []
        if self._base:
            args_strs.append(self._base.absolute_path)
        for k in ("filter", "sort", "first", "skip", "aggregation"):
            v = getattr(self, f"_{k}", None)
            if k == "query":
                v = f"({v})" if v is not None else None
            if v is not None:
                args_strs.append(f"{k}={v}")
        if self._aggregation:
            query_type = self._aggregation.op.bench_name
        else:
            query_type = "Fetch"
        if args_strs:
            args_str = f"{query_type} {', '.join(args_strs)}"
        else:
            args_str = f"{query_type} [*]"
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
        return QueryBuilder(
            node_type=self._node_type,
            base=self._base,
            filter=self._filter,
            sort=self._sort,
            first=self._first,
            skip=self._skip,
            aggregation=self._aggregation,
            options=self._options,
            # cache is not copied on purpose as it shouldn't propagate
        )

    def _copy_options(self) -> "ReadOptions":
        from bench.language.access import ReadOptions

        if self._options is None:
            return ReadOptions()
        else:
            return self._options.copy()

    def filter(self, filter: "Expression" = None, **kwargs) -> "QueryBuilder[NodeT, NodeDataT]":
        """Adds a filter clause to the query."""
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        copy = self.copy()
        copy._filter = filter & self._filter if self._filter is not None else filter
        return copy

    def sort(
        self, sort: Union[list[Union["Expression", str]], str, "Expression"] = None, *args: str
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        """Sorts the query results by the given sort criteria."""
        from bench.language.expression import coerce_sort

        copy = self.copy()
        sort = coerce_sort(self._node_cls, sort, *args)
        copy._sort = sort
        return copy

    def first(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Returns the first N results."""
        copy = self.copy()
        copy._first = count
        return copy

    def skip(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Skips the first N results."""
        copy = self.copy()
        copy._skip = count
        return copy

    def after(self, cursor: str) -> "QueryBuilder":
        copy = self.copy()
        copy._after = cursor
        return copy

    def aggregate(self, aggregation: "Expression") -> "QueryBuilder":
        if aggregation.kind != ExpressionKind.AGGREGATION:
            raise ValueError(f"expected aggregation expression, got {aggregation!r}")
        copy = self.copy()
        copy._aggregation = aggregation
        return copy

    def include(self, *properties: FieldOrProperty) -> "QueryBuilder":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.include_properties.extend(properties)
        return copy

    def exclude(self, *properties: FieldOrProperty) -> "QueryBuilder":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.exclude_properties.extend(*properties)
        return copy

    def related(self, *properties: FieldOrProperty) -> "QueryBuilder":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.related_properties.extend(*properties)
        return copy

    def ancestors(self, *node_types: NodeTypeOrClass) -> "QueryBuilder":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.ancestor_types = tuple(
            cast(type[Node], t).metatype if isinstance(t, type) else t for t in node_types
        )
        return copy

    def descendants(self, *node_types: NodeTypeOrClass) -> "QueryBuilder":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.descendant_types = tuple(
            cast(type[Node], t).metatype if isinstance(t, type) else t for t in node_types
        )
        return copy

    #
    # Fetch
    #

    def __getitem__(self, item: slice) -> Union["QueryBuilder[NodeT, NodeDataT]", NodeT]:
        if isinstance(item, slice):
            if item.stop is None:
                return self.skip(item.start or 0)
            elif item.start is not None:
                return self.skip(item.start).first(item.stop - item.start)
            else:
                return self.first(item.stop)
        else:
            raise TypeError(f"expected slice into {self!r}, got {type(item)}: {item}")

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
        results = await self.filter(filter).fetch()
        if len(results) == 1:
            return results[0]
        else:
            combined_query = self.filter(filter)
            if len(results) == 0:
                raise NodeNotFoundError(combined_query)
            else:
                raise MultipleNodesFoundError(combined_query)

    @_auto_async_to_sync
    async def fetch(self) -> list[NodeT] | tuple[NodeT, ...]:
        from bench.proto.wiring import unpack_nodes_inline

        tx = active_tx()
        connection = await tx.connect_store_to(base=self._base, node_type=self._node_type)
        result = await connection.fetch(self, FetchOptions())
        source_graph = NodeDataGraph(result.nodes)
        roots = unpack_nodes_inline(
            source_graph, parent=self._base, session=tx.session, roots=result.roots
        )
        return roots

    tolist = fetch

    @_auto_async_to_sync
    async def count(self, filter: "Expression" = None, **kwargs) -> int:
        """Returns the number of results. May refine the query."""
        from bench.language.expression import coerce_conditional, A

        filter = coerce_conditional(self._node_cls, filter, kwargs, return_none_if_empty=True)
        query = self.filter(filter) if filter is not None else self
        query = query.aggregate(A(AggregationOp.COUNT))
        connection = await active_tx().connect_store_to(
            base=query._base, node_type=query._node_type
        )
        result = await connection.aggregate(query)
        return result.aggregation.count

    @_auto_async_to_sync
    async def exists(self, filter: "Expression" = None, **kwargs) -> bool:
        """Whether any results exist. May refine the query."""
        from bench.language.expression import coerce_conditional, A

        filter = coerce_conditional(self._node_cls, filter, kwargs, return_none_if_empty=True)
        query = self.filter(filter) if filter is not None else self
        query = query.aggregate(A(AggregationOp.EXISTS))
        connection = await active_tx().connect_store_to(
            base=query._base, node_type=query._node_type
        )
        result = await connection.aggregate(query)
        return result.aggregation.exists

    #
    # Write
    #

    @_auto_async_to_sync
    async def update(self, **kwargs) -> None:
        """Update the properties of all matching nodes."""
        connection = await active_tx().connect_store_to(base=self._base, node_type=self._node_type)
        await connection.update(self, **kwargs)

    @_auto_async_to_sync
    async def delete(self) -> None:
        """Removes and deletes all matching nodes from the query's parent."""
        connection = await active_tx().connect_store_to(base=self._base, node_type=self._node_type)
        await connection.delete(self)


class FetchOptions(NamedTuple):
    count: bool = False
    lock_for_update: bool = False
    skip_locked: bool = False


class FetchResult(NamedTuple):
    nodes: list[NodeDataT] | tuple[NodeDataT, ...]
    roots: list[NodeReferenceData] | tuple[NodeReferenceData, ...]
    cursors: list[str] | tuple[str, ...]
    start_cursor: str | None
    total: int | None = None


class AggregateResult(NamedTuple):
    aggregation: AggregationData


QueryPredicate = Callable[[NodeType], bool]


class StoreEngine(abc.ABC, Generic[NodeT, NodeDataT]):
    """A store engine providing connections to operate on that backend with certain queries."""

    type: ClassVar[StoreEngineType]

    def __init__(self, store: "Store", scope: GraphScope | None, node_types: bytetuple[NodeType]):
        if store.engine != self.type:
            raise ValueError(f"store {store!r} has engine {store.engine}, not {self.type}")
        self.store = store
        self.node_types = node_types
        self.scope = scope

    def __repr__(self):
        self_str = str(self)
        if self_str:
            return f"<{self.__class__.__name__} {self_str} ({self.type.bench_name})>"
        else:
            return f"<{self.__class__.__name__} ({self.type.bench_name})>"

    @property
    def id(self) -> int | str | UUID:
        return hash(self)

    def supports(self, scope: GraphScope, node_type: NodeType) -> bool:
        if scope.bench_id is not None and (
            self.scope is None or self.scope.bench_id != scope.bench_id
        ):
            return False
        if node_type not in self.node_types:
            return False
        return True

    async def connect(self, session: "Session") -> "StoreConnection[NodeT, NodeDataT]":
        """Opens the store engine for a session."""
        raise NotImplementedError


StoreEngineT = TypeVar("StoreEngineT", bound=StoreEngine)


class StoreConnection(abc.ABC, Generic[StoreEngineT, NodeT, NodeDataT]):
    def __init__(self, engine: "StoreEngineT[NodeT, NodeDataT]", session: "Session"):
        self.engine = engine
        self.session = session

    @property
    def type(self) -> StoreEngineType:
        return self.engine.type

    def __str__(self):
        return f"session={self.session}"

    def __repr__(self):
        self_str = str(self)
        if self_str:
            return f"<{self.__class__.__name__} {self}>"
        else:
            return f"<{self.__class__.__name__}>"

    #
    # Read
    #

    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        """Read the nodes given the fetch query in the current transaction context (if any)."""
        raise StoreEngineIncapableError(self, query, reason="fetch unsupported")

    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        """Read the nodes given the aggregate query in the current transaction context (if any)."""
        raise StoreEngineIncapableError(self, query, reason="exists unsupported")

    #
    # Write
    #

    async def delete(self, query: "QueryBuilder[NodeT, NodeDataT]") -> list[EditData]:
        """
        Removes and deletes all matching nodes from the query's parent.
        Begins a transaction context if not already in one.
        """
        raise StoreEngineIncapableError(self, query, reason="delete unsupported")

    async def update(self, query: "QueryBuilder[NodeT, NodeDataT]", **kwargs) -> list[EditData]:
        """
        Update the properties of all matching nodes (in transaction, if any).
        Begins a transaction context if not already in one.
        """
        raise StoreEngineIncapableError(self, query, reason="update unsupported")

    #
    # Transaction management
    # The methods closely mirror :GraphIO service methods for universal 2PCs.
    #

    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> None:
        """Flushes edits in the current transaction context. If not in a transaction, begins one."""
        raise StoreEngineIncapableError(self, reason="flush unsupported")

    async def complete(self) -> None:
        """Completes the current transaction context. No further operations are allowed."""
        raise StoreEngineIncapableError(self, reason="complete unsupported")

    async def cancel(self) -> None:
        """Cancels the current transaction context. No further operations are allowed."""
        raise StoreEngineIncapableError(self, reason="cancel unsupported")

    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> None:
        """Commits the flushed pending and given edits in the current transaction context."""
        raise StoreEngineIncapableError(self, reason="commit unsupported")

    async def close(self):
        """Closes this connection to all further operations."""
        pass


class RemoteEngine(StoreEngine[NodeT, NodeDataT]):
    type = StoreEngineType.REMOTE

    def __init__(
        self,
        store: "Store",
        scope: GraphScope,
        node_types: bytetuple[NodeType],
        remote: GraphIoStub,
    ):
        super().__init__(store, node_types)
        self._remote = remote

    def __str__(self):
        return f"remote={self._remote}, store={self.store}"


class RemoteConnection(StoreConnection[RemoteEngine, NodeT, NodeDataT]):
    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        from bench.proto import wire, wiring

        request = wire.SearchNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_struct_maybe(query._filter),
            sort=[wiring.pack_struct(s) for s in query._sort] if query._sort else None,
            limit=query._first,
            options=wiring.pack_struct_maybe(query._options),
            count=options.count,
        )
        response = await self.engine._remote.search_nodes(request)
        return FetchResult(
            nodes=[wiring.unwrap_some_node(n) for n in response.nodes],
            roots=response.rootsy,
            cursors=response.cursors,
            start_cursor=response.start_cursor,
        )

    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        from bench.proto import wire, wiring

        request = wire.AggregateNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_struct_maybe(query._filter),
            limit=query._first,
            aggregation=query._aggregation._to_data(),
        )
        response = await self.engine._remote.aggregate_nodes(request)
        return AggregateResult(response.aggregation)


class InMemoryGraphEngine(StoreEngine):
    type = StoreEngineType.INMEMORY

    def __init__(
        self,
        store: "Store",
        scope: GraphScope,
        node_types: bytetuple[NodeType],
        node_graph: Optional[NodeGraph],
        source_graph: NodeDataGraph,
    ):
        super().__init__(store, scope, node_types)
        self.node_graph = node_graph
        self.source_graph = source_graph

    def __str__(self):
        return f"node_graph={self.node_graph!r}, source_graph={self.source_graph!r}, store={self.store}"


class PostgresEngine(StoreEngine):
    type = StoreEngineType.POSTGRES

    def __str__(self):
        return f"store={self.store!r}"

    async def connect(self, session: "Session") -> "PostgresConnection":
        from bench.sql.client import get_pg_store_connection

        conn = await get_pg_store_connection(self.store)
        cur = await conn.open()
        return PostgresConnection(self, session, conn, cur)


class PostgresConnection(StoreConnection[PostgresEngine, NodeT, NodeDataT]):
    def __init__(
        self,
        engine: "PostgresEngine",
        session: "Session",
        conn: "_PgStoreConnection",
        cur: psycopg.AsyncCursor,
    ):
        super().__init__(engine, session)
        self._conn = conn
        self._cur = cur

    async def close(self):
        await self._conn.close()

    async def fetch(self, query: "QueryBuilder[NodeT, NodeDataT]", count: bool) -> FetchResult:
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

    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        from bench.sql.engine import compile_pg_conditional, pg_exists, pg_count

        if query._aggregation.op == AggregationOp.EXISTS:
            filter = compile_pg_conditional(query._node_cls, query._filter)
            exists = await pg_exists(self._cur, query._node_cls.__table__, filter)
            return AggregateResult(AggregationData(exists=exists))
        elif query._aggregation.op == AggregationOp.COUNT:
            filter = compile_pg_conditional(query._node_cls, query._filter)
            count = await pg_count(self._cur, query._node_cls.__table__, filter)
            return AggregateResult(AggregationData(count=count))
        else:
            raise StoreEngineIncapableError(
                self, query, expr=query._aggregation, reason="unsupported"
            )

    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> None:
        from bench.sql.engine import pg_write_regular_edits

        await pg_write_regular_edits(self._cur, edits)

    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> None:
        if edits:
            from bench.sql.engine import pg_write_regular_edits

            await pg_write_regular_edits(self._cur, edits)
        await self._cur.connection.commit()

    async def cancel(self) -> None:
        await self._cur.connection.rollback()


class OpensearchStoreEngine(StoreEngine):
    type = StoreEngineType.OPENSEARCH
