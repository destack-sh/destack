import asyncio
import contextlib
import os
import random
import sys
from asyncio.subprocess import Process
from datetime import datetime
from typing import Any, Awaitable, Callable, Mapping, assert_never, override
from uuid import UUID

import grpclib
import structlog
from grpclib.client import Channel
from opentelemetry import trace

from bench.language import ClientType
from bench.pb2 import (
    HealthCheckRequest,
    HealthClient,
    RunRequest,
    RunResponse,
    RuntimeBase,
    RuntimeClient,
    ServiceKind,
    SupervisorClient,
)
from bench.proto import Network, wiring
from bench.runtime.base import RuntimeProcessMode, RuntimeServiceBase
from bench.runtime.process import RuntimeProcess
from bench.utils.oracle import Oracle
from bench.utils.signal import on_exit
from bench.utils.telemetry import set_baggage
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


RUNTIME_HEALTHCHECK_TIMEOUT = get_from_env(
    "RUNTIME_HEALTHCHECK_TIMEOUT",
    typ=int,
    default=5,
    description="interval/timeout for process healthcheck",
)


class RuntimeProcessHandle:
    """An active RuntimeProcess in this service."""

    def __init__(
        self, service: "RuntimeService", *, id: str, process_id: int, mode: RuntimeProcessMode
    ):
        self.service = service
        self.id: str = id
        self.process_id: int = process_id
        self.mode = mode

        self._client: RuntimeClient | RuntimeBase | None = None
        self._restarted_at: datetime | None = None
        self._restarts = 0

        self._local_process: RuntimeProcess | None = None
        self._port: int | None = None
        self._real_process: Process | None = None

    def __str__(self):
        return f"id={self.id}, mode={self.mode.name}, restarts={self._restarts}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def is_idle(self) -> bool:
        """Check if the process is idle (no pending requests or processing)."""
        if self.mode == RuntimeProcessMode.LOCAL:
            assert isinstance(self._local_process, RuntimeProcess), f"no process for {self!r}"
            return self._local_process.runtime.is_idle
        else:
            raise NotImplementedError(f"cannot determine is_idle for {self.mode.name} in {self!r}")

    @property
    def client(self):
        assert self._client is not None, f"no client for {self!r}"
        return self._client

    async def start(self) -> None:
        """Starts a new RuntimeProcess."""

        # start process
        if self.mode == RuntimeProcessMode.LOCAL:
            # run directly
            self._local_process = RuntimeProcess(
                id=self.id,
                bench_id=self.service._bench_id,
                supervisor=self.service._supervisor,
                network=self.service.network,
                client_type=self.service._client_type,
                client_id=self.service._client_id,
                client_access_token=self.service._client_access_token,
                computer_id=self.service._computer_id,
                oracle=self.service.oracle,
                mode=self.service._mode,
                on_error=self.service.on_error,
            )
            self._client = self._local_process
            await self._local_process.start()
        elif self.mode == RuntimeProcessMode.PROCESS:
            # run subprocess
            self._port = random.randint(60000, 65535)
            self._real_process, self._channel, self._client = await self._start_process(
                id=self.id, process_id=self.process_id, port=self._port
            )
        else:
            assert_never(self.mode)
        self._restarted_at = self.service.oracle.utc()

        # start healthcheck (if needed)
        if self.mode == RuntimeProcessMode.PROCESS:
            self.service.tasks.start_scheduled(
                every=RUNTIME_HEALTHCHECK_TIMEOUT,
                process=self.healthcheck,
                task_id=f"process.{self.id}.healthcheck",
                skip_errors=True,
            )

    async def healthcheck(self):
        """Checks the health of the process."""
        if self.mode == RuntimeProcessMode.PROCESS:
            assert self._channel is not None, f"no channel for {self!r}"
            client = HealthClient(self._channel)
            request = HealthCheckRequest()
            # if this fails, the process is automatically restarted (inside .do)
            with contextlib.suppress(Exception):  # don't care about errors up here
                await self.do(client.check(request), timeout=5)

    async def _start_process(self, *, id: str, process_id: int, port: int):
        """Creates a RuntimeProcess in a subprocess."""
        assert sys.executable, f"no python executable for {self!r}"
        argv = (
            "python",
            "bench.py",
            "serve",
            "runtime",
            "127.0.0.1",
            str(port),
            f"--process-id={process_id}",
        )
        process = await asyncio.create_subprocess_exec(*argv)
        on_exit(process.kill)  # kill on normal exit
        channel = Channel(host="127.0.0.1", port=port, ssl=False)
        client = RuntimeClient(channel)

        return process, channel, client

    async def _do_restart(self):
        """Restarts the given process."""

    async def restart(self):
        """Restarts the process."""
        if self.mode == RuntimeProcessMode.LOCAL:
            pass  # nothing
        elif self.mode == RuntimeProcessMode.PROCESS:
            # restart process
            assert self._port is not None, f"no port for {self!r}"
            if self._real_process is not None:
                try:
                    self._real_process.kill()
                    _ = await self._real_process.wait()
                except Exception as e:
                    logger.error("runtime_process.terminate.error", process=self, exc_info=e)
            self._real_process, self._channel, self._client = await self._start_process(
                id=self.id, process_id=self.process_id, port=self._port
            )
        else:
            assert_never(self.mode)
        self._restarts += 1
        self._restarted_at = self.service.oracle.utc()
        logger.debug("runtime_process.restart", process=self, restarts=self._restarts)

    async def do[T](self, func: Awaitable[T], *, timeout: float | None) -> T:
        """Await something from the given process. If it doesn't respond in time, we restart it."""
        try:
            return await asyncio.wait_for(func, timeout=timeout)
        except (asyncio.TimeoutError, grpclib.exceptions.StreamTerminatedError) as e:
            logger.error("runtime_process.timeout", process=self, error=e)
            # restart if process wasn't restarted recently
            if (
                not self._restarted_at
                or (self.service.oracle.utc() - self._restarted_at).total_seconds()
                > RUNTIME_HEALTHCHECK_TIMEOUT
            ):
                await self.restart()
            raise

    def stop(self):
        if self.mode == RuntimeProcessMode.LOCAL:
            if self._local_process is not None:
                self._local_process.stop()
        elif self.mode == RuntimeProcessMode.PROCESS:
            if self._real_process is not None:
                self._real_process.terminate()
        else:
            assert_never(self.mode)

    async def wait_stopped(self):
        if self.mode == RuntimeProcessMode.LOCAL:
            if self._local_process is not None:
                await self._local_process.wait_stopped()
        elif self.mode == RuntimeProcessMode.PROCESS:
            if self._real_process is not None:
                await self._real_process.wait()
        else:
            assert_never(self.mode)


