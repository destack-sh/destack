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

from bench.language import ClientType, NodeReference, Run
from bench.pb2 import (
    HealthCheckRequest,
    HealthClient,
    HostClient,
    RunRequest,
    RunResponse,
    RuntimeBase,
    RuntimeClient,
    ServiceKind,
)
from bench.pb2.system_grpc import SupervisorClient
from bench.proto import Network, wiring
from bench.runtime.base import RuntimeServiceBase, RuntimeThreadMode
from bench.runtime.thread import RuntimeThread
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
    description="interval/timeout for thread healthcheck",
)


class RunHandle:
    """A Run that in this service."""

    def __init__(self, service: "RuntimeService", run_ptr: NodeReference, run: Run | None):
        self.service = service
        self.run_ptr = run_ptr
        self.run = run
        self.thread: RuntimeThreadHandle | None = None
        self.started_at: datetime | None = None
        self.task: asyncio.Task | None = None


class RuntimeThreadHandle:
    """An active RuntimeThread in this service."""

    def __init__(self, service: "RuntimeService", *, id: str, mode: RuntimeThreadMode):
        self.service = service
        self.id: str = id
        self.mode = mode

        self._client: RuntimeClient | RuntimeBase | None = None
        self._restarted_at: datetime | None = None
        self._restarts = 0
        self._active_runs: list[RunHandle] = []

        # local
        self._thread: RuntimeThread | None = None
        # process
        self._port: int | None = None
        self._process: Process | None = None

    def __str__(self):
        return f"id={self.id}, mode={self.mode.name}, restarts={self._restarts}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def client(self):
        assert self._client is not None, f"no client for {self!r}"
        return self._client

    async def start(self) -> None:
        """Starts a new RuntimeThread."""

        # start thread
        if self.mode == RuntimeThreadMode.LOCAL:
            # run directly
            assert self.service._host is not None, f"no host for {self!r}"
            self._thread = RuntimeThread(
                id=self.id,
                bench_id=self.service._bench_id,
                supervisor=self.service._supervisor,
                network=self.service.network,
                client_type=self.service._client_type,
                client_id=self.service._client_id,
                client_access_token=self.service._client_access_token,
                machine_id=self.service._machine_id,
                oracle=self.service.oracle,
                mode=self.service._mode,
                on_error=self.service.on_error,
            )
            self._client = self._thread
            await self._thread.start()
        elif self.mode == RuntimeThreadMode.PROCESS:
            # run subprocess
            self._port = random.randint(60000, 65535)
            self._process, self._channel, self._client = await self._start_process(
                id=self.id, port=self._port
            )
        else:
            assert_never(self.mode)
        self._restarted_at = self.service.oracle.utc()

        # start healthcheck (if needed)
        if self.mode == RuntimeThreadMode.PROCESS:
            self.service.tasks.start_scheduled(
                every=RUNTIME_HEALTHCHECK_TIMEOUT,
                process=self.healthcheck,
                task_id=f"thread.{self.id}.healthcheck",
                skip_errors=True,
            )

    async def healthcheck(self):
        """Checks the health of the thread."""
        if self.mode == RuntimeThreadMode.PROCESS:
            assert self._channel is not None, f"no channel for {self!r}"
            client = HealthClient(self._channel)
            request = HealthCheckRequest()
            # if this fails, the thread is automatically restarted (inside .do)
            with contextlib.suppress(Exception):  # don't care about errors up here
                await self.do(client.check(request), timeout=5)

    async def _start_process(self, *, id: str, port: int):
        """Creates a RuntimeThread in a subprocess."""
        assert sys.executable, f"no python executable for {self!r}"
        argv = (
            "python",
            "bench.py",
            "serve",
            "runtime",
            "127.0.0.1",
            str(port),
            f"--thread-id={id}",
        )
        process = await asyncio.create_subprocess_exec(*argv)
        on_exit(process.kill)  # kill on normal exit
        channel = Channel(host="127.0.0.1", port=port, ssl=False)
        client = RuntimeClient(channel)

        return process, channel, client

    async def _do_restart(self):
        """Restarts the given thread."""

    async def restart(self):
        """Restarts the thread."""
        if self.mode == RuntimeThreadMode.LOCAL:
            pass  # nothing
        elif self.mode == RuntimeThreadMode.PROCESS:
            # restart process
            assert self._port is not None, f"no port for {self!r}"
            if self._process is not None:
                try:
                    self._process.kill()
                    _ = await self._process.wait()
                except Exception as e:
                    logger.error("runtime.thread.terminate.error", thread=self, exc_info=e)
            self._process, self._channel, self._client = await self._start_process(
                id=self.id, port=self._port
            )
        else:
            assert_never(self.mode)
        self._restarts += 1
        self._restarted_at = self.service.oracle.utc()
        logger.debug("runtime.thread", thread=self, restarts=self._restarts)

    async def do[T](self, func: Awaitable[T], *, timeout: float | None) -> T:
        """Await something from the given thread. If it doesn't respond in time, we restart it."""
        try:
            return await asyncio.wait_for(func, timeout=timeout)
        except (asyncio.TimeoutError, grpclib.exceptions.StreamTerminatedError) as e:
            logger.error("runtime.thread.timeout", thread=self, error=e)
            # restart if thread wasn't restarted recently
            if (
                not self._restarted_at
                or (self.service.oracle.utc() - self._restarted_at).total_seconds()
                > RUNTIME_HEALTHCHECK_TIMEOUT
            ):
                await self.restart()
            raise

    def stop(self):
        if self.mode == RuntimeThreadMode.LOCAL:
            if self._thread is not None:
                self._thread.stop()
        elif self.mode == RuntimeThreadMode.PROCESS:
            if self._process is not None:
                self._process.terminate()
        else:
            assert_never(self.mode)

    async def wait_stopped(self):
        if self.mode == RuntimeThreadMode.LOCAL:
            if self._thread is not None:
                await self._thread.wait_stopped()
        elif self.mode == RuntimeThreadMode.PROCESS:
            if self._process is not None:
                await self._process.wait()
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
        machine_id: UUID | None,
        max_threads: int,
        max_concurrency_per_thread: int,
        mode: RuntimeThreadMode,
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
            machine_id=machine_id,
            mode=mode,
            on_error=on_error,
        )

        # processing
        self._max_threads = max_threads
        self._max_concurrency_per_thread = max_concurrency_per_thread
        self._run_semaphore = asyncio.BoundedSemaphore(max_threads * max_concurrency_per_thread)
        self._threads: list[RuntimeThreadHandle] = []
        self._active_runs: list[RunHandle] = []
        self._is_stopping = False

    def __str__(self):
        return f"{self._client_id} on {self._bench_id}"

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {
            "bench_id": self._bench_id,
            "client_id": self._client_id,
            "machine_id": self._machine_id,
        }

    @property
    def host(self) -> HostClient:
        assert self._host is not None, f"no host for {self!r}"
        return self._host

    async def start(self):
        await super().start()
        asyncio.get_running_loop().set_task_factory(asyncio.eager_task_factory)
        # start threads
        with contextlib.suppress(Exception):  # ignore errors
            # ensure we're the process group leader (so subprocesses will die with us)
            #  (we ignore errors because either setsid is unavailable or we're already the leader)
            os.setsid()
        assert self._max_threads > 0, f"no threads for {self!r}"
        self._threads = [
            RuntimeThreadHandle(self, id=f"{self.id}-thread-{i}", mode=self._mode)
            for i in range(self._max_threads)
        ]
        await asyncio.gather(*(t.start() for t in self._threads))
        logger.info("runtime.start", runtime=self)

    def stop(self):
        super().stop()
        self._is_stopping = True
        for thread in self._threads:
            thread.stop()

    async def wait_stopped(self):
        await super().wait_stopped()
        await asyncio.gather(*(t.wait_stopped() for t in self._threads), return_exceptions=True)

    @tracer.start_as_current_span("runtime.run")
    async def _do_run(self, run: RunHandle):
        run.started_at = self.oracle.utc()
        self._active_runs.append(run)
        try:
            # acquire thread
            with tracer.start_as_current_span("runtime.acquire_thread"):
                await self._run_semaphore.acquire()
                run.thread = min(self._threads, key=lambda t: len(t._active_runs))
                run.thread._active_runs.append(run)
                logger.trace("runtime.acquire_thread", runtime=self, thread=run.thread)
            # run in thread
            try:
                # schedule extra healthcheck to ensure consistent termination
                extra_healthcheck = self.oracle.call_later(
                    1,
                    lambda: run.thread and asyncio.create_task(run.thread.healthcheck()),
                )
                # and run 'blocking'
                request = RunRequest(run_ptr=run.run_ptr._to_data(), is_blocking=True)
                if isinstance(run.thread.client, RuntimeBase):
                    _ = await run.thread.client.run(request, {})
                elif isinstance(run.thread.client, RuntimeClient):
                    _ = await run.thread.client.run(request)
                else:
                    assert_never(run.thread.client)
                extra_healthcheck.cancel()  # no longer needed
                logger.info("runtime.run", thread=run.thread, run=run.run_ptr, span="current")
            except Exception as e:
                logger.error("runtime.run.error", thread=run.thread, run=run.run_ptr, exc_info=e)
                raise
            finally:
                self._run_semaphore.release()
                run.thread._active_runs.remove(run)
                logger.trace("runtime.release_thread", runtime=self, thread=run.thread)
        finally:
            self._active_runs.remove(run)

    @override
    async def run(self, request: RunRequest, headers: Mapping) -> RunResponse:
        assert self._main_package, f"{self!r} has no main package"
        set_baggage(bench_id=self._bench_id, client_id=self._client_id, run_id=request.run_ptr.id)

        # refuse if stopping
        if self._is_stopping:
            raise grpclib.GRPCError(grpclib.Status.ABORTED, "Runtime is stopping")

        # process it
        run_ptr = wiring.unpack_builtin_object_validate(
            request.run_ptr, supergraph=None, expect=NodeReference
        )
        managed_run = RunHandle(service=self, run_ptr=run_ptr, run=None)
        managed_run.task = asyncio.create_task(self._do_run(managed_run))
        if request.is_blocking:
            await managed_run.task

        return RunResponse()
