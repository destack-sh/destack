import asyncio
import functools
from contextlib import asynccontextmanager
from itertools import chain
from typing import Callable, cast, override
from uuid import UUID

import betterproto
import grpclib.server
import structlog
from betterproto.lib.google.protobuf import Struct as ProtoStruct
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import Bench, Package, Run, Server, Subject
from bench.language.access import Badge, Ownable
from bench.language.bench import Client
from bench.language.connection import GraphEngine, MemoryEngine, PostgresEngine
from bench.language.const import (
    ETERNAL_NODE_TYPES,
    IN_BENCH_GLOBAL_NODE_TYPES,
    IN_BENCH_NODE_TYPES,
    LOCAL_NODE_TYPES,
    SOURCE_NODE_TYPES,
    ClientType,
    NodeType,
)
from bench.language.expression import NodeReference
from bench.language.graph import NodeDataGraphLike, NodeGraphLike, NodeSuperGraph
from bench.language.log import Log
from bench.language.property import Property
from bench.language.session import Session, SessionContext, unsuspend_session
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.language.transaction import (
    edit_data_graph,
    edit_graph,
    pack_node_delta,
)
from bench.language.user import User
from bench.proto import wire
from bench.proto.services import RpcCallable, ServiceBase
from bench.proto.wire import (
    EditData,
    GraphScope,
    HostBase,
    LogData,
    ServiceKind,
    SessionContextData,
)
from bench.proto.wiring import unpack_proto_json
from bench.system.access import CLIENT_CACHE_ENABLED, ClientCache, get_client
from bench.system.core import (
    BENCH_QUERY,
    GLOBAL_POSTGRES_ENGINE,
    GLOBAL_STORE,
    LOADED_BENCH_NODE_TYPES,
    LOADED_HOST_NODE_TYPES,
    PACKAGE_QUERY,
    HostPlugin,
    HostSpec,
    global_session,
    unpack_commit,
)
from bench.system.graph import CommitScope, GraphIoServiceBase, parse_commit_scope, validate_edit
from bench.system.provisioner import Provisioner, get_provisioners_for
from bench.system.scheduler import QueueRunPlugin
from bench.utils.func import to_uuid
from bench.utils.oracle import get_oracle
from bench.utils.utils import get_from_env
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

HOST_MEMORY_ENGINE_ENABLED = get_from_env(
    "HOST_MEMORY_ENGINE_ENABLED",
    typ=bool,
    default=True,
    description="Whether to provide in-memory engines for Bench/Package",
)


