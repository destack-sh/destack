import asyncio
from contextlib import asynccontextmanager
from itertools import chain
from typing import Any, Callable, Literal, Mapping, Sequence, cast, override
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
    LOCAL_NODE_TYPES,
    REGIONAL_NODE_TYPES,
    SOURCE_NODE_TYPES,
    STATIC_RESOURCE_NODE_TYPES,
    Bench,
    BenchStatus,
    C,
    ClientType,
    ConditionalType,
    EditType,
    Engine,
    File,
    FileBase,
    FileKind,
    GraphScope,
    InlineSourceNode,
    IsRuntime,
    Machine,
    MemoryEngine,
    NodeArea,
    NodeDataGraph,
    NodeGraph,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    Ownable,
    Package,
    PackageType,
    Query,
    Session,
    Store,
    Subject,
    TypeBaseNode,
    User,
    bittuple,
    edit_data_graph,
    edit_graph,
    pack_value_scalar,
    patch_graph,
)
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
    unpack_node_graph,
)
from bench.system.core import (
    ClientCache,
    get_s3_client_for_presigning,
    global_session,
    local_pg_engine_from_store,
    pg_engine_from_store,
)
from bench.system.graph import (
    GraphServiceBase,
    PostgresEngine,
    extract_commit_area,
    validate_edit,
)
from bench.system.provision import Provisioner, get_provisioners
from bench.system.provision.store import StoreProvisioner
from bench.utils.env import ENV
from bench.utils.func import to_uuid
from bench.utils.oracle import Oracle
from bench.utils.utils import get_from_env

from .core import HostPlugin, unpack_commit
from .database import DatabasePlugin
from .run import RunPlugin
from .trigger import MessageTriggerPlugin, ScheduleTriggerPlugin

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

S3_PRESIGNED_URL_EXPIRY = get_from_env(
    "S3_PRESIGNED_URL_EXPIRY",
    typ=int,
    default=3600,
    description="S3 presigned URL expiry (in seconds)",
)

