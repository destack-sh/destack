from collections.abc import AsyncIterator
from typing import TYPE_CHECKING, Any, Callable, override

import structlog
from opentelemetry import trace

from destack.language import (
    CLOUD,
    Change,
    Client,
    Database,
    DatabaseInfo,
    IsSubject,
    LiveStore,
    NodeReference,
    NodeType,
    Query,
    Scope,
    Session,
    Space,
    StoreType,
)
from destack.pb2 import (
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
from destack.proto import Network, ServiceBase
from destack.sharding import CellProvider, DatabaseProvider
from destack.store import PostgresStore
from destack.utils.env import ENV, get_from_env
from destack.utils.oracle import Oracle
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass

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
    Host for a Space. There is only one Host per Space.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind
    name = "host"

    def __init__(
        self,
        id: str,
        space_id: UUID,
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
        self.space_id = space_id
        self.space_ptr = NodeReference(node_type=NodeType.SPACE, id=space_id, space_id=space_id)
        self.scope = Scope(space_id=space_id)
        self.global_postgres_store = PostgresStore(
            database=global_database, types=(StoreType.GLOBAL_ENTITY,)
        )
        self.spatial_postgres_store: PostgresStore | None = None
        self.store: LiveStore = ...  # type: ignore nocheckin: LiveStore
        self.cell_provider = cell_provider
        self.database_provider = database_provider

    def __str__(self):
        return f"{self.space_id}"

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {"space_id": self.space_id}

    @property
    def is_idle(self) -> bool:
        """Check if the Host is idle (no pending requests or processing)."""
        return super().is_idle

    async def start(self) -> None:
        async with Session(store=self.global_postgres_store):
            space = await Space.get(
                where=Space.property("id").eq(self.space_id),
                Databases=Database.search(),
            ).execute_one()
            if (database := space.database) is not None:
                self.spatial_postgres_store = PostgresStore(
                    database=database.to_info(), types=(StoreType.SPATIAL_ENTITY,)
                )

    def stop(self) -> None:
        super().stop()

    async def wait_stopped(self) -> None:
        await super().wait_stopped()

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


def get_drive_bucket(space: Space) -> str:
    bucket_name = f"destack-{ENV.value}-{CLOUD.name.lower()}-{space.region.slug}-files"
    return bucket_name


def get_file_key(space: Space, sha256: str, name: str | None) -> str:
    """Gets the key for a file in the given bucket."""
    if name is None:
        return f"{space.id}/{sha256}/__UNNAMED__"
    else:
        return f"{space.id}/{sha256}/{name}"
