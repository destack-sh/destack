import asyncio
from itertools import chain
from typing import TYPE_CHECKING, Any, Collection, Mapping, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.connection import GetConnection, WatchGetUpdate
from bench.language.const import NONCE, ClientType, NodeType
from bench.language.interrupt import Interrupt, InterruptType
from bench.language.node import NodeReference
from bench.language.run import Run
from bench.proto import wiring
from bench.proto.wire import (
    RunRequest,
    RunResponse,
    RuntimeBase,
    ServiceKind,
)
from bench.runtime.base import RuntimeServiceBase
from bench.runtime.cache import RedisCache
from bench.runtime.core import DYNAMIC_CODE_GLOBALS, STATIC_CODE_GLOBALS
from bench.runtime.runtime import Runtime
from bench.utils.oracle import Oracle
from bench.utils.telemetry import set_baggage

if TYPE_CHECKING:
    from bench.runtime.service import RuntimeThreadMode

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class RunHandle:
    """A handle for a Run in a RuntimeThread."""

    def __init__(
        self, root: Run, connection: GetConnection, lock: asyncio.Lock, thread: "RuntimeThread"
    ) -> None:
        self.root = root
        self.connection = connection
        self.lock = lock
        self.thread = thread
        self.runtime = thread.runtime
        self.task: asyncio.Task | None = None
        self.log = logger.bind(root=root, thread=thread)

    def __str__(self):
        return f"{self.root} in {self.thread} [{'active' if self.is_active else 'inactive'}]"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @property
    def is_active(self) -> bool:
        return self.root.id in self.thread._active_runs

    def run(self, runs_to_resume: Collection[Run] = ()) -> None:
        """Runs the handle if it should be but isn't."""
        if self.root.status.is_terminal:
            return  # nothing to run anymore
        if self.task is None or self.task.done():
            # start 'fresh'
            self.task = asyncio.create_task(self.thread._do_run(self))
        else:
            # resume active
            self.runtime.resume(*runs_to_resume)
            logger.debug("thread.run.resume", runs=runs_to_resume)

    def on_update(self, update: WatchGetUpdate):
        runs_to_resume: set[Run] = set()
        for node in update.updated.values():
            # react to Run/Interrupt updates in Runtime
            if isinstance(node, Run):
                if node.status.is_terminal:
                    continue  # nothing to do anymore
                if node.killed_at:
                    self.kill(node)
                elif node.paused_at and (not node.resumed_at or node.paused_at > node.resumed_at):
                    self.pause(node)
                elif node.resumed_at and (not node.paused_at or node.resumed_at > node.paused_at):
                    # close open Interrupt, resume affected Runs
                    interrupt = node.interrupt
                    if (
                        interrupt
                        and interrupt.type == InterruptType.PAUSE
                        and not interrupt.status.is_closed
                    ):
                        interrupt.complete(_trigger_runtime=False)
                    runs_to_resume.add(node)
                    runs_to_resume.update(node.ancestors)
            elif isinstance(node, Interrupt):
                if node.status.is_closed:
                    runs_to_resume.update(self.runtime.get_interrupted_runs(self.root._graph, node))
        if runs_to_resume:
            self.run(runs_to_resume=runs_to_resume)

    def pause(self, run: Run):
        """Pause an owned Run."""
        # nothing to do? (pause is trapped automatically if active)
        self.log.debug("run.pause", run=run)

    def kill(self, run: Run):
        """Kill an owned Run."""
        if self.is_active:
            # kill active Run
            self.runtime.kill(run)
        else:
            # mark inner Runs/Interrupts as killed
            for node in chain(
                (run,), run._graph.iter_descendants(run, NodeType.RUN, recursive=True)
            ):
                node = cast(Run, node)
                if not node.status.is_terminal:
                    node._mark_killed()
                    self.runtime.close(run, resume=False)
            if run.id == self.root.id:
                self.close()
            else:
                self.run()  # not active, start running again
            self.runtime.session.commit_optimistic()
        self.log.debug("thread.kill", run=run)

    def close(self):
        """Close this RunHandle."""
        self.connection.close()
        del self.thread._owned_runs[self.root.id]


