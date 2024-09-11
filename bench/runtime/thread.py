import asyncio
from typing import TYPE_CHECKING, Any, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.const import NONCE, RUNTIME_NODE_TYPES, ClientType
from bench.language.graph import NodeGraph
from bench.language.run import Run
from bench.proto import wiring
from bench.proto.wire import (
    KillRunRequest,
    KillRunResponse,
    PauseRunRequest,
    PauseRunResponse,
    ProcessRunRequest,
    ProcessRunResponse,
    RunData,
    RuntimeBase,
    ServiceKind,
)
from bench.runtime.base import RuntimeServiceBase
from bench.runtime.core import DYNAMIC_CODE_GLOBALS, STATIC_CODE_GLOBALS
from bench.runtime.runtime import Runtime
from bench.utils.oracle import Oracle
from bench.utils.telemetry import set_baggage

if TYPE_CHECKING:
    from bench.runtime.service import RuntimeThreadMode

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class RuntimeThread(RuntimeServiceBase, RuntimeBase):
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
        super().__init__(
            logger=logger,
            tracer=tracer,
            oracle=oracle,
            bench_id=bench_id,
            supervisor_url=supervisor_url,
            client_type=client_type,
            client_id=client_id,
            client_access_token=client_access_token,
            server_id=server_id,
            machine_id=machine_id,
            mode=mode,
        )
        self.id = id
        self._mode = mode
        self._runtime: Runtime | None = None

    def __str__(self):
        bench_str = repr(self.bench) if self._bench else self._bench_id
        client_str = repr(self._client) if self._client else self._client_id
        return f"{self.id} as {client_str} on {bench_str}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {
            "bench_id": self._bench_id,
            "client_id": self._client_id,
            "server_id": self._server_id,
            "machine_id": self._machine_id,
            "thread_id": self.id,
        }

    async def start(self):
        await super().start()
        assert self._session is not None, f"no session for {self!r}"
        self._runtime = Runtime(
            session=self._session,
            oracle=self._oracle,
            static_glbls=STATIC_CODE_GLOBALS,
            dynamic_glbls=DYNAMIC_CODE_GLOBALS,
        )
        asyncio.get_running_loop().set_task_factory(asyncio.eager_task_factory)
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
            thread_nonce=NONCE,
        )
        with tracer.start_as_current_span("thread.process_run"):
            async with self._session.active(readonly=True):
                #  NOTE :Cleanup: figure out better way to manage :TransientGraphs in supergraph
                graph = NodeGraph(
                    scope=self._session._get_scope_for_node(self._main_package),
                    node_types=RUNTIME_NODE_TYPES,
                    supergraph=self._supergraph,
                )
                run = wiring.unpack_object_validate(
                    run_data,
                    supergraph=self._supergraph,
                    graph=graph,
                    session=self._session,
                    expect=Run,
                )
                graph.add(run)
                self._supergraph.add_graph(graph)
            try:
                await self._runtime.run(run, return_error=True, optimistic=True)
            finally:
                self._supergraph.remove_graph(graph)  # :TransientGraphs
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
