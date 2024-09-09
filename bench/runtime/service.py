import asyncio
import dataclasses
import enum
import random
import sys
from asyncio.subprocess import Process
from dataclasses import dataclass
from datetime import datetime
from typing import Any, Awaitable, assert_never, cast, override
from uuid import UUID, uuid4

import structlog
from grpclib.client import Channel
from opentelemetry import trace

from bench.language import NodeReference
from bench.language.bench import Machine
from bench.language.const import ClientType, NodeType
from bench.proto import wire
from bench.proto.services import ServiceBase
from bench.proto.wire import (
    HostClient,
    KillRunRequest,
    KillRunResponse,
    PauseRunRequest,
    PauseRunResponse,
    ProcessRunRequest,
    ProcessRunResponse,
    RpcMetadata,
    RunData,
    RuntimeBase,
    RuntimeClient,
    ServiceKind,
)
from bench.proto.wiring import pack_rpc_headers
from bench.runtime.remote import RemoteEngine
from bench.runtime.thread import RuntimeThread
from bench.utils.oracle import Oracle
from bench.utils.telemetry import set_baggage

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class RuntimeThreadMode(enum.StrEnum):
    LOCAL = "local"
    PROCESS = "process"


@dataclass(slots=True)
class ManagedThread:
    """An active runtime thread running (somehow) in this service."""

    id: int
    mode: RuntimeThreadMode
    client: RuntimeClient | RuntimeBase
    thread: RuntimeThread | None = None
    started_at: datetime | None = None
    lock: asyncio.Lock = dataclasses.field(default_factory=asyncio.Lock)

    # process
    port: int | None = None
    process: Process | None = None

    # nocheckin: periodically health check and kill active non-local threads


@dataclass(slots=True)
class ManagedRun:
    """A runtime run that is managed in this service."""

    run: RunData
    thread: ManagedThread | None
    task: asyncio.Task | None


