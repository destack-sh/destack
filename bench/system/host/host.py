import asyncio
import functools
from contextlib import asynccontextmanager
from itertools import chain
from typing import Any, Callable, cast, override
from uuid import UUID

import betterproto
import grpclib.server
import structlog
from betterproto.lib.google.protobuf import Struct as ProtoStruct
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import Bench, Drive, NodeReference, Package, Run, Server, Store, Subject
from bench.language.access import Badge, Ownable
from bench.language.bench import Branch, Client
from bench.language.connection import GraphEngine, MemoryEngine
from bench.language.const import (
    CLOUD,
    IN_BENCH_GLOBAL_NODE_TYPES,
    IN_BENCH_NODE_TYPES,
    LOADED_BENCH_NODE_TYPES,
    LOCAL_NODE_TYPES,
    RUNTIME_NODE_TYPES,
    SOURCE_NODE_TYPES,
    ClientType,
    ConditionalOp,
    EditType,
    NodeType,
)
from bench.language.expression import C
from bench.language.file import File, FileInfoBase, FileKind
from bench.language.graph import NodeDataGraphLike, NodeGraphLike, NodeSuperGraph
from bench.language.log import Log
from bench.language.node import GraphScope
from bench.language.property import Property
from bench.language.session import Session, SessionContext
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.language.transaction import (
    edit_data_graph,
    edit_graph,
)
from bench.language.user import User
from bench.language.value import pack_builtin_object_data, pack_value_scalar
from bench.proto import wire
from bench.proto.services import RpcCallable, ServiceBase
from bench.proto.wire import (
    DownloadFilesRequest,
    DownloadFilesResponse,
    DownloadFilesResponseDownloadHandle,
    EditData,
    GraphScopeData,
    HostBase,
    LogData,
    ServiceKind,
    SessionContextData,
    UploadFilesRequest,
    UploadFilesResponse,
    UploadFilesResponseUploadHandle,
)
from bench.proto.wiring import (
    pack_proto_json,
    unpack_object_validate,
    unwrap_some_node,
    wrap_some_node,
)
from bench.system.graph.graph import (
    CommitArea,
    GraphIoServiceBase,
    parse_commit_scope,
    validate_edit,
)
from bench.system.graph.postgres import PostgresEngine
from bench.system.host.core import (
    HostApi,
    HostPlugin,
    unpack_commit,
)
from bench.system.host.scheduler import QueueRunPlugin
from bench.system.provision.provisioner import (
    Provisioner,
    get_provisioners_for,
)
from bench.system.utils.access import CLIENT_CACHE_ENABLED, ClientCache, get_client
from bench.system.utils.aws import get_s3_client_for_presigning
from bench.system.utils.session import (
    global_session,
    local_pg_engine_from_store,
    pg_engine_from_store,
)
from bench.utils.env import ENV
from bench.utils.func import to_uuid
from bench.utils.oracle import Oracle
from bench.utils.utils import get_from_env
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

HOST_MEMORY_ENGINE_ENABLED = get_from_env(
    "HOST_MEMORY_ENGINE_ENABLED",
    typ=bool,
    default=True,
    description="Whether to provide in-memory caches for Bench/Package",
)
S3_PRESIGNED_URL_EXPIRY = get_from_env(
    "S3_PRESIGNED_URL_EXPIRY",
    typ=int,
    default=3600,
    description="S3 presigned URL expiry (in seconds)",
)
LOADED_HOST_NODE_TYPES = LOADED_BENCH_NODE_TYPES | SOURCE_NODE_TYPES
BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.ancestors(Branch)
    .descendants(*SOURCE_NODE_TYPES)
    .select_all()
    .exclude(Bench.encryption_key)
)


