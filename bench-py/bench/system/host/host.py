import asyncio
from collections.abc import AsyncIterator
from typing import TYPE_CHECKING, Any, Callable, override

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language import (
    CLOUD,
    Bench,
    CellRegistry,
    Change,
    Client,
    Database,
    DatabaseInfo,
    DatabaseRegistry,
    IsSubject,
    LiveStore,
    NodeReference,
    NodeType,
    Query,
    Scope,
    Session,
)
from bench.pb2 import (
    CommitRequest,
    CommitResponse,
    DownloadFilesRequest,
    DownloadFilesResponse,
    HostBase,
    QueryRequest,
    QueryResponse,
    RpcMetadata,
    ServiceKind,
    SubscribeRequest,
    SubscribeResponse,
    UploadFilesRequest,
    UploadFilesResponse,
)
from bench.proto import Network, ServiceBase
from bench.system.store.postgres import DatabaseStore
from bench.utils.env import ENV
from bench.utils.oracle import Oracle
from bench.utils.utils import get_from_env

from .plugin import HostPlugin

if TYPE_CHECKING:
    from bench.system.plugin import Provisioner

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

FILE_DOWNLOAD_URL_EXPIRY = get_from_env(
    "FILE_DOWNLOAD_URL_EXPIRY",
    typ=int,
    default=3600,  # :FileUrlExpiry
    description="File download URL expiry (in seconds)",
)


class HostService(ServiceBase, HostBase):
    """
    Host for a Bench. There is only one Host per Bench.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind
    name = "host"

    def __init__(
        self,
        id: str,
        bench_id: UUID,
        network: Network,
        oracle: Oracle,
        global_database: DatabaseInfo,
        cell_registry: CellRegistry,
        database_registry: DatabaseRegistry,
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
        self.bench_id = bench_id
        self.bench_ptr = NodeReference(node_type=NodeType.BENCH, id=bench_id, bench_id=bench_id)
        self.scope = Scope(bench_id=bench_id)
        self.provisioners: tuple[Provisioner, ...] = ()
        self.plugins: tuple[HostPlugin, ...] = ()  # incl. provisioners
        self.database_store = DatabaseStore(global_database=global_database)
        self.store: LiveStore = ...  # type: ignore nocheckin
        self.cell_registry = cell_registry
        self.database_registry = database_registry

    def __str__(self):
        return f"{self.bench_id}"

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {"bench_id": self.bench_id}

    @property
    def is_idle(self) -> bool:
        """Check if the Host is idle (no pending requests or processing)."""
        return all(plugin.is_idle for plugin in self.plugins) and super().is_idle

    async def start(self) -> None:
        async with Session(store=self.database_store):
            bench = await Bench.get(
                where=Bench.property("id").eq(self.bench_id),
                Databases=Database.search(),
            ).execute_one()
            if (database := bench.database) is not None:
                self.database_store.bench_database = database.to_info()

    def stop(self) -> None:
        super().stop()
        for plugin in self.plugins:
            plugin.close()

    async def wait_stopped(self) -> None:
        await super().wait_stopped()
        await asyncio.gather(*(plugin.wait_closed() for plugin in self.plugins))

    @override
    async def make_session(self, metadata: RpcMetadata) -> Session:
        return Session()

    @override
    async def resolve_client(
        self, request, metadata: RpcMetadata
    ) -> tuple[IsSubject | None, Client | None]:
        raise NotImplementedError

    @override
    async def query(
        self,
        request: QueryRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> QueryResponse:
        query = Query.from_proto(request.query)
        query.validate()
        query_result = await self.store.query(query)
        return QueryResponse(result=query_result.to_proto())

    @override
    async def subscribe(
        self,
        request: SubscribeRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> AsyncIterator[SubscribeResponse]:
        query = Query.from_proto(request.query)
        query.validate()
        async for update in await self.store.subscribe(query):
            yield SubscribeResponse(update=update.to_proto())

    @override
    async def commit(
        self,
        request: CommitRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> CommitResponse:
        changes = [Change.from_proto(change) for change in request.changes]
        approved_changes: list[Change] = []
        for _ in changes:
            pass  # nocheckin: validate & approve/reject changes
        results = await self.store.commit(approved_changes)
        return CommitResponse(results=[result.to_proto() for result in results])

    @override
    async def upload_files(
        self,
        request: UploadFilesRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> UploadFilesResponse:
        raise NotImplementedError

    @override
    async def download_files(
        self,
        request: DownloadFilesRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> DownloadFilesResponse:
        raise NotImplementedError


def get_drive_bucket(bench: Bench) -> str:
    bucket_name = f"bench-{ENV.value}-{CLOUD.name.lower()}-{bench.region.slug}-files"
    return bucket_name


def get_file_key(bench: Bench, sha256: str, name: str | None) -> str:
    """Gets the key for a file in the given bucket."""
    if name is None:
        return f"{bench.id}/{sha256}/__UNNAMED__"
    else:
        return f"{bench.id}/{sha256}/{name}"
