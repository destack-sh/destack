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

from destack.grpc import (
    Network,
    RpcMetadata,
    ScopeProto,
    ServiceBase,
    ServiceKind,
    SpaceBase,
)
from destack.language import Client, DatabaseInfo, IsSubject, Session
from destack.proto import (
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
from destack.utils.env import get_from_env
from destack.utils.oracle import Oracle
from destack.utils.telemetry import set_baggage
from destack.utils.uuid import UUID
from desys.sharding import DatabaseProvider, GalaxyProvider

from .space import SpaceService

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

S3_PRESIGNED_URL_EXPIRY = get_from_env(
    "S3_PRESIGNED_URL_EXPIRY",
    typ=int,
    default=3600,
    description="S3 presigned URL expiry (in seconds)",
)


class SpaceRouterService(ServiceBase, SpaceBase):
    """
    Multiplexes requests per Destack to a Space using gRPC metadata ('destack-id').
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind
    name = "space_router"

    def __init__(
        self,
        id: str,
        network: Network,
        oracle: Oracle,
        global_database: DatabaseInfo,
        galaxy_provider: GalaxyProvider,
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
        self.spaces: dict[UUID, SpaceService] = {}
        self.spaces_lock = asyncio.Lock()
        self.global_database = global_database
        self.galaxy_provider = galaxy_provider
        self.database_provider = database_provider

    def __str__(self):
        return "shards=[*]"

    async def start(self) -> None:
        await super().start()

    def stop(self) -> None:
        for space in self.spaces.values():
            space.stop()

    async def wait_stopped(self) -> None:
        await asyncio.gather(*[space.wait_stopped() for space in self.spaces.values()])

    async def _start_space(self, space_id: UUID) -> "SpaceService":
        """Starts a Space for the given Destack."""
        existing_space = self.spaces.get(space_id)
        assert existing_space is None, f"already have Space for {space_id}: {existing_space!r}"
        space = SpaceService(
            id=f"space-{space_id}",
            space_id=space_id,
            network=self.network,
            oracle=self.oracle,
            global_database=self.global_database,
            galaxy_provider=self.galaxy_provider,
            database_provider=self.database_provider,
            on_error=self.on_error,
        )
        await space.start()
        self.spaces[space_id] = space
        return space

    async def _get_space(self, request: ProtoMessage) -> "SpaceService":
        """Gets or starts a running Space for the given Destack"""

        # get request's destack id
        scope: ScopeProto | None = getattr(request, "scope")
        if scope is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing scope")
        space_id = UUID(scope.space_id)
        if space_id is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing destack scope id")

        # get space
        space = self.spaces.get(space_id)
        if space is None:
            async with self.spaces_lock:
                space = self.spaces.get(space_id)
                if space is None:
                    space = await self._start_space(space_id)
        return space

    @override
    def _wrap_rpc_func(
        self, func: Callable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        _, cardinality, _request_type, _reply_type = handler

        if cardinality == grpclib.const.Cardinality.UNARY_UNARY:

            @functools.wraps(func)
            async def _multiplexed_unary_rpc(request: ProtoMessage, metadata: RpcMetadata) -> None:
                space = await self._get_space(request)
                set_baggage(**space.get_service_baggage())
                return await getattr(space, method_name)(request, metadata)

            return _multiplexed_unary_rpc

        elif cardinality == grpclib.const.Cardinality.UNARY_STREAM:

            @functools.wraps(func)
            async def _multiplexed_unary_stream_rpc(request: ProtoMessage, metadata: RpcMetadata):
                space = await self._get_space(request)
                set_baggage(**space.get_service_baggage())
                async for response in getattr(space, method_name)(request, metadata):
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
