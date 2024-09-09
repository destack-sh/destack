import asyncio
from contextlib import asynccontextmanager
from typing import TYPE_CHECKING, cast, override
from uuid import UUID, uuid4

import cachetools
import grpclib
import grpclib.metadata
import structlog
from grpclib.client import Channel
from opentelemetry import trace

from bench.language import Bench, NodeReference, Package, Server
from bench.language.bench import Branch, Client, Machine
from bench.language.const import (
    BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    LOADED_BENCH_NODE_TYPES,
    PUBLIC_NODE_TYPES,
    RUNTIME_NODE_TYPES,
    SOURCE_NODE_TYPES,
    ClientType,
    NodeType,
)
from bench.language.graph import NodeGraph, NodeSuperGraph
from bench.language.node import EMPTY_SCOPE, GraphScope
from bench.language.run import Run
from bench.language.session import Session
from bench.language.user import User
from bench.proto import wire, wiring
from bench.proto.networking import localize_url
from bench.proto.services import ServiceBase, get_channel
from bench.proto.wire import (
    GraphScopeData,
    HostClient,
    KillRunRequest,
    KillRunResponse,
    PauseRunRequest,
    PauseRunResponse,
    ProcessRunRequest,
    ProcessRunResponse,
    ResolveHostsRequest,
    ResolveHostsRequestBenchKey,
    RpcMetadata,
    RunData,
    RuntimeBase,
    ServiceKind,
    SupervisorClient,
)
from bench.runtime.core import DYNAMIC_CODE_GLOBALS, STATIC_CODE_GLOBALS
from bench.runtime.remote import RemoteEngine
from bench.runtime.runtime import Runtime
from bench.utils.func import CriticalLock
from bench.utils.oracle import Oracle
from bench.utils.task import TaskManager
from bench.utils.telemetry import set_baggage
from bench.utils.tenacity import RETRY_GRPC_FOREVER

if TYPE_CHECKING:
    from bench.runtime.service import RuntimeThreadMode

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.descendants(*SOURCE_NODE_TYPES)
    .ancestors(Bench, Branch)
    .select_all()
    .exclude(Bench.encryption_key)
)


