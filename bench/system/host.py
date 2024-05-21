import asyncio
import functools
from typing import Callable, override
from uuid import UUID

import betterproto
import grpclib.server
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Bench, Package, Run, Subject
from bench.language.bench import Machine, ResourceStatus, ServerProfile
from bench.language.connection import PostgresEngine, StoreEngine
from bench.language.const import (
    IN_BENCH_GLOBAL_NODE_TYPES,
    IN_BENCH_NODE_TYPES,
    LOCAL_NODE_TYPES,
    NodeType,
)
from bench.language.graph import NodeGraph, NodeGraphLike, edit_graph
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
from bench.system.core import (
    GLOBAL_STORE,
    HostPlugin,
    HostSpec,
    global_session,
    local_session,
    unpack_committed_change,
)
from bench.system.graph import GraphIoServiceBase
from bench.system.provision import (
    Provisioner,
    get_provisioners_for,
    migrate_resources,
    provision_resources,
)
from bench.system.scheduling import RunPlugin
from bench.utils.func import bittuple, to_uuid
from bench.utils.utils import get_from_env_maybe

logger = structlog.get_logger(__name__)

LOCAL_MACHINE_URL = get_from_env_maybe("LOCAL_MACHINE_URL")
LOCAL_MACHINE = Machine(
    name="localhost",
    status=ResourceStatus.HEALTHY,
    connection_uri=LOCAL_MACHINE_URL,
    profile=ServerProfile.LARGE,
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
            async def _multiplexed_unary_stream_rpc(subject: Subject, request: betterproto.Message):
                host = await _get_host(request)
                async for response in getattr(host, method_name)(subject, request):
                    yield response

            return _multiplexed_unary_stream_rpc

        else:
            raise NotImplementedError(f"unexpected cardinality in {method_name}: {cardinality}")


LOADED_BENCH_NODE_TYPES: bittuple[NodeType] = bittuple(
    NodeType.HANDLE,
    NodeType.SERVER,
    NodeType.CLIENT,
    NodeType.MACHINE,
    NodeType.STORE,
    NodeType.ENVIRONMENT,
    NodeType.BRANCH,
    NodeType.PACKAGE,
)
LOADED_PACKAGE_NODE_TYPES: bittuple[NodeType] = bittuple(
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
LOADED_NODE_TYPES = LOADED_BENCH_NODE_TYPES | LOADED_PACKAGE_NODE_TYPES
BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.descendants(*LOADED_PACKAGE_NODE_TYPES)
    .ancestors(Bench)
    .select_all()
    .exclude(Bench.encryption_key)
)


class Host(GraphIoServiceBase, HostBase, HostSpec):
    """
    Host for a Bench, providing the OS-level functions (lifecycle, resources & runtime management).
    Clients interact with a Bench exclusively through its Host.
    """

    def __init__(self, bench_id: UUID):
        GraphIoServiceBase.__init__(self, bench_id=bench_id, node_types=IN_BENCH_NODE_TYPES)

        self.bench_id = bench_id
        self._bench: Bench | None = None
        self._scope: GraphScope = GraphScope(bench_id=str(bench_id))
        self._global_pg_engine: PostgresEngine | None = None
        # NOTE: currently we only have one local engine because we only have one branch :Branching
        #  but later we'll need different engines for every 'full' branch (separate Neon branch)
        self._local_pg_engine: PostgresEngine | None = None
        self._main_package: Package | None = None
        self._packages: dict[UUID, Package] = {}

        self._provisioners: tuple[Provisioner, ...] = ()
        self._plugins: tuple[HostPlugin, ...] = ()
        self._runs_to_queue: asyncio.Queue[Run] = asyncio.Queue()

    def __str__(self):
        return f"{self._bench or self.bench_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def bench(self) -> Bench:
        assert self._bench is not None, f"bench not loaded in {self}"
        return self._bench

    @override
    def get_package(self, package_id: UUID) -> Package | None:
        return self._packages.get(package_id)

    @override
    def _get_engines(self, scope: GraphScope) -> tuple[StoreEngine, ...]:
        # TODO :Performance!: support in-memory engines in Host (from local data graph)
        assert self._global_pg_engine is not None, f"global pg engine not initialized in {self!r}"
        assert self._local_pg_engine is not None, f"local pg engine not initialized in {self!r}"
        return (self._global_pg_engine, self._local_pg_engine)

    async def start(self) -> None:
        start = asyncio.get_event_loop().time()

        async with global_session(self.on_graph_commit) as session:
            # load bench
            self._bench = await BENCH_QUERY.get(id=self.bench_id)
            self._global_pg_engine = PostgresEngine(
                store=GLOBAL_STORE,
                bench=self._bench,
                scope=self._scope,
                node_types=IN_BENCH_GLOBAL_NODE_TYPES,
            )
            assert self._bench.main_environment, f"{self._bench!r} has no main environment"
            assert self._bench.main_branch, f"{self._bench!r} has no main branch"
            self._local_pg_engine = PostgresEngine(
                store=self._bench.main_environment.store,
                bench=self._bench,
                scope=self._scope,
                node_types=LOCAL_NODE_TYPES,
            )

            # prepare plugins
            self._provisioners = tuple(get_provisioners_for(self, self._bench))
            self._plugins = (RunPlugin(self, self._bench),) + self._provisioners
            await asyncio.gather(*(plugin.start() for plugin in self._plugins))

            # auto-provision
            # NOTE: we provision manually here (instead of in Provisioner plugins)
            #  because we may edit the resources manually or 'offline'.
            # Also, we manually migrate here on Host start because not sure where else to do it.
            await provision_resources(self._bench.resources, self._provisioners, session)
            await migrate_resources(self._bench.resources, self._provisioners, session)

            await session.commit()
        session.untrack_many(self._bench)

        # preload main packages
        async with local_session(self._scope, (self._global_pg_engine,)):
            self._main_package = await PACKAGE_QUERY.get(id=self._bench.main_branch.main_package_id)
            self._packages[self._main_package.id] = self._main_package
        session.untrack_many(self._main_package)

        # start tasks
        ...

        logger.info(
            "host.start",
            host=self,
            plugins=self._plugins,
            duration=asyncio.get_event_loop().time() - start,
        )

    def close(self) -> None:
        super().close()
        for plugin in self._plugins:
            plugin.close()

    async def wait_closed(self) -> None:
        await super().wait_closed()
        await asyncio.gather(*(plugin.wait_closed() for plugin in self._plugins))

    @override
    def _amend_graph_commit(self, graph: NodeGraph, edits: list[EditData]) -> list[EditData]:
        # nocheckin: create Logs for edits (but how/where/when to compact?)
        return edits

    @override
    def _on_graph_commit(
        self, graph: NodeGraphLike, edits: list[EditData], cascaded_edits: list[EditData]
    ):
        assert self._bench is not None, f"bench not loaded in {self!r} for {edits!r}"

        # apply edits to loaded graphs (bench/package)
        for edit in edits:
            if NodeType(edit.node_type) not in LOADED_NODE_TYPES:
                continue  # not loaded
            if edit.origin is None:
                continue  # origin is us (=Host)
            node_data = wiring.unwrap_some_node(edit.node)
            if hasattr(node_data, "package_ptr"):
                package_id = to_uuid(getattr(node_data, "package_ptr").id)
                assert package_id is not None, f"missing package id in {edit!r}"
                package = self._packages.get(package_id)
                assert package is not None, f"package not loaded for edit {edit!r}"
                edited_graph = package._graph
                options = PACKAGE_QUERY._options
            else:
                edited_graph = self._bench._graph
                options = BENCH_QUERY._options
            edit_graph(edited_graph, (edit,), options)

        # feed commit to plugins
        commit = unpack_committed_change(graph, edits, cascaded_edits)
        logger.debug("host.on_commit", host=self, commit=commit)
        for plugin in self._plugins:
            if commit.edited_types & plugin.watch_types:
                trimmed_commit = commit.trim_to(plugin.watch_types)
                plugin.on_graph_commit(trimmed_commit)
                logger.debug(
                    "host.on_commit.plugin", host=self, plugin=plugin, commit=trimmed_commit
                )

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