class RuntimeThread(RuntimeServiceBase, RuntimeBase):
    """
    A 'thread' for executing Runs in a Runtime in some Session. Runs may be paused, resumed and killed.
    A RuntimeThread may reside in any logical thread or process (incl. main), depending on context.
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
        self._lock_by_run: dict[UUID, asyncio.Lock] = {}  # locks for each Run
        self._owned_runs: dict[UUID, RunHandle] = {}  # Runs this Thread is responsible for
        self._active_runs: dict[UUID, RunHandle] = {}  # Runs currently active in this Thread

    def __str__(self):
        bench_str = repr(self.bench) if self._bench else self._bench_id
        client_str = repr(self._client) if self._client else self._client_id
        return f"{self.id} as {client_str} on {bench_str}"

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {
            "bench_id": self._bench_id,
            "client_id": self._client_id,
            "server_id": self._server_id,
            "machine_id": self._machine_id,
            "thread_id": self.id,
        }

    def _set_baggage(self):
        set_baggage(
            bench_id=self._bench_id,
            client_id=self._client_id,
            server_id=self._server_id,
            machine_id=self._machine_id,
            thread_id=self.id,
            thread_nonce=NONCE,
        )

    @property
    def runtime(self) -> Runtime:
        assert self._runtime is not None, f"no runtime for {self!r}"
        return self._runtime

    async def start(self):
        await super().start()
        assert self._session is not None, f"no session for {self!r}"
        assert self._bench is not None, f"no bench for {self!r}"
        cache = RedisCache(bench=self._bench)
        self._runtime = Runtime(
            session=self._session,
            cache=cache,
            oracle=self._oracle,
            thread=self,
            static_glbls=STATIC_CODE_GLOBALS,
            dynamic_glbls=DYNAMIC_CODE_GLOBALS,
        )
        asyncio.get_running_loop().set_task_factory(asyncio.eager_task_factory)
        logger.info("thread.start", process=self, bench=self._bench)

    async def _load(self, run_ptr: NodeReference) -> RunHandle:
        """Load the Run tree for execution."""
        assert run_ptr.id is not None, f"missing id for {run_ptr!r}"
        assert self._session is not None, f"no session for {self!r}"
        assert self._main_package is not None, f"no main package for {self!r}"

        # synchronize access so we don't load the same Run twice in case of overlapping requests
        if run_ptr.id in self._lock_by_run:
            lock = self._lock_by_run[run_ptr.id]
        else:
            lock = asyncio.Lock()
            self._lock_by_run[run_ptr.id] = lock

        async with lock:
            if run_ptr.id not in self._owned_runs:
                # actually load the Run
                self._set_baggage()
                with tracer.start_as_current_span("thread.load"):
                    async with self._session.active(readonly=True):
                        run = await Run.include_descendants(NodeType.RUN, NodeType.INTERRUPT).get(
                            run_ptr, live=True
                        )
                    assert run.package_id == self._main_package.id, f"{run!r} is not in {self!r}"
                    assert run.root_ptr is None, f"{run!r} is not a root Run"
                    assert isinstance(run._connection, GetConnection), f"{run!r} has no connection"
                    handle = RunHandle(run, run._connection, lock, self)
                    self._owned_runs[run_ptr.id] = handle
                    run._connection.on_update(handle.on_update)
            else:
                # already loaded
                handle = self._owned_runs[run_ptr.id]
        return handle

    async def _do_run(self, handle: RunHandle) -> None:
        """Process a Run (once) until termination/interruption."""
        assert self._session is not None, f"no session for {self!r}"
        assert self._runtime is not None, f"no runtime for {self!r}"

        self._set_baggage()
        with tracer.start_as_current_span("thread.run"):
            # process run
            self._active_runs[handle.root.id] = handle
            try:
                await self._runtime.run(handle.root, return_error=True, optimistic=True)
                logger.info("thread.run", process=self, run=handle.root, span="current")

                # done, close handle
                if handle.root.status.is_terminal:
                    handle.close()
            finally:
                del self._active_runs[handle.root.id]

    def pause(self, run: Run):
        """Pause an owned Run."""
        handle = self._owned_runs.get(run.root_id or run.id)
        assert handle is not None, f"no handle for {run!r} in {self!r}"
        handle.pause(run)

    def resume(self, run: Run):
        """Resume an owned Run or Interrupt."""
        handle = self._owned_runs.get(run.root_id or run.id)
        assert handle is not None, f"no handle for {run!r} in {self!r}"
        handle.run()
        self.runtime.resume(run)

    def kill(self, run: Run):
        """Kill an owned Run."""
        handle = self._owned_runs.get(run.root_id or run.id)
        assert handle is not None, f"no handle for {run!r} in {self!r}"
        handle.kill(run)

    @override
    async def run(self, request: RunRequest, headers: Mapping) -> RunResponse:
        # TODO :Robustness: Run.created_epoch may be ahead of our own epoch if the sync takes longer to
        #  arrive than the request from the scheduler (both from our Host). This means the caller/user
        #  may expect a different current state than we actually have (so we may be behind).
        run_ptr = wiring.unpack_object_validate(
            request.run_ptr, supergraph=None, expect=NodeReference
        )
        handle = await self._load(run_ptr)
        handle.run()
        if request.is_blocking and handle.task is not None:
            await handle.task
        return RunResponse()
