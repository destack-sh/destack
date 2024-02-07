import asyncio
import functools
import urllib
from datetime import datetime, timedelta
from typing import AsyncIterator, Callable
from uuid import UUID

import betterproto
import grpclib
import grpclib.server
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Expression, NodeReference, Organization, User
from bench.language.access import ReadOptions, adapt_read_options, Subject
from bench.language.const import IN_PACKAGE_NODE_TYPES, NodeType, SUB_BENCH_NODE_TYPES
from bench.language.node import Bench, Package
from bench.language.tree import NodeDataTree
from bench.proto import wiring
from bench.proto.services import BenchServiceBase, RpcCallable
from bench.proto.wire import (
    AggregateNodesRequest,
    AggregateNodesResponse,
    CommitEditsRequest,
    CommitEditsResponse,
    DownloadFilesRequest,
    DownloadFilesResponse,
    KillRunRequest,
    KillRunResponse,
    BenchHostBase,
    BenchHostStub,
    PushEditsRequest,
    PushEditsResponse,
    PushServerLogsRequest,
    ReadNodesRequest,
    ReadNodesResponse,
    RunProxyBlockRequest,
    RunProxyBlockResponse,
    SearchLogsRequest,
    SearchLogsResponse,
    SearchNodesRequest,
    SearchNodesResponse,
    StartRunRequest,
    StartRunResponse,
    UploadFilesRequest,
    UploadFilesResponse,
    WatchEditsRequest,
    WatchEditsResponse,
    WatchLogsRequest,
    WatchLogsResponse,
)
from bench.system.utils import detached_session, get_s3_client, validate_bench_data_many
from bench.settings import GLOBAL_PROJECT_BUCKET_NAME
from bench.sql.engine import pg_read_node
from bench.utils.func import to_uuid

logger = structlog.get_logger("package_host")

LOADED_SOURCE_TYPES: tuple[NodeType, ...] = tuple(
    nt
    for nt in IN_PACKAGE_NODE_TYPES
    if nt.id < NodeType.SESSION.id and nt not in (NodeType.RECORD,)
)


class BenchHostMultiplexer(BenchServiceBase, BenchHostBase):
    """
    Multiplexes requests per Bench to a BenchHost using gRPC metadata ('bench-id').
    Hosts are loaded for all active Benches; new ones 'ping' the multiplexer service to add themselves.
    """

    def __init__(self):
        super().__init__()
        self._bench_hosts: dict[UUID, "BenchHost"] = {}
        self._bench_hosts_lock = asyncio.Lock()

    def __str__(self):
        return "shards=*"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start_quick(self) -> None:
        async with detached_session():
            benches: list[Bench] = await Bench.tolist()
        await asyncio.gather(*(self._start_host(bench.id, bench.head_id) for bench in benches))

    def close(self) -> None:
        for host in self._bench_hosts.values():
            host.close()

    async def wait_closed(self) -> None:
        await asyncio.gather(*[host.wait_closed() for host in self._bench_hosts.values()])

    async def _start_host(self, bench_id: UUID) -> "BenchHost":
        host = BenchHost(bench_id)
        await host.start_quick()
        return host

    def _wrap_rpc_func(
        self, func: RpcCallable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        @functools.wraps(func)
        async def _multiplexed_rpc(subject: Subject, request: betterproto.Message) -> None:
            bench_id = getattr(request, "bench_id")
            if bench_id is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"missing bench scope")
            bench_id = to_uuid(bench_id)

            # get bench host
            host = self._bench_hosts.get(bench_id)
            if host is None:
                async with self._bench_hosts_lock:
                    # check again in case another request added it
                    host = self._bench_hosts.get(bench_id)
                    if host is None:
                        host = await self._start_host(bench_id)
                        self._bench_hosts[bench_id] = host

            # forward to host
            await getattr(host, method_name)(subject, request)

        return _multiplexed_rpc


