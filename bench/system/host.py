import asyncio
import functools
import urllib
from datetime import datetime, timedelta
from typing import Callable, Collection, Optional
from uuid import UUID

import betterproto
import grpclib.server
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import (
    Bench,
    NodeReference,
    Organization,
    Package,
    User,
    Handle,
    Server,
    Store,
    Cache,
    Drive,
    Branch,
)
from bench.language.access import Subject
from bench.language.const import IN_BENCH_NODE_TYPES, IN_PACKAGE_NODE_TYPES, NodeType
from bench.language.file import GLOBAL_PROJECT_BUCKET_NAME
from bench.language.graph import NodeDataGraph, NodeGraph
from bench.proto.services import BenchServiceBase, RpcCallable
from bench.proto.wire import (
    DownloadFilesRequest,
    DownloadFilesResponse,
    EditData,
    GraphScope,
    HostBase,
    HostStub,
    RunIntrinsicBlockRequest,
    RunIntrinsicBlockResponse,
    UploadFilesRequest,
    UploadFilesResponse,
)
from bench.system.graph import GraphIoService
from bench.system.resource import get_s3_client
from bench.system.utils import global_session
from bench.utils.func import to_uuid

logger = structlog.get_logger("package_host")

LOADED_SOURCE_TYPES: tuple[NodeType, ...] = tuple(
    nt
    for nt in IN_PACKAGE_NODE_TYPES
    if nt.id < NodeType.SESSION.id and nt not in (NodeType.RECORD,)
)


class HostMultiplexer(BenchServiceBase, HostBase):
    """
    Multiplexes requests per Bench to a Host using gRPC metadata ('bench-id').
    Hosts are loaded for all active Benches; new ones 'ping' the multiplexer service to add themselves.
    """

    def __init__(self):
        super().__init__()
        self._hosts: dict[UUID, "Host"] = {}
        self._hosts_lock = asyncio.Lock()

    def __str__(self):
        return "shards=[*]"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start_quick(self) -> None:
        async with global_session():
            benches: list[Bench] = await Bench.tolist()
        await asyncio.gather(*(self._start_host(bench.id, bench.head_id) for bench in benches))

    def close(self) -> None:
        for host in self._hosts.values():
            host.close()

    async def wait_closed(self) -> None:
        await asyncio.gather(*[host.wait_closed() for host in self._hosts.values()])

    async def _start_host(self, bench_id: UUID) -> "Host":
        host = Host(bench_id)
        await host.start_quick()
        return host

    def _wrap_rpc_func(
        self, func: RpcCallable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        @functools.wraps(func)
        async def _multiplexed_rpc(subject: Subject, request: betterproto.Message) -> None:
            scope: GraphScope | None = getattr(request, "scope")
            if scope is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing bench scope")
            bench_id = to_uuid(scope.bench_id)

            # get bench host
            host = self._hosts.get(bench_id)
            if host is None:
                async with self._hosts_lock:
                    # check again in case another request added it
                    host = self._hosts.get(bench_id)
                    if host is None:
                        host = await self._start_host(bench_id)
                        self._hosts[bench_id] = host

            # forward to host
            await getattr(host, method_name)(subject, request)

        return _multiplexed_rpc


class Host(BenchServiceBase[HostStub], HostBase, GraphIoService):
    """
    Host for a Bench, providing the OS-level functions (lifecycle, resources & runtime management)..
    Clients interact with a Bench exclusively through its Host.
    """

    def __init__(self, bench_id: UUID):
        BenchServiceBase.__init__(self, loopback_stub_to=HostStub)
        GraphIoService.__init__(self, bench_id=bench_id, node_types=IN_BENCH_NODE_TYPES)
        self.bench_id = bench_id
        self._bench: Bench | None = None
        self._owner: User | Organization | None = None
        self._packages: dict[str, Package] = {}

    def __str__(self):
        return f"{self._bench or self.bench_id}"

    def __repr__(self):
        return f"<{self.__class__.__name} {self}>"

    @property
    def bench(self) -> Bench:
        assert self._bench is not None, f"bench not loaded in {self}"
        return self._bench

    async def start_quick(self) -> None:
        async with global_session():
            self._bench = await Bench.descendants(
                Handle, Server, Store, Cache, Drive, Branch, Package
            ).get(id=self.bench_id)

    def _on_graph_edited_inner(self, edits: list[EditData], scopes: Collection[NodeReference]):
        # TODO :Broken: apply edits to loaded data & live nodes
        # TODO :Incomplete: re-interp packages after edit (update notices, ...)
        ...

    #
    # Files
    #

    async def upload_files(
        self, subject: Subject, request: "UploadFilesRequest"
    ) -> "UploadFilesResponse":
        expires_in = 60 * 60  # 1 hour
        presigned_urls: list[str] = []
        for file in request.files:
            presigned = get_s3_client().generate_presigned_post(
                Bucket=GLOBAL_PROJECT_BUCKET_NAME,
                Key=f"{file.id}/{file.name}",
                ExpiresIn=expires_in,  # 1 hour
                Fields={},
            )
            if "url" not in presigned:
                raise RuntimeError(f"failed to generate presigned post for {self}: {presigned}")
            # encode the url as a string (with parameters)
            encoded_params = urllib.parse.urlencode(presigned["fields"])
            encoded_url = f"{presigned['url']}?{encoded_params}"
            presigned_urls.append(encoded_url)
        expires_at = datetime.utcnow() + timedelta(seconds=expires_in)
        return UploadFilesResponse(post_urls=presigned_urls, expires_at=expires_at)

    async def download_files(
        self, subject: Subject, request: "DownloadFilesRequest"
    ) -> "DownloadFilesResponse":
        expires_in = 60 * 60  # 1 hour
        presigned_urls: list[str] = []
        for file in request.files:
            get_url = get_s3_client().generate_presigned_url(
                ClientMethod="get_object",
                Params={
                    "Bucket": GLOBAL_PROJECT_BUCKET_NAME,
                    "Key": f"{file.id}/{file.name}",
                },
                ExpiresIn=expires_in,
            )
            presigned_urls.append(get_url)
        expires_at = datetime.utcnow() + timedelta(seconds=expires_in)
        return DownloadFilesResponse(get_urls=presigned_urls, expires_at=expires_at)

    #
    # Runs
    #

    async def run_intrinsic_block(
        self, subject: Subject, request: "RunIntrinsicBlockRequest"
    ) -> "RunIntrinsicBlockResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
