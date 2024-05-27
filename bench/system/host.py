import asyncio
import functools
from contextlib import asynccontextmanager
from itertools import chain
from typing import Callable, cast, override
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
from bench.language.expression import NodeReference
from bench.language.graph import NodeGraphLike, edit_graph
from bench.language.log import Log
from bench.language.property import Property
from bench.language.query import NodeNotFoundError
from bench.language.session import Session
from bench.proto import wire, wiring
from bench.proto.services import BenchServiceBase, RpcCallable
from bench.proto.wire import EditData, GraphScope, HostBase, LogData, ServiceKind
from bench.system.core import (
    BENCH_QUERY,
    GLOBAL_POSTGRES_ENGINE,
    GLOBAL_STORE,
    LOADED_HOST_NODE_TYPES,
    PACKAGE_QUERY,
    HostPlugin,
    HostSpec,
    global_session,
    unpack_commit,
)
from bench.system.graph import GraphIoServiceBase
from bench.system.provisioner import Provisioner, get_provisioners_for
from bench.system.scheduler import QueueRunPlugin
from bench.utils.dt import monotime, utcnow
from bench.utils.func import to_uuid
from bench.utils.utils import get_from_env_maybe
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)

LOCAL_MACHINE_URL = get_from_env_maybe("LOCAL_MACHINE_URL")
LOCAL_MACHINE = Machine(
    name="localhost",
    status=ResourceStatus.HEALTHY,
    connection_uri=LOCAL_MACHINE_URL,
    profile=MachineProfile.MEDIUM,
)


