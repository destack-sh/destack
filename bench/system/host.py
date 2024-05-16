import asyncio
import functools
from contextlib import asynccontextmanager
from typing import AsyncIterator, Callable
from uuid import UUID

import betterproto
import grpclib.server
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Bench, Organization, Package, Subject, User
from bench.language.connection import PostgresEngine, StoreEngine
from bench.language.const import IN_BENCH_NODE_TYPES, NodeType
from bench.language.graph import NodeGraph, edit_graph
from bench.language.session import Session
from bench.proto import wiring
from bench.proto.services import BenchServiceBase, RpcCallable
from bench.proto.wire import (
    DownloadFilesRequest,
    DownloadFilesResponse,
    EditData,
    GraphScope,
    HostBase,
    UploadFilesRequest,
    UploadFilesResponse,
)
from bench.system.client import GLOBAL_STORE, global_session
from bench.system.graph import GraphIoServiceBase
from bench.system.neon import NEON_LOCAL, prepare_local_stores
from bench.system.resource import migrate_local_stores, provision_pending_resources
from bench.utils.func import to_uuid

logger = structlog.get_logger(__name__)


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
            async def _multiplexed_unary_stream_rpc(subject: Subject, request: betterproto.Message):
                host = await _get_host(request)
                async for response in getattr(host, method_name)(subject, request):
                    yield response

            return _multiplexed_unary_stream_rpc

        else:
            raise NotImplementedError(f"unexpected cardinality in {method_name}: {cardinality}")


LOADED_BENCH_NODE_TYPES: tuple[NodeType, ...] = (
    NodeType.HANDLE,
    NodeType.SERVER,
    NodeType.STORE,
    NodeType.ENVIRONMENT,
    NodeType.BRANCH,
    NodeType.PACKAGE,
)
LOADED_PACKAGE_NODE_TYPES: tuple[NodeType, ...] = (
    NodeType.DEPENDENCY,
    NodeType.UPGRADE,
    NodeType.SPACE,
    NodeType.LINK,
    NodeType.NOTICE,
    NodeType.BLOCK,
    NodeType.TRIGGER,
    NodeType.FIELD,
    NodeType.QUERY,
    NodeType.STEP,
    NodeType.VIEW,
)
BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).include_all()
PACKAGE_QUERY = (
    Package.descendants(*LOADED_PACKAGE_NODE_TYPES)
    .ancestors(Bench)
    .include_all()
    .exclude(Bench.encryption_key)
)


class Host(GraphIoServiceBase, HostBase):
    """
    Host for a Bench, providing the OS-level functions (lifecycle, resources & runtime management).
    Clients interact with a Bench exclusively through its Host.
    """

    def __init__(self, bench_id: UUID):
        GraphIoServiceBase.__init__(self, bench_id=bench_id, node_types=IN_BENCH_NODE_TYPES)
        self.bench_id = bench_id
        self._bench: Bench | None = None
        self._bench_scope: GraphScope = GraphScope(bench_id=str(bench_id))
        self._bench_pg_engine: PostgresEngine | None = None
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
    def engines(self) -> tuple[StoreEngine, ...]:
        # TODO :Broken :Performance!: support local engines in Host (in-memory from local data graph)
        assert self._bench_pg_engine is not None, f"pg engine not initialized in {self!r}"
        return (self._bench_pg_engine,)

    async def start(self) -> None:
        start = asyncio.get_event_loop().time()

        # load bench
        async with global_session() as session:
            self._bench = await BENCH_QUERY.get(id=self.bench_id)
            self._bench_pg_engine = PostgresEngine(
                store=GLOBAL_STORE, bench=self._bench, node_types=IN_BENCH_NODE_TYPES
            )
            assert self._bench.main_branch is not None, f"{self._bench!r} has no main branch"
            await provision_pending_resources(self._bench, session, commit_per=True)

            # prepare/migrate resources
            if NEON_LOCAL:
                await prepare_local_stores(self._bench)
            # NOTE :Robustness: unsure when to migrate local stores :StoreMigration
            await migrate_local_stores(self._bench, session)
            await session.commit()
        session.untrack_many(self._bench)

        # preload main packages
        async with local_session(engines=(self._bench_pg_engine,)):
            self._main_package = await PACKAGE_QUERY.get(id=self._bench.main_branch.main_package_id)
            self._packages[self._main_package.id] = self._main_package
        session.untrack_many(self._main_package)

        logger.info("host.start", host=self, duration=asyncio.get_event_loop().time() - start)

    def close(self) -> None:
        pass

    async def wait_closed(self) -> None:
        pass

    def _on_graph_edited(self, scopes: tuple[GraphScope, ...], edits: list[EditData]):
        if self._bench is None:
            return  # not started yet

        # apply edits to loaded graphs (bench/package)
        for edit in edits:
            node_data = wiring.unwrap_some_node(edit.node)
            if hasattr(node_data, "package_ptr"):
                package_id = to_uuid(getattr(node_data, "package_ptr").id)
                assert package_id is not None, f"missing package id in {edit!r}"
                package = self._packages.get(package_id)
                if package is None:
                    continue
                graph = package._graph
                options = PACKAGE_QUERY._options
            else:
                graph = self._bench._graph
                options = BENCH_QUERY._options

            assert isinstance(graph, NodeGraph), f"unexpected graph type: {graph!r}"
            edit_graph(graph, (edit,), options)

        # TODO :Incomplete: re-interp packages for edit (update notices, fire signals, ...)

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


@asynccontextmanager
async def local_session(engines: tuple[StoreEngine, ...]) -> AsyncIterator[Session]:
    """Session for local operations (no remote calls)."""
    session = Session(_engines=engines)
    async with session:
        yield session