class RuntimeThread(ServiceBase, RuntimeBase):
    """
    A 'thread' for executing Runs in a Runtime in some Session.
    A RuntimeThread processes one top-level Run at a time. It may be paused, resumed or killed.
    A RuntimeThread may reside in any thread or process (incl. main), depending on the context.
    """

    kind = ServiceKind.INTERNAL

    def __init__(
        self,
        *,
        id: int,
        bench_id: UUID,
        supervisor_url: str,
        client_type: ClientType,
        client_id: UUID,
        client_access_token: str,
        server_id: UUID | None,
        machine_id: UUID | None,
        oracle: Oracle,
        mode: "RuntimeThreadMode",
    ):
        super().__init__(logger=logger, tracer=tracer, oracle=oracle)

        self.id = id
        self._nonce = uuid4()

        # bench stuff
        self._supervisor = SupervisorClient(get_channel(supervisor_url))
        self._bench_id = bench_id
        self._bench_ptr = NodeReference(
            type=NodeType.BENCH, id=bench_id, ck=bench_id, bench_id=bench_id
        )
        self._supergraph = NodeSuperGraph(self._bench_ptr)
        self._bench: Bench | None = None
        self._main_package: Package | None = None

        # context
        self._client_type = client_type
        self._client_id = client_id
        self._client_access_token = client_access_token
        self._rpc_metadata = RpcMetadata(
            client_type=cast(wire.ClientType, client_type),
            client_id=str(self._client_id),
            client_access_token=self._client_access_token,
        )
        self._rpc_headers = wiring.pack_rpc_headers(self._rpc_metadata)
        self._server_id = server_id
        self._machine_id = machine_id
        self._client: Client | None = None
        self._server: Server | None = None
        self._machine: Machine | None = None
        self._oracle = oracle

        # processing
        self._mode = mode
        self._session: Session | None = None
        self._runtime: Runtime | None = None
        self._tx_lock: asyncio.Lock = CriticalLock(
            name=f"{self.__class__.__name__}_{self._bench_id or ''}_{self.id}"
        )
        self._tasks = TaskManager(owner=self, logger=logger, oracle=oracle)

    def __str__(self):
        bench_str = repr(self.bench) if self._bench else self._bench_id
        client_str = repr(self._client) if self._client else self._client_id
        return f"{self.id} as {client_str} on {bench_str}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def bench(self) -> Bench:
        assert self._bench is not None, f"no bench for {self!r}"
        return self._bench

    @property
    def main_package(self) -> Package:
        assert self._main_package is not None, f"no main package for {self!r}"
        return self._main_package

    @property
    def epoch(self) -> int:
        assert self._bench is not None, f"no bench for {self!r}"
        assert self._main_package is not None, f"no main package for {self!r}"
        # NOTE :Cleanup: get current runtime epoch from session? supergraph?
        #  (feels clumsy and incorrect to get it just from bench/package here)
        return max(self._bench.connection.epoch, self._main_package.connection.epoch)

    @asynccontextmanager
    async def session(self, *, readonly: bool = False):
        """Gets exclusive query and edit access to the main session."""
        assert self._session is not None, f"no session for {self!r}"
        async with self._tx_lock, self._session.active(readonly=readonly):
            yield self._session

    async def start(self):
        # setup host
        self._host = await self._get_host_client(self._bench_id)
        bench_scope = GraphScopeData(
            metatype=wire.ObjectType.GRAPH_SCOPE, bench_id=str(self._bench_id)
        )
        self._engines = (
            # global engine
            RemoteEngine(
                scope=EMPTY_SCOPE._to_data(),
                node_types=PUBLIC_NODE_TYPES,
                remote=self._supervisor,
                write_retry=RETRY_GRPC_FOREVER,
                rpc_metadata=self._rpc_metadata,
            ),
            # bench engine
            RemoteEngine(
                scope=bench_scope,
                node_types=BENCH_NODE_TYPES | IN_PACKAGE_NODE_TYPES,
                remote=self._host,
                write_retry=RETRY_GRPC_FOREVER,
                rpc_metadata=self._rpc_metadata,
            ),
        )

        # setup thread
        self._session = Session(
            _is_readonly=False,
            _default_scope=GraphScope(bench_id=self._bench_id)._to_data(),
            _engines=self._engines,
            _supervisor=self._supervisor,
            _host=self._host,
            _supergraph=self._supergraph,
            _oracle=self._oracle,
        )
        await self._session.open(set_in_context=False)

        # connect to host
        async with self.session(readonly=True):
            # get bench
            self._bench = await BENCH_QUERY.get(self._bench_ptr, live=True)
            main_branch = self._bench.main_branch
            assert main_branch, f"{self._bench!r} has no main branch"
            assert main_branch.main_package_id, f"{main_branch!r} has no main package"
            main_server = self._bench.main_server
            assert main_server, f"{self._bench!r} has no main server"
            self._client = main_server.clients.get(self._client_id)
            assert self._client, f"{main_server!r} has no client {self._client_id}"
            if self._server_id:
                self._server = self._bench.servers.get(self._server_id)
            if self._machine_id:
                self._machine = main_server.machines.get(self._machine_id)

            # get package
            self._main_package = await PACKAGE_QUERY.get(main_branch.main_package_ptr, live=True)
            self._session.parent = self._main_package

        # update session context
        self._session.client = self._client
        self._session.machine = self._machine
        self._session.server = self._server
        self._session.user = self._client.parent if isinstance(self._client.parent, User) else None
        self._session._subject = self._client.parent
        self._session._origin = (
            self._client.to_origin(nonce=self._nonce)._to_data() if self._client else None
        )

        # finally, start processing runs
        self._runtime = Runtime(
            session=self._session,
            oracle=self._oracle,
            static_glbls=STATIC_CODE_GLOBALS,
            dynamic_glbls=DYNAMIC_CODE_GLOBALS,
        )
        logger.info("thread.start", process=self, bench=self._bench)

    async def _do_process_run(self, run_data: RunData) -> None:
        # TODO :Robustness Run.created_epoch may be ahead of our own epoch if the sync takes longer to
        #  arrive than the request from the scheduler (both from our Host). This means the caller/user
        #  may expect a different current state than we actually have (so we may be behind).
        assert self._session is not None, f"no session for {self!r}"
        assert self._runtime is not None, f"no runtime for {self!r}"
        assert self._main_package is not None, f"no main package for {self!r}"
        package = self._main_package
        assert (
            run_data.parent_ptr and UUID(run_data.package_ptr.id) == package.id
        ), f"{run_data!r} not in {package!r}"
        set_baggage(
            bench_id=self._bench_id,
            client_id=self._client_id,
            server_id=self._server_id,
            machine_id=self._machine_id,
            thread_id=self.id,
        )

        with tracer.start_as_current_span("thread.process_run"):
            async with self._session.active(readonly=True):
                # TODO :Incomplete: watch entire Run (tree) while running to handle pause/abort/...
                #  (and maybe figure out better way to manage :TransientGraphs in supergraph)
                graph = NodeGraph(
                    scope=self._session._get_scope_for_node(package),
                    node_types=RUNTIME_NODE_TYPES,
                    supergraph=self._supergraph,
                )
                run = wiring.unpack_object_validate(
                    run_data,
                    supergraph=self._supergraph,
                    graph=graph,
                    parent=package,
                    session=self._session,
                    expect=Run,
                )
                graph.add(run)
                self._supergraph.add_graph(graph)
            try:
                await self._runtime.run_run(run, return_error=True)
            finally:
                self._supergraph.remove_graph(graph)
            logger.info("thread.process_run", process=self, run=run, span="current")

    @override
    async def process_run(self, request: ProcessRunRequest) -> ProcessRunResponse:
        await self._do_process_run(request.run)
        return ProcessRunResponse()

    @override
    async def pause_run(self, request: PauseRunRequest) -> PauseRunResponse:
        assert self._runtime is not None, f"no runtime for {self!r}"
        run = self._supergraph.get(UUID(request.run.id))
        if not isinstance(run, Run):
            return PauseRunResponse(is_processed=False)
        await self._runtime.pause_run(run)
        return PauseRunResponse(is_processed=True)

    @override
    async def kill_run(self, request: KillRunRequest) -> KillRunResponse:
        assert self._runtime is not None, f"no runtime for {self!r}"
        run = self._supergraph.get(UUID(request.run.id))
        if not isinstance(run, Run):
            return KillRunResponse(is_processed=False)
        await self._runtime.abort_run(run)
        return KillRunResponse(is_processed=True)

    @cachetools.cached({})
    async def _get_host_client(self, bench_id: UUID) -> HostClient:
        request = ResolveHostsRequest(benches=[ResolveHostsRequestBenchKey(id=str(bench_id))])
        retry = RETRY_GRPC_FOREVER.new(self.oracle)
        while retry.should_retry:
            retry.on_attempt()
            try:
                response = await self._supervisor.resolve_hosts(
                    request,
                    metadata=self._rpc_headers,
                    deadline=grpclib.metadata.Deadline.from_timeout(5),
                )
                host_info = response.hosts[0]
                host_info.domain = localize_url(host_info.domain)
                self.logger.info(
                    "runtime.resolve_host",
                    supervisor=self._supervisor,
                    bench_id=bench_id,
                    host_domain=host_info.domain,
                    host_port=host_info.grpc_port,
                )
                host_channel = Channel(
                    host=host_info.domain, port=host_info.grpc_port, ssl=host_info.ssl
                )
                return HostClient(host_channel)
            except Exception as e:
                interval = retry.get_wait_interval()
                self.logger.error(
                    "runtime.resolve_host.error", bench_id=bench_id, exc_info=e, interval=interval
                )
                if not retry.on_error(e):
                    raise
                await self.oracle.sleep(interval)
        else:
            raise retry.to_error(operation=request)

    def close(self):
        self._tasks.close()

    async def wait_closed(self):
        await self._tasks.wait_closed()
