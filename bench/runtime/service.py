import asyncio
import contextlib
import random
import sys
from asyncio.subprocess import Process
from dataclasses import dataclass
from datetime import datetime
from typing import Any, Awaitable, assert_never, override
from uuid import UUID, uuid4

import grpclib
import structlog
from grpclib.client import Channel
from opentelemetry import trace

from bench.language.const import RUNTIME_NODE_TYPES, ClientType, RunStatus
from bench.language.graph import NodeGraph
from bench.language.run import Run
from bench.proto import wiring
from bench.proto.wire import (
    HealthCheckRequest,
    HealthClient,
    HostClient,
    KillRunRequest,
    KillRunResponse,
    PauseRunRequest,
    PauseRunResponse,
    ProcessRunRequest,
    ProcessRunResponse,
    RunData,
    RuntimeBase,
    RuntimeClient,
    ServiceKind,
)
from bench.runtime.base import RuntimeServiceBase, RuntimeThreadMode
from bench.runtime.thread import RuntimeThread
from bench.utils.oracle import Oracle
from bench.utils.telemetry import set_baggage
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


RUNTIME_HEALTHCHECK_TIMEOUT = get_from_env(
    "RUNTIME_HEALTHCHECK_TIMEOUT",
    typ=int,
    default=5,
    description="tnterval/timeout for thread healthcheck",
)


@dataclass(slots=True)
class ManagedRun:
    """A runtime run that is managed in this service."""

    service: "RuntimeService"
    run_data: RunData
    run: Run
    thread: "ManagedThread | None" = None
    started_at: datetime | None = None  # tracked outside Run to avoid interference with thread
    task: asyncio.Task | None = None

    def _do_terminate(self, status: RunStatus):
        self.run.status = status
        self.run.terminated_at = self.service.oracle.utc()
        if self.run.started_at:
            self.run.duration = (self.run.terminated_at - self.run.started_at).total_seconds()
        elif self.started_at:
            self.run.duration = (self.run.terminated_at - self.started_at).total_seconds()


