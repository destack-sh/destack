import asyncio
import functools
from typing import Callable
from uuid import UUID

import betterproto
import grpclib.server
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import (
    Bench,
    Branch,
    Cache,
    Drive,
    Environment,
    Handle,
    Organization,
    Package,
    Server,
    Store,
    User,
)
from bench.language.access import Subject
from bench.language.const import IN_BENCH_NODE_TYPES, IN_PACKAGE_NODE_TYPES, NodeType
from bench.language.graph import filter_edits
from bench.language.query import PostgresEngine, StoreEngine
from bench.proto.services import BenchServiceBase, RpcCallable
from bench.proto.wire import (
    DownloadFilesRequest,
    DownloadFilesResponse,
    EditData,
    GraphScope,
    HostBase,
    RunIntrinsicBlockRequest,
    RunIntrinsicBlockResponse,
    UploadFilesRequest,
    UploadFilesResponse,
)
from bench.system.client import GLOBAL_STORE, global_session
from bench.system.graph import GraphIoService
from bench.system.resource import provision_pending_resources
from bench.utils.func import to_uuid

logger = structlog.get_logger(__name__)

LOADED_SOURCE_TYPES: tuple[NodeType, ...] = tuple(
    nt
    for nt in IN_PACKAGE_NODE_TYPES
    if nt.id < NodeType.SESSION.id and nt not in (NodeType.RECORD,)
)


class HostMultiplexer(BenchServiceBase, HostBase):
    """
    Multiplexes requests per Bench to a Host using gRPC metadata ('bench-id').
    Also provides some process-level shared functionality.
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

    async def start(self) -> None:
        async with global_session():
            benches: list[Bench] = await Bench.tolist()
        await asyncio.gather(*(self._start_host(bench.id) for bench in benches))

    def close(self) -> None:
        for host in self._hosts.values():
            host.close()

    async def wait_closed(self) -> None:
        await asyncio.gather(*[host.wait_closed() for host in self._hosts.values()])

    async def _start_host(self, bench_id: UUID) -> "Host":
        host = Host(bench_id)
        await host.start()
        return host

    def _wrap_rpc_func(
        self, func: RpcCallable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        _, cardinality, request_type, reply_type = handler

        async def _get_host(request: betterproto.Message) -> Host:
            """Gets or starts a running Host for the given Bench"""

            # get request's bench id
            scope: GraphScope | None = getattr(request, "scope")
            if scope is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing scope")
            bench_id = to_uuid(scope.bench_id)
            if bench_id is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing bench scope id")

            # get host
            host = self._hosts.get(bench_id)
            if host is None:
                async with self._hosts_lock:
                    host = self._hosts.get(bench_id)
                    if host is None:
                        host = await self._start_host(bench_id)
                        self._hosts[bench_id] = host
            return host

        if cardinality == grpclib.const.Cardinality.UNARY_UNARY:

            @functools.wraps(func)
            async def _multiplexed_unary_rpc(
                subject: Subject, request: betterproto.Message
            ) -> None:
                host = await _get_host(request)
                return await getattr(host, method_name)(subject, request)

            return _multiplexed_unary_rpc

        elif cardinality == grpclib.const.Cardinality.UNARY_STREAM:

            @functools.wraps(func)
            async def _multiplexed_unary_stream_rpc(
                subject: Subject, request: betterproto.Message
            ) -> None:
                host = await _get_host(request)
                async for response in getattr(host, method_name)(subject, request):
                    yield response

            return _multiplexed_unary_stream_rpc

        else:
            raise NotImplementedError(f"unexpected cardinality in {method_name}: {cardinality}")


class Host(GraphIoService, HostBase):
    """
    Host for a Bench, providing the OS-level functions (lifecycle, resources & runtime management)..
    Clients interact with a Bench exclusively through its Host.
    """

    def __init__(self, bench_id: UUID):
        GraphIoService.__init__(self, bench_id=bench_id, node_types=IN_BENCH_NODE_TYPES)
        self.bench_id = bench_id
        self._bench: Bench | None = None
        self._bench_scope: GraphScope = GraphScope(bench_id=str(bench_id))
        self._bench_pg_engine = PostgresEngine(
            GLOBAL_STORE, scope=self._bench_scope, node_types=IN_BENCH_NODE_TYPES
        )
        self._owner: User | Organization | None = None
        self._main_package: Package | None = None
        self._packages: dict[UUID, Package] = {}

    def __str__(self):
        return f"{self._bench or self.bench_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def bench(self) -> Bench:
        assert self._bench is not None, f"bench not loaded in {self}"
        return self._bench

    @property
    def main_store(self) -> Store:
        return self.bench.main_environment.store

    @property
    def engines(self) -> tuple[StoreEngine, ...]:
        # TODO :Broken :Performance: use local in memory engines in Host (where possible)
        #  also provide & use bench-specific store engines
        return (self._bench_pg_engine,)

    async def start(self) -> None:
        async with global_session() as session:
            start = asyncio.get_event_loop().time()
            self._bench = (
                await Bench.descendants(
                    Handle, Server, Store, Cache, Drive, Environment, Branch, Package
                )
                .include_all()
                .get(id=self.bench_id)
            )
            await provision_pending_resources(self._bench, session)
            await session.commit()
            logger.info("host.start", host=self, duration=asyncio.get_event_loop().time() - start)

    def close(self) -> None:
        pass

    async def wait_closed(self) -> None:
        pass

    def _on_graph_edited_inner(self, scopes: list[GraphScope], edits: list[EditData]):
        # TODO :Incomplete: re-interp packages after edit (update notices, ...?)
        # apply edits to the nodes we have loaded
        for scope in scopes:
            if scope.package_id is not None:
                root = self._packages[to_uuid(scope.package_id)]
            else:
                root = self._bench
            edits = filter_edits(root._read_options, edits)
            root._apply_edits(edits)

    #
    # Files
    #

    async def upload_files(
        self, subject: Subject, request: "UploadFilesRequest"
    ) -> "UploadFilesResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def download_files(
        self, subject: Subject, request: "DownloadFilesRequest"
    ) -> "DownloadFilesResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # Runs
    #

    async def run_intrinsic_block(
        self, subject: Subject, request: "RunIntrinsicBlockRequest"
    ) -> "RunIntrinsicBlockResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
