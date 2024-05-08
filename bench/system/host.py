import asyncio
import functools
from typing import Any, Callable
from uuid import UUID

import betterproto
import grpclib.server
import structlog
from groq import AsyncGroq
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Bench, Block, Organization, Package, Subject, User
from bench.language.code_ import run_code_script
from bench.language.const import (
    IN_BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    BlockType,
    NodeType,
    RunStatus,
)
from bench.language.graph import NodeGraph, edit_graph
from bench.language.projection import project_node, render, render_node
from bench.language.query import PostgresEngine, StoreEngine
from bench.language.run import Run, RunError, RunKind
from bench.language.session import Session
from bench.proto import wiring
from bench.proto.services import BenchServiceBase, RpcCallable
from bench.proto.wire import (
    DownloadFilesRequest,
    DownloadFilesResponse,
    EditData,
    GraphScope,
    HostBase,
    RunRequest,
    RunResponse,
    UploadFilesRequest,
    UploadFilesResponse,
)
from bench.system.client import GLOBAL_STORE, global_session
from bench.system.graph import GraphIoServiceBase
from bench.system.resource import provision_pending_resources
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import to_uuid
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)

LOADED_SOURCE_TYPES: tuple[NodeType, ...] = tuple(
    nt
    for nt in IN_PACKAGE_NODE_TYPES
    if nt.id < NodeType.SESSION.id and nt not in (NodeType.RECORD,)
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


LOADED_BENCH_NODE_TYPES: tuple[NodeType, ...] = (
    NodeType.HANDLE,
    NodeType.SERVER,
    NodeType.STORE,
    NodeType.CACHE,
    NodeType.DRIVE,
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
PACKAGE_QUERY = Package.descendants(*LOADED_PACKAGE_NODE_TYPES).ancestors(Bench).include_all()


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
        self._bench_pg_engine = PostgresEngine(
            GLOBAL_STORE, scope=self._bench_scope, node_types=IN_BENCH_NODE_TYPES
        )
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
        # nocheckin :Broken :Performance: use local in memory engines in Host (where possible)
        return (self._bench_pg_engine,)

    async def start(self) -> None:
        async with global_session() as session:
            start = asyncio.get_event_loop().time()
            # load bench
            self._bench = await BENCH_QUERY.get(id=self.bench_id)
            assert self._bench.main_branch is not None, f"{self._bench!r} has no main branch"
            await provision_pending_resources(self._bench, session)

            # preload main packages
            self._main_package = await PACKAGE_QUERY.get(id=self._bench.main_branch.main_package_id)
            self._packages[self._main_package.id] = self._main_package

            await session.commit()
            logger.info("host.start", host=self, duration=asyncio.get_event_loop().time() - start)
        # NOTE :Architecture: untracking should probably happen automatically?
        self._bench._untrack_rec()
        for package in self._packages.values():
            package._untrack_rec()

    def close(self) -> None:
        pass

    async def wait_closed(self) -> None:
        pass

    def _on_graph_edited(self, scopes: tuple[GraphScope, ...], edits: list[EditData]):
        if self._bench is None:
            return  # not loaded yet

        # apply edits to loaded graphs (bench/package)
        for edit in edits:
            node_data = wiring.unwrap_some_node(edit.node)
            if hasattr(node_data, "package_ptr"):
                package_id = to_uuid(getattr(node_data, "package_ptr").id)
                assert package_id is not None, f"missing package id in {edit!r}"
                package = self._packages.get(package_id)
                if package is None:
                    continue
                graph = package._root_graph
                options = PACKAGE_QUERY._options
            else:
                graph = self._bench._root_graph
                options = BENCH_QUERY._options

            assert isinstance(graph, NodeGraph), f"unexpected graph type: {graph!r}"
            edit_graph(graph, (edit,), options)

        # TODO :Incomplete: re-interp packages after edit? (update notices, ...)

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

    #
    # Runs
    #

    async def run(self, subject: Subject, request: RunRequest) -> RunResponse:
        # nocheckin remove/move explicit run (refactor into Runtime) :Demo
        # get context
        package_id = to_uuid(request.block.package_ptr.id)
        if package_id is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing package id")
        package = self._packages.get(package_id)
        if package is None:
            raise GRPCError(GRPCStatus.NOT_FOUND, f"package {package_id} not found/loaded")
        block = package._root_graph.get(UUID(request.block.id))
        if block is None:
            raise GRPCError(GRPCStatus.NOT_FOUND, f"block {request.block.id} not found")
        if not isinstance(block, Block):
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"not a block: {block!r}")

        logs: list[str] = []

        def get_local_vars(session: Session):
            context: dict[str, Any] = {"session": session, "self": block}
            for sibling in block.parent.blocks:
                if sibling.py_ident and sibling.py_ident not in context:
                    context[sibling.py_ident] = sibling
            parent = block
            while parent is not None:
                parent = parent.parent
                if parent and parent.py_ident and parent.py_ident not in context:
                    context[parent.py_ident] = parent

            # nocheckin: super hacky log
            def _print(*args, **kwargs):
                print(*args, **kwargs)
                logs.append(" ".join((repr(a) for a in args)))

            context["print"] = _print
            context["render"] = render
            return context

        # run (naive for :Demo)
        if block.type == BlockType.CODE:
            session = Session(
                parent=package, _engines=self.engines, _fallback_engine=self.engines[0]
            )
            async with session:
                started_at = utcnow_with_tz()
                package._track_rec(session)
                run = Run(
                    parent=package,
                    kind=RunKind.BLOCK,
                    block=block,
                    status=RunStatus.RUNNING,
                    started_at=started_at,
                )
                try:
                    # actually run
                    code_str = block.code.to_string() if block.code else ""
                    run_code_script(code_str, get_local_vars(session))
                    await session.commit()
                    self.on_graph_edited((request.scope,), session.tx.edits)
                except Exception as e:
                    logger.exception("run.code.error", e=e)
                    run.error = RunError.from_exception(e)
                finally:
                    package._untrack_rec()
                    run.terminated_at = utcnow_with_tz()
                    run.duration = (run.terminated_at - started_at).total_seconds()
                return RunResponse(run=run._to_data(), logs=logs)

        elif block.type == BlockType.TEXT:
            session = Session(
                parent=package, _engines=self.engines, _fallback_engine=self.engines[0]
            )
            async with session:
                started_at = utcnow_with_tz()
                package._track_rec(session)
                run = Run(
                    parent=package,
                    kind=RunKind.BLOCK,
                    block=block,
                    status=RunStatus.RUNNING,
                    started_at=started_at,
                )
                try:
                    # get code str
                    groq_client = AsyncGroq(api_key=get_from_env("GROQ_API_KEY"))
                    projection = project_node(block)
                    rendered_projection = render_node(projection)
                    print(rendered_projection)
                    completion = await groq_client.chat.completions.create(
                        messages=[
                            {
                                "role": "system",
                                "content": "\
                                    You are a succinct and helpful assistant.\
                                    Your responses are run in Python, so answer only with valid Python code.\
                                    You may use basic Python features.\
                                    Don't use any external libraries. To return an answer, print(...) it.\
                                    If you absolutely cannot answer, just 'pass'.\
                                    ",
                            },
                            {
                                "role": "user",
                                "content": """\
Examples: 

# returning a single option
print(ChoiceBlock.fields.Option2)

# returning a class instance
print(ClassBlock(field1=True, field2=Option1))
""",
                            },
                            {
                                "role": "user",
                                "content": f"Context\n:{rendered_projection}",
                            },
                            {
                                "role": "user",
                                "content": "Now generate the simplest code that will return an answer to the given context. Just calling print(...) with a value that's not a Block/Field/.. instance is fine too. You may ignore any irrelevant context.",
                            },
                        ],
                        model="llama3-70b-8192",
                    )
                    if not completion.choices:
                        raise ValueError(f"no completion: {completion}")
                    code_str = completion.choices[0].message.content

                    # run code
                    print(code_str)
                    run_code_script(code_str, get_local_vars(session))
                    await session.commit()
                    self.on_graph_edited((request.scope,), session.tx.edits)
                except Exception as e:
                    logger.exception("run.text.error", e=e)
                    run.error = RunError.from_exception(e)
                finally:
                    package._untrack_rec()
                    run.terminated_at = utcnow_with_tz()
                    run.duration = (run.terminated_at - started_at).total_seconds()
                return RunResponse(run=run._to_data(), logs=logs)
        else:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"cannot run block: {block!r}")
