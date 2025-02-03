import asyncio
from itertools import chain
from typing import TYPE_CHECKING, Any, Collection, Mapping, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import (
    NONCE,
    ClientType,
    GetConnection,
    Interruption,
    InterruptionType,
    NodeReference,
    NodeType,
    Run,
    WatchGetUpdate,
)
from bench.language.core.const import RUNTIME_NODE_TYPES
from bench.pb2 import (
    RunRequest,
    RunResponse,
    RuntimeBase,
    ServiceKind,
)
from bench.pb2.system_grpc import SupervisorClient
from bench.proto import Network, wiring
from bench.runtime.base import RuntimeServiceBase
from bench.runtime.code.context import DYNAMIC_CODE_GLOBALS, STATIC_CODE_GLOBALS
from bench.runtime.core import RedisCache, Runtime
from bench.utils.oracle import Oracle
from bench.utils.telemetry import set_baggage

if TYPE_CHECKING:
    from bench.runtime.runtime import RuntimeThreadMode

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class _RunHandle:
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
        """Runs the handle if it should be running but isn't."""
        if self.root.status.is_terminal:
            return  # nothing to run anymore
        if self.task is None or self.task.done():
            # start 'fresh'
            self.task = asyncio.create_task(self.thread._do_run(self))
        else:
            # resume active
            self.runtime.resume_run(*runs_to_resume)
            logger.debug("thread.run.resume", runs=runs_to_resume)

    def on_update(self, connection: GetConnection, update: WatchGetUpdate):
        """React to updates on Runs/Interruptions :SupergraphWatch."""
        runs_to_resume: set[Run] = set()
        for node in update.updated.values():
            # react to Run/Interruption updates in Runtime
            if isinstance(node, Run):
                if node.status.is_terminal:
                    continue  # nothing to do anymore
                if node.stopped_at:
                    self.stop(node)
                elif node.paused_at and (not node.resumed_at or node.paused_at > node.resumed_at):
                    self.pause(node)
                elif node.resumed_at and (not node.paused_at or node.resumed_at > node.paused_at):
                    # close open Interruption, resume affected Runs
                    interruption = node.interruption
                    if (
                        interruption
                        and interruption.type == InterruptionType.PAUSE
                        and not interruption.status.is_closed
                    ):
                        interruption.complete(_trigger_runtime=False)
                    runs_to_resume.add(node)
                    runs_to_resume.update(node.ancestors)
            elif isinstance(node, Interruption):
                if node.status.is_closed:
                    runs_to_resume.update(self.runtime.get_interrupted_runs(self.root._graph, node))
        if runs_to_resume:
            self.run(runs_to_resume=runs_to_resume)

    def pause(self, run: Run):
        """Pause a Run."""
        # nothing to do? (pause is trapped automatically if active)
        self.log.debug("run.pause", run=run)

    def stop(self, run: Run):
        """Stop a Run."""
        if self.is_active:
            # kill active Run
            self.runtime.stop_run(run)
        else:
            # mark inner Runs/Interrupts as killed
            for node in chain(
                (run,), run._graph.iter_descendants(run, NodeType.RUN, recursive=True)
            ):
                node = cast(Run, node)
                if not node.status.is_terminal:
                    node._mark_stopped()
                    self.runtime.close_run(run, resume=False)
            if run.id == self.root.id:
                self.close()
            else:
                self.run()  # not active, start running again
            self.runtime.session.commit_optimistic()
        self.log.debug("run.stop", run=run)

    def close(self):
        """Close this RunHandle."""
        self.connection.close(release=True)
        if self.root.id in self.thread._managed_runs:
            del self.thread._managed_runs[self.root.id]
        self.log.trace("run.close", run=self.root)


