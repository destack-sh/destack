#
# Queries
#
import abc
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Generic,
    NamedTuple,
    Optional,
    TypeVar,
    Union,
    cast,
    override,
)
from uuid import UUID

import psycopg
import structlog

from bench.language.const import (
    AggregationOp,
    BenchError,
    NodeType,
    StoreConnectionType,
)
from bench.language.node import Node
from bench.proto.wire import (
    AggregationData,
    AnyNodeData,
    EditData,
    ExpressionData,
    GraphIoStub,
    GraphScope,
    HostStub,
    NodeReferenceData,
    ReadOptionsData,
    RpcMetadata,
    SupervisorStub,
)
from bench.utils.func import bittuple
from bench.utils.tenacity import RETRY_GRPC, RetryOptions, retry

if TYPE_CHECKING:
    from bench.language import Bench, Expression, Field, Property, Session, Store
    from bench.language.query import QueryBuilder
    from bench.proto.monkey import _PatchedRpcMetadata
    from bench.sql.client import _PgStoreConnection

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)

NodeT = TypeVar("NodeT", bound=Node)
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type[Node]]


class StoreEngineError(BenchError, ValueError):
    def __init__(
        self,
        engine: Union["StoreEngine", "StoreConnection", StoreConnectionType],
        query: Optional["QueryBuilder"] = None,
        expression: Union["Expression", list["Expression"], None] = None,
        reason: str | None = None,
    ):
        if query is not None:
            action = repr(query)
        elif expression is not None:
            action = repr(expression)
        else:
            action = "<unknown action>"
        super().__init__(f"{engine} cannot {action}: {reason or '<unknown error>'}")
        self.engine = engine
        self.query = query
        self.expression = expression
        self.reason = reason


class ConnectionIncapableError(StoreEngineError):
    pass


class FetchOptions(NamedTuple):
    count: bool = False  # type: ignore
    lock_for_update: bool = False
    skip_locked: bool = False


class FetchResult(NamedTuple):
    nodes: list[AnyNodeData] | tuple[AnyNodeData, ...]
    roots: list[NodeReferenceData] | tuple[NodeReferenceData, ...]
    cursors: list[str] | tuple[str, ...]
    start_cursor: str | None
    total: int | None = None
    epoch: int | None = None


class FlushResult(NamedTuple):
    revisions: list[int]
    cascaded_edits: list[EditData]


class AggregateResult(NamedTuple):
    aggregation: AggregationData


class StoreEngine(abc.ABC, Generic[NodeT, NodeDataT]):
    """A store engine provides Connections to query some Store-like thing."""

    type: ClassVar[StoreConnectionType]

    def __init__(
        self,
        scope: GraphScope,
        node_types: tuple[NodeType, ...] | bittuple[NodeType],
    ):
        self.scope = scope
        self.node_types = node_types

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
        if self.scope.bench_id and self.scope.bench_id != scope.bench_id:
            return False
        if self.scope.package_id and self.scope.package_id != scope.package_id:
            return False
        return not node_type not in self.node_types

    async def connect(self, session: "Session") -> "StoreConnection":
        """Opens the store engine for a session."""
        raise NotImplementedError


class StoreConnection(abc.ABC, Generic[NodeT, NodeDataT]):
    def __init__(self, session: "Session"):
        self.session = session

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
        raise ConnectionIncapableError(self, query, reason="fetch unsupported")

    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        """Read the nodes given the aggregate query in the current transaction context (if any)."""
        raise ConnectionIncapableError(self, query, reason="exists unsupported")

    #
    # Transaction management
    # The methods closely mirror :GraphIO service methods for universal 2PCs.
    #

    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResult:
        """
        Flushes edits in the current transaction context. If not in a transaction, begins one.
        If this is a primary store, must return the accepted revisions for every edit (in order).
        """
        raise ConnectionIncapableError(self, reason="flush unsupported")

    async def complete(self) -> None:
        """Completes the current transaction context. No further operations are allowed."""
        raise ConnectionIncapableError(self, reason="complete unsupported")

    async def cancel(self) -> None:
        """Cancels the current transaction context. No further operations are allowed."""
        raise ConnectionIncapableError(self, reason="cancel unsupported")

    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResult:
        """
        Commits the flushed pending and given edits in the current transaction context.
        If this is a primary store, must return the accepted revisions for every edit (in order).
        """
        raise ConnectionIncapableError(self, reason="commit unsupported")

    async def close(self):
        """Closes this connection to all further operations."""
        pass


