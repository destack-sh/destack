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
from bench.language.bench import Machine, MachineProfile, ResourceStatus
from bench.language.connection import PostgresEngine, StoreEngine
from bench.language.const import (
    IN_BENCH_GLOBAL_NODE_TYPES,
    IN_BENCH_NODE_TYPES,
    LOCAL_NODE_TYPES,
    NodeType,
)
from bench.language.graph import NodeGraphLike, edit_graph
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
from bench.system.core import (
    GLOBAL_POSTGRES_ENGINE,
    GLOBAL_STORE,
    HostPlugin,
    HostSpec,
    global_session,
    unpack_commit,
)
from bench.system.graph import GraphIoServiceBase
from bench.system.provisioner import Provisioner, get_provisioners_for
from bench.system.scheduler import QueueRunPlugin
from bench.utils.dt import monotime
from bench.utils.func import bittuple, to_uuid
from bench.utils.utils import get_from_env_maybe

logger = structlog.get_logger(__name__)

LOCAL_MACHINE_URL = get_from_env_maybe("LOCAL_MACHINE_URL")
LOCAL_MACHINE = Machine(
    name="localhost",
    status=ResourceStatus.HEALTHY,
    connection_uri=LOCAL_MACHINE_URL,
    profile=MachineProfile.MEDIUM,
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
        """Starts a Host for the given Bench."""
        existing_host = self._hosts.get(bench_id)
        assert existing_host is None, f"already have Host for {bench_id}: {existing_host!r}"
        host = Host(bench_id)
        await host.start()
        self._hosts[bench_id] = host
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
    Host for a Bench, providing the OS-level functionality (lifecycle, resources, scheduling, etc.).
    There is only one Host per Bench. Clients interact with the Bench exclusively via its Host.
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
        self._engines: tuple[StoreEngine, ...] = ()
        self._main_package: Package | None = None
        self._packages: dict[UUID, Package] = {}
        self._session: Session | None = None

        self._provisioners: tuple[Provisioner, ...] = ()
        self._plugins: tuple[HostPlugin, ...] = ()  # incl. provisioners
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
    def on_error(self, source: HostPlugin, error: Exception) -> None:
        pass  # error is already reported, we just keep running

    @property
    def session(self) -> Session:
        assert self._session is not None, f"session not ready in {self!r}"
        return self._session

    @override
    def get_engines(self) -> tuple[StoreEngine, ...]:
        return self._engines

    @property
    def graphs(self) -> tuple[NodeGraphLike, ...]:
        assert self._bench is not None, f"bench not loaded in {self!r}"
        graphs = tuple(node._graph for node in (self._bench, *self._packages.values()))
        return graphs

    async def start(self) -> None:
        start = monotime()

        # load bench / main pcakages
        #  (in different session because we don't have the actual engines yet)
        async with self.new_session(engines=(GLOBAL_POSTGRES_ENGINE,)):
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
        self._bench._untrack_rec()

        # we open one Session for the entire lifecycle of the Host
        # TODO :Performance!: support in-memory engines in Host (from loaded graphs)
        self._engines = (self._global_pg_engine, self._local_pg_engine)
        self._session = Session(
            parent=None,
            _default_scope=self.scope,
            _engines=self._engines,
            _extend_commit_hook=self.extend_commit,
            _on_commit_hook=self.on_commit,
        )
        self._bench._track_rec(self._session)
        await self._session.open()

        # start plugins
        self._provisioners = tuple(get_provisioners_for(self, self._bench))
        self._plugins = (QueueRunPlugin(self, self._bench),) + self._provisioners
        await asyncio.gather(*(plugin.start(self._session) for plugin in self._plugins))
        await self._session.commit()
        # wait for plugins to finish processing any commits (and error early)
        await asyncio.gather(*(plugin.wait_step(timeout=10) for plugin in self._plugins))

        # preload main packages
        self._main_package = await PACKAGE_QUERY.get(id=self._bench.main_branch.main_package_id)
        self._packages[self._main_package.id] = self._main_package

        logger.info("host.start", host=self, plugins=self._plugins, duration=monotime() - start)

    def close(self) -> None:
        super().close()
        for plugin in self._plugins:
            plugin.close()

    async def wait_closed(self) -> None:
        await super().wait_closed()
        await asyncio.gather(*(plugin.wait_closed() for plugin in self._plugins))
        if self._session is not None:
            await self._session.close()

    @override
    async def extend_commit(
        self, graph: NodeGraphLike, edits: list[EditData], cascaded_edits: list[EditData]
    ) -> list[EditData]:
        # TODO :Incomplete: run plugins to extend commit

        # create signals
        ...

        # add logs
        # TODO :Incomplete: logs
        return []

    @override
    async def _on_commit(
        self, graph: NodeGraphLike, edits: list[EditData], cascaded_edits: list[EditData]
    ):
        assert self._session is not None, f"session not ready in {self!r}"
        assert self._bench is not None, f"bench not loaded in {self!r} for {edits!r}"

        # apply edits to loaded graphs (bench/package)
        self._session.suspend()  # don't trigger the edits we're just applying
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
        self._session.resume()

        # run plugins on commit (in main session)
        commit = unpack_commit(self.graphs + (graph,), edits, cascaded_edits)
        logger.debug("host.on_commit", host=self, commit=commit)
        for plugin in self._plugins:
            if commit.edited_types & plugin.watch_types:
                trimmed_commit = commit.trim_to(plugin.watch_types)
                await plugin.on_commit(self._session, trimmed_commit)
                logger.debug(
                    "host.on_commit.plugin", host=self, plugin=plugin, commit=trimmed_commit
                )
        await self._session.commit()

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
