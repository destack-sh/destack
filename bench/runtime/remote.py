from functools import wraps
from typing import AsyncIterator, cast, override

import structlog
from opentelemetry import trace

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
    WatchGetUpdate,
    WatchSearchUpdate,
    WritableChannel,
)
from bench.language.const import NodeType, ReadType
from bench.language.graph import NodeDataGraph
from bench.language.node import Node
from bench.language.query import QueryBuilder
from bench.language.session import Session
from bench.proto.wire import (
    EditData,
    ExpressionData,
    GraphIoClient,
    GraphScopeData,
    HostClient,
    ReadOptionsData,
    RpcMetadata,
    SupervisorClient,
    WatchGetRequest,
    WatchSearchRequest,
)
from bench.utils.func import bittuple, repr_enums
from bench.utils.tenacity import RETRY_GRPC, RETRY_GRPC_FOREVER, RetryOptions

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class RemoteEngine(GraphEngine["RemoteChannel"]):
    """An engine that proxies to a remote graph store."""

    def __init__(
        self,
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
        remote: GraphIoClient | HostClient | SupervisorClient,
        rpc_metadata: RpcMetadata,
        write_retry: RetryOptions = RETRY_GRPC,
    ):
        super().__init__(scope, node_types)
        from bench.proto.wiring import pack_rpc_headers

        self.remote = remote
        self.rpc_metadata = rpc_metadata
        self.rpc_headers = pack_rpc_headers(rpc_metadata)
        self.write_retry = write_retry

    def __str__(self):
        return f"scope={self.scope!r}, node_types={repr_enums(self.node_types)}, remote={self.remote.__class__.__name__}"

    @override
    async def connect(self, session: "Session") -> "RemoteChannel":
        return RemoteChannel(self, session)

    @property
    def includes_hidden(self) -> bool:
        return True

    @property
    def is_readonly(self) -> bool:
        return False


class RemoteChannel(WritableChannel[RemoteEngine]):
    """A channel to a remote graph."""

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @override
    async def reconnect(self):
        pass  # remote channels use a shared client

    @override
    async def close(self):
        pass  # remote channels use a shared client

    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: ConnectionOptions
    ) -> type[Connection]:
        if query._read_type == ReadType.GET:
            return RemoteGetConnection
        elif query._read_type == ReadType.SEARCH:
            return RemoteSearchConnection
        elif query._read_type == ReadType.AGGREGATE:
            return RemoteAggregateConnection
        else:
            raise RuntimeError(f"unsupported read type {query._read_type}")

    @property
    @override
    def read_retry(self):
        return RETRY_GRPC_FOREVER

    @staticmethod
    def _rpc(func):
        """Wraps an RPC function with tracing & retries."""
        method_name = func.__name__

        @wraps(func)
        @tracer.start_as_current_span(f"remote.{method_name}")
        async def wrapper(self: "RemoteChannel", *args, **kwargs):
            retry = self.engine.write_retry.new(self.session._oracle)
            while retry.should_retry:
                retry.on_attempt()
                try:
                    return await func(self, *args, **kwargs)
                except Exception as e:
                    logger.error(f"remote.{method_name}.error", channel=self, exc_info=True)
                    retry.on_error(e)
                    if not isinstance(e, self.read_retry.retry_on):
                        raise
                    if retry.should_retry:
                        await self.session._oracle.sleep(retry.get_wait_interval())
            error = retry.to_error()
            if isinstance(error, (OSError,)):
                raise ChannelUnavailableError(
                    self, args[0] if args else None, reason=str(error)
                ) from error
            else:
                raise error

        return wrapper

    @override
    @_rpc
    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResultData:
        raise ChannelIncapableError(self, edits, reason="flush not yet supported")

    @override
    @_rpc
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> CommitResultData:
        from bench.proto import wire

        edits = list(edits)
        request = wire.CommitTransactionRequest(
            id=str(self.session.tx.id),
            edits=edits,
            scope=self.engine.scope,
            context=self.session._get_context(),
        )
        response = await self.engine.remote.commit_transaction(
            request, metadata=self.engine.rpc_headers
        )
        return CommitResultData(
            revisions=response.revisions, cascaded_edits=response.cascaded_edits
        )