class RemoteEngine(StoreEngine[NodeT, NodeDataT]):
    """An engine that proxies to a remote graph store."""

    type = StoreConnectionType.REMOTE

    def __init__(
        self,
        scope: GraphScope,
        node_types: tuple[NodeType, ...] | bittuple[NodeType],
        remote: GraphIoStub | HostStub | SupervisorStub,
        rpc_metadata: RpcMetadata,
        retry: RetryOptions = RETRY_GRPC,
    ):
        super().__init__(scope, node_types)
        self.remote = remote
        self.rpc_metadata = rpc_metadata
        self.rpc_headers = cast("_PatchedRpcMetadata", rpc_metadata).to_headers()
        self.retry = retry

    def __str__(self):
        return f"scope={self.scope!r}, node_types=[{', '.join(t.bench_name for t in self.node_types)}], remote={self.remote.__class__.__name__}"

    @override
    async def connect(self, session: "Session") -> "RemoteConnection":
        return RemoteConnection(self, session)


class RemoteConnection(StoreConnection[NodeT, NodeDataT]):
    """A connection to a remote graph."""

    def __init__(self, engine: "RemoteEngine", session: "Session"):
        super().__init__(session)
        self.engine = engine

    @override
    @retry(
        lambda self, *args, **kwargs: self.engine.retry,
        on_error=lambda self, query, options, e: logger.error(
            "remote.fetch.error", connection=self, query=query, options=options, exc_info=e
        ),
    )
    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        from bench.proto import wire, wiring

        request = wire.SearchNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_struct_maybe(query._filter, ExpressionData),
            sort=(
                [wiring.pack_struct(s, ExpressionData) for s in query._sort] if query._sort else []
            ),
            first=query._first,
            options=wiring.pack_struct_maybe(query._options, ReadOptionsData),
            count=options.count,
            scope=self.engine.scope,
        )
        response = await self.engine.remote.search_nodes(request, metadata=self.engine.rpc_headers)
        return FetchResult(
            nodes=[wiring.unwrap_some_node(n) for n in response.nodes],
            roots=response.roots,
            cursors=response.cursors,
            start_cursor=response.start_cursor,
            total=response.total,
            epoch=response.epoch,
        )

    @override
    @retry(
        lambda self, *args, **kwargs: self.engine.retry,
        on_error=lambda self, query, e: logger.error(
            "remote.aggregate.error", connection=self, query=query, exc_info=e
        ),
    )
    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        from bench.proto import wire, wiring

        assert query._aggregation is not None, f"{query!r} has no aggregation"
        request = wire.AggregateNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_struct_maybe(query._filter, expect=ExpressionData),
            aggregation=cast(ExpressionData, query._aggregation._to_data()),
            scope=self.engine.scope,
        )
        response = await self.engine.remote.aggregate_nodes(
            request, metadata=self.engine.rpc_headers
        )
        return AggregateResult(response.aggregation)

    @override
    @retry(
        lambda self, *args, **kwargs: self.engine.retry,
        on_error=lambda self, edits, e: logger.error(
            "remote.commit.error", connection=self, edits=edits, exc_info=e
        ),
    )
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResult:
        from bench.proto import wire

        edits = list(edits)
        request = wire.CommitTransactionRequest(
            id=str(self.session.tx.id), edits=edits, scope=self.engine.scope
        )
        response = await self.engine.remote.commit_transaction(
            request, metadata=self.engine.rpc_headers
        )
        return FlushResult(revisions=response.revisions, cascaded_edits=response.cascaded_edits)


