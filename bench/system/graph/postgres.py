from functools import wraps
from typing import override

import psycopg
import structlog
from opentelemetry import trace

from bench.language.bench import Bench, Store
from bench.language.connection import (
    AggregateConnection,
    AggregateResultData,
    ChannelIncapableError,
    ChannelUnavailableError,
    CommitResultData,
    Connection,
    ConnectionOptions,
    FlushResultData,
    GetConnection,
    GetResultData,
    GraphEngine,
    SearchConnection,
    SearchResultData,
    WritableChannel,
)
from bench.language.const import AggregationOp, NodeType, ReadType
from bench.language.graph import NodeDataGraph
from bench.language.node import Node, NodeReference
from bench.language.query import QueryBuilder, ReadOptions
from bench.language.session import Session
from bench.proto.wire import (
    AggregationData,
    EditData,
    GraphScopeData,
)
from bench.sql.client import AsyncPostgresConnection, get_pg_pool
from bench.sql.engine import (
    TABLE_BY_NODE_TYPE,
    BenchContext,
    pg_compile_conditional_maybe,
    pg_count,
    pg_edit,
    pg_exists,
    pg_get_node_graph,
    pg_search_node_graph,
)
from bench.utils.func import bittuple, repr_enums

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class PostgresEngine(GraphEngine):
    """An engine that talks directly to a Postgres store."""

    def __init__(
        self,
        store: "Store",
        bench: "Bench",
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
    ):
        super().__init__(scope, node_types)
        self.store = store
        self.bench = bench
        self.context = BenchContext(bench=self.bench)

    def __str__(self):
        return (
            f"scope={self.scope!r}, node_types={repr_enums(self.node_types)}, store={self.store!r}"
        )

    @override
    async def channel(self, session: "Session") -> "PostgresChannel":
        pool = get_pg_pool(self.store)
        conn = await pool.acquire()
        channel = PostgresChannel(self, session, conn)
        logger.trace("postgres.channel.open", channel=channel, conn=conn)
        return channel

    @property
    def includes_hidden(self) -> bool:
        return True

    @property
    def is_readonly(self) -> bool:
        return False


def _pg_method(func):
    """Wraps a Postgres function with tracing & error wrapping."""
    method_name = func.__name__

    @wraps(func)
    @tracer.start_as_current_span(f"postgres.{method_name}")
    async def wrapper(self: "PostgresChannel", *args, **kwargs):
        from bench.sql.engine import SqlConnectionError

        try:
            return await func(self, *args, **kwargs)
        except (SqlConnectionError, psycopg.OperationalError) as e:
            raise ChannelUnavailableError(self, args[0] if args else None, reason=str(e)) from e

    return wrapper


class PostgresChannel(WritableChannel[PostgresEngine]):
    """A channel to a Postgres store (usually maps to a postgres connection)."""

    def __init__(
        self,
        engine: "PostgresEngine",
        session: "Session",
        connection: "AsyncPostgresConnection",
    ):
        super().__init__(engine, session)
        self.connection = connection

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @property
    def cur(self) -> psycopg.AsyncCursor:
        return self.connection.cursor

    @override
    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: ConnectionOptions
    ) -> type[Connection]:
        if query._read_type == ReadType.GET:
            return PostgresGetConnection
        elif query._read_type == ReadType.SEARCH:
            return PostgresSearchConnection
        elif query._read_type == ReadType.AGGREGATE:
            return PostgresAggregateConnection
        else:
            raise RuntimeError(f"unsupported read type {query._read_type}")

    @override
    @_pg_method
    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResultData:
        new_revisions, cascaded_edits = await pg_edit(
            cur=self.cur, ctx=self.engine.context, edits=edits
        )
        return FlushResultData(revisions=new_revisions, cascaded_edits=cascaded_edits)

    @override
    @_pg_method
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> CommitResultData:
        new_revisions, cascaded_edits = await pg_edit(
            cur=self.cur, ctx=self.engine.context, edits=edits
        )
        await self.connection.commit()
        return CommitResultData(revisions=new_revisions, cascaded_edits=cascaded_edits)

    @override
    async def reset(self):
        await self.connection.close()
        self.connection = await self.connection.pool.acquire()

    @override
    @_pg_method
    async def close(self):
        if not self.cur.connection.broken:
            await self.cur.connection.rollback()
        await self.connection.close()
        logger.trace("postgres.channel.close", channel=self, conn=self.connection)


class PostgresGetConnection[T: Node](GetConnection[PostgresChannel, T]):
    """Get from a Postgres channel."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> GetResultData:
        assert query._roots, f"{query!r} has no roots"
        graph = NodeDataGraph(scope=self.scope, node_types=self.node_types)
        roots_ptr = [r._to_data() for r in query._roots]
        _ = await pg_get_node_graph(
            cur=self.channel.cur,
            ctx=self.channel.engine.context,
            root_type=query._node_type,
            roots=roots_ptr,
            options=query._options or ReadOptions(),
            visited_graph=graph,
        )
        return GetResultData(graph=graph, roots_ptr=roots_ptr, epoch=None, connection_token=None)


class PostgresSearchConnection[T: Node](SearchConnection[PostgresChannel, T]):
    """Search a Postgres channel."""

    @override
    @_pg_method
    async def _do_read(self, query: "QueryBuilder") -> SearchResultData:
        node_table = TABLE_BY_NODE_TYPE[query._node_type]
        roots, graph = await pg_search_node_graph(
            cur=self.channel.cur,
            ctx=self.channel.engine.context,
            scope=self.channel.engine.scope,
            node_type=query._node_type,
            options=query._options or ReadOptions(),
            filter=query._filter,
            sort=query._sort,
            first=query._first,
            skip=query._skip,
        )
        if self.options.count:
            total = await pg_count(
                cur=self.channel.cur,
                ctx=self.channel.engine.context,
                table=node_table,
                where=pg_compile_conditional_maybe(query._node_cls, query._filter),
            )
        else:
            total = None
        return SearchResultData(
            graph=graph,
            roots=roots,
            roots_ptr=[NodeReference._ref_data_from_node_data(r) for r in roots],
            total=total,
            epoch=None,
            connection_token=None,
        )


class PostgresAggregateConnection(AggregateConnection):
    """Aggregate a Postgres channel."""

    @override
    @_pg_method
    async def _do_read(self, query: "QueryBuilder") -> AggregateResultData:
        node_table = TABLE_BY_NODE_TYPE[query._node_type]
        assert query._aggregation is not None
        where = pg_compile_conditional_maybe(query._node_cls, query._filter)
        if query._aggregation.op == AggregationOp.EXISTS:
            exists = await pg_exists(
                cur=self.channel.cur, ctx=self.channel.engine.context, table=node_table, where=where
            )
            return AggregateResultData(
                aggregation=AggregationData(exists=exists), epoch=None, connection_token=None
            )
        elif query._aggregation.op == AggregationOp.COUNT:
            count = await pg_count(
                cur=self.channel.cur, ctx=self.channel.engine.context, table=node_table, where=where
            )
            return AggregateResultData(
                aggregation=AggregationData(count=count), epoch=None, connection_token=None
            )
        else:
            raise ChannelIncapableError(
                self, query, expression=query._aggregation, reason="unsupported"
            )
