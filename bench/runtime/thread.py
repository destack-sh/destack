import asyncio
from contextlib import asynccontextmanager
from typing import TYPE_CHECKING
from uuid import UUID, uuid4

import structlog
from opentelemetry import trace

from bench.language import Bench, NodeReference, Package, Server
from bench.language.bench import Branch, Client, Machine
from bench.language.connection import GraphEngine
from bench.language.const import (
    LOADED_BENCH_NODE_TYPES,
    RUNTIME_NODE_TYPES,
    SOURCE_NODE_TYPES,
    NodeType,
)
from bench.language.graph import NodeGraph, NodeSuperGraph
from bench.language.node import GraphScope
from bench.language.run import Run
from bench.language.session import Session
from bench.language.user import User
from bench.proto import wiring
from bench.proto.wire import HostClient, RunData, SupervisorClient
from bench.runtime.core import DYNAMIC_CODE_GLOBALS, STATIC_CODE_GLOBALS
from bench.runtime.runtime import Runtime
from bench.utils.func import CriticalLock
from bench.utils.oracle import Oracle
from bench.utils.task import TaskManager

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.descendants(*SOURCE_NODE_TYPES)
    .ancestors(Bench, Branch)
    .select_all()
    .exclude(Bench.encryption_key)
)


class RuntimeThread:
    """
    A thread for actually executing Runs in a Runtime in some Session with its own state.
    Should be isolated in a separate Process for security and snapshotting.
    """

    def __init__(
        self,
        *,
        id: int,
        bench_id: UUID,
        supervisor: SupervisorClient,
        host: HostClient,
        client_id: UUID,
        server_id: UUID | None,
        machine_id: UUID | None,
        engines: tuple[GraphEngine, ...],
        process_queue: asyncio.Queue[RunData],
        oracle: Oracle,
    ):
        self.id = id
        self._nonce = uuid4()

        # bench stuff
        self._supervisor = supervisor
        self._host = host
        self._bench_id = bench_id
        self._bench_ptr = NodeReference(
            type=NodeType.BENCH, id=bench_id, ck=bench_id, bench_id=bench_id
        )
        self._supergraph = NodeSuperGraph(self._bench_ptr)
        self._bench: Bench | None = None
        self._main_package: Package | None = None

        # context
        self._client_id = client_id
        self._server_id = server_id
        self._machine_id = machine_id
        self._client: Client | None = None
        self._server: Server | None = None
        self._machine: Machine | None = None
        self._engines = engines
        self._oracle = oracle

        # processing
        self._session: Session | None = None
        self._runner: Runtime | None = None
        self._tx_lock: asyncio.Lock = CriticalLock(
            name=f"{self.__class__.__name__}_{self._bench_id or ''}_{self.id}"
        )
        self._run_queue = process_queue
        self._tasks = TaskManager(owner=self, logger=logger, oracle=oracle)

    def __str__(self):
        bench_str = repr(self.bench) if self.bench else self._bench_id
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

        # connect
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
        self._runner = Runtime(
            session=self._session,
            oracle=self._oracle,
            static_glbls=STATIC_CODE_GLOBALS,
            dynamic_glbls=DYNAMIC_CODE_GLOBALS,
        )
        self._tasks.start_queue(
            self._run_queue,
            self._process_run_queue,
            f"{self.bench.slug}_run{self.id}",
            skip_errors=True,
        )
        logger.info("thread.start", process=self, bench=self._bench)

    @tracer.start_as_current_span("thread.process_run")
    async def _process_run_queue(self, run_data: RunData):
        # TODO :Robustness Run.created_epoch may be ahead of our own epoch if the sync takes longer to
        #  arrive than the request from the scheduler (both from our Host). This means the caller/user
        #  may expect a different current state than we actually have (so we may be behind).
        assert self._session is not None, f"no session for {self!r}"
        assert self._runner is not None, f"no runner for {self!r}"
        assert self._main_package is not None, f"no main package for {self!r}"
        package = self._main_package
        assert (
            run_data.parent_ptr and UUID(run_data.package_ptr.id) == package.id
        ), f"{run_data!r} not in {package!r}"
        async with self._session.active(readonly=True):
            # TODO :Incomplete: watch entire Run (tree) while running to handle pause/abort/...
            #  (and maybe figure out better way to manage transient graphs in supergraph)
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
            await self._runner.run_run(run, suppress_error=True)
        finally:
            self._supergraph.remove_graph(graph)
        logger.info("thread.process_run", process=self, run=run, span="current")

    def close(self):
        self._tasks.close()

    async def wait_closed(self):
        await self._tasks.wait_closed()