class PostgresEngine(StoreEngine[NodeT, NodeDataT], Generic[NodeT, NodeDataT]):
    """An engine that uses postgres connections."""

    type = StoreConnectionType.POSTGRES

    def __init__(
        self,
        store: "Store",
        bench: "Bench",
        scope: GraphScope,
        node_types: tuple[NodeType, ...] | bittuple[NodeType],
    ):
        super().__init__(scope, node_types)
        self.store = store
        self.bench = bench

    def __str__(self):
        return f"scope={self.scope!r}, node_types=[{', '.join(t.bench_name for t in self.node_types)}], store={self.store!r}"

    @override
    async def connect(self, session: "Session") -> "PostgresConnection":
        from bench.sql.client import get_pg_store_connection

        conn = get_pg_store_connection(self.store)
        cur = await conn.open()
        return PostgresConnection(self, session, conn, cur)


class PostgresConnection(StoreConnection[NodeT, NodeDataT], Generic[NodeT, NodeDataT]):
    """A specific connection to a Postgres store."""

    def __init__(
        self,
        engine: "PostgresEngine",
        session: "Session",
        conn: "_PgStoreConnection",
        cur: psycopg.AsyncCursor,
    ):
        super().__init__(session)
        self.engine = engine
        self.conn = conn
        self.cur = cur

    @override
    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        from bench.language import NodeReference, ReadOptions
        from bench.sql.engine import _pg_compile_conditional_maybe, pg_count, pg_search_node_graph

        assert query._node_cls.__table__ is not None, f"{query._node_cls} has no table"
        roots, graph = await pg_search_node_graph(
            cur=self.cur,
            node_type=query._node_type,
            options=query._options or ReadOptions(),
            filter=query._filter,
            sort=query._sort,
            first=query._first,
            skip=query._skip,
        )
        if options.count:
            total = await pg_count(
                cur=self.cur,
                table=query._node_cls.__table__,
                where=_pg_compile_conditional_maybe(query._node_cls, query._filter),
            )
        else:
            total = None
        return FetchResult(
            roots=[NodeReference.from_node_data(r) for r in roots.nodes],
            nodes=list(graph.nodes),
            cursors=roots.cursors,
            start_cursor=roots.start_cursor,
            total=total,
        )

    @override
    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        from bench.sql.engine import _pg_compile_conditional_maybe, pg_count, pg_exists

        assert query._node_cls.__table__ is not None, f"{query._node_cls} has no table"
        assert query._aggregation is not None
        where = _pg_compile_conditional_maybe(query._node_cls, query._filter)
        if query._aggregation.op == AggregationOp.EXISTS:
            exists = await pg_exists(self.cur, query._node_cls.__table__, where=where)
            return AggregateResult(AggregationData(exists=exists))
        elif query._aggregation.op == AggregationOp.COUNT:
            count = await pg_count(self.cur, query._node_cls.__table__, where=where)
            return AggregateResult(AggregationData(count=count))
        else:
            raise ConnectionIncapableError(
                self, query, expression=query._aggregation, reason="unsupported"
            )

    @override
    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResult:
        from bench.sql.engine import pg_edit

        new_revisions, cascaded_edits = await pg_edit(self.cur, edits)
        return FlushResult(revisions=new_revisions, cascaded_edits=cascaded_edits)

    @override
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResult:
        from bench.sql.engine import pg_edit

        new_revisions, cascaded_edits = await pg_edit(self.cur, edits)
        await self.cur.connection.commit()
        return FlushResult(revisions=new_revisions, cascaded_edits=cascaded_edits)

    @override
    async def cancel(self) -> None:
        await self.cur.connection.rollback()

    async def close(self):
        await self.cur.connection.rollback()
        await self.conn.close()


class SplitConnection(StoreConnection[NodeT, NodeDataT], Generic[NodeT, NodeDataT]):
    """Multiple read connections to the same store."""

    @override
    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        # nocheckin: SplitConnection.fetch
        scope = (
            self.session.tx._get_scope_for_node(query._base)
            if query._base
            else self.session._default_scope
        )
        engine = self.session.tx._get_engine_for(scope, query._node_type)
        connection = await self.session.tx._get_engine_connection(engine)
        return await connection.fetch(query, options)

    @override
    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        raise NotImplementedError("nocheckin: SplitConnection.aggregate")