class RemoteGetConnection[T: Node](GetConnection[RemoteChannel, T]):
    """Search a remote channel live."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> GetResultData:
        from bench.proto import wire, wiring

        assert query._roots, f"{query!r} has no roots"
        engine = self.channel.engine
        roots_ptr = [r._to_data() for r in query._roots]
        request = wire.GetNodesRequest(
            roots=roots_ptr,
            options=query._options._to_data() if query._options else None,
            scope=engine.scope,
        )
        response = await self.channel.engine.remote.get_nodes(request, metadata=engine.rpc_headers)
        nodes = [wiring.unwrap_some_node(n) for n in response.nodes]
        graph = NodeDataGraph(scope=engine.scope, node_types=engine.node_types, nodes=nodes)
        return GetResultData(
            graph=graph,
            roots_ptr=roots_ptr,
            epoch=response.epoch,
            connection_token=response.connection_token,
        )

    @override
    async def _do_subscribe(
        self, query: "QueryBuilder", result: GetResultData
    ) -> AsyncIterator[WatchGetUpdate]:
        from bench.proto import wiring

        assert result.connection_token is not None, f"{result!r} has no token"
        assert result.epoch is not None, f"{result!r} has no epoch"
        watch_req = WatchGetRequest(
            scope=self.scope, connection_token=result.connection_token, since_epoch=result.epoch
        )
        async for rep in self.channel.engine.remote.watch_get(watch_req):
            update = WatchGetUpdate(
                edits=rep.edits,
                cascaded_edits=rep.cascaded_edits,
                added_nodes=[wiring.unwrap_some_node(n) for n in rep.added_nodes],
                removed_nodes_ptr=rep.removed_nodes_ptr,
                epoch=rep.epoch,
            )
            yield update


class RemoteSearchConnection[T: Node](SearchConnection[RemoteChannel, T]):
    """Search a remote channel live."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> SearchResultData:
        from bench.proto import wire, wiring

        engine = self.channel.engine
        request = wire.SearchNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_object_maybe(query._filter, ExpressionData),
            sort=(
                [wiring.pack_object(s, ExpressionData) for s in query._sort] if query._sort else []
            ),
            first=query._first,
            options=wiring.pack_object_maybe(query._options, ReadOptionsData),
            count=self.options.count,
            scope=engine.scope,
        )
        response = await self.channel.engine.remote.search_nodes(
            request, metadata=engine.rpc_headers
        )
        nodes = [wiring.unwrap_some_node(n) for n in response.nodes]
        graph = NodeDataGraph(scope=engine.scope, node_types=engine.node_types, nodes=nodes)
        roots = [graph[cast(str, r.id)] for r in response.roots_ptr]
        return SearchResultData(
            graph=graph,
            roots=roots,
            roots_ptr=response.roots_ptr,
            total=response.total,
            epoch=response.epoch,
            connection_token=response.connection_token,
        )

    @override
    async def _do_subscribe(
        self, query: "QueryBuilder", result: SearchResultData
    ) -> AsyncIterator[WatchSearchUpdate]:
        from bench.proto import wiring

        assert result.connection_token is not None, f"{result!r} has no token"
        assert result.epoch is not None, f"{result!r} has no epoch"
        watch_req = WatchSearchRequest(
            scope=self.scope, connection_token=result.connection_token, since_epoch=result.epoch
        )
        async for rep in self.channel.engine.remote.watch_search(watch_req):
            update = WatchSearchUpdate(
                edits=rep.edits,
                cascaded_edits=rep.cascaded_edits,
                added_nodes=[wiring.unwrap_some_node(n) for n in rep.added_nodes],
                removed_nodes_ptr=rep.removed_nodes_ptr,
                roots_ptr=rep.roots_ptr,
                total=rep.total,
                epoch=rep.epoch,
            )
            yield update


class RemoteAggregateConnection(AggregateConnection[RemoteChannel]):
    """Aggregate a remote channel live."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> AggregateResultData:
        from bench.proto import wire, wiring

        engine = self.channel.engine
        assert query._aggregation is not None, f"{query!r} has no aggregation"
        request = wire.AggregateNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_object_maybe(query._filter, expect=ExpressionData),
            aggregation=cast(ExpressionData, query._aggregation._to_data()),
            scope=engine.scope,
        )
        response = await engine.remote.aggregate_nodes(request, metadata=engine.rpc_headers)
        return AggregateResultData(
            aggregation=response.aggregation,
            epoch=response.epoch,
            connection_token=response.connection_token,
        )

    # NOTE :Incomplete: RemoteAggregateConnection subscription