class ManagedThread:
    """An active runtime thread running (somehow) in this service."""

    def __init__(self, service: "RuntimeService", *, id: int, mode: RuntimeThreadMode):
        self.service = service
        self.id = id
        self.mode = mode

        self._client: RuntimeClient | RuntimeBase | None = None
        self._started_at: datetime | None = None
        self._lock = asyncio.Lock()
        self._restarts = 0

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
        """Starts a new thread."""

        # start thread
        if self.mode == RuntimeThreadMode.LOCAL:
            # run directly
            assert self.service._host is not None, f"no host for {self!r}"
            self._client = self._thread = RuntimeThread(
                id=self.id,
                bench_id=self.service._bench_id,
                supervisor_url=self.service._supervisor_url,
                client_type=self.service._client_type,
                client_id=self.service._client_id,
                client_access_token=self.service._client_access_token,
                server_id=self.service._server_id,
                machine_id=self.service._machine_id,
                oracle=self.service.oracle,
                mode=self.service._mode,
            )
            await self._thread.start()
        elif self.mode == RuntimeThreadMode.PROCESS:
            # start subprocess
            self._port = random.randint(60000, 65535)
            self._process, self._channel, self._client = await self._start_process(
                id=self.id, port=self._port
            )
        else:
            assert_never(self.mode)
        self._started_at = self.service.oracle.utc()

        # start healthcheck (if needed)
        if self.mode == RuntimeThreadMode.PROCESS:
            self.service.tasks.start_scheduled(
                every=RUNTIME_HEALTHCHECK_TIMEOUT,
                process=self.healthcheck,
                task_id=f"thread.{self.id}.healthcheck",
                skip_errors=True,
            )

    async def healthcheck(self):
        """Periodically checks the health of the thread."""
        if self.mode == RuntimeThreadMode.PROCESS:
            assert self._channel is not None, f"no channel for {self!r}"
            client = HealthClient(self._channel)
            request = HealthCheckRequest()
            # if this fails, the thread is automatically restarted (inside .do)
            with contextlib.suppress(Exception):  # don't care about errors up here
                await self.do(client.check(request), timeout=5)

    async def _start_process(self, *, id: int, port: int):
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
        channel = Channel(host="127.0.0.1", port=port, ssl=False)
        client = RuntimeClient(channel)
        return process, channel, client

    async def _do_restart(self):
        """Restarts the given thread."""
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
                    logger.error("runtime.restart_thread.terminate.error", thread=self, exc_info=e)
            self._process, self._channel, self._client = await self._start_process(
                id=self.id, port=self._port
            )
        else:
            assert_never(self.mode)
        self._restarts += 1
        self._started_at = self.service.oracle.utc()
        logger.debug("runtime.restart_thread", thread=self, restarts=self._restarts)

    async def restart(self):
        """Restarts the thread."""
        # 'kill' any active runs
        async with self.service.session() as session:
            for run in self.service._active_runs:
                if run.thread == self:
                    run._do_terminate(RunStatus.ABORTED)
            await session.commit(optimistic=True)
        # and force restart
        await self._do_restart()

    async def do[T](self, func: Awaitable[T], *, timeout: float | None) -> T:
        """Await something from the given thread. If it doesn't respond in time, we restart it."""
        try:
            return await asyncio.wait_for(func, timeout=timeout)
        except (asyncio.TimeoutError, grpclib.exceptions.StreamTerminatedError) as e:
            logger.error("runtime.in_thread.timeout", thread=self, error=e)
            # restart if thread wasn't restarted recently
            if (
                not self._started_at
                or (self.service.oracle.utc() - self._started_at).total_seconds()
                > RUNTIME_HEALTHCHECK_TIMEOUT
            ):
                await self.restart()
            raise

    def close(self):
        if self.mode == RuntimeThreadMode.LOCAL:
            if self._thread is not None:
                self._thread.close()
        elif self.mode == RuntimeThreadMode.PROCESS:
            if self._process is not None:
                self._process.terminate()
        else:
            assert_never(self.mode)

    async def wait_closed(self):
        if self.mode == RuntimeThreadMode.LOCAL:
            if self._thread is not None:
                await self._thread.wait_closed()
        elif self.mode == RuntimeThreadMode.PROCESS:
            if self._process is not None:
                await self._process.wait()
        else:
            assert_never(self.mode)


