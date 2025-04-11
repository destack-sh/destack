import asyncio
from contextlib import asynccontextmanager
from typing import TYPE_CHECKING, Any, Callable, Mapping, Sequence, cast, override
from uuid import UUID

import structlog
from google.protobuf.message import Message as ProtoMessage
from google.protobuf.struct_pb2 import Struct as ProtoStruct
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from more_itertools import first
from opentelemetry import trace

from bench.language import (
    BENCH_NODE_TYPES,
    CLOUD,
    LOADED_PACKAGE_NODE_TYPES,
    LOCAL_NODE_TYPES,
    REGIONAL_NODE_TYPES,
    Bench,
    BenchStatus,
    C,
    ClientType,
    Computer,
    ConditionalType,
    EditType,
    Engine,
    File,
    FileBase,
    FileKind,
    GraphScope,
    InlineNode,
    IsRuntime,
    MemoryEngine,
    NodeArea,
    NodeDataGraph,
    NodeGraph,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    Ownable,
    Package,
    PolicySubject,
    Query,
    Session,
    Store,
    TypeBaseNode,
    User,
    bittuple,
    edit_data_graph,
    edit_graph,
    pack_value_scalar,
    patch_graph,
)
from bench.language.core.const import AUTOLOAD_DESCENDANT_TYPES, BENCH_ID, QueryType
from bench.language.core.object import EMPTY_SCOPE_DATA
from bench.pb2 import MessageData
from bench.proto import (
    DownloadFilesRequest,
    DownloadFilesResponse,
    EditData,
    GraphScopeData,
    HostBase,
    Network,
    RpcMetadata,
    ServiceKind,
    UploadFilesRequest,
    UploadFilesResponse,
    unpack_builtin_object_validate,
)
from bench.proto.wiring import unwrap_some_node
from bench.system.core import (
    ClientCache,
    get_s3_client_for_presigning,
    global_session,
    local_pg_engine_from_store,
    pg_engine_from_store,
)
from bench.system.graph import GraphServiceBase, PostgresEngine, is_edit_in_scope
from bench.utils.env import ENV, Env
from bench.utils.oracle import Oracle
from bench.utils.utils import get_from_env

from .commit import unpack_commit
from .plugin import HostPlugin

if TYPE_CHECKING:
    from bench.system.plugin import Provisioner

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

FILE_DOWNLOAD_URL_EXPIRY = get_from_env(
    "FILE_DOWNLOAD_URL_EXPIRY",
    typ=int,
    default=3600,  # :FileUrlExpiry
    description="File download URL expiry (in seconds)",
)

BENCH_QUERY = Bench.include_descendants(
    NodeType.HANDLE, NodeType.PACKAGE, *LOADED_PACKAGE_NODE_TYPES
).select_all()


