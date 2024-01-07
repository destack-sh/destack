import asyncio
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from datetime import timedelta
from functools import partial
from typing import Any, Optional
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async

from bench.language import File, Module, RunError, Statement
from bench.language.const import (
    INTERP_NODE_TYPES,
    RUNNABLE_STATEMENT_TYPES,
    ModuleReference,
    NodeTrackingLevel,
    RunStatus,
    SessionAccessLevel,
)
from bench.language.module import _NodeChange
from bench.language.packer import map_value, unpack_value_flat
from bench.language.run import RunErrorKind
from bench.language.session import Session
from bench.proto import wire
from bench.proto.wire import EditData, RunData, StartRunResponseErrorType
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import get_from_env, required_field, sentry_capture
from bench.utils.uuidt import UUIDT

WORKER_RUN_TIMEOUT = get_from_env("WORKER_RUN_TIMEOUT", 3000, type_cast=int)
WORKER_ACTIVE_TIMEOUT = timedelta(seconds=30)
WORKER_ACTIVE_PUBLISH_INTERVAL = 10
WORKER_SCHEDULE_BLOCK_AHEAD = 1

logger = structlog.get_logger(__name__)


@dataclass
class ModuleInfo:
    bench_id: UUID
    os_name: str
    pg_name: str


@dataclass(repr=False, slots=True)
class Job:
    priority: int = 10  # default

    def __lt__(self, other: "Job"):
        return self.priority < other.priority

    def __gt__(self, other):
        return self.priority > other.priority


@dataclass(repr=False, slots=True)
class RunJob(Job):
    """Execute a run, the primary job of a worker process."""

    run_data: RunData = required_field()
    session_id: UUID = required_field()
    global_value: dict[str, Any] | None = None
    tags: list[str] | None = None
    task: asyncio.Task | None = None
    session: Optional[Session] = None
    started: asyncio.Event = field(default_factory=asyncio.Event)
    terminated: asyncio.Event = field(default_factory=asyncio.Event)
    exception: Optional[Exception] = None

    def __str__(self):
        return f"{self.run_data.id}"

    def __repr__(self):
        return f"<RunJob {self}>"


@dataclass(repr=False, slots=True)
class MakeJob(Job):
    """Update the module given some edits."""

    edits: list[EditData] = required_field()


class RunStartError(Exception):
    def __init__(self, type: StartRunResponseErrorType):
        self.type = type


