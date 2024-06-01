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
from opentelemetry import trace

from bench.language import Bench, Package, Run, Server, Subject
from bench.language.access import Badge, Ownable
from bench.language.bench import Client, Machine, MachineProfile, ResourceStatus
from bench.language.connection import InMemoryEngine, PostgresEngine, StoreEngine
from bench.language.const import (
    IN_BENCH_GLOBAL_NODE_TYPES,
    IN_BENCH_NODE_TYPES,
    LOCAL_NODE_TYPES,
    SELF_LOGGED_NODE_TYPES,
    NodeType,
)
from bench.language.expression import NodeReference
from bench.language.graph import NodeDict, NodeGraphLike
from bench.language.log import Log
from bench.language.property import Property
from bench.language.session import Session, unsuspend_session
from bench.language.transaction import (
    edit_data_graph,
    edit_graph,
    pack_node_delta,
    sync_graph_revisions,
)
from bench.language.user import User
from bench.proto import wire
from bench.proto.services import BenchServiceBase, RpcCallable
from bench.proto.wire import EditData, GraphScope, HostBase, LogData, ServiceKind
from bench.system.access import get_client_cached
from bench.system.core import (
    BENCH_QUERY,
    GLOBAL_POSTGRES_ENGINE,
    GLOBAL_STORE,
    LOADED_BENCH_NODE_TYPES,
    LOADED_HOST_NODE_TYPES,
    LOADED_PACKAGE_NODE_TYPES,
    PACKAGE_QUERY,
    HostPlugin,
    HostSpec,
    global_session,
    unpack_commit,
)
from bench.system.graph import GraphIoServiceBase
from bench.system.provisioner import Provisioner, get_provisioners_for
from bench.system.scheduler import QueueRunPlugin
from bench.utils.func import to_uuid
from bench.utils.utils import get_from_env_maybe
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

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

    async def _get_host(self, request: betterproto.Message) -> "Host":
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

    async def _get_subject(
        self, request: betterproto.Message, metadata: wire.RpcMetadata
    ) -> Subject:
        host = await self._get_host(request)
        return await host._get_subject(request, metadata)

    def _wrap_rpc_func(
        self, func: RpcCallable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        _, cardinality, _request_type, _reply_type = handler

        if cardinality == grpclib.const.Cardinality.UNARY_UNARY:

            @functools.wraps(func)
            async def _multiplexed_unary_rpc(
                subject: Subject, request: betterproto.Message
            ) -> None:
                host = await self._get_host(request)
                return await getattr(host, method_name)(subject, request)

            return _multiplexed_unary_rpc

        elif cardinality == grpclib.const.Cardinality.UNARY_STREAM:

            @functools.wraps(func)
            async def _multiplexed_unary_stream_rpc(subject: Subject, request: betterproto.Message):
                host = await self._get_host(request)
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
        self._main_package: Package | None = None
        self._scope: GraphScope = GraphScope(bench_id=str(bench_id))
        self._global_pg_engine: PostgresEngine | None = None
        # NOTE: currently we only have one local engine because we only have one branch :Branching
        #  but later we'll need different engines for every 'full' branch (separate Neon branch)
        self._local_pg_engine: PostgresEngine | None = None
        self._engines: tuple[StoreEngine, ...] = ()

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

    @property
    def main_package(self) -> Package:
        assert self._main_package is not None, f"main package not loaded in {self}"
        return self._main_package

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
        )

    @override
    @asynccontextmanager
    async def session(self, *, readonly: bool = False, autocommit: bool = False):
        """Gets exclusive query and edit access to the main session. :ExclusiveHostSession"""
        assert self._session is not None, f"no session for {self!r}"
        async with self.tx_lock, unsuspend_session(
            self._session, readonly=readonly, autocommit=autocommit
        ):
            yield self._session

    @property
    def graphs(self) -> tuple[NodeGraphLike, ...]:
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"main package not loaded in {self!r}"
        return self._bench._graph, self._main_package._graph

    @override
    def get_engines(self) -> tuple[StoreEngine, ...]:
        return self._engines

    @tracer.start_as_current_span("host.get_subject")
    async def _get_subject(
        self, request: betterproto.Message, metadata: wire.RpcMetadata
    ) -> Subject:
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._session is not None, f"session not ready in {self!r}"

        # get client
        is_staff = False
        user: User | None = None
        owned: list[Ownable] = []
        if metadata.client_id and metadata.client_access_token:
            if metadata.client_type is None:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "missing client type")
            client_id = UUID(metadata.client_id)
            if metadata.client_type != wire.ClientType.BENCH_SERVER:
                # user client
                async with global_session() as session:
                    client = await get_client_cached(
                        session, client_id, metadata.client_access_token
                    )
                assert isinstance(client.parent, User), f"unexpected client: {client!r}"
                if client.parent.main_bench_id == self._bench.id:
                    owned = [client.parent, self._bench]
                else:
                    owned = [client.parent]
                is_staff = client.parent.is_staff
                user = client.parent
            else:
                # server client
                client = self._bench._graph.get(client_id)
                if not isinstance(client, Client):
                    raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid client id")
                assert isinstance(client.parent, Server)
                owned = [self._bench]  # servers own the bench for now
        else:
            client = None

        # get badges
        badges: list[Badge] = []
        for presented_badge in metadata.badges:
            badge = self._bench._graph.get(UUID(presented_badge.id))
            if not isinstance(badge, Badge):
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge id")
            if badge.key and badge.key != presented_badge.key:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge key")
            if badge.password and badge.password != presented_badge.password:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge password")
            badges.append(badge)

        # NOTE :Incomplete: get roles/memberships/identities/... for subject in Host

        subject = Subject(
            is_authenticated=client is not None,
            is_staff=is_staff,
            client=client,
            user=user,
            badges=badges,
            owned=owned,
        )
        return subject

    @tracer.start_as_current_span("host.start")
    async def start(self) -> None:
        trace.get_current_span().set_attribute("bench_id", str(self.bench_id))

        # load bench
        #  (in different session because we don't have the actual engines yet)
        async with self.request_session(engines=(GLOBAL_POSTGRES_ENGINE,)) as session:
            self._bench = await BENCH_QUERY.get(id=self.bench_id)
            assert self._bench.main_environment, f"{self._bench!r} has no main environment"
            assert self._bench.main_environment.store, f"{self._bench!r} has no main store"
            assert self._bench.main_branch, f"{self._bench!r} has no main branch"
            self._bench._untrack_rec()
            session.parent = self._bench.main_branch.main_package  # add bench hack for pg context

            # preload main packages
            self._main_package = await PACKAGE_QUERY.get(id=self._bench.main_branch.main_package_id)
            self._main_package._untrack_rec()
        trace.get_current_span().set_attribute("bench", self._bench.slug)

        # setup main engines
        self._global_pg_engine = PostgresEngine(
            store=GLOBAL_STORE,
            bench=self._bench,
            scope=self._scope,
            node_types=IN_BENCH_GLOBAL_NODE_TYPES,
        )
        self._local_pg_engine = PostgresEngine(
            store=self._bench.main_environment.store,
            bench=self._bench,
            scope=self._scope,
            node_types=LOCAL_NODE_TYPES,
        )
        self._engines = (
            # in order of priority (prefer in-memory)
            InMemoryEngine(
                scope=self._scope, node_types=LOADED_BENCH_NODE_TYPES, graph=self._bench._data_graph
            ),
            InMemoryEngine(
                scope=self._scope,
                node_types=LOADED_PACKAGE_NODE_TYPES,
                graph=self._main_package._data_graph,
            ),
            self._global_pg_engine,
            self._local_pg_engine,
        )
        # we open one Session for the entire lifecycle of the Host
        self._session = Session(
            parent=self._bench.main_branch.main_package,
            _is_readonly=False,
            _default_scope=self.scope,
            _engines=self._engines,
            _commit=self._commit_system_session,
        )
        self._bench._track_rec(self._session)
        self._main_package._track_rec(self._session)
        await self._session.open(in_context=False)

        # resume log
        async with self.session(readonly=True):
            last_epoch = (
                await Log.order_by(cast(Property, Log.created_epoch).desc())
                .limit(1)
                .scalar_maybe("created_epoch")
            )
            if last_epoch is not None:
                self.epoch = last_epoch
            else:
                logger.debug("host.start.no_logs", host=self, bench=self._bench)

        # start plugins
        with tracer.start_as_current_span("host.start.plugins"):
            self._provisioners = tuple(get_provisioners_for(self, self._bench))
            self._plugins = (QueueRunPlugin(self, self._bench), *self._provisioners)
            await asyncio.gather(*(plugin.start() for plugin in self._plugins))
            # wait for plugins to finish processing any commits (and to error early)
            await asyncio.gather(*(plugin.wait_step(timeout=10) for plugin in self._plugins))
        logger.info(
            "host.start", host=self, epoch=self.epoch, plugins=self._plugins, span="current"
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
    @tracer.start_as_current_span("host.extend_commit")
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
        package_ptr = session.package.to_ref()._to_data()
        bench_ptr = session.bench.to_ref()._to_data()
        log_edits: list[EditData] = []
        for edit in chain(edits, extended_edits):
            node_type = NodeType(edit.node_ptr.type)
            if node_type in SELF_LOGGED_NODE_TYPES:
                continue
            assert edit.epoch is not None, f"epoch not set in {edit!r}"
            log_data = LogData(
                metatype=wire.ObjectType.LOG,
                id=str(UUIDT()),
                revision=0,
                parent_ptr=package_ptr,
                package_ptr=package_ptr,
                bench_ptr=bench_ptr,
                created_at=edit.edited_at,
                created_epoch=edit.epoch,
                created_by_ptr=edit.subject_ptr,
                # NOTE :Architecture: ideally we shouldn't need to store updated_* for Logs
                updated_at=edit.edited_at,
                updated_epoch=edit.epoch,
                updated_by_ptr=edit.subject_ptr,
                kind=wire.LogKind.EDIT,
                level=wire.LogLevel.INFO,
                type=cast(wire.AccessType, edit.type),
                node_ptr=edit.node_ptr,
                properties=edit.properties,
                old_node_packed=edit.old_node_packed,
                new_node_packed=edit.new_node_packed,
                new_revision=edit.revision,
                # TODO :Incomplete: log session context
            )
            create_log_edit = EditData(
                id=log_data.id,
                type=wire.EditType.CREATE,
                scope=edit.scope,
                node_ptr=NodeReference.from_node_data(log_data),
                origin=None,
                epoch=edit.epoch,
                revision=log_data.revision,
                new_node_packed=pack_node_delta(log_data),
            )
            log_edits.append(create_log_edit)
        extended_edits.extend(log_edits)

        return extended_edits, epoch

    @override
    @tracer.start_as_current_span("host.on_commit")
    async def _on_commit(
        self, graph: NodeGraphLike, edits: list[EditData], cascaded_edits: list[EditData]
    ):
        assert self._session is not None, f"session not ready in {self!r}"
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        # apply edits to loaded graphs (bench/package)
        self._session.suppress()  # don't trigger the edits we're just applying
        bench_edits: list[EditData] = []
        package_edits: list[EditData] = []
        for edit in edits:
            node_type = NodeType(edit.node_ptr.type)
            if node_type not in LOADED_HOST_NODE_TYPES:
                continue  # not loaded
            if edit.scope.package_id:
                package_id = to_uuid(edit.scope.package_id)
                assert package_id == self._main_package.id, f"bad package id: {package_id!r}"
                package_edits.append(edit)
            else:
                bench_edits.append(edit)
        if bench_edits:
            # filter the in memory edits to only those with an origin
            # (we/Host/system don't have an 'origin' and edit our nodes directly in the session)
            inmemory_bench_edits = tuple(e for e in bench_edits if e.origin is not None)
            options = BENCH_QUERY._options
            edit_graph(self._bench._graph, inmemory_bench_edits, options)
            edit_data_graph(self._bench._data_graph, bench_edits, options, bump=True)
            sync_graph_revisions(self._bench._data_graph, self._bench._graph)
        if package_edits:
            # same as above but for the main package
            inmemory_package_edits = tuple(e for e in package_edits if e.origin is not None)
            options = PACKAGE_QUERY._options
            edit_graph(self._main_package._graph, inmemory_package_edits, options)
            edit_data_graph(self._main_package._data_graph, package_edits, options, bump=True)
            sync_graph_revisions(self._bench._data_graph, self._bench._graph)
        self._session.unsuppress()

        # run plugins on commit (in main session)
        was_suspended = self._session.is_suspended
        if was_suspended:  # we may be nested in a Session.commit already
            self._session.unsuspend()
        self._session.track_many(*graph.nodes)
        commit = unpack_commit(
            session=self._session,
            graphs=(*self.graphs, graph),
            edits=edits,
            cascaded_edits=cascaded_edits,
        )
        for plugin in self._plugins:
            if commit.edited_types & plugin.watch_types:
                with tracer.start_as_current_span(
                    "host.on_commit.plugin", attributes={"plugin": plugin.name}
                ):
                    trimmed_commit = commit.trim_to(plugin.watch_types)
                    await plugin.on_commit(self._session, trimmed_commit)
                    logger.debug(
                        "host.on_commit.plugin", host=self, plugin=plugin, commit=trimmed_commit
                    )
        await self._session.commit(_skip_lock=True)  # already in a locked section
        if was_suspended:
            self._session.suspend()
        logger.debug("host.on_commit", host=self, commit=commit, span="current")

    async def _commit_system_session(
        self, session: Session
    ) -> tuple[list[EditData], list[EditData]]:
        """
        Commits our main session for us (the system) from outside a request context.
        This should emulate what GraphService.commit_transaction does (skipping validation).
        """
        assert session is self._session, f"session other than own in {self!r}"
        assert session._tx is not None, f"no active transaction in {session!r}"
        edit_graph = NodeDict(session._edited_nodes_by_id)

        # assign epochs
        epoch = self.epoch
        for edit in session._tx.edits:
            epoch += 1
            edit.epoch = epoch

        # flush edits to get cascaded edits
        edits, cascaded_edits = await session._tx.flush()

        # extend commit
        new_edits, epoch = await self.extend_commit(
            session, edit_graph, edits, epoch, cascaded_edits
        )
        session._tx._add_pending_edits(new_edits)

        # commit
        edits, cascaded_edits = await session._tx.commit()

        # handle on commit
        self.epoch = epoch
        await self.on_commit(edit_graph, edits, epoch, cascaded_edits)
        return edits, cascaded_edits