class RuntimeThread(RuntimeServiceBase, RuntimeBase):
    """
    A 'thread' for executing Runs in a Runtime in some Session. Runs may be paused, resumed and killed.
    A RuntimeThread may reside in any logical thread or process (incl. main), depending on context.
    """

    kind = ServiceKind.INTERNAL
    name = "runtime_thread"

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
        machine_id: UUID | None,
        mode: "RuntimeThreadMode",
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
        )
        self.id = id
        self._mode = mode
        self._runtime: Runtime | None = None
        self._lock_by_run: dict[UUID, asyncio.Lock] = {}  # locks for each Run
        self._managed_runs: dict[UUID, _RunHandle] = {}  # Runs this Thread is responsible for
        self._active_runs: dict[UUID, _RunHandle] = {}  # Runs currently active in this Thread

    def __str__(self):
        bench_str = repr(self.bench) if self._bench else self._bench_id
        client_str = repr(self._client) if self._client else self._client_id
        return f"{self.id} as {client_str} on {bench_str}"

    @override
    def get_service_baggage(self) -> dict[str, Any]:
        return {
            "bench_id": self._bench_id,
            "client_id": self._client_id,
            "machine_id": self._machine_id,
            "thread_id": self.id,
        }

    def _set_baggage(self):
        set_baggage(
            bench_id=self._bench_id,
            client_id=self._client_id,
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
        logger.info("runtime_thread.start", process=self, bench=self._bench)

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
        for run in tuple(self._managed_runs.values()):
            if run.task is not None:
                await run.task
            run.close()

    async def _load_run(self, run_ptr: NodeReference) -> _RunHandle:
        """Load the Run for execution."""
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
            if run_ptr.id not in self._managed_runs:
                # actually load the Run
                self._set_baggage()
                with tracer.start_as_current_span("thread.load"):
                    async with self._session.active(readonly=True):
                        run = await Run.include_descendants(
                            NodeType.RUN, NodeType.RUN_SPAN, NodeType.INTERRUPTION
                        ).get(run_ptr, live=True)
                        run._graph.add_types(*RUNTIME_NODE_TYPES)
                    assert run.bench_id == self._bench_id, f"{run!r} is not in {self!r}"
                    assert run.root_ptr is None, f"{run!r} is not a root Run"
                    assert isinstance(run._connection, GetConnection), f"{run!r} has no connection"
                    handle = _RunHandle(run, run._connection, lock, self)
                    self._managed_runs[run_ptr.id] = handle
                    run._connection.on_update(handle.on_update)
            else:
                # already loaded
                handle = self._managed_runs[run_ptr.id]
        return handle

    async def _do_run(self, handle: _RunHandle) -> None:
        """Process a Run (once) until termination/interruption."""
        assert self._session is not None, f"no session for {self!r}"
        assert self._runtime is not None, f"no runtime for {self!r}"

        self._set_baggage()
        with tracer.start_as_current_span("thread.run"):
            # process run
            self._active_runs[handle.root.id] = handle
            try:
                await self._runtime.run(handle.root, return_error=True, optimistic=True)
                logger.info("runtime_thread.run", process=self, run=handle.root, span="current")
                # done, close handle
                if handle.root.status.is_terminal:
                    handle.close()
            finally:
                del self._active_runs[handle.root.id]

    def pause_run(self, run: Run):
        """Pause a Run."""
        handle = self._managed_runs.get(run.root_id or run.id)
        assert handle is not None, f"no handle for {run!r} in {self!r}"
        handle.pause(run)

    def resume_run(self, run: Run):
        """Resume a Run."""
        handle = self._managed_runs.get(run.root_id or run.id)
        assert handle is not None, f"no handle for {run!r} in {self!r}"
        handle.run()
        self.runtime.resume_run(run)

    def stop_run(self, run: Run):
        """Stop a Run."""
        handle = self._managed_runs.get(run.root_id or run.id)
        assert handle is not None, f"no handle for {run!r} in {self!r}"
        handle.stop(run)

    @override
    async def run(self, request: RunRequest, headers: Mapping) -> RunResponse:
        # TODO :Robustness: Run.created_epoch may be ahead of our own epoch if the sync takes longer to
        #  arrive than the request from the scheduler (both from our Host). This means the caller/user
        #  may expect a different current state than we actually have (so we may be behind).
        run_ptr = wiring.unpack_builtin_object_validate(
            request.run_ptr, supergraph=None, expect=NodeReference
        )
        handle = await self._load_run(run_ptr)
        handle.run()
        if request.is_blocking and handle.task is not None:
            await handle.task
        return RunResponse()
