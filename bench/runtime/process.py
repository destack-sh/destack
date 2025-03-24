import asyncio
from typing import TYPE_CHECKING, Any, Callable, Mapping, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import NONCE, ClientType, NodeReference
from bench.pb2 import (
    RunRequest,
    RunResponse,
    RuntimeBase,
    ServiceKind,
)
from bench.pb2.system_grpc import SupervisorClient
from bench.proto import Network, wiring
from bench.runtime.base import RuntimeServiceBase
from bench.runtime.code import DYNAMIC_CODE_GLOBALS, STATIC_CODE_GLOBALS
from bench.runtime.core import RedisCache, Runtime
from bench.utils.oracle import Oracle
from bench.utils.telemetry import set_baggage

if TYPE_CHECKING:
    from bench.runtime.runtime import RuntimeProcessMode

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class RuntimeProcess(RuntimeServiceBase, RuntimeBase):
    """
    A 'process' for executing Runs in a Runtime in some Session. Runs may be paused, resumed and killed.
    A RuntimeProcess may reside in any logical process (incl. main), depending on context.
    TODO :Incomplete: 'hibernate'/release Runs away from this RuntimeProcess after some time
     (to make space for other Runs if a Run is interrupted & inactive for a while) :HibernateRuns
    """

    kind = ServiceKind.INTERNAL
    name = "runtime_process"

    def __init__(
        self,
        *,
        id: str,
        bench_id: UUID,
        supervisor: SupervisorClient,
        oracle: Oracle,
        network: Network,
        client_type: ClientType,
        client_id: UUID,
        client_access_token: str,
        computer_id: UUID | None,
        mode: "RuntimeProcessMode",
        on_error: Callable[[BaseException], None] | None = None,
    ):
        super().__init__(
            id=id,
            logger=logger,
            tracer=tracer,
            network=network,
            oracle=oracle,
            bench_id=bench_id,
            supervisor=supervisor,
            client_type=client_type,
            client_id=client_id,
            client_access_token=client_access_token,
            computer_id=computer_id,
            mode=mode,
            on_error=on_error,
        )
        self.id = id
        self._mode = mode
        self._runtime: Runtime | None = None

    def __str__(self):
        bench_str = repr(self.bench) if self._bench else self._bench_id
        client_str = repr(self._client) if self._client else self._client_id
        return f"{self.id} as {client_str} on {bench_str}"

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {
            "bench_id": self._bench_id,
            "client_id": self._client_id,
            "computer_id": self._computer_id,
            "process_id": self.id,
        }

    def _set_baggage(self):
        set_baggage(
            bench_id=self._bench_id,
            client_id=self._client_id,
            computer_id=self._computer_id,
            process_id=self.id,
            process_nonce=NONCE,
        )

    @property
    def runtime(self) -> Runtime:
        assert self._runtime is not None, f"no runtime for {self!r}"
        return self._runtime

    async def start(self):
        await super().start()
        assert self._session is not None, f"no session for {self!r}"
        assert self._bench is not None, f"no bench for {self!r}"
        asyncio.get_running_loop().set_task_factory(asyncio.eager_task_factory)
        cache = RedisCache(bench=self._bench)
        self._runtime = Runtime(
            session=self._session,
            cache=cache,
            oracle=self.oracle,
            process=self,
            static_glbls=STATIC_CODE_GLOBALS,
            dynamic_glbls=DYNAMIC_CODE_GLOBALS,
            on_error=self.on_error,
        )
        await self._runtime.start()
        logger.info("runtime_process.start", process=self, bench=self._bench)

    @override
    def stop(self) -> None:
        if self._runtime is not None:
            self._runtime.stop()
        super().stop()

    @override
    async def wait_stopped(self) -> None:
        if self._runtime is not None:
            await self._runtime.wait_stopped()
        await super().wait_stopped()

    @override
    async def run(self, request: RunRequest, headers: Mapping) -> RunResponse:
        # NOTE :Robustness: Runs may be out of sync with our state because they're pushed separately
        #  (this should be fixed when we switch to :PullRuns instead of pushing in RunPlugin)
        run_ptrs = [
            wiring.unpack_builtin_object_validate(run_ptr, supergraph=None, expect=NodeReference)
            for run_ptr in request.run_ptrs
        ]
        self._set_baggage()
        await asyncio.gather(
            *(self.runtime.run(run_ptr, _return_error=True) for run_ptr in run_ptrs)
        )
        return RunResponse()
