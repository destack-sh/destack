#
# Queries
#
from functools import wraps
from typing import (
    TYPE_CHECKING,
    AsyncIterator,
    cast,
    override,
)

import structlog
from grpclib import GRPCError
from opentelemetry import trace

from bench import pb2
from bench.language.core import (
    Node,
    NodeDataGraph,
    NodeType,
    QueryType,
    bittuple,
    repr_enums,
    repr_scope,
)
from bench.pb2.common_pb2 import RpcMetadata
from bench.pb2.lang_pb2 import EditData, ExpressionData, GraphScopeData
from bench.pb2.system_grpc import GraphClient, HostClient, SupervisorClient
from bench.pb2.system_pb2 import WatchGetRequest, WatchSearchRequest
from bench.utils.telemetry import TELEMETRY, capture_exception
from bench.utils.tenacity import RETRY_GRPC, RETRY_GRPC_FOREVER, RetryOptions

from .connection import Connection, GetConnection, SearchConnection
from .engine import (
    CommitResultData,
    ConnectionOptions,
    Engine,
    EngineIncapableError,
    EngineUnavailableError,
    FlushResultData,
    GetResultData,
    SearchResultData,
    WatchGetUpdateData,
    WatchSearchUpdateData,
    WritableConnector,
)

if TYPE_CHECKING:
    from bench.language import LegacyQuery, Session

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def _grpc_wrap_error(query: "LegacyQuery", e: GRPCError):
    """Wraps a GRPCError in something more harmonized."""
    return e  # NOTE :UX: wrap remote grpc errors


class RemoteEngine(Engine["RemoteConnector"]):
    """An engine that proxies to a remote graph store."""

    def __init__(
        self,
        name: str,
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
        remote: GraphClient | HostClient | SupervisorClient,
        rpc_metadata: RpcMetadata,
        write_retry: RetryOptions = RETRY_GRPC,
    ):
        super().__init__(name, scope, node_types)
        from bench.proto.wiring import pack_rpc_headers

        self.remote = remote
        self.rpc_metadata = rpc_metadata
        self.rpc_headers = pack_rpc_headers(rpc_metadata)
        self.write_retry = write_retry

    def __str__(self):
        return f"{self.name} [scope={repr_scope(self.scope)}, node_types={repr_enums(self.node_types)}, remote={self.remote.__class__.__name__}]"

    @override
    async def connector(self, session: "Session") -> "RemoteConnector":
        return RemoteConnector(self, session)

    @property
    def include_removed(self) -> bool:
        return True

    @property
    def is_readonly(self) -> bool:
        return False


class RemoteConnector(WritableConnector[RemoteEngine]):
    """A connector to a remote graph."""

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @override
    async def reset(self):
        pass  # remote connectors use a shared client

    @override
    async def close(self):
        pass  # remote connectors use a shared client

    def _get_connection_cls(
        self, query: "LegacyQuery", scope: GraphScopeData, options: ConnectionOptions
    ) -> type[Connection]:
        if query._type == QueryType.GET:
            return RemoteGetConnection
        elif query._type == QueryType.SEARCH:
            return RemoteSearchConnection
        else:
            raise RuntimeError(f"unsupported read type {query._type}")

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
        async def wrapper(self: "RemoteConnector", *args, **kwargs):
            retry = self.engine.write_retry.new(self.session._oracle)
            while retry.should_retry:
                retry.on_attempt()
                try:
                    return await func(self, *args, **kwargs)
                except Exception as e:
                    logger.error(f"remote.{method_name}.error", connector=self, exc_info=True)
                    if TELEMETRY:
                        capture_exception(e)
                    retry.on_error(e)
                    if not isinstance(e, self.read_retry.retry_on):
                        raise
                    if retry.should_retry:
                        await self.session._oracle.sleep(retry.get_wait_interval())
            error = retry.to_error()
            if isinstance(error, (OSError,)):
                raise EngineUnavailableError(
                    self, args[0] if args else None, reason=str(error)
                ) from error
            else:
                raise error

        return wrapper

    @override
    @_rpc
    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResultData:
        raise EngineIncapableError(self, edits, reason="flush not yet supported")

    @override
    @_rpc
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> CommitResultData:
        edits = list(edits)
        request = pb2.CommitTransactionRequest(
            id=str(self.session.tx.id),
            edits=edits,
            scope=self.engine.scope,
            context=self.session._get_context(),
        )
        response = await self.engine.remote.commit_transaction(
            request, metadata=self.engine.rpc_headers
        )
        return CommitResultData(cascaded_edits=list(response.cascaded_edits))