class WorkerProcess:
    """Worker process to run one 'thread' for a Bench module."""

    def __init__(self, module_id: UUID, node: "WorkerNode", process_id: Optional[str]):
        self.node = node
        self.module_id = module_id
        self.bench_id: Optional[UUID] = None  # set in init (requires runtime fetch)
        self.ready = asyncio.Event()

        self.module: Module | None = None

        self.queue: asyncio.Queue[RunJob | MakeJob] = asyncio.PriorityQueue()
        self.executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="worker")
        self.log = logger.bind(
            worker_set=str(self.node.worker_set_id),
            worker=str(self.node.worker_id),
            worker_process=str(process_id),
            module_id=str(self.module_id),
        )

        self._active_runs: dict[UUID, RunJob] = {}
        self._prepared_runs: dict[UUID, RunJob] = {}
        self._last_run_job: Optional[RunJob] = None
        self._pending_created_runs: dict[UUID, wire.RunData] = {}
        self._session_lock = asyncio.Lock()

    @property
    def session_lock(self):
        return self._session_lock

    @property
    def active(self) -> bool:
        last_run_recent = (
            self._last_run_job is not None
            and self._last_run_job.run_data.created_at > utcnow_with_tz() - WORKER_ACTIVE_TIMEOUT
        )
        return last_run_recent or not self.queue.empty()

    async def start(self):
        # get & interp module
        source, info = await self.node.read_module(self.module_id)
        self.bench_id = info.bench_id
        try:
            self.module = await sync_to_async(Module.make)(
                source=source.nodes,
                bench_id=info.bench_id,
                os_name=info.os_name,
                pg_name=info.pg_name,
            )
            for dependency_name in DEFAULT_DEPENDENCIES:
                dependency_ref = ModuleReference(name=dependency_name, version="x", id=None)
                dependency = self.node._cached_modules[dependency_ref]
                self.module.add_dependency(dependency)
            self.log = self.log.bind(module=self.module.name)
        except BaseException as e:
            self.log.error("module.init.failed", exc_info=e)
            raise RuntimeError(f"failed to initialize module worker {self}")
        self.ready.set()

        # recover any prescheduled runs for this process
        try:
            rep: NMessage[RepPullWorkerRunsPayload] = await request(
                NMessageType.PULL_WORKER_RUNS,
                ReqPullWorkerRunsPayload(
                    bench_id=self.node.bench_id,
                    module_id=self.module_id,
                    worker_set_id=self.node.worker_set_id,
                    worker_id=self.node.worker_id,
                    worker_process_id=None,
                ),
                RepPullWorkerRunsPayload,
                retry=3,
                timeout=20,
            )
            logger.debug("worker.recover", runs=rep.p.runs)
            for run in rep.p.runs:
                self.add_run(run)
        except Exception as e:
            logger.error("worker.recover.failed", exc_info=e)

    async def run(self):
        """Runs the module worker main processing loop"""

        # first interp
        self.log.info("worker.start")
        await self.start()

        # process run tasks ad infinitum
        await self._process_jobs_forever()

    async def _process_jobs_forever(self):
        while True:
            job = await self.queue.get()
            if isinstance(job, MakeJob):
                await self._do_make_job(job)
                continue
            if job.run_data.status != RunStatus.QUEUED:
                continue  # cancelled
            try:
                await self._do_run_job(job, WORKER_RUN_TIMEOUT)
            except RunError as e:
                self.log.debug("run.failed", job=job, exc_info=e)
            except BaseException as e:
                self.log.error("run.failed.internal", job=job, sentry=sentry_capture(e), exc_info=e)
            finally:
                self.queue.task_done()

    async def on_module_changed(self, edits: list[EditData]):
        self.log.debug("worker.make.queue", edits=edits)
        await self.queue.put(MakeJob(edits=edits, priority=0))

    async def _do_make_job(self, job: MakeJob) -> None:
        """Actually update the module with the given edits."""

        old_source = self.module._source.deepcopy()
        try:
            # apply edits
            now = utcnow_with_tz()
            edits = [e for e in job.edits if e.node_type not in INTERP_NODE_TYPES]
            self.module._apply_edits(edits)
            duration = utcnow_with_tz() - now
            self.log.info("worker.make", edits=edits, duration=duration.total_seconds())
        except Exception as e:
            self.log.error("worker.make.error", job=job, exc_info=e)
            # revert edits
            self.module._reset_from_source(old_source)

    def add_run(
        self,
        run_data: RunData,
        session_id: Optional[UUID] = None,
        global_value: dict | None = None,
        tags: list[str] | None = None,
    ) -> RunJob:
        """
        Registers a run to be processed by this worker process.
        If scheduled, the run will be queued after any remaining delay.
        """

        if run_data.id in self._prepared_runs:
            raise RunStartError(StartRunResponseErrorType.ALREADY_PREPARED)

        session_id = session_id or UUIDT()
        job = RunJob(run_data=run_data, session_id=session_id, global_value=global_value, tags=tags)

        def _enqueue(priority: int):
            self._prepared_runs[run_data.id] = job
            job.priority = priority
            run_data.status = RunStatus.QUEUED
            self.queue.put_nowait(job)
            self._pending_created_runs[run_data.id] = run_data
            self.log.debug("worker.queue", job=job)
            if job.run_data.id in self._prepared_runs:
                del self._prepared_runs[job.run_data.id]

        # add to queue (now or later if scheduled)
        if run_data.scheduled_at:
            run_data.status = RunStatus.SCHEDULED
            now = utcnow_with_tz()
            delay = (run_data.scheduled_at - now).total_seconds() - WORKER_SCHEDULE_BLOCK_AHEAD
            if delay > 0:
                self._prepared_runs[run_data.id] = job
                self._pending_created_runs[run_data.id] = run_data
                asyncio.get_running_loop().call_later(delay, _enqueue, 0)  # high priority
            else:
                _enqueue(0)
            self.log.info(
                "worker.schedule",
                job=job,
                scheduled_at=run_data.scheduled_at,
                delay=delay,
                run_data=run_data,
            )

        else:
            _enqueue(10)  # default priority

        return job

    async def _do_run_job(self, job: RunJob, timeout: float) -> None:
        """
        Actually runs the job, and updates the job with the result.
        """

        statement = None
        try:
            # init i.e. prepare session and run
            job.started.set()
            self.log.info("worker.run", job=job, timeout=timeout)

            # instantiate arguments
            if job.run_data.statement_ck:
                statement = self.module.lookup(job.run_data.statement_ck)
                if statement is None or statement.type not in RUNNABLE_STATEMENT_TYPES:
                    raise RunError(
                        kind=RunErrorKind.Runtime,
                        type=StartRunResponseErrorType.INVALID_RUN,
                        message=f"statement {statement!r} cannot be run",
                        statement=statement,
                    )
            else:
                code = (job.run_data.value or {}).get("code")
                if code is None:
                    raise RunError(
                        kind=RunErrorKind.Runtime,
                        type=StartRunResponseErrorType.INVALID_RUN,
                        message="missing code for anonymous run",
                        statement=None,
                    )
                scope = (job.run_data.value or {}).get("scope")
                if scope is not None:
                    scope = self.module.resolve(UUID(scope))
                else:
                    scope = self.module
                statement = Statement.code(code=code)
                if job.tags:
                    statement.tags.create_many(*job.tags)
                statement._track = NodeTrackingLevel.ANONYMOUS
                assert isinstance(scope, File), f"anonymous scope must be a file: {scope!r}"
                scope.children.append(statement, _trigger=_NodeChange.UpdateLists)
                statement._clear_rec()
                statement._interp_rec()
                if statement.issues:
                    raise RunError(
                        kind=RunErrorKind.Runtime,
                        type=StartRunResponseErrorType.INVALID_RUN,
                        message=f"invalid anonymous run: {statement.issues}",
                        statement=statement,
                    )

            inputs = map_value(
                job.run_data.inputs,
                statement,
                map_k=lambda f: (f._typed_key, f.py_ident),
                map_v=partial(unpack_value_flat, scope=self.module),
                is_output=False,
            )

            # create session
            job.session = Session(
                id=job.session_id,
                ck=job.session_id,
                module=self.module,
                bench_id=self.bench_id,
                access_level=job.run_data.access_level or SessionAccessLevel.Read,
                worker_id=self.node.worker_id,
                worker_process_id=None,
                trigger_type=job.run_data.trigger_type,
                trigger_id=job.run_data.trigger_id,
                _runtime=self,
                _root_run_id=job.run_data.id,
                _root_run_value=job.run_data.value,
                _global_run_value=job.global_value,
                _track=NodeTrackingLevel.NONE,  # :ManualSessionTracking
            )
            # mark run as already created if it was scheduled (a bit hacky)
            if job.run_data.scheduled_at:
                job.session._tracer._flushed_session_node_ids.add(job.run_data.id)

            # run in active session
            self.module._activate_rec(job.session)
            if statement._track == NodeTrackingLevel.ANONYMOUS:
                statement._activate_self(job.session)

            # wait out remaining schedule delay if needed (should be very short)
            now = utcnow_with_tz()
            if job.run_data.scheduled_at and job.run_data.scheduled_at > now:
                # wait out the schedule delay if needed
                delay = (job.run_data.scheduled_at - now).total_seconds()
                assert delay < WORKER_SCHEDULE_BLOCK_AHEAD, "schedule delay too long"
                self.log.info("worker.run.delay", job=job, delay=delay)
                await asyncio.sleep(delay)

            # run
            job.task = asyncio.create_task(self._do_run_in_session(job.session, statement, inputs))
            self._active_runs[job.run_data.id] = job
            self._last_run_job = job
            await asyncio.wait_for(job.task, timeout=timeout)
        except BaseException as e:
            job.exception = e
            raise
        finally:
            # remove root run from our own dirty runs (for queue/schedule) to avoid race condition
            #  (where a queued run may be saved after a fast run has completed and flushed)
            if job.run_data.id in self._pending_created_runs:
                del self._pending_created_runs[job.run_data.id]
            if job.run_data.id in self._active_runs:
                del self._active_runs[job.run_data.id]

            # deactivate session
            self.module._deactivate_rec()
            # remove anonymous statement if needed
            if (
                not job.run_data.statement_ck
                and statement
                and statement.parent
                and statement in self.module._local_tree  # may not exist if reset on error
            ):
                statement.parent.children.remove(statement, _trigger=_NodeChange.UpdateLists)

            job.terminated.set()

    async def _do_run_in_session(
        self, session: Session, statement: Statement, inputs: dict
    ) -> None:
        """Actually runs the statement in the session"""

        await session._open()
        try:
            await statement(**inputs)
            # we autocommit at the end of the top-level run
        except BaseException as e:
            error = RunError(
                kind=RunErrorKind.Runtime,
                type=type(e).__name__,
                message=str(e),
                statement=statement,
            )
            raise error from e
        finally:
            await session._close()

    async def kill_run(self, run_id: UUID) -> bool:
        run = self._active_runs.get(run_id)
        if run is None or run.task is None:
            logger.debug("worker.kill.not_found", run_id=run_id)
            return False
        else:
            run.task.cancel()
            logger.debug("worker.kill", run=run)
            return True