class RuntimeService(ServiceBase, RuntimeBase):
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
        super().__init__(logger=logger, tracer=tracer, oracle=oracle)
        self._nonce = uuid4()

        # context
        self._supervisor_url = supervisor_url
        if client_type == ClientType.BENCH_MACHINE and machine_id is None:
            raise ValueError(f"missing machine_id for {client_type} {client_id}")
        self._client_type = client_type
        self._client_id = client_id
        self._client_access_token = client_access_token
        self._rpc_metadata = RpcMetadata(
            client_type=cast(wire.ClientType, client_type),
            client_id=str(self._client_id),
            client_access_token=self._client_access_token,
        )
        self._rpc_headers = pack_rpc_headers(self._rpc_metadata)
        self._server_id = server_id
        self._machine_id = machine_id
        self._machine: Machine | None = None
        self._engines: tuple[RemoteEngine, ...] = ()
        self._host: HostClient | None = None
        self._bench_id = bench_id
        self._bench_ptr = NodeReference(
            type=NodeType.BENCH, id=bench_id, ck=bench_id, bench_id=bench_id
        )

        # processing
        self._mode = mode
        self._max_threads = max_threads
        self._available_threads: asyncio.Queue[ManagedThread] = asyncio.Queue()
        self._threads: list[ManagedThread] = []
        self._active_runs: list[ManagedRun] = []

    def __str__(self):
        return f"{self._client_id} on {self._bench_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def host(self) -> HostClient:
        assert self._host is not None, f"no host for {self!r}"
        return self._host

    async def start(self):
        # start threads
        assert self._max_threads > 0, f"no threads for {self!r}"
        self._threads = await asyncio.gather(
            *(self._start_thread(i) for i in range(self._max_threads))
        )
        for thread in self._threads:
            self._available_threads.put_nowait(thread)
        logger.info("runtime.start", runtime=self)

    def close(self):
        super().close()

    async def _start_thread(self, id: int) -> ManagedThread:
        """Starts a new thread."""
        if self._mode == RuntimeThreadMode.LOCAL:
            # run directly
            assert self._host is not None, f"no host for {self!r}"
            thread = RuntimeThread(
                id=id,
                bench_id=self._bench_id,
                supervisor_url=self._supervisor_url,
                client_type=self._client_type,
                client_id=self._client_id,
                client_access_token=self._client_access_token,
                server_id=self._server_id,
                machine_id=self._machine_id,
                oracle=self.oracle,
                mode=self._mode,
            )
            await thread.start()
            managed_thread = ManagedThread(
                id=id,
                mode=self._mode,
                client=thread,
                thread=thread,
                started_at=self.oracle.utc(),
            )
            return managed_thread
        elif self._mode == RuntimeThreadMode.PROCESS:
            # start subprocess
            port = random.randint(60000, 65535)
            process, _, client = await self._start_thread_process(id=id, port=port)
            managed_thread = ManagedThread(
                id=id,
                mode=self._mode,
                client=client,
                started_at=self.oracle.utc(),
                process=process,
                port=port,
            )
            return managed_thread
        else:
            assert_never(self._mode)

    async def _start_thread_process(self, *, id: int, port: int):
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

    async def _restart_thread(self, thread: ManagedThread):
        """Restarts the given thread."""
        if thread.mode == RuntimeThreadMode.LOCAL:
            pass  # nothing
        elif thread.mode == RuntimeThreadMode.PROCESS:
            # restart process
            assert thread.process is not None, f"no process for {thread!r}"
            assert thread.port is not None, f"no port for {thread!r}"
            thread.process.terminate()
            thread.process, _, thread.client = await self._start_thread_process(
                id=thread.id, port=thread.port
            )
        else:
            assert_never(thread.mode)
        thread.started_at = self.oracle.utc()

    async def _do_in_thread(
        self, thread: ManagedThread, func: Awaitable[Any], *, timeout: float | None
    ):
        """Await something from the given thread. If it doesn't respond in time, we restart it."""
        try:
            await asyncio.wait_for(func, timeout=timeout)
        except asyncio.TimeoutError as e:
            logger.error("runtime.in_thread.timeout", error=e)
            raise

    @tracer.start_as_current_span("runtime.process_run")
    async def _do_process_run(self, run: ManagedRun):
        set_baggage(bench_id=self._bench_id, client_id=self._client_id, run_id=run.run.id)
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
                request = ProcessRunRequest(run=run.run, is_blocking=True)
                _ = await self._do_in_thread(
                    run.thread, run.thread.client.process_run(request), timeout=None
                )
            finally:
                # release thread
                self._available_threads.put_nowait(run.thread)
        finally:
            self._active_runs.remove(run)

    @override
    async def process_run(self, request: ProcessRunRequest) -> ProcessRunResponse:
        run = ManagedRun(run=request.run, thread=None, task=None)
        run.task = asyncio.create_task(self._do_process_run(run))
        if request.is_blocking:
            await run.task
        return ProcessRunResponse()

    @override
    async def pause_run(self, request: PauseRunRequest) -> PauseRunResponse:
        # find active run
        for run in self._active_runs:
            if run.run.id == request.run.id:
                if run.thread is None:
                    # not yet running, cancel directly
                    assert run.task is not None, f"no task for {run!r}"
                    run.task.cancel()
                else:
                    # try to pause in thread
                    request = PauseRunRequest(run=run.run)
                    _ = await self._do_in_thread(
                        run.thread, run.thread.client.pause_run(request), timeout=None
                    )
                return PauseRunResponse(is_processed=True)
        else:
            return PauseRunResponse(is_processed=False)

    @override
    async def kill_run(self, request: KillRunRequest) -> KillRunResponse:
        # find active run
        for run in self._active_runs:
            if run.run.id == request.run.id:
                if run.thread is None:
                    # not yet running, cancel directly
                    assert run.task is not None, f"no task for {run!r}"
                    run.task.cancel()
                else:
                    # try to kill in thread
                    request = KillRunRequest(run=run.run)
                    _ = await self._do_in_thread(
                        run.thread, run.thread.client.kill_run(request), timeout=None
                    )
                return KillRunResponse(is_processed=True)
        else:
            return KillRunResponse(is_processed=False)