class HostRouter(BenchServiceBase, HostBase):
    """
    Multiplexes requests per Bench to a Host using gRPC metadata ('bench-id').
    Also provides some process-level shared functionality.
    Hosts are loaded for all active Benches; new ones 'ping' the multiplexer service to add themselves.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind

    def __init__(self):
        super().__init__()
        self._hosts: dict[UUID, Host] = {}
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
        _, cardinality, _request_type, _reply_type = handler

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


class Host(GraphIoServiceBase, HostBase, HostSpec):
    """
    Host for a Bench, providing the OS-level functionality (lifecycle, resources, scheduling, etc.).
    There is only one Host per Bench. Clients interact with the Bench exclusively via its Host.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind

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

        # processing
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

    def request_session(
        self,
        *,
        engines: tuple[StoreEngine, ...] | None = None,
        readonly: bool = True,
    ):
        """Gets a new session for processing a request."""
        return Session(
            parent=self._main_package,
            _is_readonly=readonly,
            _default_scope=self.scope,
            _engines=engines if engines is not None else self.get_engines(),
            _epoch=self.epoch,
            _extend_commit_hook=self.extend_commit,
            _on_commit_hook=self.on_commit,
        )

    @override
    @asynccontextmanager
    async def session(self, *, readonly: bool = False, autocommit: bool = False):
        """Gets exclusive query and edit access to the main session. :ExclusiveHostSession"""
        async with self._tx_lock:
            assert self._session is not None, f"session not ready in {self!r}"
            was_readonly = self._session._is_readonly
            self._session._is_readonly = readonly
            self._session._epoch = self.epoch
            self._session.unsuspend()
            yield self._session
            if autocommit:
                await self._session.commit()
            elif self._session.tx.edits:
                raise RuntimeError(f"uncommitted edits in {self!r}: {self._session.tx.edits!r}")
            self._session.suspend()  # suspend by default
            self._session._is_readonly = was_readonly

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

        # load bench
        #  (in different session because we don't have the actual engines yet)
        async with self.request_session(engines=(GLOBAL_POSTGRES_ENGINE,)):
            self._bench = await BENCH_QUERY.get(id=self.bench_id)
            self._bench._untrack_rec()

        # setup main engines
        self._global_pg_engine = PostgresEngine(
            store=GLOBAL_STORE,
            bench=self._bench,
            scope=self._scope,
            node_types=IN_BENCH_GLOBAL_NODE_TYPES,
        )
        assert self._bench.main_environment, f"{self._bench!r} has no main environment"
        assert self._bench.main_environment.store, f"{self._bench!r} has no main store"
        assert self._bench.main_branch, f"{self._bench!r} has no main branch"
        self._local_pg_engine = PostgresEngine(
            store=self._bench.main_environment.store,
            bench=self._bench,
            scope=self._scope,
            node_types=LOCAL_NODE_TYPES,
        )

        # we open one Session for the entire lifecycle of the Host
        # TODO :Performance!: support in-memory engines in Host (from loaded graphs)
        self._engines = (self._global_pg_engine, self._local_pg_engine)
        self._session = Session(
            parent=self._bench.main_branch.main_package,
            _is_readonly=False,
            _default_scope=self.scope,
            _engines=self._engines,
            _extend_commit_hook=self.extend_commit,
            _on_commit_hook=self.on_commit,
        )
        self._bench._track_rec(self._session)
        await self._session.open(in_context=False)

        # preload main packages
        async with self.session(readonly=True):
            self._main_package = await PACKAGE_QUERY.get(id=self._bench.main_branch.main_package_id)
            self._packages[self._main_package.id] = self._main_package

            # get current epoch from log
            try:
                self.epoch = (
                    await Log.order_by(cast(Property, Log.created_epoch).desc())
                    .limit(1)
                    .scalar("created_epoch")
                )
            except NodeNotFoundError as e:
                self.epoch = 0
                logger.debug("host.reset", host=self, epoch=self.epoch, exc_info=e)

        # start plugins
        self._provisioners = tuple(get_provisioners_for(self, self._bench))
        self._plugins = (QueueRunPlugin(self, self._bench), *self._provisioners)
        await asyncio.gather(*(plugin.start() for plugin in self._plugins))
        # wait for plugins to finish processing any commits (and error early)
        await asyncio.gather(*(plugin.wait_step(timeout=10) for plugin in self._plugins))
        logger.info(
            "host.start",
            host=self,
            epoch=self.epoch,
            plugins=self._plugins,
            duration=monotime() - start,
        )

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
        self,
        session: Session,
        graph: NodeGraphLike,
        edits: list[EditData],
        epoch: int,
        cascaded_edits: list[EditData],
    ) -> tuple[list[EditData], int]:
        extended_edits: list[EditData] = []
        # NOTE :Incomplete: run plugins to extend commit (not needed yet)

        # create signals
        ...

        # add logs
        now = utcnow()
        package_ptr = session.package.to_ref()._to_data()
        bench_ptr = session.bench.to_ref()._to_data()
        for edit in chain(edits, extended_edits):
            if edit.node_type == wire.ObjectType.LOG:
                continue  # don't log logs
            node = wiring.unwrap_some_node(edit.node)
            assert edit.epoch is not None, f"epoch not set in {edit!r}"
            log_data = LogData(
                metatype=wire.ObjectType.LOG,
                id=str(UUIDT()),
                parent_ptr=package_ptr,
                package_ptr=package_ptr,
                bench_ptr=bench_ptr,
                created_at=now,
                created_epoch=edit.epoch,
                updated_at=now,
                updated_epoch=edit.epoch,
                kind=wire.LogKind.EDIT,
                level=wire.LogLevel.INFO,
                type=cast(wire.AccessType, edit.type),
                node_ptr=NodeReference.from_node_data(node),
                properties=edit.properties,
            )
            create_log_edit = EditData(
                id=log_data.id,
                type=wire.EditType.CREATE,
                scope=edit.scope,
                node_type=cast(wire.NodeType, log_data.metatype),
                origin=None,
                epoch=edit.epoch,
                node=wiring.wrap_some_node(log_data),
                # TODO :Incomplete: log session context
            )
            extended_edits.append(create_log_edit)

        return extended_edits, epoch

    @override
    async def _on_commit(
        self, graph: NodeGraphLike, edits: list[EditData], cascaded_edits: list[EditData]
    ):
        assert self._session is not None, f"session not ready in {self!r}"
        assert self._bench is not None, f"bench not loaded in {self!r} for {edits!r}"
        start = monotime()

        # apply edits to loaded graphs (bench/package)
        self._session.suppress()  # don't trigger the edits we're just applying
        for edit in edits:
            if NodeType(edit.node_type) not in LOADED_HOST_NODE_TYPES:
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
        self._session.unsuppress()

        # run plugins on commit (in main session)
        was_suspended = self._session.is_suspended
        if was_suspended:  # we may be nested in a Session.commit already
            self._session.unsuspend()
        self._session.track_many(*graph.nodes)
        commit = unpack_commit((*self.graphs, graph), edits, cascaded_edits)
        for plugin in self._plugins:
            if commit.edited_types & plugin.watch_types:
                trimmed_commit = commit.trim_to(plugin.watch_types)
                await plugin.on_commit(self._session, trimmed_commit)
                logger.debug(
                    "host.on_commit.plugin", host=self, plugin=plugin, commit=trimmed_commit
                )
        await self._session.commit(_skip_lock=True)  # already in a locked section
        if was_suspended:
            self._session.suspend()
        logger.debug("host.on_commit", host=self, commit=commit, duration=monotime() - start)
