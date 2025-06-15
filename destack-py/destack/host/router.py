import asyncio
import functools
from collections.abc import AsyncIterator
from typing import Callable, override

import grpclib.server
import structlog
from google.protobuf.message import Message as ProtoMessage
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from destack.language import Client, DatabaseInfo, IsSubject, Session
from destack.pb2 import (
    CommitRequest,
    CommitResponse,
    DownloadFilesRequest,
    DownloadFilesResponse,
    QueryRequest,
    QueryResponse,
    SubscribeRequest,
    SubscribeResponse,
    UploadFilesRequest,
    UploadFilesResponse,
)
from destack.proto import (
    HostBase,
    Network,
    RpcMetadata,
    ScopeData,
    ServiceBase,
    ServiceKind,
)
from destack.sharding import CellProvider, DatabaseProvider
from destack.utils.env import get_from_env
from destack.utils.oracle import Oracle
from destack.utils.telemetry import set_baggage
from destack.utils.uuid import UUID

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
    Multiplexes requests per Destack to a Host using gRPC metadata ('destack-id').
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind
    name = "host_router"

    def __init__(
        self,
        id: str,
        network: Network,
        oracle: Oracle,
        global_database: DatabaseInfo,
        cell_provider: CellProvider,
        database_provider: DatabaseProvider,
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
        self.global_database = global_database
        self.cell_provider = cell_provider
        self.database_provider = database_provider

    def __str__(self):
        return "shards=[*]"

    async def start(self) -> None:
        await super().start()

    def stop(self) -> None:
        for host in self.hosts.values():
            host.stop()

    async def wait_stopped(self) -> None:
        await asyncio.gather(*[host.wait_stopped() for host in self.hosts.values()])

    async def _start_host(self, space_id: UUID) -> "HostService":
        """Starts a Host for the given Destack."""
        existing_host = self.hosts.get(space_id)
        assert existing_host is None, f"already have Host for {space_id}: {existing_host!r}"
        host = HostService(
            id=f"host-{space_id}",
            space_id=space_id,
            network=self.network,
            oracle=self.oracle,
            global_database=self.global_database,
            cell_provider=self.cell_provider,
            database_provider=self.database_provider,
            on_error=self.on_error,
        )
        await host.start()
        self.hosts[space_id] = host
        return host

    async def _get_host(self, request: ProtoMessage) -> "HostService":
        """Gets or starts a running Host for the given Destack"""

        # get request's destack id
        scope: ScopeData | None = getattr(request, "scope")
        if scope is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing scope")
        space_id = UUID(scope.space_id)
        if space_id is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing destack scope id")

        # get host
        host = self.hosts.get(space_id)
        if host is None:
            async with self.hosts_lock:
                host = self.hosts.get(space_id)
                if host is None:
                    host = await self._start_host(space_id)
        return host

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

    @override
    async def query(
        self,
        request: QueryRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> QueryResponse:
        raise NotImplementedError  # implemented in wrap

    @override
    async def subscribe(
        self,
        request: SubscribeRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> AsyncIterator[SubscribeResponse]:
        raise NotImplementedError  # implemented in wrap
        yield ...

    @override
    async def commit(
        self,
        request: CommitRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> CommitResponse:
        raise NotImplementedError  # implemented in wrap

    @override
    async def upload_files(
        self,
        request: UploadFilesRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> UploadFilesResponse:
        raise NotImplementedError  # implemented in wrap

    @override
    async def download_files(
        self,
        request: DownloadFilesRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> DownloadFilesResponse:
        raise NotImplementedError  # implemented in wrap