class RemoteGetConnection[T: Node](GetConnection[RemoteConnector, T]):
    """Search a remote connector live."""

    @override
    async def _do_read(self, query: "LegacyQuery") -> GetResultData:
        from bench.proto import wiring

        assert query._roots, f"{query!r} has no roots"
        engine = self.connector.engine
        roots_ptr = [r._to_data() for r in query._roots]
        request = pb2.GetNodesRequest(
            scope=engine.scope,
            roots=roots_ptr,
            base_type_ptr=query._base_type._to_ref_data() if query._base_type else None,
            ancestor_types=[wiring.pack_enum(NodeType, t) for t in query._ancestor_types],
            descendant_types=[wiring.pack_enum(NodeType, t) for t in query._descendant_types],
            select=query._select._to_data() if query._select else None,
            include_removed=query._include_removed,
        )
        try:
            response = await self.connector.engine.remote.get_nodes(
                request, metadata=engine.rpc_headers
            )
        except GRPCError as e:
            raise _grpc_wrap_error(query, e) from e
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
        self, query: "LegacyQuery", token: str | None, epoch: int
    ) -> AsyncIterator[WatchGetUpdateData]:
        from bench.proto import unary_stream_rpc, wiring

        assert token is not None, f"{self!r} has no token"
        assert epoch is not None, f"{self!r} has no epoch"
        watch_req = WatchGetRequest(scope=self.scope, connection_token=token, since_epoch=epoch)
        async for rep in unary_stream_rpc(self.connector.engine.remote.watch_get, watch_req):
            update = WatchGetUpdateData(
                edits=list(rep.edits),
                cascaded_edits=list(rep.cascaded_edits),
                added_nodes=[wiring.unwrap_some_node(n) for n in rep.added_nodes],
                removed_nodes_ptr=list(rep.removed_nodes_ptr),
                epoch=rep.epoch,
            )
            yield update


class RemoteSearchConnection[T: Node](SearchConnection[RemoteConnector, T]):
    """Search a remote connector live."""

    @override
    async def _do_read(self, query: "LegacyQuery") -> SearchResultData:
        from bench.proto import wiring

        engine = self.connector.engine
        request = pb2.SearchNodesRequest(
            scope=engine.scope,
            node_type=wiring.pack_enum(NodeType, query._node_type),
            base_type_ptr=query._base_type._to_ref_data() if query._base_type else None,
            filter=wiring.pack_builtin_object_maybe(query._filter, ExpressionData),
            sort=(
                [wiring.pack_builtin_object(s, ExpressionData) for s in query._sort]
                if query._sort
                else []
            ),
            ancestor_types=[wiring.pack_enum(NodeType, t) for t in query._ancestor_types],
            descendant_types=[wiring.pack_enum(NodeType, t) for t in query._descendant_types],
            count=self.options.count,
            select=query._select._to_data() if query._select else None,
        )
        if query._first:
            request.first = query._first
        try:
            response = await self.connector.engine.remote.search_nodes(
                request, metadata=engine.rpc_headers
            )
        except GRPCError as e:
            raise _grpc_wrap_error(query, e) from e
        nodes = [wiring.unwrap_some_node(n) for n in response.nodes]
        graph = NodeDataGraph(scope=engine.scope, node_types=engine.node_types, nodes=nodes)
        roots = [graph[cast(str, r.id)] for r in response.roots_ptr]
        return SearchResultData(
            graph=graph,
            roots=roots,
            roots_ptr=list(response.roots_ptr),
            total=response.total,
            epoch=response.epoch,
            connection_token=response.connection_token,
        )

    @override
    async def _do_subscribe(
        self, query: "LegacyQuery", token: str | None, epoch: int
    ) -> AsyncIterator[WatchSearchUpdateData]:
        from bench.proto import unary_stream_rpc, wiring

        assert token is not None, f"{self!r} has no token"
        assert epoch is not None, f"{self!r} has no epoch"
        watch_req = WatchSearchRequest(scope=self.scope, connection_token=token, since_epoch=epoch)
        async for rep in unary_stream_rpc(self.connector.engine.remote.watch_search, watch_req):
            update = WatchSearchUpdateData(
                edits=list(rep.edits),
                cascaded_edits=list(rep.cascaded_edits),
                added_nodes=[wiring.unwrap_some_node(n) for n in rep.added_nodes],
                removed_nodes_ptr=list(rep.removed_nodes_ptr),
                roots_ptr=list(rep.roots_ptr),
                total=rep.total,
                epoch=rep.epoch,
            )
            yield update
