import asyncio
import functools
from typing import AsyncIterator, Callable, Mapping, override

import grpclib.server
import structlog
from fastuuid import UUID
from google.protobuf.message import Message as ProtoMessage
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import (
    Bench,
    Database,
    NodeArea,
    PolicySubject,
)
from bench.proto import (
    CommitTransactionRequest,
    CommitTransactionResponse,
    DownloadFilesRequest,
    DownloadFilesResponse,
    GetNodesRequest,
    GetNodesResponse,
    GraphScopeData,
    HostBase,
    Network,
    RpcMetadata,
    SearchNodesRequest,
    SearchNodesResponse,
    ServiceBase,
    ServiceKind,
    UploadFilesRequest,
    UploadFilesResponse,
    WatchGetRequest,
    WatchGetResponse,
    WatchSearchRequest,
    WatchSearchResponse,
)
from bench.system.core import global_session, pg_engine_from_database
from bench.utils.func import to_uuid
from bench.utils.oracle import Oracle
from bench.utils.telemetry import set_baggage
from bench.utils.utils import get_from_env

from .host import HostService

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

S3_PRESIGNED_URL_EXPIRY = get_from_env(
    "S3_PRESIGNED_URL_EXPIRY",
    typ=int,
    default=3600,
    description="S3 presigned URL expiry (in seconds)",
)


class HostRouterService(ServiceBase, HostBase):
    """
    Multiplexes requests per Bench to a Host using gRPC metadata ('bench-id').
    Also provides some process-level shared functionality.
    Hosts are loaded for all active Benches; new ones 'ping' the multiplexer service to add themselves.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind
    name = "host_router"

    def __init__(
        self,
        id: str,
        global_database: Database,
        regional_database: Database,
        network: Network,
        oracle: Oracle,
        on_error: Callable[[BaseException], None] | None,
    ):
        super().__init__(
            id=id,
            logger=logger,
            tracer=tracer,
            network=network,
            oracle=oracle,
            on_error=on_error,
        )
        self.hosts: dict[UUID, HostService] = {}
        self.hosts_lock = asyncio.Lock()
        self._global_database = global_database
        self._global_pg_engine = pg_engine_from_database(
            "pg-global", global_database, NodeArea.GLOBAL
        )
        self._regional_database = regional_database
        self._regional_pg_engine = pg_engine_from_database(
            f"pg-regional-{regional_database.region.name.lower()}",
            regional_database,
            NodeArea.REGIONAL,
        )

    def __str__(self):
        return "shards=[*]"

    async def start(self) -> None:
        await super().start()
        async with global_session(self._global_database, (self._global_pg_engine,), self.oracle):
            benches: list[Bench] = await Bench.search()
        await asyncio.gather(*(self._start_host(bench.id) for bench in benches))

    def stop(self) -> None:
        for host in self.hosts.values():
            host.stop()

    async def wait_stopped(self) -> None:
        await asyncio.gather(*[host.wait_stopped() for host in self.hosts.values()])

    async def _start_host(self, bench_id: UUID) -> "HostService":
        """Starts a Host for the given Bench."""
        existing_host = self.hosts.get(bench_id)
        assert existing_host is None, f"already have Host for {bench_id}: {existing_host!r}"
        host = HostService(
            id=f"host-{bench_id}",
            bench_id=bench_id,
            global_database=self._global_database,
            regional_database=self._regional_database,
            network=self.network,
            oracle=self.oracle,
            on_error=self.on_error,
        )
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

    async def get_request_subject(
        self, request: ProtoMessage, metadata: RpcMetadata
    ) -> PolicySubject:
        host = await self._get_host(request)
        return await host.get_request_subject(request, metadata)

    @override
    def _wrap_rpc_func(
        self, func: Callable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        _, cardinality, _request_type, _reply_type = handler

        if cardinality == grpclib.const.Cardinality.UNARY_UNARY:

            @functools.wraps(func)
            async def _multiplexed_unary_rpc(request: ProtoMessage, metadata: RpcMetadata) -> None:
                host = await self._get_host(request)
                set_baggage(**host.get_service_baggage())
                return await getattr(host, method_name)(request, metadata)

            return _multiplexed_unary_rpc

        elif cardinality == grpclib.const.Cardinality.UNARY_STREAM:

            @functools.wraps(func)
            async def _multiplexed_unary_stream_rpc(request: ProtoMessage, metadata: RpcMetadata):
                host = await self._get_host(request)
                set_baggage(**host.get_service_baggage())
                async for response in getattr(host, method_name)(request, metadata):
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
