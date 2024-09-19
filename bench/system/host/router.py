import asyncio
import functools
from typing import AsyncIterator, Callable, Mapping, override
from uuid import UUID

import grpclib.server
import structlog
from google.protobuf.message import Message as ProtoMessage
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import Bench, Package, Store, Subject
from bench.language.bench import Branch
from bench.language.const import LOADED_BENCH_NODE_TYPES, SOURCE_NODE_TYPES
from bench.proto.services import ServiceBase
from bench.proto.wire import GraphScopeData, HostBase, ServiceKind
from bench.proto.wire.common_pb2 import RpcMetadata
from bench.proto.wire.system_pb2 import (
    AggregateNodesRequest,
    AggregateNodesResponse,
    CommitTransactionRequest,
    CommitTransactionResponse,
    DownloadFilesRequest,
    DownloadFilesResponse,
    GetNodesRequest,
    GetNodesResponse,
    SearchNodesRequest,
    SearchNodesResponse,
    UploadFilesRequest,
    UploadFilesResponse,
    WatchAggregateRequest,
    WatchAggregateResponse,
    WatchGetRequest,
    WatchGetResponse,
    WatchSearchRequest,
    WatchSearchResponse,
)
from bench.system.host.service import HostService
from bench.system.utils.session import global_session, pg_engine_from_store
from bench.utils.func import to_uuid
from bench.utils.oracle import Oracle
from bench.utils.telemetry import set_baggage
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

HOST_MEMORY_ENGINE_ENABLED = get_from_env(
    "HOST_MEMORY_ENGINE_ENABLED",
    typ=bool,
    default=True,
    description="Whether to provide in-memory caches for Bench/Package",
)
S3_PRESIGNED_URL_EXPIRY = get_from_env(
    "S3_PRESIGNED_URL_EXPIRY",
    typ=int,
    default=3600,
    description="S3 presigned URL expiry (in seconds)",
)
LOADED_HOST_NODE_TYPES = LOADED_BENCH_NODE_TYPES | SOURCE_NODE_TYPES
BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.ancestors(Branch)
    .descendants(*SOURCE_NODE_TYPES)
    .select_all()
    .exclude(Bench.encryption_key)
)


class HostRouterService(ServiceBase, HostBase):
    """
    Multiplexes requests per Bench to a Host using gRPC metadata ('bench-id').
    Also provides some process-level shared functionality.
    Hosts are loaded for all active Benches; new ones 'ping' the multiplexer service to add themselves.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind

    def __init__(self, global_store: Store, oracle: Oracle):
        super().__init__(logger=logger, tracer=tracer, oracle=oracle)
        self.hosts: dict[UUID, HostService] = {}
        self.hosts_lock = asyncio.Lock()
        self._global_store = global_store
        self._global_pg_engine = pg_engine_from_store(global_store)

    def __str__(self):
        return "shards=[*]"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start(self) -> None:
        await super().start()
        async with global_session(self._global_store, (self._global_pg_engine,), self.oracle):
            benches: list[Bench] = await Bench.search()
        await asyncio.gather(*(self._start_host(bench.id) for bench in benches))

    def close(self) -> None:
        for host in self.hosts.values():
            host.close()

    async def wait_closed(self) -> None:
        await asyncio.gather(*[host.wait_closed() for host in self.hosts.values()])

    async def _start_host(self, bench_id: UUID) -> "HostService":
        """Starts a Host for the given Bench."""
        existing_host = self.hosts.get(bench_id)
        assert existing_host is None, f"already have Host for {bench_id}: {existing_host!r}"
        host = HostService(bench_id, self._global_store, self.oracle)
        await host.start()
        self.hosts[bench_id] = host
        return host

    async def _get_host(self, request: ProtoMessage) -> "HostService":
        """Gets or starts a running Host for the given Bench"""

        # get request's bench id
        scope: GraphScopeData | None = getattr(request, "scope")
        if scope is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing scope")
        bench_id = to_uuid(scope.bench_id)
        if bench_id is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing bench scope id")

        # get host
        host = self.hosts.get(bench_id)
        if host is None:
            async with self.hosts_lock:
                host = self.hosts.get(bench_id)
                if host is None:
                    host = await self._start_host(bench_id)
        return host

    async def get_request_subject(self, request: ProtoMessage, metadata: RpcMetadata) -> Subject:
        host = await self._get_host(request)
        return await host.get_request_subject(request, metadata)

    @override
    def _wrap_rpc_func(
        self, func: Callable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        _, cardinality, _request_type, _reply_type = handler

        if cardinality == grpclib.const.Cardinality.UNARY_UNARY:

            @functools.wraps(func)
            async def _multiplexed_unary_rpc(subject: Subject, request: ProtoMessage) -> None:
                host = await self._get_host(request)
                set_baggage(**host.get_service_baggage())
                return await getattr(host, method_name)(subject, request)

            return _multiplexed_unary_rpc

        elif cardinality == grpclib.const.Cardinality.UNARY_STREAM:

            @functools.wraps(func)
            async def _multiplexed_unary_stream_rpc(subject: Subject, request: ProtoMessage):
                host = await self._get_host(request)
                set_baggage(**host.get_service_baggage())
                async for response in getattr(host, method_name)(subject, request):
                    yield response

            return _multiplexed_unary_stream_rpc

        else:
            raise NotImplementedError(f"unexpected cardinality in {method_name}: {cardinality}")

    #
    # Stub methods (implemented by HostService, forwarded in _wrap_rpc)
    #

    @override
    async def get_nodes(self, request: "GetNodesRequest", headers: Mapping) -> "GetNodesResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    def watch_get(
        self, request: "WatchGetRequest", headers: Mapping
    ) -> AsyncIterator["WatchGetResponse"]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        yield WatchGetResponse()  # unreachable

    @override
    async def search_nodes(
        self, request: "SearchNodesRequest", headers: Mapping
    ) -> "SearchNodesResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    def watch_search(
        self, request: "WatchSearchRequest", headers: Mapping
    ) -> AsyncIterator["WatchSearchResponse"]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        yield WatchSearchResponse()  # unreachable

    @override
    async def aggregate_nodes(
        self, request: "AggregateNodesRequest", headers: Mapping
    ) -> "AggregateNodesResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    def watch_aggregate(
        self, request: "WatchAggregateRequest", headers: Mapping
    ) -> AsyncIterator["WatchAggregateResponse"]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        yield WatchAggregateResponse()  # unreachable

    @override
    async def commit_transaction(
        self, request: "CommitTransactionRequest", headers: Mapping
    ) -> "CommitTransactionResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    @override
    async def upload_files(
        self, request: "UploadFilesRequest", headers: Mapping
    ) -> "UploadFilesResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    @override
    async def download_files(
        self, request: "DownloadFilesRequest", headers: Mapping
    ) -> "DownloadFilesResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