class HostRouter(ServiceBase, HostBase):
    """
    Multiplexes requests per Bench to a Host using gRPC metadata ('bench-id').
    Also provides some process-level shared functionality.
    Hosts are loaded for all active Benches; new ones 'ping' the multiplexer service to add themselves.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind

    def __init__(self, global_store: Store, oracle: Oracle):
        super().__init__(logger=logger, tracer=tracer, oracle=oracle)
        self.hosts: dict[UUID, Host] = {}
        self.hosts_lock = asyncio.Lock()
        self._global_store = global_store
        self._global_pg_engine = pg_engine_from_store(global_store)

    def __str__(self):
        return "shards=[*]"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start(self) -> None:
        await super().start()
        async with global_session(self._global_store, (self._global_pg_engine,), self.oracle):
            benches: list[Bench] = await Bench.search()
        await asyncio.gather(*(self._start_host(bench.id) for bench in benches))

    def close(self) -> None:
        for host in self.hosts.values():
            host.close()

    async def wait_closed(self) -> None:
        await asyncio.gather(*[host.wait_closed() for host in self.hosts.values()])

    async def _start_host(self, bench_id: UUID) -> "Host":
        """Starts a Host for the given Bench."""
        existing_host = self.hosts.get(bench_id)
        assert existing_host is None, f"already have Host for {bench_id}: {existing_host!r}"
        host = Host(bench_id, self._global_store, self.oracle)
        await host.start()
        self.hosts[bench_id] = host
        return host

    async def _get_host(self, request: betterproto.Message) -> "Host":
        """Gets or starts a running Host for the given Bench"""

        # get request's bench id
        scope: GraphScopeData | None = getattr(request, "scope")
        if scope is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing scope")
        bench_id = to_uuid(scope.bench_id)
        if bench_id is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing bench scope id")

        # get host
        host = self.hosts.get(bench_id)
        if host is None:
            async with self.hosts_lock:
                host = self.hosts.get(bench_id)
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


class Host(GraphIoServiceBase, HostApi, HostBase):
    """
    Host for a Bench, providing the OS-level functionality (lifecycle, resources, scheduling, etc.).
    There is only one Host per Bench. Clients interact with the Bench exclusively via its Host.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind

    def __init__(self, bench_id: UUID, global_store: Store, oracle: Oracle):
        GraphIoServiceBase.__init__(
            self,
            bench_id=bench_id,
            node_types=IN_BENCH_NODE_TYPES,
            logger=logger,
            tracer=tracer,
            oracle=oracle,
        )

        self.bench_id = bench_id
        self.bench_ptr = NodeReference(
            type=NodeType.BENCH, id=bench_id, ck=bench_id, bench_id=bench_id
        )
        self._global_store = global_store
        self._global_pg_engine_unscoped = pg_engine_from_store(global_store)
        self._supergraph = NodeSuperGraph(self.bench_ptr)
        self._client_cache = ClientCache()
        self._bench: Bench | None = None
        self._main_package: Package | None = None
        self._scope: GraphScopeData = GraphScope(bench_id=bench_id)._to_data()
        self._global_pg_engine: PostgresEngine | None = None
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
        pass  # error is already reported, we just keep running?

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

    @property
    @override
    def split_reads(self):
        return True

    @property
    def global_store(self) -> Store:
        return self._global_store

    def global_session(self, readonly: bool = False):
        return global_session(
            store=self.global_store,
            engines=(self._global_pg_engine_unscoped,),
            supergraph=self._supergraph,
            oracle=self.oracle,
            readonly=readonly,
        )

    @override
    @asynccontextmanager
    async def session(self, *, readonly: bool = False, commit: bool = False):
        """Gets exclusive query and edit access to the main session. :ExclusiveHostSession"""
        assert self._session is not None, f"no session for {self!r}"
        async with self.tx_lock, self._session.active(readonly=readonly):
            self._session._system_epoch = self.epoch
            yield self._session
            if commit:
                await self._session.commit()

    @tracer.start_as_current_span("host.get_request_subject")
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
            if metadata.client_type != wire.ClientType.BENCH_MACHINE:
                # user client
                if CLIENT_CACHE_ENABLED and self._client_cache.has(client_id):
                    client = await self._client_cache.get(client_id, metadata.client_access_token)
                else:
                    async with self.global_session():
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
        #  feels a bit indirect and clumsy, but it has to be the same supergraph during the request.
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

    async def start(self) -> None:
        trace.get_current_span().set_attribute("bench_id", str(self.bench_id))
        await super().start()

        # load bench
        #  (in different session because we don't have the actual engines yet)
        async with self.global_session(readonly=True) as session:
            # get bench main store so we can get all bench data (some of which is local)
            tmp_bench = await Bench.descendants(Store).select_all().get(self.bench_ptr)
            assert tmp_bench.main_store, f"{tmp_bench!r} has no main store"
            tmp_bench._untrack_rec()
            session._engines += (local_pg_engine_from_store(tmp_bench.main_store),)

            # load full bench
            self._bench = await BENCH_QUERY.get(self.bench_ptr)
            assert self._bench.main_store, f"{self._bench!r} has no main store"
            assert self._bench.main_branch, f"{self._bench!r} has no main branch"
            assert self._bench.main_branch.main_package, f"{self._bench!r} has no main package"
            session.parent = self._bench.main_branch.main_package  # patch in bench for pg context
            session._default_scope = GraphScope(bench_id=self.bench_id)._to_data()

            # load packages
            self._main_package = await PACKAGE_QUERY.get(self._bench.main_branch.main_package_ptr)

            # cleanup
            self._supergraph.remove_graph(tmp_bench._graph)
            del tmp_bench
            self._bench._untrack_rec()
            self._main_package._untrack_rec()

        # setup main engines
        # (overwrite global pg engine now that we have the full bench as context)
        self._global_pg_engine = PostgresEngine(
            store=self.global_store,
            bench=self._bench,
            scope=self._scope,
            node_types=IN_BENCH_GLOBAL_NODE_TYPES,
        )
        self._local_pg_engine = PostgresEngine(
            store=self._bench.main_store,
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
                    includes_hidden=False,
                ),
                MemoryEngine(
                    scope=self._scope,
                    node_types=SOURCE_NODE_TYPES,
                    graph=self._main_package._data_graph,
                    includes_hidden=False,
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
            _split_read=True,
            _oracle=self.oracle,
        )
        self._bench._track_rec(self._session)
        self._main_package._track_rec(self._session)
        await self._session.open(set_in_context=False)
        self._session.suspend()

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

    #
    # Graph
    #

    @override
    @tracer.start_as_current_span("host.prepare_commit")
    def _prepare_commit(
        self, subject: Subject, context: SessionContext, edits: list[EditData]
    ) -> tuple[CommitArea, int]:
        assert subject.client and subject.client_ptr, f"no client for {subject!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        # prepare commit
        scope = parse_commit_scope(edits, base_graph=self._main_package._data_graph)
        now = self.oracle.utc()
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
        supergraph: NodeSuperGraph,
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

        def _split_node_packed_sensitive(
            node_type: NodeType, node_packed: dict[str, Any] | None
        ) -> tuple[ProtoStruct | None, ProtoStruct | None]:
            if node_packed is None:
                return None, None
            node_cls = NODE_CLASS_BY_TYPE[node_type]
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
                node_secret_packed_struct = pack_proto_json(node_secret_packed)
            else:
                node_secret_packed_struct = None
            return pack_proto_json(node_packed), node_secret_packed_struct

        # add logs
        context_data: SessionContextData = (
            context._to_data() if context is not None else SessionContextData()
        )
        package_ptr = session.package._to_plain_ref_data()
        bench_ptr = session.bench._to_plain_ref_data()
        log_edits: list[EditData] = []
        for edit in chain(edits, extended_edits):
            node_type = NodeType(edit.node_ptr.type)
            if node_type in RUNTIME_NODE_TYPES:
                continue
            assert edit.revision is not None, f"revision not set in {edit!r}"
            assert edit.epoch is not None, f"epoch not set in {edit!r}"
            # pack old/new node
            node_cls = NODE_CLASS_BY_TYPE[node_type]
            properties = tuple(node_cls.__properties_by_id__[p] for p in edit.properties)
            if edit.old_node_partial:
                old_node = unwrap_some_node(edit.old_node_partial)
                old_node_packed = pack_builtin_object_data(old_node, only=properties or None)
            else:
                old_node_packed = None
            if edit.new_node_partial:
                new_node = unwrap_some_node(edit.new_node_partial)
                new_node_packed = pack_builtin_object_data(new_node, only=properties or None)
            else:
                new_node_packed = None
            # break out secret properties
            old_node_packed, old_node_secret_packed = _split_node_packed_sensitive(
                node_type, old_node_packed
            )
            new_node_packed, new_node_secret_packed = _split_node_packed_sensitive(
                node_type, new_node_packed
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
                # meta
                kind=wire.LogKind.CHANGE,
                level=wire.LogLevel.INFO,
                undo_of_ptr=edit.undo_of_ptr,
                # content
                type=cast(wire.AccessType, edit.type),
                node_ptr=edit.node_ptr,
                properties=edit.properties,
                old_node_packed=old_node_packed,
                old_node_secret_packed=old_node_secret_packed,
                new_node_packed=new_node_packed,
                new_node_secret_packed=new_node_secret_packed,
                new_revision=edit.revision,
                category=edit.category,
                vignette=edit.vignette,
                # session context
                block_ptr=edit.context.block_ptr if edit.context else None,
                step_ptr=edit.context.step_ptr if edit.context else None,
                session_ptr=edit.context.session_ptr if edit.context else None,
                run_ptr=edit.context.run_ptr if edit.context else None,
                run_root_ptr=edit.context.run_root_ptr if edit.context else None,
                client_ptr=context_data.client_ptr,
                machine_ptr=context_data.machine_ptr,
                server_ptr=context_data.server_ptr,
                user_ptr=context_data.user_ptr,
                identity_ptr=edit.context.identity_ptr if edit.context else None,
            )
            create_log_edit = EditData(
                id=log_data.id,
                type=wire.EditType.CREATE,
                scope=edit.scope,
                node_ptr=NodeReference._ref_data_from_node_data(log_data),
                origin=None,
                epoch=edit.epoch,
                revision=log_data.revision,
                new_node_partial=wrap_some_node(log_data),
                edited_at=log_data.created_at,
                subject_ptr=edit.subject_ptr,
            )
            log_edits.append(create_log_edit)

        # add edits to session
        extended_edits.extend(log_edits)

        return extended_edits

    @override
    @tracer.start_as_current_span("host.on_commit")
    async def on_commit(
        self,
        supergraph: NodeSuperGraph,
        graph: NodeGraphLike,
        data_graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        await super().on_commit(supergraph, graph, data_graph, edits, cascaded_edits, epoch)

        assert self._session is not None, f"session not ready in {self!r}"
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        # apply edits to loaded graphs (bench/package)
        bench_edits: list[EditData] = []
        package_edits: list[EditData] = []
        for i, edit in enumerate(chain(edits, cascaded_edits)):
            is_cascaded = i >= len(edits)
            if is_cascaded and edit.type in (EditType.ARCHIVE, EditType.DELETE, EditType.ERASE):
                continue  # remove cascades are implicit
            node_type = NodeType(edit.node_ptr.type)
            if node_type not in LOADED_HOST_NODE_TYPES:
                continue  # not loaded
            if node_type in LOADED_BENCH_NODE_TYPES:
                bench_edits.append(edit)
            if node_type in SOURCE_NODE_TYPES:
                assert edit.scope.package_id, f"no package id in {edit!r}"
                package_id = to_uuid(edit.scope.package_id)
                assert package_id == self._main_package.id, f"bad package id: {package_id!r}"
                package_edits.append(edit)
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
        async with self._session.active():
            self._session.track_many(*graph.nodes)
            commit = unpack_commit(
                session=self._session,
                supergraph=supergraph,
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
                        logger.trace(
                            "host.on_commit.plugin",
                            host=self,
                            plugin=plugin,
                            commit=trimmed_commit,
                            span="current",
                        )
            await self._session.commit(_skip_lock=True)  # already in a locked section
        logger.debug(
            "host.on_commit",
            host=self,
            added=commit.added,
            updated=commit.updated,
            removed=commit.removed,
            span="current",
        )

    #
    # Files
    #

    async def upload_files(
        self, subject: Subject, request: UploadFilesRequest
    ) -> UploadFilesResponse:
        # TODO :Broken :Security: evaluate file upload access
        s3_client = get_s3_client_for_presigning(request.environment)
        handles: list[UploadFilesResponseUploadHandle] = []
        for file_data in request.files:
            # get drive (from in-memory graph)
            if (
                file_data.kind not in (FileKind.DRIVE, FileKind.DRIVE_INLINE)
                or not file_data.sha256
            ):
                raise GRPCError(
                    GRPCStatus.INVALID_ARGUMENT,
                    f"unexpected file: {file_data!r} (kind={file_data.kind}, sha256={file_data.sha256})",
                )
            if file_data.drive_ptr:
                drive = self.bench._graph.get(UUID(file_data.drive_ptr.id))
                if not isinstance(drive, Drive):
                    raise GRPCError(
                        GRPCStatus.INVALID_ARGUMENT,
                        f"unexpected drive: {file_data.drive_ptr!r}->{drive!r}",
                    )
            else:  # default to main drive
                drive = self.bench.main_drive
                assert drive, f"{self.bench!r} has no main drive"
                file_data.drive_ptr = drive._to_plain_ref_data()

            # presign post URL
            file_key = get_file_key(drive, file_data.sha256, file_data.title)
            file_metadata: dict[str, str] = {"2": file_data.id}
            for prop in FileInfoBase.__declared_properties__.values():
                if prop.id is None or prop.id < 50:
                    continue  # exclude content
                prop_value = getattr(file_data, prop.name)
                if prop_value is not None:
                    packed_value = pack_value_scalar(prop_value, prop.type_info)
                    if not isinstance(packed_value, str):
                        packed_value = str(packed_value)
                    file_metadata[prop.id_as_str] = packed_value
            file_fields = {
                f"x-amz-meta-{k.lower().replace('_', '-')}": v for k, v in file_metadata.items()
            }
            if file_data.mime_type:
                file_fields["Content-Type"] = file_data.mime_type
            file_fields["Content-Length"] = str(file_data.size)
            presigned_post: dict = s3_client.generate_presigned_post(
                Bucket=get_drive_bucket(drive),
                Key=file_key,
                Fields=file_fields,
                Conditions=[{k: v} for k, v in file_fields.items()],
                ExpiresIn=S3_PRESIGNED_URL_EXPIRY,
            )
            fields = ProtoStruct().from_dict(presigned_post["fields"])
            get_url = s3_client.generate_presigned_url(
                "get_object",
                Params={"Bucket": get_drive_bucket(drive), "Key": file_key},
                ExpiresIn=S3_PRESIGNED_URL_EXPIRY,
            )
            handle = UploadFilesResponseUploadHandle(
                file=file_data, post_url=presigned_post["url"], fields=fields, get_url=get_url
            )
            handles.append(handle)

        return UploadFilesResponse(handles=handles)

    async def download_files(
        self, subject: Subject, request: DownloadFilesRequest
    ) -> DownloadFilesResponse:
        # TODO :Broken :Security!: evaluate file download access
        s3_client = get_s3_client_for_presigning(request.environment)

        # get files
        async with self.session(readonly=True):
            files_refs = [
                unpack_object_validate(ref, supergraph=None, expect=NodeReference)
                for ref in request.files
            ]
            files = await File.search(
                C(ConditionalOp.IN, property=File.id, value=[f.id for f in files_refs])
            )

        # get pre-signed URLs
        handles: list[DownloadFilesResponseDownloadHandle] = []
        for file in files:
            if file.kind not in (FileKind.DRIVE, FileKind.DRIVE_INLINE) or not file.sha256:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"unexpected file: {file.kind}")
            drive = file.drive
            assert drive, f"no drive for {file!r}"
            bucket = get_drive_bucket(drive)
            file_key = get_file_key(drive, file.sha256, file.title)
            get_url = s3_client.generate_presigned_url(
                "get_object",
                Params={"Bucket": bucket, "Key": file_key},
                ExpiresIn=S3_PRESIGNED_URL_EXPIRY,
            )
            handle = DownloadFilesResponseDownloadHandle(file=file._to_data(), get_url=get_url)
            handles.append(handle)

        return DownloadFilesResponse(handles=handles)


@tracer.start_as_current_span("host.validate_context")
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
    if subject.client.type == ClientType.BENCH_MACHINE:
        if not context.machine or context.machine.parent != subject.server:
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT,
                f"bad machine context for {subject!r}: {context.machine!r}",
            )
    # NOTE :Incomplete: validate Edit context in Host


def get_drive_bucket(drive: Drive) -> str:
    bucket_name = f"bench-{ENV.value}-{CLOUD.name.lower()}-{drive.region.slug}-files"
    return bucket_name


def get_file_key(drive: Drive, sha256: str, title: str) -> str:
    """Gets the key for a file in the given bucket."""
    file_key = f"{drive.id}/{sha256}/{title}"
    return file_key
