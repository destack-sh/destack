from collections.abc import AsyncIterator
from typing import TYPE_CHECKING, Any, Callable, override

import structlog
from opentelemetry import trace

from destack.grpc import Network, ServiceBase
from destack.language import (
    Client,
    Database,
    DatabaseInfo,
    IsSubject,
    NodeReference,
    NodeType,
    Oracle,
    Query,
    Session,
    Space,
    StoreKey,
)
from destack.proto import (
    AppendRequest,
    AppendResponse,
    QueryRequest,
    QueryResponse,
    RpcMetadata,
    ServiceKind,
    SpaceBase,
    SubscribeRequest,
    SubscribeResponse,
)
from destack.store import BufferedStore
from destack.utils.env import get_from_env
from destack.utils.uuid import UUID
from desys.sharding import DatabaseProvider, GalaxyProvider
from desys.store import PostgresEntityStore

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


class SpaceService(ServiceBase, SpaceBase):
    """
    Service for a Space. There is only one active SpaceService per Space at a time.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind
    name = "space"

    def __init__(
        self,
        id: str,
        space_id: UUID,
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
        self.space_id = space_id
        self.space_ptr = NodeReference(type=NodeType.SPACE, id=space_id, space_id=space_id)
        self.global_postgres_store = PostgresEntityStore(
            database=global_database, keys=(StoreKey.ENTITY_PRIMARY,)
        )
        self.spatial_postgres_store: PostgresEntityStore | None = None
        self.store: BufferedStore | None = None
        self.galaxy_provider = galaxy_provider
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
                self.spatial_postgres_store = PostgresEntityStore(
                    database=database.to_info(), keys=(StoreKey.ENTITY_PRIMARY,)
                )
                self.store = BufferedStore(self.global_postgres_store, self.spatial_postgres_store)
            else:
                self.store = BufferedStore(self.global_postgres_store)

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
        assert self.store is not None, f"no store ready in {self!r}"
        query = Query.from_proto(request.query)
        # query = transform_query(query, subject, client)
        query_result = await self.store.query(query)
        return QueryResponse(result=query_result.to_proto())

    @override
    async def append(
        self,
        request: AppendRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> AppendResponse:
        assert self.store is not None, f"no store ready in {self!r}"
        raise NotImplementedError
        # changes = [Change.from_proto(change) for change in request.changes]
        # approved_changes: list[Change] = []
        # nocheckin: access control (approve/reject/amend Queries & Changes, :RejectedEvents)
        # for _ in changes:
        #     pass
        # results = await self.store.commit(approved_changes)
        # return CommitResponse(results=[result.to_proto() for result in results])

    @override
    async def subscribe(
        self,
        request: SubscribeRequest,
        session: Session,
        subject: IsSubject | None,
        client: Client | None,
        metadata: RpcMetadata,
    ) -> AsyncIterator[SubscribeResponse]:
        assert self.store is not None, f"no store ready in {self!r}"
        query = Query.from_proto(request.query)
        async for update in self.store.subscribe(query):
            yield SubscribeResponse(update=update.to_proto())
