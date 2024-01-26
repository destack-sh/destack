import asyncio
import functools
import urllib
from datetime import datetime, timedelta
from typing import AsyncIterator, Callable
from uuid import UUID

import grpclib
import grpclib.server
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language.const import IN_PACKAGE_NODE_TYPES, NodeType
from bench.language.node import Bench, Package
from bench.language.tree import NodeDataTree
from bench.proto import wire
from bench.proto.services import BenchServiceBase
from bench.proto.wire import (
    AggregateNodesRequest,
    AggregateNodesResponse,
    CommitEditsRequest,
    CommitEditsResponse,
    DownloadFilesRequest,
    DownloadFilesResponse,
    KillRunRequest,
    KillRunResponse,
    PackageHostBase,
    PasteNodesRequest,
    PasteNodesResponse,
    PushEditsRequest,
    PushEditsResponse,
    PushWorkerLogsRequest,
    ReadNodesRequest,
    ReadNodesResponse,
    RunProxyBlockRequest,
    RunProxyBlockResponse,
    SearchLogsRequest,
    SearchLogsResponse,
    SearchNodesRequest,
    SearchNodesResponse,
    SnapshotPackageRequest,
    SnapshotPackageResponse,
    StartRunRequest,
    StartRunResponse,
    UploadFilesRequest,
    UploadFilesResponse,
    WatchEditsRequest,
    WatchEditsResponse,
    WatchLogsRequest,
    WatchLogsResponse,
    PackageHostStub,
)
from bench.server.utils import (
    check_authenticated,
    check_authenticated_worker,
    detached_session,
    get_s3_client,
    validate_bench_data_many,
)
from bench.settings import GLOBAL_PROJECT_BUCKET_NAME
from bench.sql.engine import pg_read_node
from bench.utils.func import to_uuid

logger = structlog.get_logger("package_host")

LOADED_SOURCE_TYPES: tuple[NodeType, ...] = tuple(
    nt
    for nt in IN_PACKAGE_NODE_TYPES
    if nt.id < NodeType.SESSION.id and nt not in (NodeType.RECORD,)
)


