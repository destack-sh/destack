import asyncio
from typing import TYPE_CHECKING, Any, AsyncIterator, Callable, Mapping, override

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language import CLOUD, Bench, Database, Edit, NodeReference, NodeType, Query, Scope
from bench.pb2 import (
    CommitRequest,
    CommitResponse,
    DownloadFilesRequest,
    DownloadFilesResponse,
    HostBase,
    QueryRequest,
    QueryResponse,
    ServiceKind,
    SubscribeRequest,
    SubscribeResponse,
    UploadFilesRequest,
    UploadFilesResponse,
)
from bench.proto import Network, ServiceBase
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
        global_database: Database,
        regional_database: Database,
        network: Network,
        oracle: Oracle,
        on_error: Callable[[BaseException], None] | None,
    ):
        self.bench_id = bench_id
        self.bench_ptr = NodeReference(
            node_type=NodeType.BENCH, id=bench_id, ck=bench_id, bench_id=bench_id
        )
        self.scope = Scope(bench_id=bench_id).to_proto()
        self.provisioners: tuple[Provisioner, ...] = ()
        self.plugins: tuple[HostPlugin, ...] = ()  # incl. provisioners
        self.store: Store = None

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
        raise NotImplementedError

    def stop(self) -> None:
        super().stop()
        for plugin in self.plugins:
            plugin.close()

    async def wait_stopped(self) -> None:
        await super().wait_stopped()
        await asyncio.gather(*(plugin.wait_closed() for plugin in self.plugins))

    @override
    async def query(self, request: QueryRequest, headers: Mapping) -> QueryResponse:
        query = Query.from_proto(request.query)
        query.validate()
        query_result = await self.store.query(query)
        return QueryResponse(result=query_result.to_proto())

    @override
    async def subscribe(
        self, request: SubscribeRequest, headers: Mapping
    ) -> AsyncIterator[SubscribeResponse]:
        query = Query.from_proto(request.query)
        query.validate()
        async for update in await self.store.subscribe(query):
            yield SubscribeResponse(update=update.to_proto())

    @override
    async def commit(self, request: CommitRequest, headers: Mapping) -> CommitResponse:
        edits = [Edit.from_proto(edit) for edit in request.edits]
        edits, cascaded_edits = await self.store.commit(edits)
        return CommitResponse(edits=edits, cascaded_edits=cascaded_edits)

    @override
    async def upload_files(
        self, request: UploadFilesRequest, headers: Mapping
    ) -> UploadFilesResponse:
        raise NotImplementedError

    @override
    async def download_files(
        self, request: DownloadFilesRequest, headers: Mapping
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
