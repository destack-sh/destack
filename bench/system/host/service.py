import asyncio
from contextlib import asynccontextmanager
from itertools import chain
from typing import Any, Mapping, Sequence, cast, override
from uuid import UUID

import structlog
from google.protobuf.message import Message as ProtoMessage
from google.protobuf.struct_pb2 import Struct as ProtoStruct
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import Bench, Drive, NodeReference, Package, Run, Server, Store, Subject
from bench.language.access import Badge, Ownable
from bench.language.bench import Branch, Client
from bench.language.block import Block
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
from bench.language.transaction import edit_data_graph, edit_graph
from bench.language.user import User
from bench.language.value import pack_builtin_object_data, pack_value_scalar
from bench.proto import wire
from bench.proto.wire import (
    DownloadFilesRequest,
    DownloadFilesResponse,
    EditData,
    GraphScopeData,
    HostBase,
    LogData,
    ServiceKind,
    SessionContextData,
    UploadFilesRequest,
    UploadFilesResponse,
)
from bench.proto.wire.common_pb2 import RpcMetadata
from bench.proto.wiring import (
    pack_proto_json,
    unpack_object_validate,
    unwrap_some_node,
    wrap_some_node,
)
from bench.system.graph.graph import (
    GraphIoServiceBase,
    extract_commit_area,
    validate_edit,
)
from bench.system.graph.postgres import PostgresEngine
from bench.system.host.core import Host, HostPlugin, unpack_commit
from bench.system.host.database import DatabasePlugin
from bench.system.host.scheduler import RunPlugin, ScheduleTriggerPlugin, SignalTriggerPlugin
from bench.system.provision.provisioner import Provisioner, get_provisioners_for
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
BENCH_QUERY = Bench.include_descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.include_ancestors(Branch)
    .include_descendants(*SOURCE_NODE_TYPES)
    .select_all()
    .exclude(Bench.encryption_key)
)