class PackageHostMultiplexer(BenchServiceBase, PackageHostBase):
    """
    Multiplexes requests per package to a PackageHost using gRPC metadata ('bench-id' and 'package-id').
    Hosts are loaded for all active packages; new ones 'ping' the multiplexer to add themselves.
    """

    def __init__(self):
        super().__init__()
        self._hosts_by_package_id: dict[UUID, "PackageHost"] = {}

    def __str__(self):
        return "shards=*"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start_quick(self) -> None:
        async with detached_session():
            benches: list[Bench] = await Bench.tolist()
        await asyncio.gather(*(self._start_host(bench.id, bench.head_id) for bench in benches))

    def close(self) -> None:
        for host in self._hosts_by_package_id.values():
            host.close()

    async def wait_closed(self) -> None:
        await asyncio.gather(*[host.wait_closed() for host in self._hosts_by_package_id.values()])

    async def _start_host(self, bench_id: UUID, package_id: UUID) -> "PackageHost":
        host = PackageHost(bench_id, package_id)
        await host.start_quick()
        return host

    def _wrap_rpc_func(
        self, func: Callable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        @functools.wraps(func)
        async def proxied_method(stream: grpclib.server.Stream) -> None:
            bench_id = to_uuid(self.metadata.bench_id)
            package_id = to_uuid(self.metadata.package_id)

            # get package host
            host = self._hosts_by_package_id.get(package_id)
            if host is None:
                host = await self._start_host(bench_id, package_id)
                self._hosts_by_package_id[package_id] = host

            # forward to host
            host._stream.set(stream)
            host._metadata.set(self.metadata)
            await getattr(host, method_name)(stream)

        return proxied_method


class PackageHost(BenchServiceBase[PackageHostStub], PackageHostBase):
    """
    Host for an (active) Bench package. Manages basically everything that's not actually running it.
    Any client (frontend, worker, ...) connects to this to do anything with the package.
    """

    def __init__(self, bench_id: UUID, package_id: UUID):
        super().__init__(loopback_stub_to=PackageHostStub)
        self.bench_id = bench_id
        self.package_id = package_id
        self._bench: Bench | None = None
        self._package: Package | None = None

    def __str__(self):
        return f"{self._package or self.package_id}"

    def __repr__(self):
        return f"<PackageHost {self}>"

    @property
    def package_source(self) -> NodeDataTree:
        assert self._package is not None, f"package not loaded in {self}"
        return self._package._source

    @property
    def bench(self) -> Bench:
        assert self._bench is not None, f"bench not loaded in {self}"
        return self._bench

    @property
    def package(self) -> Package:
        assert self._package is not None, f"package not loaded in {self}"
        return self._package

    async def start_quick(self) -> None:
        async with detached_session() as session:
            self._bench: Bench = await pg_read_node(
                session=session,
                root_type=NodeType.BENCH,
                root_id=self.bench_id,
                descendant_types=(NodeType.BADGE,),
            )
            self._package: Package = await pg_read_node(
                session=session,
                root_type=NodeType.PACKAGE,
                root_id=self.package_id,
                descendant_types=LOADED_SOURCE_TYPES,
                parent=self.bench,
            )

    #
    # General Bench IO for this package and global nodes :BenchIO
    #

    async def read_nodes(self, read_nodes_request: "ReadNodesRequest") -> "ReadNodesResponse":
        if read_nodes_request.node_type == wire.NodeType.RECORD:
            # forward to record query (stored custom)
            raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        async with detached_session(read_only=True):
            await check_authenticated(self.metadata)
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def search_nodes(
        self, search_nodes_request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        if search_nodes_request.node_type == wire.NodeType.RECORD:
            # forward to record query (stored custom)
            raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        else:
            pass
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def aggregate_nodes(
        self, aggregate_nodes_request: "AggregateNodesRequest"
    ) -> "AggregateNodesResponse":
        if aggregate_nodes_request.node_type == wire.NodeType.RECORD:
            # forward to record query (stored custom)
            raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def commit_edits(
        self, commit_edits_request: "CommitEditsRequest"
    ) -> "CommitEditsResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def watch_edits(
        self, watch_edits_request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # Package-specific stuff
    #

    async def push_edits(self, push_edits_request: "PushEditsRequest") -> "PushEditsResponse":
        _ = await check_authenticated_worker(self.metadata)
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def paste_nodes(self, paste_nodes_request: "PasteNodesRequest") -> "PasteNodesResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def snapshot(
        self, snapshot_package_request: "SnapshotPackageRequest"
    ) -> "SnapshotPackageResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # Files
    #

    async def upload_files(
        self, upload_files_request: "UploadFilesRequest"
    ) -> "UploadFilesResponse":
        validate_bench_data_many(*upload_files_request.files)
        expires_in = 60 * 60  # 1 hour
        presigned_urls: list[str] = []
        for file in upload_files_request.files:
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
        self, download_files_request: "DownloadFilesRequest"
    ) -> "DownloadFilesResponse":
        validate_bench_data_many(*download_files_request.files)
        expires_in = 60 * 60  # 1 hour
        presigned_urls: list[str] = []
        for file in download_files_request.files:
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
    # Logs
    #

    async def search_logs(self, search_logs_request: "SearchLogsRequest") -> "SearchLogsResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def watch_logs(
        self, watch_logs_request: "WatchLogsRequest"
    ) -> AsyncIterator["WatchLogsResponse"]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def push_worker_logs(
        self, push_worker_logs_request: "PushWorkerLogsRequest"
    ) -> "PushWorkerLogsRequest":
        _ = await check_authenticated_worker(self.metadata)
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # Runs
    #

    async def start_run(self, start_run_request: "StartRunRequest") -> "StartRunResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def kill_run(self, kill_run_request: "KillRunRequest") -> "KillRunResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def run_proxy_block(
        self, run_proxy_block_request: "RunProxyBlockRequest"
    ) -> "RunProxyBlockResponse":
        _ = await check_authenticated_worker(self.metadata)
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