class HostService(GraphServiceBase, HostBase):
    """
    Host for a Bench, providing the OS-level functionality (lifecycle, resources, scheduling, etc.).
    There is only one Host per Bench. Clients interact with the Bench exclusively via its Host.
    """

    kind = ServiceKind.PUBLIC  # :ServiceKind
    name = "host"

    def __init__(
        self,
        id: str,
        bench_id: UUID,
        global_store: Store,
        regional_store: Store,
        network: Network,
        oracle: Oracle,
        on_error: Callable[[BaseException], None] | None = None,
    ):
        GraphServiceBase.__init__(
            self,
            id=id,
            bench_id=bench_id,
            node_types=BENCH_NODE_TYPES,
            logger=logger,
            tracer=tracer,
            network=network,
            oracle=oracle,
            scope=GraphScope(bench_id=bench_id)._to_data(),
            on_error=on_error,
        )

        self.bench_id = bench_id
        self.bench_ptr = NodeReference(
            node_type=NodeType.BENCH, id=bench_id, ck=bench_id, bench_id=bench_id
        )
        self._global_store = global_store
        self._global_pg_engine_unscoped = pg_engine_from_store(
            "pg-global", global_store, NodeArea.GLOBAL
        )
        self._regional_store = regional_store
        self._regional_pg_engine_unscoped = pg_engine_from_store(
            f"pg-regional-{regional_store.region.name.lower()}",
            regional_store,
            NodeArea.REGIONAL,
        )
        self._supergraph = NodeSuperGraph(name="Host", root_ptr=self.bench_ptr)
        self._client_cache = ClientCache(ttl=60, supergraph=self._supergraph)
        self._bench: Bench | None = None
        self._main_package: Package | None = None
        self._global_pg_engine: PostgresEngine | None = None
        self._regional_pg_engine: PostgresEngine | None = None
        self._local_pg_engine: PostgresEngine | None = None
        self._engines: tuple[Engine, ...] = ()
        self._local_epoch = 0
        self._session: Session | None = None
        self._provisioners: tuple[Provisioner, ...] = ()
        self._plugins: tuple[HostPlugin, ...] = ()  # incl. provisioners

    def __str__(self):
        return f"{self._bench or self.bench_id}"

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {"bench_id": self.bench_id}

    @property
    def scope(self) -> GraphScopeData:
        return self._scope

    @property
    def is_idle(self) -> bool:
        """Check if the Host is idle (no pending requests or processing)."""
        return all(plugin.is_idle for plugin in self._plugins) and super().is_idle

    @property
    def bench(self) -> Bench:
        assert self._bench is not None, f"bench not loaded in {self}"
        return self._bench

    @property
    def main_package(self) -> Package:
        assert self._main_package is not None, f"main package not loaded in {self}"
        return self._main_package

    @property
    def graphs(self) -> tuple[NodeGraph, ...]:
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"main package not loaded in {self!r}"
        return self._bench._graph, self._main_package._graph

    @override
    def get_engines(self) -> tuple[Engine, ...]:
        return self._engines

    @property
    @override
    def request_session_parent(self):
        return self._bench

    @property
    @override
    def split_reads(self):
        return True

    @property
    def global_store(self) -> Store:
        return self._global_store

    def global_session(self, readonly: bool = False):
        return global_session(
            node=None,
            engines=(self._global_pg_engine_unscoped, self._regional_pg_engine_unscoped),
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
        async with graph_lock_ctx, self._session.active():
            self._session._local_epoch = self._local_epoch
            yield self._session
            if commit:
                await self._session.commit()

    @tracer.start_as_current_span("host.get_request_subject")
    async def get_request_subject(
        self, request: ProtoMessage, metadata: RpcMetadata
    ) -> PolicySubject:
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._session is not None, f"session not ready in {self!r}"

        # get client
        is_staff = False
        user: User | None = None
        computer: Computer | None = None
        owned: list[Ownable] = []
        if metadata.client_id and metadata.client_access_token:
            if not metadata.client_type:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "missing client type")
            client_id = UUID(metadata.client_id)
            if self._client_cache.has(client_id):
                client = await self._client_cache.get_and_check(
                    client_id, metadata.client_access_token
                )
            else:
                async with self.global_session():
                    client = await self._client_cache.get_and_check(
                        client_id, metadata.client_access_token
                    )
            if isinstance(client.parent, User):
                if client.parent.main_bench_id == self._bench.id:
                    owned = [client.parent, self._bench]  # type: ignore
                else:
                    owned = [client.parent]  # type: ignore
                is_staff = client.parent.is_staff
                user = client.parent
            elif isinstance(client.parent, Bench):
                computer = client.computer
                owned = [self._bench]  # NOTE :Security: Computers own their Benches for now
            else:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid client parent")
        else:
            client = None

        # NOTE :Incomplete: get roles/memberships/... for subject in this bench

        # use new supergraph instance for session
        supergraph = self._supergraph.instance(name="Request")
        subject = PolicySubject(
            is_authenticated=client is not None,
            is_staff=is_staff,
            client=client,
            user=user,
            computer=computer,
            owned=owned,  # type: ignore
            _supergraph=supergraph,
        )
        return subject

    @override
    async def resolve_request_base(self, node_ptr: NodeReference | UUID) -> TypeBaseNode | None:
        base_id = node_ptr.id if isinstance(node_ptr, NodeReference) else node_ptr
        assert base_id, f"no base id in {node_ptr!r}"
        node = self.main_package._graph.get(base_id)
        if not isinstance(node, InlineNode):
            return None
        return cast(TypeBaseNode, node)

    def get_provisioners(self: "HostService", bench: Bench) -> list["Provisioner"]:
        """Gets all available provisioners for the Bench in *this* environment"""

        provisioners: list[type[Provisioner]]
        if ENV == Env.TEST or ENV == Env.DEV:
            from bench.system.plugin import (
                DockerComputerProvisioner,
                LocalhostStoreProvisioner,
                ScalerProvisioner,
            )

            provisioners = [
                ScalerProvisioner,
                DockerComputerProvisioner,
                LocalhostStoreProvisioner,
            ]
        elif ENV == Env.STAGE or ENV == Env.PROD:
            from bench.system.plugin import (
                KubernetesComputerProvisioner,
                NeonStoreProvisioner,
                ScalerProvisioner,
            )

            provisioners = [
                ScalerProvisioner,
                NeonStoreProvisioner,
                KubernetesComputerProvisioner,
            ]
        else:
            raise RuntimeError(f"unexpected environment: {ENV!r}")

        return [provisioner(self, bench) for provisioner in provisioners]

    async def _activate(self, session: Session, bench: Bench) -> None:
        """Initializes the given Bench for the first time."""
        from bench.system.plugin import StoreProvisioner

        assert bench.status == BenchStatus.RESERVED, f"{bench!r} has unexpected status"
        assert bench.main_store is not None, f"{bench!r} has no main store"

        # use temporary session in HostService during setup
        session.parent = bench
        self._session = session

        # immediately provision local Store
        provisioners = self.get_provisioners(bench)
        store_provisioner = first(
            (p for p in provisioners if isinstance(p, StoreProvisioner)), None
        )
        assert store_provisioner is not None, f"{bench!r} has no store provisioner"
        await store_provisioner.provision(bench.main_store)
        for provisioner in provisioners:
            provisioner.close()
        await asyncio.gather(*(provisioner.wait_closed() for provisioner in provisioners))

        # commit
        bench.status = BenchStatus.ACTIVATED
        await session.commit()
        self._session = None
        logger.info("host.activate", host=self, bench=bench)

    async def start(self) -> None:
        from bench.system.plugin import (
            ClaimPlugin,
            DatabasePlugin,
            RunPlugin,
            ScheduleTriggerPlugin,
        )

        # start base service
        trace.get_current_span().set_attribute("bench_id", str(self.bench_id))
        await super().start()

        # load bench
        #  (in different session because we don't have the local engines yet)
        async with self.global_session() as session:
            # load our bench
            self._bench = await BENCH_QUERY.get(self.bench_ptr, mode="both")
            assert self._bench.main_store is not None, f"{self._bench!r} has no main store"
            assert self._bench.main_package is not None, f"{self._bench!r} has no main package"
            self._main_package = self._bench.main_package
            session.parent = self._bench  # patch in bench for pg context
            session._default_scope = GraphScope(bench_id=self.bench_id)._to_data()
            session._engines += (
                local_pg_engine_from_store(
                    name=f"pg-local-{self._bench.slug}", store=self._bench.main_store
                ),
            )

            # activate if needed
            if self._bench.status < BenchStatus.ACTIVATED:
                await self._activate(session, self._bench)

            # cleanup
            self._bench._untrack_rec()

        # setup main engines
        # (overwrite global pg engine now that we have t he full bench as context)
        database_plugin = DatabasePlugin(self, self._bench, self._main_package)
        self._global_pg_engine = PostgresEngine(
            name="pg-global",
            store=self.global_store,
            bench=self._bench,
            scope=EMPTY_SCOPE_DATA,
            node_types=bittuple(NodeType.BENCH, NodeType.CLIENT, NodeType.USER),
            context=database_plugin.context,
        )
        self._regional_pg_engine = PostgresEngine(
            name=f"pg-regional-{self._bench.main_store.region.name.lower()}",
            store=self._regional_store,
            bench=self._bench,
            scope=self._scope,
            node_types=REGIONAL_NODE_TYPES,
            context=database_plugin.context,
        )
        self._local_pg_engine = PostgresEngine(
            name=f"pg-local-{self._bench.slug}",
            store=self._bench.main_store,
            bench=self._bench,
            scope=self._scope,
            node_types=LOCAL_NODE_TYPES,
            context=database_plugin.context,
        )
        inmemory_engines = (
            MemoryEngine(
                name="inmemory-bench",
                scope=self._scope,
                node_types=bittuple(*BENCH_QUERY.all_node_types),
                graph=self._bench._data_graph,
                include_removed=False,
            ),
        )
        self._engines = (
            *inmemory_engines,  # prefer in memory engines
            self._global_pg_engine,
            self._regional_pg_engine,
            self._local_pg_engine,
        )
        # we open one Session for the entire lifecycle of the Host
        self._session = Session(
            parent=self._bench,
            _default_scope=self.scope,
            _engines=self._engines,
            _pre_commit=self._pre_commit_hook,
            _post_commit=self._post_commit_hook,
            _supergraph=self._bench._supergraph,
            _split_read=True,
            _oracle=self.oracle,
            _skip_add_self=True,
        )
        self._bench._track_rec(self._session)
        await self._session.open(_set_in_context=True)
        self._session.suspend()

        # start plugins
        self._provisioners = tuple(self.get_provisioners(self._bench))
        self._plugins = (
            database_plugin,
            *self._provisioners,
            ScheduleTriggerPlugin(self, self._bench),
            RunPlugin(self, self._bench),
            ClaimPlugin(self, self._bench),
        )
        await asyncio.gather(*(plugin.start() for plugin in self._plugins))
        await asyncio.gather(*(plugin.wait_idle(timeout=10) for plugin in self._plugins))

        # handle builtin bench
        if self.bench_id == BENCH_ID:
            # sync builtins if we're the builtin bench
            from bench.builtin import BuiltinPackage, sync_node

            async with self.session(readonly=False) as session:
                sync_node(
                    parent=self._bench,
                    target=self._main_package,
                    reference=BuiltinPackage,
                    recursive=True,
                )
                edits, _ = await session.commit()
                logger.info(
                    "host.sync_builtins",
                    host=self,
                    bench=self._bench,
                    main_package=self._main_package,
                    builtin_package=BuiltinPackage,
                    edits=len(edits),
                )
        else:
            # add builtin bench directly to every bench
            from bench.builtin import BuiltinPackage

            BuiltinPackageGraph = BuiltinPackage._graph.copy()
            BuiltinPackageGraph.supergraph = self._supergraph
            self._supergraph.add_graph(BuiltinPackageGraph)

        logger.info(
            "host.start",
            host=self,
            bench=self._bench,
            main_package=self._main_package,
            plugins=self._plugins,
        )

    def stop(self) -> None:
        super().stop()
        for plugin in self._plugins:
            plugin.close()

    async def wait_stopped(self) -> None:
        await super().wait_stopped()
        await asyncio.gather(*(plugin.wait_closed() for plugin in self._plugins))
        if self._session is not None:
            await self._session.close()

    #
    # Graph
    #

    @override
    @tracer.start_as_current_span("host.prepare_commit")
    def _parse_commit(self, subject: PolicySubject, context: IsRuntime, edits: Sequence[EditData]):
        assert subject.client and subject.client_ptr, f"no client for {subject!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        # prepare commit
        scope = self._extract_commit_scope(edits)
        now = self.oracle.utc()
        for edit in edits:
            self._validate_edit(edit, subject, now)

        # add any threads
        # NOTE :Cleanup: manually loading more stuff for Thread feels wrong :AutoLoading
        #  (also it only works when we're creating Nodes and thus have Edit.node_data)
        for edit in edits:
            if edit.node_ptr.node_type == NodeType.MESSAGE:
                if edit.HasField("node_data"):
                    message = cast(MessageData, unwrap_some_node(edit.node_data))
                    scope.add_scope(message.thread_ptr)

        # check context
        validate_context(subject, context, edits)

        return scope

    @override
    def _adapt_read_query(self, subject: PolicySubject, query: Query) -> Query:
        query = super()._adapt_read_query(subject, query)

        # restrict to this bench if it's a in-bench query
        #  (non-local because those are already in-bench)
        if (
            self._bench is not None
            and NodeType.BENCH in query._node_cls.__roots__
            and query._node_cls.__area__ != NodeArea.LOCAL
        ):
            query = query.where(query._node_cls.get_property("bench").eq(self._bench.to_ref()))

        # always load members for joinables
        if query._type == QueryType.GET and (
            descendant_types := AUTOLOAD_DESCENDANT_TYPES.get(query._node_type)
        ):
            query = query.include_descendants(*descendant_types)

        return query

    def _update_loaded_graphs(
        self,
        *,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
    ) -> None:
        """
        Applies the given edits to our unpacked or data (or both) graphs.
        NOTE :Architecture :Cleanup: Host loaded graphs feels very similar to system Connections
        """
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"
        bench_graph = self._bench._graph
        bench_data_graph = self._bench._data_graph

        def _apply_edit(edit: EditData):
            # filter the in memory edits to only those with an origin (we = system has origin = null)
            if edit.origin.id:
                edit_graph(
                    graph=bench_graph,
                    supergraph=self._supergraph,
                    edits=(edit,),
                    include_removed=False,
                    validate=False,
                )
            edit_data_graph(
                graph=bench_data_graph,
                edits=(edit,),
                include_removed=False,
            )

        bench_id = str(self._bench.id)
        for edit in edits:
            if is_edit_in_scope(edit, bench_data_graph, root_id=bench_id):
                _apply_edit(edit)
        for edit in cascaded_edits:
            if edit.type in (EditType.ARCHIVE, EditType.DELETE, EditType.ERASE):
                continue  # remove cascades are implicit
            if is_edit_in_scope(edit, bench_data_graph, root_id=bench_id):
                _apply_edit(edit)

    @override
    @tracer.start_as_current_span("host.pre_commit")
    async def pre_commit(
        self,
        session: Session,
        graph: NodeGraph,
        data_graph: NodeDataGraph,
        context: IsRuntime | None,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
    ) -> None:
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        # optimistically apply commit :ConcurrentHost
        self._update_loaded_graphs(edits=edits, cascaded_edits=cascaded_edits)

        # replace copied source nodes in the temporary 'graph' with our loaded originals
        #  (this ensure that we have all the descendants for source nodes loaded)
        for copied_node in graph.nodes:
            original_node = self._main_package._graph.get(copied_node.id)
            if original_node is not None:
                graph.update(original_node)

        # run plugins
        commit = unpack_commit(
            session=session,
            graph=graph,
            supergraph=session._supergraph,  # use original session's supergraph
            edits=edits,
            cascaded_edits=cascaded_edits,
            epoch=self._local_epoch,
        )
        for plugin in self._plugins:
            if plugin.watch_types is None or commit.edited_types & plugin.watch_types:
                await plugin.pre_commit(session, commit)

    @override
    @tracer.start_as_current_span("host.post_commit")
    async def post_commit(
        self,
        session: Session,
        graph: NodeGraph,
        data_graph: NodeDataGraph,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
    ):
        await super().post_commit(session, graph, data_graph, edits, cascaded_edits)

        assert self._session is not None, f"session not ready in {self!r}"

        # run plugins on commit (in main session)
        async with self._session.active():
            self._session._track_many(*graph.nodes, force=True)  # may come from other session
            commit = unpack_commit(
                session=self._session,
                graph=graph,
                supergraph=session._supergraph,  # use original session's supergraph
                edits=edits,
                cascaded_edits=cascaded_edits,
                epoch=self._local_epoch,
            )
            for plugin in self._plugins:
                if plugin.watch_types is None or commit.edited_types & plugin.watch_types:
                    with tracer.start_as_current_span(
                        "host.post_commit.plugin", attributes={"plugin": plugin.name}
                    ):
                        if plugin.watch_types is not None:
                            trimmed_commit = commit.trim_to(plugin.watch_types)
                        else:
                            trimmed_commit = commit
                        await plugin.post_commit(self._session, trimmed_commit)
                        logger.trace(
                            "host.post_commit.plugin",
                            host=self,
                            plugin=plugin,
                            commit=trimmed_commit,
                            span="current",
                        )
            await self._session.commit()
        logger.info(
            "host.post_commit",
            host=self,
            added=commit.added,
            updated=commit.updated,
            removed=commit.removed,
            span="current",
        )

    @override
    async def post_commit_failed(self, session: Session, exc: BaseException):
        # reload graphs (discard optimistic edits)
        assert self._session is not None, f"session not ready in {self!r}"
        assert self._bench is not None, f"bench not loaded in {self!r}"
        async with self._session.active():
            bench = await BENCH_QUERY.get(self.bench_ptr, mode="both")
            patch_graph(old_graph=self._bench._graph, new_graph=bench._graph)

        # run plugins
        for plugin in self._plugins:
            await plugin.post_commit_failed(session, exc)

    #
    # Files
    #

    async def upload_files(
        self, request: UploadFilesRequest, headers: Mapping
    ) -> UploadFilesResponse:
        # TODO :Security: evaluate file upload access
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
                if file_data.parent_ptr.bench_id != str(self.bench.id):
                    raise GRPCError(
                        GRPCStatus.INVALID_ARGUMENT,
                        f"unexpected parent: {file_data.parent_ptr!r}->{self.bench!r}",
                    )
            else:  # default to main drive
                file_data.parent_ptr.CopyFrom(self.main_package._to_ref_data())

            # presign post URL
            file_key = get_file_key(self.bench, file_data.sha256, file_data.name)
            file_metadata: dict[str, str] = {"2": file_data.id}
            for prop in FileBase.__declared_properties__.values():
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
                Bucket=get_drive_bucket(self.bench),
                Key=file_key,
                Fields=file_fields,
                Conditions=[{k: v} for k, v in file_fields.items()],
                ExpiresIn=FILE_DOWNLOAD_URL_EXPIRY,
            )
            fields = ProtoStruct()
            fields.update(presigned_post["fields"])
            get_url = s3_client.generate_presigned_url(
                "get_object",
                Params={"Bucket": get_drive_bucket(self.bench), "Key": file_key},
                ExpiresIn=FILE_DOWNLOAD_URL_EXPIRY,
            )
            handle = UploadFilesResponse.UploadHandle(
                file=file_data, post_url=presigned_post["url"], fields=fields, get_url=get_url
            )
            handles.append(handle)

        return UploadFilesResponse(handles=handles)

    async def download_files(
        self, request: DownloadFilesRequest, headers: Mapping
    ) -> DownloadFilesResponse:
        # TODO :Security!: evaluate file download access
        s3_client = get_s3_client_for_presigning(request.environment)

        # get files
        async with self.session():
            files_refs = [
                unpack_builtin_object_validate(ref, supergraph=None, expect=NodeReference)
                for ref in request.files
            ]
            files = await File.search(
                C(ConditionalType.IN, property=File.id, value=[f.id for f in files_refs])
            )

        # get pre-signed URLs
        handles: list[DownloadFilesResponse.DownloadHandle] = []
        for file in files:
            if file.kind not in (FileKind.DRIVE, FileKind.DRIVE_INLINE) or not file.sha256:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"unexpected file: {file.kind}")
            bucket = get_drive_bucket(self.bench)
            file_key = get_file_key(self.bench, file.sha256, file.name)
            get_url = s3_client.generate_presigned_url(
                "get_object",
                Params={"Bucket": bucket, "Key": file_key},
                ExpiresIn=FILE_DOWNLOAD_URL_EXPIRY,
            )
            handle = DownloadFilesResponse.DownloadHandle(file=file._to_data(), get_url=get_url)
            handles.append(handle)

        return DownloadFilesResponse(handles=handles)


@tracer.start_as_current_span("host.validate_context")
def validate_context(subject: PolicySubject, context: IsRuntime, edits: Sequence[EditData]):
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
    if subject.client.type == ClientType.COMPUTER:
        if not context.computer_ptr or context.computer_ptr.id != subject.computer_id:
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT,
                f"bad computer context for {subject!r}: {context.computer!r}",
            )


def get_drive_bucket(bench: Bench) -> str:
    bucket_name = f"bench-{ENV.value}-{CLOUD.name.lower()}-{bench.region.slug}-files"
    return bucket_name


def get_file_key(bench: Bench, sha256: str, name: str | None) -> str:
    """Gets the key for a file in the given bucket."""
    if name is None:
        return f"{bench.id}/{sha256}/__UNNAMED__"
    else:
        return f"{bench.id}/{sha256}/{name}"