BENCH_QUERY = Bench.include_descendants(
    NodeType.HANDLE, NodeType.PACKAGE, *STATIC_RESOURCE_NODE_TYPES
).select_all()
PACKAGE_QUERY = (
    Package.include_ancestors(Bench)
    .include_descendants(*SOURCE_NODE_TYPES, NodeType.TASK)
    .select_all()
    .deselect(Bench.encryption_key)
)


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
    async def get_request_subject(self, request: ProtoMessage, metadata: RpcMetadata) -> Subject:
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._session is not None, f"session not ready in {self!r}"

        # get client
        is_staff = False
        user: User | None = None
        machine: Machine | None = None
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
                    owned = [client.parent, self._bench]
                else:
                    owned = [client.parent]
                is_staff = client.parent.is_staff
                user = client.parent
            elif isinstance(client.parent, Bench):
                machine = client.machine
                owned = [self._bench]  # NOTE :Security: Machines own their Benches for now
            else:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid client parent")
        else:
            client = None

        # NOTE :Incomplete: get roles/memberships/... for subject in this bench

        # use new supergraph instance for session
        supergraph = self._supergraph.instance(name="Request")
        subject = Subject(
            is_authenticated=client is not None,
            is_staff=is_staff,
            client=client,
            user=user,
            machine=machine,
            owned=owned,
            _supergraph=supergraph,
        )
        return subject

    @override
    def resolve_request_base(self, node_ptr: NodeReference | UUID) -> TypeBaseNode | None:
        base_id = node_ptr.id if isinstance(node_ptr, NodeReference) else node_ptr
        assert base_id, f"no base id in {node_ptr!r}"
        node = self.main_package._graph.get(base_id)
        if not isinstance(node, InlineSourceNode):
            return None
        return cast(TypeBaseNode, node)

    async def _activate(self, session: Session, bench: Bench) -> None:
        """Initializes the given Bench for the first time."""
        assert bench.status == BenchStatus.RESERVED, f"{bench!r} has unexpected status"
        assert bench.main_store, f"{bench!r} has no main store"

        # use temporary session in HostService during setup
        session.parent = bench
        self._session = session

        # immediately provision local Store
        provisioners = get_provisioners(self, bench)
        store_provisioner = first(
            (p for p in provisioners if isinstance(p, StoreProvisioner)), None
        )
        assert store_provisioner is not None, f"{bench!r} has no store provisioner"
        await store_provisioner.provision(bench.main_store)
        for provisioner in provisioners:
            provisioner.close()
        await asyncio.gather(*(provisioner.wait_closed() for provisioner in provisioners))

        # main Package
        main_package = bench.packages.create(type=PackageType.MAIN, name="Main", slug="main")
        await session.flush(optimistic=True)
        bench.main_package = main_package

        # commit
        bench.status = BenchStatus.ACTIVATED
        await session.commit()
        self._session = None
        logger.info("host.activate", host=self, bench=bench)

    async def start(self) -> None:
        trace.get_current_span().set_attribute("bench_id", str(self.bench_id))
        await super().start()

        # load bench
        #  (in different session because we don't have the local engines yet)
        async with self.global_session() as session:
            # get bench main store so we can get all bench data (some of which is local)
            tmp_bench = await Bench.include_descendants(Store).select_all().get(self.bench_ptr)
            assert tmp_bench.main_store, f"{tmp_bench!r} has no main store"
            session._engines += (
                local_pg_engine_from_store(
                    name=f"pg-local-{tmp_bench.slug}", store=tmp_bench.main_store
                ),
            )

            # initialize bench if not already initialized
            if tmp_bench.status < BenchStatus.ACTIVATED:
                await self._activate(session, tmp_bench)

            # load full bench
            tmp_bench._untrack_rec()
            self._bench = await BENCH_QUERY.get(self.bench_ptr, mode="both")
            assert self._bench.main_store, f"{self._bench!r} has no main store"
            session.parent = self._bench  # patch in bench for pg context
            session._default_scope = GraphScope(bench_id=self.bench_id)._to_data()

            # load packages
            self._main_package = await PACKAGE_QUERY.get(self._bench.main_package_ptr, mode="both")

            # cleanup (discard temporary session)
            tmp_bench.connection.detach()
            del tmp_bench
            self._bench._untrack_rec()
            self._main_package._untrack_rec()

        # setup main engines
        # (overwrite global pg engine now that we have the full bench as context)
        database_plugin = DatabasePlugin(self, self._bench, self._main_package)
        self._global_pg_engine = PostgresEngine(
            name="pg-global",
            store=self.global_store,
            bench=self._bench,
            scope=self._scope,
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
                include_deleted=False,
            ),
            MemoryEngine(
                name="inmemory-main-package",
                scope=self._scope,
                node_types=SOURCE_NODE_TYPES,
                graph=self._main_package._data_graph,
                include_deleted=False,
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
            _on_commit_prepare=self._on_commit_prepare_hook,
            _on_commit=self._on_commit_hook,
            _supergraph=self._bench._supergraph,
            _split_read=True,
            _oracle=self.oracle,
            _skip_add_self=True,
        )
        self._bench._track_rec(self._session)
        self._main_package._track_rec(self._session)
        await self._session.open(_set_in_context=True)
        self._session.suspend()

        # start plugins
        self._provisioners = tuple(get_provisioners(self, self._bench))
        self._plugins = (
            database_plugin,
            *self._provisioners,
            ScheduleTriggerPlugin(self, self._bench),
            MessageTriggerPlugin(self, self._bench),
            RunPlugin(self, self._bench),
        )
        await asyncio.gather(*(plugin.start() for plugin in self._plugins))
        await asyncio.gather(*(plugin.wait_idle(timeout=10) for plugin in self._plugins))
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
    def _parse_commit(self, subject: Subject, context: IsRuntime, edits: Sequence[EditData]):
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
    def _adapt_read_query(self, subject: Subject, query: Query) -> Query:
        query = super()._adapt_read_query(subject, query)

        # restrict to this bench if it's a in-bench query
        #  (non-local because those are already in-bench)
        if (
            self._bench is not None
            and NodeType.BENCH in query._node_cls.__roots__
            and query._node_cls.__area__ != NodeArea.LOCAL
        ):
            query = query.where(query._node_cls.get_property("bench").eq(self._bench.to_ref()))

        return query

    def _update_loaded_graphs(
        self,
        *,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
        scope: Literal["unpacked", "data", "both"],
    ) -> None:
        """Applies the given edits to our unpacked or data (or both) graphs."""
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        bench_edits: list[EditData] = []
        package_edits: list[EditData] = []
        for i, edit in enumerate(chain(edits, cascaded_edits)):
            is_cascaded = i >= len(edits)
            if is_cascaded and edit.type in (EditType.DELETE, EditType.ERASE):
                continue  # remove cascades are implicit
            node_type = NodeType(edit.node_ptr.node_type)
            if (
                node_type not in self._bench._graph.node_types
                and node_type not in self._main_package._graph.node_types
            ):
                continue  # not loaded
            if node_type in self._bench._graph.node_types:
                bench_edits.append(edit)
            if node_type in self._main_package._graph.node_types and edit.scope.package_id:
                package_id = to_uuid(edit.scope.package_id)
                assert package_id == self._main_package.id, f"bad package id: {package_id!r}"
                package_edits.append(edit)
        for root_node, subedits in (
            (self._bench, bench_edits),
            (self._main_package, package_edits),
        ):
            # filter the in memory edits to only those with an origin (we = system has origin = null)
            external_edits = tuple(e for e in subedits if e.origin.id)
            if scope == "both" or scope == "unpacked":
                edit_graph(
                    graph=root_node._graph,
                    supergraph=self._supergraph,
                    edits=external_edits,
                    include_deleted=False,
                    validate=False,
                )
            if scope == "both" or scope == "data":
                edit_data_graph(root_node._data_graph, subedits, include_deleted=False)

    def _reset_loaded_graphs(self):
        """
        Resets the in-memory graphs to match the data graphs. Patches in-place.
        """
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        new_bench_graph = unpack_node_graph(
            self._bench._data_graph, self._supergraph, session=self._session
        )
        patch_graph(old_graph=self._bench._graph, new_graph=new_bench_graph)

        new_package_graph = unpack_node_graph(
            self._main_package._data_graph, self._supergraph, session=self._session
        )
        patch_graph(old_graph=self._main_package._graph, new_graph=new_package_graph)

    @override
    @tracer.start_as_current_span("host.on_commit_prepare")
    async def on_commit_prepare(
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

        # optimistically apply commit (to in-memory unpacked only) :ConcurrentHost
        self._update_loaded_graphs(edits=edits, cascaded_edits=cascaded_edits, scope="unpacked")
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
                await plugin.on_commit_prepare(session, commit)

    @override
    @tracer.start_as_current_span("host.on_commit")
    async def on_commit(
        self,
        session: Session,
        graph: NodeGraph,
        data_graph: NodeDataGraph,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
    ):
        await super().on_commit(session, graph, data_graph, edits, cascaded_edits)

        assert self._session is not None, f"session not ready in {self!r}"
        assert self._bench is not None, f"bench not loaded in {self!r}"
        assert self._main_package is not None, f"package not loaded in {self!r}"

        # apply edits to loaded data graphs (see on_commit_prepare above for optimistic counterpart)
        self._update_loaded_graphs(edits=edits, cascaded_edits=cascaded_edits, scope="data")

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
                        "host.on_commit.plugin", attributes={"plugin": plugin.name}
                    ):
                        if plugin.watch_types is not None:
                            trimmed_commit = commit.trim_to(plugin.watch_types)
                        else:
                            trimmed_commit = commit
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

    @override
    async def on_commit_failed(self, session: Session, exc: BaseException):
        # restore in memory unpacked graphs from data graphs
        #  (we apply edits optimistically above in on_commit_prepare)
        self._reset_loaded_graphs()

        # run plugins
        for plugin in self._plugins:
            await plugin.on_commit_failed(session, exc)

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
                if file_data.parent_ptr.id != str(self.bench.id):
                    raise GRPCError(
                        GRPCStatus.INVALID_ARGUMENT,
                        f"unexpected parent: {file_data.parent_ptr!r}->{self.bench!r}",
                    )
            else:  # default to main drive
                file_data.parent_ptr.CopyFrom(self.bench._to_ref_data())

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
                ExpiresIn=S3_PRESIGNED_URL_EXPIRY,
            )
            fields = ProtoStruct()
            fields.update(presigned_post["fields"])
            get_url = s3_client.generate_presigned_url(
                "get_object",
                Params={"Bucket": get_drive_bucket(self.bench), "Key": file_key},
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
                ExpiresIn=S3_PRESIGNED_URL_EXPIRY,
            )
            handle = DownloadFilesResponse.DownloadHandle(file=file._to_data(), get_url=get_url)
            handles.append(handle)

        return DownloadFilesResponse(handles=handles)


@tracer.start_as_current_span("host.validate_context")
def validate_context(subject: Subject, context: IsRuntime, edits: Sequence[EditData]):
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
    if subject.client.type == ClientType.MACHINE:
        if not context.machine or context.machine != subject.machine:
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT,
                f"bad machine context for {subject!r}: {context.machine!r}",
            )
    # NOTE :Incomplete: validate Edit context in Host


def get_drive_bucket(bench: Bench) -> str:
    bucket_name = f"bench-{ENV.value}-{CLOUD.name.lower()}-{bench.region.slug}-files"
    return bucket_name


def get_file_key(bench: Bench, sha256: str, name: str | None) -> str:
    """Gets the key for a file in the given bucket."""
    if name is None:
        return f"{bench.id}/{sha256}/__UNNAMED__"
    else:
        return f"{bench.id}/{sha256}/{name}"