class HostRouter(ServiceBase, HostBase):
    """
    Multiplexes requests per Bench to a Host using gRPC metadata ('bench-id').
    Also provides some process-level shared functionality.
    Hosts are loaded for all active Benches; new ones 'ping' the multiplexer service to add themselves.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind

    def __init__(self):
        super().__init__(logger=logger, tracer=tracer)
        self._hosts: dict[UUID, Host] = {}
        self._hosts_lock = asyncio.Lock()

    def __str__(self):
        return "shards=[*]"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start(self) -> None:
        await super().start()
        async with global_session():
            benches: list[Bench] = await Bench.search()
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

    async def get_request_subject(
        self, request: betterproto.Message, metadata: wire.RpcMetadata
    ) -> Subject:
        host = await self._get_host(request)
        return await host.get_request_subject(request, metadata)

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
        GraphIoServiceBase.__init__(
            self, bench_id=bench_id, node_types=IN_BENCH_NODE_TYPES, logger=logger, tracer=tracer
        )

        self.bench_id = bench_id
        self.bench_ptr = NodeReference(
            type=NodeType.BENCH, id=bench_id, ck=bench_id, bench_id=bench_id
        )
        self._supergraph = NodeSuperGraph(self.bench_ptr)
        self._client_cache = ClientCache()
        self._bench: Bench | None = None
        self._main_package: Package | None = None
        self._scope: GraphScope = GraphScope(bench_id=str(bench_id))
        self._global_pg_engine: PostgresEngine | None = None
        # NOTE: currently we only have one local engine because we only have one branch :Branching
        #  but later we may need different engines for every 'full' branch
        #  (separate Neon branch with separate compute endpoint with its own connection info)
        self._local_pg_engine: PostgresEngine | None = None
        self._engines: tuple[GraphEngine, ...] = ()

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

    @property
    def graphs(self) -> tuple[NodeGraphLike, ...]:
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"main package not loaded in {self!r}"
        return self._bench._graph, self._main_package._graph

    @override
    def get_engines(self) -> tuple[GraphEngine, ...]:
        return self._engines

    @property
    @override
    def request_session_parent(self):
        return self._main_package

    @override
    @asynccontextmanager
    async def session(self, *, readonly: bool = False, autocommit: bool = False):
        """Gets exclusive query and edit access to the main session. :ExclusiveHostSession"""
        assert self._session is not None, f"no session for {self!r}"
        async with self.tx_lock, unsuspend_session(
            self._session, readonly=readonly, autocommit=autocommit
        ):
            self._session._epoch = self.epoch
            yield self._session

    @tracer.start_as_current_span("host.get_subject")
    async def get_request_subject(
        self, request: betterproto.Message, metadata: wire.RpcMetadata
    ) -> Subject:
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._session is not None, f"session not ready in {self!r}"

        # get client
        is_staff = False
        user: User | None = None
        server: Server | None = None
        owned: list[Ownable] = []
        if metadata.client_id and metadata.client_access_token:
            if metadata.client_type is None:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "missing client type")
            client_id = UUID(metadata.client_id)
            if metadata.client_type != wire.ClientType.BENCH_SERVER:
                # user client
                if CLIENT_CACHE_ENABLED and self._client_cache.has(client_id):
                    client = await self._client_cache.get(client_id, metadata.client_access_token)
                else:
                    async with global_session(_supergraph=self._supergraph):
                        if CLIENT_CACHE_ENABLED:
                            client = await self._client_cache.get(
                                client_id, metadata.client_access_token
                            )
                        else:
                            client = await get_client(client_id, metadata.client_access_token)
                assert isinstance(client.parent, User), f"unexpected client: {client!r}"
                if client.parent.main_bench_id == self._bench.id:
                    owned = [client.parent, self._bench]
                else:
                    owned = [client.parent]
                is_staff = client.parent.is_staff
                user = client.parent
            else:
                # server client (must be in bench graph)
                client = self._bench._graph.get(client_id)
                if not isinstance(client, Client):
                    raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid client id")
                assert isinstance(client.parent, Server)
                server = client.parent
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

        # NOTE :Incomplete: get roles/memberships/identities/... for subject in this bench

        # use new supergraph instance for session
        # NOTE :Cleanup :Architecture: putting the request supergraph in the request subject
        #  feels a bit indirect and clumsy.
        supergraph = self._supergraph.instance()
        subject = Subject(
            is_authenticated=client is not None,
            is_staff=is_staff,
            client=client,
            user=user,
            server=server,
            badges=badges,
            owned=owned,
            _supergraph=supergraph,
        )
        return subject

    @tracer.start_as_current_span("host.start")
    async def start(self) -> None:
        trace.get_current_span().set_attribute("bench_id", str(self.bench_id))
        await super().start()

        # load bench
        #  (in different session because we don't have the actual engines yet)
        async with self.request_session(
            supergraph=self._supergraph, engines=(GLOBAL_POSTGRES_ENGINE,)
        ) as session:
            self._bench = await BENCH_QUERY.get(self.bench_ptr)
            assert self._bench.main_environment, f"{self._bench!r} has no main environment"
            assert self._bench.main_environment.store, f"{self._bench!r} has no main store"
            assert self._bench.main_branch, f"{self._bench!r} has no main branch"
            self._bench._untrack_rec()
            session.parent = self._bench.main_branch.main_package  # add bench hack for pg context

            # preload main packages
            self._main_package = await PACKAGE_QUERY.get(self._bench.main_branch.main_package_ptr)
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
        self._engines = (self._global_pg_engine, self._local_pg_engine)
        if HOST_MEMORY_ENGINE_ENABLED:
            inmemory_engines = (
                MemoryEngine(
                    scope=self._scope,
                    node_types=LOADED_BENCH_NODE_TYPES,
                    graph=self._bench._data_graph,
                ),
                MemoryEngine(
                    scope=self._scope,
                    node_types=SOURCE_NODE_TYPES,
                    graph=self._main_package._data_graph,
                ),
            )
            self._engines = (*inmemory_engines, *self._engines)  # in order of priority
        # we open one Session for the entire lifecycle of the Host
        self._session = Session(
            parent=self._bench.main_branch.main_package,
            _is_readonly=False,
            _default_scope=self.scope,
            _engines=self._engines,
            _custom_commit=self._commit_system_session,
            _supergraph=self._bench._supergraph,
            _split_reads=True,
        )
        self._bench._track_rec(self._session)
        self._main_package._track_rec(self._session)
        await self._session.open(set_in_context=False)

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
                self.epoch = 1  # start at 1
                logger.debug("host.start.no_logs", host=self, bench=self._bench)

        # start plugins
        with tracer.start_as_current_span("host.start.plugins"):
            self._provisioners = tuple(get_provisioners_for(self, self._bench))
            self._plugins = (QueueRunPlugin(self, self._bench), *self._provisioners)
            await asyncio.gather(*(plugin.start() for plugin in self._plugins))
            # wait for plugins to finish processing any commits (and to error early)
            await asyncio.gather(*(plugin.wait_idle(timeout=10) for plugin in self._plugins))
        logger.info(
            "host.start",
            host=self,
            bench=self._bench,
            main_package=self._main_package,
            epoch=self.epoch,
            plugins=self._plugins,
            memory=HOST_MEMORY_ENGINE_ENABLED,
            span="current",
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
    @tracer.start_as_current_span("host.prepare_commit")
    def _prepare_commit(
        self, subject: Subject, context: SessionContext, edits: list[EditData]
    ) -> tuple[CommitScope, int]:
        assert subject.client and subject.client_ptr, f"no client for {subject!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        # prepare commit
        scope = parse_commit_scope(edits, base_graph=self._main_package._data_graph)
        now = get_oracle().utc()
        epoch = self.epoch
        for edit in edits:
            validate_edit(edit, subject, now)
            epoch += 1
            edit.epoch = epoch

        # check context
        validate_context(subject, context, edits)

        return scope, epoch

    @override
    @tracer.start_as_current_span("host.extend_commit")
    async def extend_commit(
        self,
        session: Session,
        context: SessionContext | None,
        graph: NodeGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
    ) -> list[EditData]:
        extended_edits: list[EditData] = []
        # NOTE :Incomplete: run plugins to extend commit (not needed yet)

        # create signals
        ...

        def _split_node_packed_secret(
            node_type: NodeType, node_packed_struct: ProtoStruct | None
        ) -> tuple[ProtoStruct | None, ProtoStruct | None]:
            # NOTE :Incomplete: we ignore nested :SecretValues (inside value properties) for now
            if node_packed_struct is None:
                return None, None
            node_cls = NODE_CLASS_BY_TYPE[node_type]
            node_packed = unpack_proto_json(node_packed_struct)
            node_secret_packed = {}
            for prop_key, prop_value in node_packed.items():
                prop = node_cls.__properties_by_id__.get(int(prop_key))
                assert prop is not None, f"missing prop {prop_key} in {node_cls!r}"
                if prop.is_sensitive:
                    node_secret_packed[prop_key] = prop_value
            # prune secret properties from node_packed (after to avoid concurrent modification)
            for prop_key in node_secret_packed:
                del node_packed[prop_key]
            # only keep secret properties if there are any
            if len(node_secret_packed) > 0:
                node_secret_packed_struct = ProtoStruct.from_dict(node_secret_packed)
            else:
                node_secret_packed_struct = None
            return ProtoStruct.from_dict(node_packed), node_secret_packed_struct

        # add logs
        context_data: SessionContextData = (
            context._to_data() if context is not None else SessionContextData()
        )
        package_ptr = session.package._to_ref_data()
        bench_ptr = session.bench._to_ref_data()
        log_edits: list[EditData] = []
        for edit in chain(edits, extended_edits):
            node_type = NodeType(edit.node_ptr.type)
            if node_type in ETERNAL_NODE_TYPES:
                continue
            assert edit.revision is not None, f"revision not set in {edit!r}"
            assert edit.epoch is not None, f"epoch not set in {edit!r}"
            # break out secret properties
            old_node_packed, old_node_secret_packed = _split_node_packed_secret(
                node_type, edit.old_node_packed
            )
            new_node_packed, new_node_secret_packed = _split_node_packed_secret(
                node_type, edit.new_node_packed
            )
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
                # NOTE :Architecture: ideally we shouldn't need to store updated_* for Logs?
                updated_at=edit.edited_at,
                updated_epoch=edit.epoch,
                updated_by_ptr=edit.subject_ptr,
                kind=wire.LogKind.EDIT,
                level=wire.LogLevel.INFO,
                type=cast(wire.AccessType, edit.type),
                node_ptr=edit.node_ptr,
                properties=edit.properties,
                old_node_packed=old_node_packed,
                old_node_secret_packed=old_node_secret_packed,
                new_node_packed=new_node_packed,
                new_node_secret_packed=new_node_secret_packed,
                new_revision=edit.revision,
                block_ptr=edit.context.block_ptr if edit.context else None,
                step_ptr=edit.context.step_ptr if edit.context else None,
                session_ptr=edit.context.session_ptr if edit.context else None,
                run_ptr=edit.context.run_ptr if edit.context else None,
                run_root_ptr=edit.context.run_root_ptr if edit.context else None,
                client_ptr=context_data.client_ptr,
                machine_ptr=context_data.machine_ptr,
                server_ptr=context_data.server_ptr,
                user_ptr=context_data.user_ptr,
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
                edited_at=log_data.created_at,
            )
            log_edits.append(create_log_edit)

        # add edits to session
        extended_edits.extend(log_edits)
        session.tx._add_pending_edits(log_edits)

        return extended_edits

    @override
    @tracer.start_as_current_span("host.on_commit")
    async def on_commit(
        self,
        graph: NodeGraphLike,
        data_graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        await super().on_commit(graph, data_graph, edits, cascaded_edits, epoch)

        assert self._session is not None, f"session not ready in {self!r}"
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        # apply edits to loaded graphs (bench/package)
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
        for root_node, options, subedits in (
            (self._bench, BENCH_QUERY._options, bench_edits),
            (self._main_package, PACKAGE_QUERY._options, package_edits),
        ):
            # filter the in memory edits to only those with an origin (we = system has origin = null)
            external_edits = tuple(e for e in subedits if e.origin is not None)
            edit_graph(
                graph=root_node._graph,
                supergraph=self._supergraph,
                edits=external_edits,
                options=options,
                track=False,
                validate=False,
            )
            for edit in subedits:
                # manually patch revisions since we skipped some edits above
                assert edit.revision is not None, f"revision not set in {edit!r}"
                edited_node = root_node._graph.get(UUID(edit.node_ptr.id))
                if edited_node is not None:
                    edited_node.revision = edit.revision
            # and apply all edits to the data graph
            edit_data_graph(root_node._data_graph, subedits, options)

        # run plugins on commit (in main session)
        was_suspended = self._session.is_suspended
        if was_suspended:  # we may be nested in a Session.commit already
            self._session.unsuspend()
        self._session.track_many(*graph.nodes)
        commit = unpack_commit(
            session=self._session,
            supergraph=self._supergraph,
            graphs=(*self.graphs, graph),
            edits=edits,
            cascaded_edits=cascaded_edits,
            epoch=self.epoch,
        )
        for plugin in self._plugins:
            if commit.edited_types & plugin.watch_types:
                with tracer.start_as_current_span(
                    "host.on_commit.plugin", attributes={"plugin": plugin.name}
                ):
                    trimmed_commit = commit.trim_to(plugin.watch_types)
                    await plugin.on_commit(self._session, trimmed_commit)
                    logger.debug(
                        "host.on_commit.plugin",
                        host=self,
                        plugin=plugin,
                        commit=trimmed_commit,
                        span="current",
                    )
        await self._session.commit(_skip_lock=True)  # already in a locked section
        if was_suspended:
            self._session.suspend()
        logger.debug(
            "host.on_commit",
            host=self,
            added=commit.added,
            updated=commit.updated,
            removed=commit.removed,
            span="current",
        )


def validate_context(subject: Subject, context: SessionContext, edits: list[EditData]):
    """Checks the session context and per edit context for consistency."""
    assert subject.client and subject.client_ptr, f"no client for {subject!r}"
    if not context.client_ptr or context.client_ptr.id != subject.client_ptr.id:
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT,
            f"bad client context for {subject!r}: {context.client_ptr!r}",
        )
    if subject.user_ptr and (not context.user_ptr or context.user_ptr.id != subject.user_ptr.id):
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT,
            f"bad user context for {subject!r}: {context.user_ptr!r}",
        )
    if subject.server_ptr and (
        not context.server_ptr or context.server_ptr.id != subject.server_ptr.id
    ):
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT,
            f"bad server context for {subject!r}: {context.server_ptr!r}",
        )
    if subject.client.type == ClientType.BENCH_SERVER:
        if not context.machine or context.machine.parent != subject.server:
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT,
                f"bad machine context for {subject!r}: {context.machine!r}",
            )
    # NOTE :Incomplete: validate Edit context in Host