class BenchHost(BenchServiceBase[BenchHostStub], BenchHostBase):
    """
    Host for an (active) Bench package. Manages basically everything that's not actually running it.
    Any client (frontend, server, ...) connects to this to do anything with the package.
    """

    def __init__(self, bench_id: UUID, package_id: UUID):
        super().__init__(loopback_stub_to=BenchHostStub)
        self.bench_id = bench_id
        self.package_id = package_id
        self._bench: Bench | None = None
        self._owner: User | Organization | None = None
        self._packages: dict[str, Package] = {}

    def __str__(self):
        return f"{self._package or self.package_id}"

    def __repr__(self):
        return f"<{self.__class__.__name} {self}>"

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
                options=ReadOptions(related_properties=(Bench.owner, Bench.head)),
            )
            self._owner = self._bench.owner
            # self._package: Package = await pg_read_node(
            #     session=session,
            #     root_type=NodeType.PACKAGE,
            #     root_id=self.package_id,
            #     descendant_types=LOADED_SOURCE_TYPES,
            #     parent=self.bench,
            # )

    #
    # General Bench IO for this package :BenchIO
    #

    async def read_nodes(
        self, subject: Subject, request: "ReadNodesRequest"
    ) -> "ReadNodesResponse":
        roots: tuple[NodeReference, ...] = tuple(wiring.unpack_struct(r) for r in request.roots)
        if any(root.node_type not in SUB_BENCH_NODE_TYPES for root in roots):
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "bench IO can't read global")
        options: ReadOptions = (
            wiring.unpack_struct_interp_maybe(request.options, self._package)
            or ReadOptions.default()
        )
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def search_nodes(
        self, subject: Subject, request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        node_type = wiring.unpack_enum(NodeType, request.node_type)
        if node_type not in SUB_BENCH_NODE_TYPES:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "bench IO can't read global")
        adapted_options = adapt_read_options(subject, node_type, ReadOptions.default())
        filter: Expression | None = wiring.unpack_struct_interp_maybe(request.filter, self._package)
        sort: list[Expression] = [
            wiring.unpack_struct_interp(s, self._package) for s in request.sort
        ] or None

        if request.node_type == NodeType.RECORD:
            raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        elif request.node_type in (NodeType.SESSION, NodeType.RUN, NodeType.PAUSE):
            raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        else:
            # we don't support generic server-side 'node search' yet
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"cannot search {request.node_type}")

    async def aggregate_nodes(
        self, subject: Subject, request: "AggregateNodesRequest"
    ) -> "AggregateNodesResponse":
        node_type: NodeType = wiring.unpack_enum(NodeType, request.type)
        if node_type not in SUB_BENCH_NODE_TYPES:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "bench IO can't read global")
        filter: Expression | None = wiring.unpack_struct_interp_maybe(request.filter, self._package)
        sort: list[Expression] = [
            wiring.unpack_struct_interp(s, self._package) for s in request.sort
        ] or None
        aggregation: Expression = wiring.unpack_struct_interp(request.aggregation, self._package)

        if request.node_type == NodeType.RECORD:
            raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        elif request.node_type in (NodeType.SESSION, NodeType.RUN, NodeType.PAUSE):
            raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        else:
            # we don't support generic server-side 'node search' yet
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"cannot aggregate {request.node_type}")

    async def commit_edits(
        self, subject: Subject, request: "CommitEditsRequest"
    ) -> "CommitEditsResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def watch_edits(
        self, subject: Subject, request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # Package-specific stuff
    #

    async def push_edits(
        self, subject: Subject, request: "PushEditsRequest"
    ) -> "PushEditsResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # Files
    #

    async def upload_files(
        self, subject: Subject, request: "UploadFilesRequest"
    ) -> "UploadFilesResponse":
        validate_bench_data_many(*request.files)
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
        validate_bench_data_many(*request.files)
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
    # Logs
    #

    async def search_logs(
        self, subject: Subject, request: "SearchLogsRequest"
    ) -> "SearchLogsResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def watch_logs(
        self, subject: Subject, request: "WatchLogsRequest"
    ) -> AsyncIterator["WatchLogsResponse"]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def push_server_logs(
        self, subject: Subject, request: "PushServerLogsRequest"
    ) -> "PushServerLogsRequest":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # Runs
    #

    async def start_run(self, subject: Subject, request: "StartRunRequest") -> "StartRunResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def kill_run(self, subject: Subject, request: "KillRunRequest") -> "KillRunResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def run_proxy_block(
        self, subject: Subject, request: "RunProxyBlockRequest"
    ) -> "RunProxyBlockResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