class HostService(GraphIoServiceBase, Host, HostBase):
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
            scope=GraphScope(bench_id=bench_id)._to_data(),
        )

        self.bench_id = bench_id
        self.bench_ptr = NodeReference(
            node_type=NodeType.BENCH, id=bench_id, ck=bench_id, bench_id=bench_id
        )
        self._global_store = global_store
        self._global_pg_engine_unscoped = pg_engine_from_store(global_store)
        self._supergraph = NodeSuperGraph(self.bench_ptr)
        self._client_cache = ClientCache(ttl=60)
        self._bench: Bench | None = None
        self._main_package: Package | None = None
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

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {"bench_id": self.bench_id}

    @property
    def scope(self) -> GraphScopeData:
        return self._scope

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
        if readonly:  # noqa: SIM108
            graph_lock_ctx = self._graph_lock.read("all")
        else:
            graph_lock_ctx = self._graph_lock.write("all")
        async with graph_lock_ctx, self._session.active(readonly=readonly):
            self._session._local_epoch = self.epoch
            yield self._session
            if commit:
                await self._session.commit()

    @tracer.start_as_current_span("host.get_request_subject")
    async def get_request_subject(self, request: ProtoMessage, metadata: RpcMetadata) -> Subject:
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._session is not None, f"session not ready in {self!r}"

        # get client
        is_staff = False
        user: User | None = None
        server: Server | None = None
        owned: list[Ownable] = []
        if metadata.client_id and metadata.client_access_token:
            if not metadata.client_type:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "missing client type")
            client_id = UUID(metadata.client_id)
            if metadata.client_type != wire.ClientType.CLIENT_TYPE_BENCH_MACHINE:
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

    @override
    def resolve_request_block(self, block_ptr: NodeReference | UUID) -> Block | None:
        block_ck = block_ptr.ck if isinstance(block_ptr, NodeReference) else block_ptr
        assert block_ck, f"no block ck in {block_ptr!r}"
        node = self.main_package._graph.get(block_ck)
        if not isinstance(node, Block):
            return None
        return node

    async def start(self) -> None:
        trace.get_current_span().set_attribute("bench_id", str(self.bench_id))
        await super().start()

        # load bench
        #  (in different session because we don't have the actual engines yet)
        async with self.global_session(readonly=True) as session:
            # get bench main store so we can get all bench data (some of which is local)
            tmp_bench = await Bench.include_descendants(Store).select_all().get(self.bench_ptr)
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
        database_plugin = DatabasePlugin(self, self._bench, self._main_package)
        self._global_pg_engine = PostgresEngine(
            store=self.global_store,
            bench=self._bench,
            scope=self._scope,
            node_types=IN_BENCH_GLOBAL_NODE_TYPES,
            context=database_plugin.context,
        )
        self._local_pg_engine = PostgresEngine(
            store=self._bench.main_store,
            bench=self._bench,
            scope=self._scope,
            node_types=LOCAL_NODE_TYPES,
            context=database_plugin.context,
        )
        self._engines = (self._global_pg_engine, self._local_pg_engine)
        if HOST_MEMORY_ENGINE_ENABLED:
            inmemory_engines = (
                MemoryEngine(
                    scope=self._scope,
                    node_types=LOADED_BENCH_NODE_TYPES,
                    graph=self._bench._data_graph,
                    include_deleted=False,
                ),
                MemoryEngine(
                    scope=self._scope,
                    node_types=SOURCE_NODE_TYPES,
                    graph=self._main_package._data_graph,
                    include_deleted=False,
                ),
            )
            self._engines = (*inmemory_engines, *self._engines)  # in order of priority
        # we open one Session for the entire lifecycle of the Host
        self._session = Session(
            parent=self._bench.main_branch.main_package,
            _is_readonly=False,
            _default_scope=self.scope,
            _engines=self._engines,
            _extend_commit=self._extend_commit_hook,
            _on_commit=self._on_commit_hook,
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
        self._provisioners = tuple(get_provisioners_for(self, self._bench))
        self._plugins = (
            RunPlugin(self, self._bench),
            SignalTriggerPlugin(self, self._bench),
            ScheduleTriggerPlugin(self, self._bench),
            database_plugin,
            *self._provisioners,
        )
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
    def _prepare_commit(self, subject: Subject, context: SessionContext, edits: Sequence[EditData]):
        assert subject.client and subject.client_ptr, f"no client for {subject!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        # prepare commit
        scope = extract_commit_area(edits, base_graph=self._main_package._data_graph)
        now = self.oracle.utc()
        for edit in edits:
            validate_edit(edit, subject, now)

        # check context
        validate_context(subject, context, edits)

        return scope

    @override
    @tracer.start_as_current_span("host.extend_commit")
    async def extend_commit(
        self,
        session: Session,
        graph: NodeGraphLike,
        data_graph: NodeDataGraphLike,
        context: SessionContext | None,
        edits: Sequence[EditData],
    ) -> Sequence[EditData]:
        extended_edits: list[EditData] = []

        # run plugins
        commit = unpack_commit(
            session=session,
            graph=graph,
            supergraph=session._supergraph,
            edits=edits,
            cascaded_edits=(),
            epoch=self.epoch,
        )
        for plugin in self._plugins:
            await plugin.extend_commit(session, commit)

        # create signals
        ...

        def _trim_node_packed_sensitive(node_type: NodeType, node_packed: dict[str, Any] | None):
            if node_packed is None:
                return None
            node_cls = NODE_CLASS_BY_TYPE[node_type]
            for prop_key in tuple(node_packed.keys()):
                prop = node_cls.__properties_by_id__.get(int(prop_key))
                assert prop is not None, f"missing prop {prop_key} in {node_cls!r}"
                if prop.is_sensitive:
                    del node_packed[prop_key]
            return pack_proto_json(node_packed)

        # add logs
        context_data: SessionContextData = (
            context._to_data() if context is not None else SessionContextData()
        )
        package_ptr = session.package._to_plain_ref_data()
        bench_ptr = session.bench._to_plain_ref_data()
        log_edits: list[EditData] = []
        for edit in chain(edits, extended_edits):
            node_type = NodeType(edit.node_ptr.node_type)
            if node_type in RUNTIME_NODE_TYPES:
                continue
            assert edit.epoch is not None, f"epoch not set in {edit!r}"
            # pack old/new node
            if edit.HasField("node_data"):
                old_node = unwrap_some_node(edit.node_data)
                node_data = pack_builtin_object_data(old_node)
            else:
                node_data = None
            # trim secret properties
            node_data = _trim_node_packed_sensitive(node_type, node_data)
            log_data = LogData(
                metatype=wire.ObjectType.OBJECT_TYPE_LOG,
                id=str(UUIDT()),
                parent_ptr=package_ptr,
                package_ptr=package_ptr,
                bench_ptr=bench_ptr,
                created_at=edit.edited_at,
                created_epoch=edit.epoch,
                updated_at=edit.edited_at,
                updated_epoch=edit.epoch,
                # meta
                kind=wire.LogKind.LOG_KIND_CHANGE,
                level=wire.LogLevel.LOG_LEVEL_INFO,
                # content
                type=cast(wire.AccessType, edit.type),
                operations=edit.operations,
            )
            # meta
            if edit.category != 0:
                log_data.category = edit.category
            if edit.HasField("subject_ptr"):
                log_data.created_by_ptr.CopyFrom(edit.subject_ptr)
                log_data.updated_by_ptr.CopyFrom(edit.subject_ptr)
            if edit.HasField("undo_of_ptr"):
                log_data.undo_of_ptr.CopyFrom(edit.undo_of_ptr)
            # content
            if edit.HasField("node_ptr"):
                log_data.node_ptr.CopyFrom(edit.node_ptr)
            if edit.HasField("vignette"):
                log_data.vignette.CopyFrom(edit.vignette)
            if node_data is not None:
                log_data.node_data.CopyFrom(node_data)
            # session context
            if edit.context.session_ptr.metatype != 0:
                log_data.session_ptr.CopyFrom(edit.context.session_ptr)
            if edit.context.run_ptr.metatype != 0:
                log_data.run_ptr.CopyFrom(edit.context.run_ptr)
            if edit.context.run_root_ptr.metatype != 0:
                log_data.run_root_ptr.CopyFrom(edit.context.run_root_ptr)
            if context_data.client_ptr.metatype != 0:
                log_data.client_ptr.CopyFrom(context_data.client_ptr)
            if context_data.machine_ptr.metatype != 0:
                log_data.machine_ptr.CopyFrom(context_data.machine_ptr)
            if context_data.server_ptr.metatype != 0:
                log_data.server_ptr.CopyFrom(context_data.server_ptr)
            if context_data.user_ptr.metatype != 0:
                log_data.user_ptr.CopyFrom(context_data.user_ptr)
            if edit.context.identity_ptr.metatype != 0:
                log_data.identity_ptr.CopyFrom(edit.context.identity_ptr)
            create_log_edit = EditData(
                id=log_data.id,
                type=wire.EditType.EDIT_TYPE_CREATE,
                scope=edit.scope,
                node_ptr=NodeReference._ref_data_from_node_data(log_data),
                epoch=edit.epoch,
                node_data=wrap_some_node(log_data),
                edited_at=log_data.created_at,
            )
            if edit.HasField("subject_ptr"):
                create_log_edit.subject_ptr.CopyFrom(edit.subject_ptr)
            log_edits.append(create_log_edit)

        # add edits to session
        extended_edits.extend(log_edits)

        return extended_edits

    @override
    @tracer.start_as_current_span("host.on_commit")
    async def on_commit(
        self,
        session: Session,
        graph: NodeGraphLike,
        data_graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
    ):
        await super().on_commit(session, graph, data_graph, edits, cascaded_edits)

        assert self._session is not None, f"session not ready in {self!r}"
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        # apply edits to loaded graphs (bench/package)
        bench_edits: list[EditData] = []
        package_edits: list[EditData] = []
        for i, edit in enumerate(chain(edits, cascaded_edits)):
            is_cascaded = i >= len(edits)
            if is_cascaded and edit.type in (EditType.DELETE, EditType.ERASE):
                continue  # remove cascades are implicit
            node_type = NodeType(edit.node_ptr.node_type)
            if node_type not in LOADED_HOST_NODE_TYPES:
                continue  # not loaded
            if node_type in LOADED_BENCH_NODE_TYPES:
                bench_edits.append(edit)
            if node_type in SOURCE_NODE_TYPES:
                assert edit.scope.package_id, f"no package id in {edit!r}"
                package_id = to_uuid(edit.scope.package_id)
                assert package_id == self._main_package.id, f"bad package id: {package_id!r}"
                package_edits.append(edit)
        for root_node, subedits in (
            (self._bench, bench_edits),
            (self._main_package, package_edits),
        ):
            # filter the in memory edits to only those with an origin (we = system has origin = null)
            external_edits = tuple(e for e in subedits if e.origin.id)
            edit_graph(
                graph=root_node._graph,
                supergraph=self._supergraph,
                edits=external_edits,
                include_deleted=False,
                validate=False,
            )
            # and apply all edits to our cached data graphs
            edit_data_graph(root_node._data_graph, subedits, include_deleted=False)

        # run plugins on commit (in main session)
        async with self._session.active():
            self._session.track_many(*graph.nodes, force=True)  # may come from other session
            commit = unpack_commit(
                session=self._session,
                graph=graph,
                supergraph=session._supergraph,  # use original session's supergraph
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
            await self._session.commit()
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
        self, request: UploadFilesRequest, headers: Mapping
    ) -> UploadFilesResponse:
        # TODO :Broken :Security: evaluate file upload access
        s3_client = get_s3_client_for_presigning(request.environment)
        handles: list[UploadFilesResponse.UploadHandle] = []
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
            if file_data.parent_ptr.metatype != 0:
                drive = self.bench._graph.get(UUID(file_data.parent_ptr.id))
                if not isinstance(drive, Drive):
                    raise GRPCError(
                        GRPCStatus.INVALID_ARGUMENT,
                        f"unexpected drive: {file_data.parent_ptr!r}->{drive!r}",
                    )
            else:  # default to main drive
                drive = self.bench.main_drive
                assert drive, f"{self.bench!r} has no main drive"
                file_data.parent_ptr.CopyFrom(drive._to_plain_ref_data())

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
                    file_metadata[prop.key] = packed_value
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
            fields = ProtoStruct()
            fields.update(presigned_post["fields"])
            get_url = s3_client.generate_presigned_url(
                "get_object",
                Params={"Bucket": get_drive_bucket(drive), "Key": file_key},
                ExpiresIn=S3_PRESIGNED_URL_EXPIRY,
            )
            handle = UploadFilesResponse.UploadHandle(
                file=file_data, post_url=presigned_post["url"], fields=fields, get_url=get_url
            )
            handles.append(handle)

        return UploadFilesResponse(handles=handles)

    async def download_files(
        self, request: DownloadFilesRequest, headers: Mapping
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
        handles: list[DownloadFilesResponse.DownloadHandle] = []
        for file in files:
            if file.kind not in (FileKind.DRIVE, FileKind.DRIVE_INLINE) or not file.sha256:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"unexpected file: {file.kind}")
            drive = file.parent
            assert drive, f"no drive for {file!r}"
            bucket = get_drive_bucket(drive)
            file_key = get_file_key(drive, file.sha256, file.title)
            get_url = s3_client.generate_presigned_url(
                "get_object",
                Params={"Bucket": bucket, "Key": file_key},
                ExpiresIn=S3_PRESIGNED_URL_EXPIRY,
            )
            handle = DownloadFilesResponse.DownloadHandle(file=file._to_data(), get_url=get_url)
            handles.append(handle)

        return DownloadFilesResponse(handles=handles)


@tracer.start_as_current_span("host.validate_context")
def validate_context(subject: Subject, context: SessionContext, edits: Sequence[EditData]):
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