class RuntimeService(RuntimeServiceBase, RuntimeBase):
    kind = ServiceKind.INTERNAL  # :ServiceKind
    name = "runtime"

    def __init__(
        self,
        *,
        id: str,
        supervisor: SupervisorClient,
        network: Network,
        oracle: Oracle,
        bench_id: UUID,
        client_type: ClientType,
        client_id: UUID,
        client_access_token: str,
        computer_id: UUID | None,
        max_processs: int,
        mode: RuntimeProcessMode,
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

        # processing
        self._max_processs = max_processs
        self._processs: list[RuntimeProcessHandle] = []
        self._is_stop_requested = False

    def __str__(self):
        return f"{self._client_id} on {self._bench_id}"

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {
            "bench_id": self._bench_id,
            "client_id": self._client_id,
            "computer_id": self._computer_id,
        }

    @property
    def is_idle(self) -> bool:
        """Check if the service is idle (no pending requests or processing)."""
        return all(t.is_idle for t in self._processs) and super().is_idle

    async def start(self):
        await super().start()
        asyncio.get_running_loop().set_task_factory(asyncio.eager_task_factory)
        # start processs
        with contextlib.suppress(Exception):  # ignore errors
            # ensure we're the process group leader (so subprocesses will die with us)
            #  (we ignore errors because either setsid is unavailable or we're already the leader)
            os.setsid()
        assert self._max_processs > 0, f"no processs for {self!r}"
        assert self._max_processs == 1, f"multi-processs not supported for {self!r}"  # :RunRouting
        self._processs = [
            RuntimeProcessHandle(self, id=f"{self.id}-process-{i}", process_id=i, mode=self._mode)
            for i in range(self._max_processs)
        ]
        await asyncio.gather(*(t.start() for t in self._processs))
        logger.info("runtime.start", runtime=self)

    def stop(self):
        super().stop()
        self._is_stop_requested = True
        for process in self._processs:
            process.stop()

    async def wait_stopped(self):
        await super().wait_stopped()
        await asyncio.gather(*(t.wait_stopped() for t in self._processs), return_exceptions=True)

    @override
    async def run(self, request: RunRequest, headers: Mapping) -> RunResponse:
        assert self._main_package, f"{self!r} has no main package"

        # refuse if stopping
        if self._is_stop_requested:
            raise grpclib.GRPCError(grpclib.Status.ABORTED, "Runtime is stopping")

        # run in process
        set_baggage(bench_id=self._bench_id, client_id=self._client_id)
        process = self._processs[0]  # :RunRouting
        try:
            # and run 'blocking'
            if isinstance(process.client, RuntimeBase):
                _ = await process.client.run(request, {})
            elif isinstance(process.client, RuntimeClient):
                _ = await process.client.run(request)
            else:
                assert_never(process.client)
            logger.info(
                "runtime_service.run",
                process=process,
                run_ptrs=[wiring.describe_node_ptr(run_ptr) for run_ptr in request.run_ptrs],
                span="current",
            )
        except Exception as e:
            logger.error(
                "runtime_service.run.error",
                process=process,
                run_ptrs=[wiring.describe_node_ptr(run_ptr) for run_ptr in request.run_ptrs],
                exc_info=e,
            )
            raise

        return RunResponse()