class RuntimeService(RuntimeServiceBase, RuntimeBase):
    kind = ServiceKind.INTERNAL  # :ServiceKind

    def __init__(
        self,
        *,
        supervisor_url: str,
        bench_id: UUID,
        client_type: ClientType,
        client_id: UUID,
        client_access_token: str,
        server_id: UUID | None,
        machine_id: UUID | None,
        max_threads: int,
        oracle: Oracle,
        mode: RuntimeThreadMode,
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
        self._nonce = uuid4()

        # processing
        self._max_threads = max_threads
        self._available_threads: asyncio.Queue[ManagedThread] = asyncio.Queue()
        self._threads: list[ManagedThread] = []
        self._active_runs: list[ManagedRun] = []

    def __str__(self):
        return f"{self._client_id} on {self._bench_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {
            "bench_id": self._bench_id,
            "client_id": self._client_id,
            "server_id": self._server_id,
            "machine_id": self._machine_id,
        }

    @property
    def host(self) -> HostClient:
        assert self._host is not None, f"no host for {self!r}"
        return self._host

    async def start(self):
        await super().start()
        # start threads
        assert self._max_threads > 0, f"no threads for {self!r}"
        self._threads = [
            ManagedThread(self, id=i, mode=self._mode) for i in range(self._max_threads)
        ]
        await asyncio.gather(*(t.start() for t in self._threads))
        for thread in self._threads:
            self._available_threads.put_nowait(thread)
        logger.info("runtime.start", runtime=self)

    def close(self):
        super().close()
        for thread in self._threads:
            thread.close()

    async def wait_closed(self):
        await super().wait_closed()
        await asyncio.gather(*(t.wait_closed() for t in self._threads))

    @tracer.start_as_current_span("runtime.process_run")
    async def _do_process_run(self, run: ManagedRun):
        run.started_at = self.oracle.utc()
        self._active_runs.append(run)
        try:
            # acquire thread
            with tracer.start_as_current_span("runtime.acquire_thread"):
                try:
                    run.thread = await self._available_threads.get()
                except asyncio.CancelledError as e:
                    # cancelled before we got a thread
                    logger.trace("runtime.process_run.cancel", error=e)
                    return
            # run in thread
            try:
                # schedule extra healthcheck to ensure consistent termination
                extra_healthcheck = self.oracle.call_later(
                    1,
                    lambda: run.thread and asyncio.create_task(run.thread.healthcheck()),
                )
                # and run 'blocking'
                request = ProcessRunRequest(run=run.run_data, is_blocking=True)
                _ = await run.thread.client.process_run(request)
                extra_healthcheck.cancel()  # no longer needed
                logger.info(
                    "runtime.process_run", thread=run.thread, run=run.run_data, span="current"
                )
            except Exception as e:
                logger.error(
                    "runtime.process_run.error", thread=run.thread, run=run.run_data, exc_info=e
                )
                raise
            finally:
                # release thread
                self._available_threads.put_nowait(run.thread)

        finally:
            self._active_runs.remove(run)

    @override
    async def process_run(self, request: ProcessRunRequest) -> ProcessRunResponse:
        assert self._main_package, f"{self!r} has no main package"
        set_baggage(bench_id=self._bench_id, client_id=self._client_id, run_id=request.run.id)

        # unpack run
        async with self.session() as session:
            graph = NodeGraph(  # :TransientGraphs
                scope=session._get_scope_for_node(self._main_package),
                node_types=RUNTIME_NODE_TYPES,
                supergraph=self._supergraph,
            )
            run = wiring.unpack_object(
                request.run,
                supergraph=self._supergraph,
                graph=graph,
                session=session,
                expect=Run,
            )
            graph.add(run)
            run.status = RunStatus.QUEUED
            self._supergraph.add_graph(graph)
            managed_run = ManagedRun(service=self, run_data=request.run, run=run)
            await session.commit(optimistic=True)

        # process it (queue and run)
        try:
            managed_run.task = asyncio.create_task(self._do_process_run(managed_run))
            if request.is_blocking:
                await managed_run.task
        finally:
            self._supergraph.remove_graph(graph)  # :TransientGraphs

        return ProcessRunResponse()

    @override
    async def pause_run(self, request: PauseRunRequest) -> PauseRunResponse:
        # find active run
        for run in self._active_runs:
            if run.run_data.id == request.run.id:
                if run.thread is None:
                    # not yet running, cancel directly
                    assert run.task is not None, f"no task for {run!r}"
                    run.task.cancel()
                    async with self.session() as session:
                        run.run.status = RunStatus.PAUSED
                        await session.commit(optimistic=True)
                else:
                    # pause in thread
                    request = PauseRunRequest(run=run.run_data)
                    _ = await run.thread.do(run.thread.client.pause_run(request), timeout=None)
                return PauseRunResponse(is_processed=True)
        else:
            return PauseRunResponse(is_processed=False)

    @override
    async def kill_run(self, request: KillRunRequest) -> KillRunResponse:
        # find active run
        for run in self._active_runs:
            if run.run_data.id == request.run.id:
                if run.thread is None:
                    # not yet running, cancel directly
                    assert run.task is not None, f"no task for {run!r}"
                    run.task.cancel()
                    async with self.session() as session:
                        run._do_terminate(RunStatus.CANCELLED)
                        await session.commit(optimistic=True)
                else:
                    # kill in thread
                    request = KillRunRequest(run=run.run_data)
                    _ = await run.thread.do(run.thread.client.kill_run(request), timeout=None)
                return KillRunResponse(is_processed=True)
        else:
            return KillRunResponse(is_processed=False)
