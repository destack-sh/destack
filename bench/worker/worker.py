import asyncio
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from datetime import timedelta
from functools import partial
from typing import Any, Optional, Union
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async

from bench.language import Blob, Module, Run, RunError, Secret, Statement, wire
from bench.language.builtin import symbolx_lib
from bench.language.const import (
    INTERP_NODE_TYPES,
    RUNNABLE_STATEMENT_TYPES,
    ModuleReference,
    NodeTrackingLevel,
    RunStatus,
    SessionAccessLevel,
)
from bench.language.edit import EditData
from bench.language.libs import DEFAULT_DEPENDENCIES, DEFAULT_MODULES
from bench.language.model import ModelError
from bench.language.module import _NodeChange
from bench.language.packer import map_value, unkey_value, unpack_value_flat
from bench.language.run import RunErrorKind
from bench.language.session import RuntimeHost, Session
from bench.language.wire import RunData
from bench.msg.core import (
    NMessage,
    handle_reply,
    message_handler,
    nc_init,
    publish,
    request,
    subscribe,
)
from bench.msg.messages import (
    ClientOrigin,
    LogsChangedPayload,
    ModuleChangedPayload,
    NMessageType,
    RepDownloadBlobPayload,
    RepGetEnvironmentPayload,
    RepKillRunPayload,
    RepMarkUploadedBlobPayload,
    RepPingWorkerSetPayload,
    RepPullWorkerRunsPayload,
    RepReadModulePayload,
    RepRevealSecretPayload,
    RepRunInferencePayload,
    RepRunStatementPayload,
    RepStartRunPayload,
    RepUploadBlobPayload,
    RepWriteEditsPayload,
    RepWriteSessionPayload,
    ReqDownloadBlobPayload,
    ReqGetEnvironmentPayload,
    ReqKillRunPayload,
    ReqMarkUploadedBlobPayload,
    ReqPingWorkerSetPayload,
    ReqPullWorkerRunsPayload,
    ReqReadModulePayload,
    ReqRevealSecretPayload,
    ReqRunInferencePayload,
    ReqRunStatementPayload,
    ReqStartRunPayload,
    ReqUploadBlobPayload,
    ReqWriteEditsPayload,
    ReqWriteSessionPayload,
    StartRunErrorType,
)
from bench.utils.cache import redis
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import wrap_task
from bench.utils.monitoring import Monitored
from bench.utils.task import TaskManager
from bench.utils.utils import get_from_env, required_field, sentry_capture
from bench.utils.uuidt import UUIDT
from bench.worker.environment import WORKER_ENVIRONMENT_DATA

WORKER_RUN_TIMEOUT = get_from_env("WORKER_RUN_TIMEOUT", 3000, type_cast=int)
WORKER_ACTIVE_TIMEOUT = timedelta(seconds=30)
WORKER_ACTIVE_PUBLISH_INTERVAL = 10
WORKER_SCHEDULE_BLOCK_AHEAD = 1

logger = structlog.get_logger(__name__)


@dataclass
class ModuleInfo:
    project_id: UUID
    os_name: str
    pg_name: str


class WorkerNode(Monitored):
    """
    A user worker to run user code, generally one worker process per project (Bench).
    For local development a node can host multiple Benches.
    TODO @Architecture: merge WorkerNode/WorkerHost, processes should be 1:1 with ModuleWorkerProcess
     (see :BE-213)
    """

    def __init__(
        self,
        worker_node_id: str,
        worker_set_id: UUID | None,
        project_id: UUID | None,
        module_id: UUID | None,
    ):
        self.worker_set_id = worker_set_id
        self.worker_node_id = worker_node_id
        self.project_id = project_id
        self.module_id = module_id
        self.workers: dict[UUID, ModuleWorkerProcess] = {}
        self.subs = []
        self.tasks = TaskManager()
        self._cached_module_source: dict[
            ModuleReference, tuple[wire.ModuleTreeData, ModuleInfo]
        ] = {}
        self._cached_modules: dict[ModuleReference, Module] = {}
        self._ready = asyncio.Event()
        self.log = logger.bind(
            worker_node=self.worker_node_id,
            worker_set=self.worker_set_id,
            project_id=self.project_id,
        )

    def __str__(self):
        return f"{self.project_id} {self.worker_set_id} {self.worker_node_id}"

    def __repr__(self):
        return f"<WorkerNode {self}>"

    @property
    def ready(self):
        return self._ready.is_set()

    @property
    def client(self):
        return ClientOrigin(type="user-worker", id=self.worker_node_id, nonce=None)

    async def run(self):
        await nc_init.wait()
        self.log.info("start")

        # get default modules info (we need their project_id/os_name/pg_name/...)
        # (maybe cache it or get it more efficiently? maybe compare versions?)
        for module_name, module in DEFAULT_MODULES.items():
            module_ref = ModuleReference(name=module_name, version="x", id=None)
            module_loaded, info = await self.read_module(module_ref)
            module._project_id = info.project_id
            module._os_name = info.os_name
            module._pg_name = info.pg_name
            self._cached_module_source[module_ref] = module, info
        # and add other default dependencies
        for module_name in DEFAULT_DEPENDENCIES:
            module_ref = ModuleReference(name=module_name, version="x", id=None)
            module_loaded, info = await self.read_module(module_ref)
            self._cached_module_source[module_ref] = module_loaded, info
            module = await sync_to_async(Module.make)(
                source=module_loaded.nodes,
                project_id=info.project_id,
                os_name=info.os_name,
                pg_name=info.pg_name,
            )
            self._cached_modules[module_ref] = module

        # topics for .project.module or just .project
        m_routing = f"{self.project_id}.*" if self.project_id else ">"
        p_routing = f"{self.project_id}" if self.project_id else "*"
        self.subs = [
            await subscribe(f"{NMessageType.MODULE_CHANGED}.{m_routing}", cb=self.module_changed),
            await handle_reply(f"{NMessageType.START_RUN}.{m_routing}", self.start_run),
            await handle_reply(f"{NMessageType.KILL_RUN}.{m_routing}", self.kill_run),
            await handle_reply(f"{NMessageType.GET_ENVIRONMENT}.{p_routing}", self.get_environment),
            await handle_reply(f"{NMessageType.PING_WORKER_SET}.{p_routing}", self.ping),
        ]
        if self.worker_set_id is not None:  # only mark as active if not a local worker
            self.tasks.start(
                self.mark_as_active_if_active_forever(interval=WORKER_ACTIVE_PUBLISH_INTERVAL)
            )

        if self.module_id is not None:
            await self._prepare_worker(self.module_id)

        self._ready.set()

    async def run_forever(self):
        # run forever until cancelled
        try:
            asyncio.create_task(self.run())
            await self._ready.wait()
            await asyncio.Event().wait()
        finally:
            await self.stop()

    def _get_worker(self, module_id: UUID) -> "ModuleWorkerProcess":
        if module_id not in self.workers:
            # start module worker if not already started
            worker = ModuleWorkerProcess(module_id=module_id, node=self, process_id=None)
            self.workers[module_id] = worker
            asyncio.create_task(wrap_task(worker.run(), "worker_run_" + str(module_id)))
        return self.workers[module_id]

    async def _prepare_worker(self, module_id: UUID) -> "ModuleWorkerProcess":
        worker = self._get_worker(module_id)
        if not worker.ready.is_set():
            await worker.ready.wait()
        return worker

    async def _mark_worker_as_active(self):
        # :WorkerSetActive
        active_key = f"worker_set.{self.worker_set_id}.{self.worker_node_id}.last_active_at"
        self.log.debug("mark_as_active", active_key=active_key)
        if not await redis.set(active_key, value=utcnow_with_tz().isoformat(), ex=24 * 60 * 60):
            self.log.error("mark_as_active.failed", active_key=active_key)
        else:
            self.log.debug("mark_as_active.done")

    async def mark_as_active_if_active_forever(self, interval: int):
        while True:
            if any(w.active for w in self.workers.values()):
                await self._mark_worker_as_active()
            await asyncio.sleep(interval)

    @message_handler
    async def module_changed(self, msg: NMessage[ModuleChangedPayload]):
        if msg.p.module_id not in self.workers:
            # ignore if we don't have a worker for this module
            return
        if not msg.p.has_origin(self.client.id):
            worker = await self._prepare_worker(msg.p.module_id)
            await worker.on_module_changed(msg.p.edits)

    @message_handler
    async def start_run(self, msg: NMessage[ReqStartRunPayload]):
        self.log.debug("run.start", msg=msg, block=msg.p.block)
        worker = await self._prepare_worker(msg.p.module_id)
        run_id = msg.p.run_id or UUIDT()

        try:
            # get statement
            if msg.p.statement:
                statement = worker.module.lookup(msg.p.statement)
                if statement is None or statement.type not in RUNNABLE_STATEMENT_TYPES:
                    raise RunStartError(StartRunErrorType.INVALID_RUN)
            else:
                statement = None

            # key inputs if needed
            if not msg.p.keyed and statement:
                inputs = map_value(
                    msg.p.inputs,
                    statement,
                    map_k=lambda f: (f.py_ident, f._typed_key),
                    is_output=False,
                )
            else:
                inputs = msg.p.inputs

            # create run data
            now = utcnow_with_tz()
            status = RunStatus.Queued if msg.p.scheduled_at is None else RunStatus.Scheduled
            # only create session id if not scheduled
            run_data = RunData(
                id=run_id,
                project_id=worker.project_id,
                module_id=msg.p.module_id,
                worker_node_id=self.worker_node_id,
                worker_process_id=None,
                statement_id=statement.id if statement else None,
                statement_type=statement.type if statement else None,
                statement_ck=statement.ck if statement else None,
                statement_path=None,
                session_id=None,
                trigger_type=msg.p.trigger_type,
                trigger_id=msg.p.trigger_id,
                root_id=None,
                parent_id=None,
                created_at=now,
                updated_at=now,
                scheduled_at=msg.p.scheduled_at,
                started_at=None,
                terminated_at=None,
                status=status,
                inputs=inputs,
                outputs=None,
                error=None,
                value=msg.p.root_value,
                access_level=msg.p.access_level or SessionAccessLevel.Read,
            )

            # store session id separately from run because we write the run data directly
            #  (and the session doesn't actually exist until the run starts)
            session_id = msg.p.session_id or (UUIDT() if not msg.p.scheduled_at else None)
            job = worker.add_run(
                run_data, session_id, global_value=msg.p.global_value, tags=msg.p.tags
            )
            error = None
        except RunStartError as e:
            self.log.error("run.start.error", msg=msg, error=e)
            await msg.reply(RepStartRunPayload(error=e.type))
            return

        if msg.p.block is not None:
            try:
                await job.started.wait()
                await asyncio.wait_for(job.terminated.wait(), timeout=msg.p.block)
            except asyncio.TimeoutError:
                pass  # ignore
            # job can fail to start even once successfully queued (e.g. maybe statement is invalid)
            if (
                isinstance(job.exception, RunError)
                and job.exception.type == StartRunErrorType.INVALID_RUN
            ):
                await msg.reply(RepStartRunPayload(error=job.exception.type))
                return

        # update job's run_data from session
        # (this doesn't feel like the right place for this, but we always need to do it to reply)
        if job.session and job.session._tracer.runs:
            run = job.session._tracer.runs[job.run_data.id]
            job.run_data = wire.pack_data(run)
            last_logs = job.session._tracer.cached_logs[:50]
        else:
            last_logs = None

        logs = [wire.pack_data(log) for log in last_logs] if last_logs else None
        run_data = job.run_data if job else None
        if not msg.p.keyed_return:
            # unkey inputs/outputs/value
            run_data.inputs = unkey_value(run_data.inputs, statement, is_output=False)
            run_data.outputs = unkey_value(run_data.outputs, statement, is_output=True)
            run_data.value = unkey_value(
                run_data.value, symbolx_lib.resolve(".reflect.RunMetadata")
            )
        rep = RepStartRunPayload(error=error, run=run_data, run_id=run_id, logs=logs)
        await msg.reply(rep)

    @message_handler
    async def kill_run(self, msg: NMessage[ReqKillRunPayload]):
        logger.debug("run.kill", msg=msg)
        if msg.p.module_id not in self.workers:
            await msg.reply(RepKillRunPayload(success=False))
        else:
            worker = await self._prepare_worker(msg.p.module_id)
            success = await worker.kill_run(msg.p.run_id)
            await msg.reply(RepKillRunPayload(success=success))

    @message_handler
    async def get_environment(self, msg: NMessage[ReqGetEnvironmentPayload]):
        await msg.reply(RepGetEnvironmentPayload(environment=WORKER_ENVIRONMENT_DATA))

    @message_handler
    async def ping(self, msg: NMessage[ReqPingWorkerSetPayload]):
        await msg.reply(RepPingWorkerSetPayload(success=self.healthy))

    async def read_module(
        self, ref: ModuleReference | UUID
    ) -> tuple[wire.ModuleTreeData, ModuleInfo]:
        """Gets a modules wire data"""
        log = self.log.bind(ref=ref)
        cached = self._cached_module_source.get(ref)
        if cached is not None:
            log.debug("module.fetch", cached=True)
            return cached
        module_rep: NMessage[RepReadModulePayload] = await request(
            NMessageType.READ_MODULE,
            ReqReadModulePayload(ref),
            RepReadModulePayload,
            retry=5,
            timeout=15,
            retry_delay=10,
        )
        if module_rep.p.module.committed:
            self._cached_module_source[ref] = module_rep.p.module, module_rep.p.project_id
        log.debug("module.fetch", cached=False)
        return module_rep.p.module, ModuleInfo(
            project_id=module_rep.p.project_id,
            os_name=module_rep.p.os_name,
            pg_name=module_rep.p.pg_name,
        )

    async def stop(self):
        self.log.info("stop")
        await self.tasks.stop()
        await asyncio.gather(*[sub.unsubscribe() for sub in self.subs])
        self._ready.clear()


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
    def __init__(self, type: StartRunErrorType):
        self.type = type


class ModuleWorkerProcess(RuntimeHost):
    """
    A user worker that helps run a specific module.
    Generally, a worker process is intended to process one run at a time (for now).
    """

    def __init__(self, module_id: UUID, node: "WorkerNode", process_id: Optional[str]):
        self.node = node
        self.module_id = module_id
        self.project_id: Optional[UUID] = None  # set in init (requires runtime fetch)
        self.ready = asyncio.Event()

        self.module: Module | None = None

        self.queue: asyncio.Queue[RunJob | MakeJob] = asyncio.PriorityQueue()
        self.executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="worker")
        self.log = logger.bind(
            worker_set=str(self.node.worker_set_id),
            worker_node=str(self.node.worker_node_id),
            worker_process=str(process_id),
            module_id=str(self.module_id),
        )

        self._active_runs: dict[UUID, RunJob] = {}
        self._prepared_runs: dict[UUID, RunJob] = {}
        self._last_run_job: Optional[RunJob] = None
        self._dirty_dangling_runs: dict[UUID, wire.RunData] = {}

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
        self.project_id = info.project_id
        try:
            self.module = await sync_to_async(Module.make)(
                source=source.nodes,
                project_id=info.project_id,
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
        self.node.tasks.start(self._flush_dirty_runs_forever(interval=0.1))
        self.ready.set()

        # recover any prescheduled runs for this process
        try:
            rep: NMessage[RepPullWorkerRunsPayload] = await request(
                NMessageType.PULL_WORKER_RUNS,
                ReqPullWorkerRunsPayload(
                    project_id=self.node.project_id,
                    module_id=self.module_id,
                    worker_set_id=self.node.worker_set_id,
                    worker_node_id=self.node.worker_node_id,
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
            if job.run_data.status != RunStatus.Queued:
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

        try:
            # apply edits
            now = utcnow_with_tz()
            edits = [e for e in job.edits if e.mnt not in INTERP_NODE_TYPES]
            self.module._apply_edits(edits)
            duration = utcnow_with_tz() - now
            self.log.info("worker.make", edits=edits, duration=duration.total_seconds())
        except ModelError as e:
            self.log.error("worker.make.error", job=job, exc_info=e)

    def add_run(
        self,
        run_data: RunData,
        session_id: Optional[UUID] = None,
        global_value: dict | None = None,
        tags: list[str] | None = None,
    ) -> RunJob:
        """
        Registers a run to be processed by this worker process.
        If scheduled, the run will be queued after the delay.
        """

        if run_data.id in self._prepared_runs:
            raise RunStartError(StartRunErrorType.ALREADY_PREPARED)

        job = RunJob(run_data=run_data, session_id=session_id, global_value=global_value, tags=tags)

        def _enqueue(priority: int):
            self._prepared_runs[run_data.id] = job
            job.priority = priority
            run_data.status = RunStatus.Queued
            self.queue.put_nowait(job)
            self._dirty_dangling_runs[run_data.id] = run_data
            self.log.debug("worker.queue", job=job)
            if job.run_data.id in self._prepared_runs:
                del self._prepared_runs[job.run_data.id]

        # add to queue (now or later if scheduled)
        if run_data.scheduled_at:
            run_data.status = RunStatus.Scheduled
            now = utcnow_with_tz()
            delay = (run_data.scheduled_at - now).total_seconds() - WORKER_SCHEDULE_BLOCK_AHEAD
            if delay > 0:
                self._prepared_runs[run_data.id] = job
                self._dirty_dangling_runs[run_data.id] = run_data
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

        try:
            # init i.e. prepare session and run
            job.started.set()
            self.log.info("worker.run", job=job, timeout=timeout)

            # instantiate arguments
            if job.run_data.statement_id:
                statement = self.module.lookup(job.run_data.statement_id)
                if statement is None or statement.type not in RUNNABLE_STATEMENT_TYPES:
                    raise RunError(
                        kind=RunErrorKind.Runtime,
                        type=StartRunErrorType.INVALID_RUN,
                        message=f"statement {statement!r} cannot be run",
                        statement=statement,
                    )
            else:
                code = (job.run_data.value or {}).get("code")
                if code is None:
                    raise RunError(
                        kind=RunErrorKind.Runtime,
                        type=StartRunErrorType.INVALID_RUN,
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
                scope.children.append(statement, _trigger=_NodeChange.UpdateLists)
                statement._clear_rec()
                statement._interp_rec()
                if statement.issues:
                    raise RunError(
                        kind=RunErrorKind.Runtime,
                        type=StartRunErrorType.INVALID_RUN,
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
                module=self.module,
                runtime=self,
                access_level=job.run_data.access_level or SessionAccessLevel.Read,
                worker_node_id=self.node.worker_node_id,
                worker_process_id=None,
                trigger_type=job.run_data.trigger_type,
                trigger_id=job.run_data.trigger_id,
                root_run_id=job.run_data.id,
                root_run_value=job.run_data.value,
                global_run_value=job.global_value,
                id=job.session_id,
            )

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
            if job.run_data.id in self._dirty_dangling_runs:
                del self._dirty_dangling_runs[job.run_data.id]
            if job.run_data.id in self._active_runs:
                del self._active_runs[job.run_data.id]

            # deactivate session
            self.module._deactivate_rec()
            # remove anonymous statement if needed
            if not job.run_data.statement_id and "statement" in locals() and statement.parent:
                statement.parent.children.remove(statement, _trigger=_NodeChange.UpdateLists)

            job.terminated.set()

    async def _do_run_in_session(
        self, session: Session, statement: Statement, inputs: dict
    ) -> None:
        """Actually runs the statement in the session"""

        await session.open()
        try:
            await statement(**inputs)
        except BaseException as e:
            error = RunError(
                kind=RunErrorKind.Runtime,
                type=type(e).__name__,
                message=str(e),
                statement=statement,
            )
            raise error from e
        finally:
            await session.close()

    async def kill_run(self, run_id: UUID) -> bool:
        run = self._active_runs.get(run_id)
        if run is None or run.task is None:
            logger.debug("worker.kill.not_found", run_id=run_id)
            return False
        else:
            run.task.cancel()
            logger.debug("worker.kill", run=run)
            return True

    async def _flush_dirty_runs_forever(self, interval: float):
        while True:
            if self._dirty_dangling_runs:
                logger.debug("worker.flush_dirty_runs", runs=len(self._dirty_dangling_runs))
                runs = list(self._dirty_dangling_runs.values())
                self._dirty_dangling_runs = {}
                success = await self.push_session(session=None, runs=runs)
                if not success:
                    logger.error("worker.flush_dirty_runs.failed", runs=len(runs))
            await asyncio.sleep(interval)

    async def commit_edits(self, edits: list[EditData], refresh_index: bool) -> bool:
        # ignore non-semantic changes (will have to be smarter when we :BumpProperly)
        self.log.debug("module.commit_edits", edits=edits)
        req = ReqWriteEditsPayload(
            module_id=self.module_id,
            edits=edits,
            client=self.node.client,
            refresh_index=refresh_index,
        )
        rep: NMessage[RepWriteEditsPayload] = await request(
            NMessageType.WRITE_EDITS, req, RepWriteEditsPayload, retry=3
        )
        self.log.debug("module.commit_edits.done", edits=edits)
        return rep.p.success

    async def push_session(
        self, session: Optional["Session"], runs: list[Union["Run", wire.RunData]]
    ) -> bool:
        self.log.debug("session.write", session=session, runs=len(runs))

        session_data = wire.pack_data(session) if session else None
        runs_data = [wire.pack_data(run) if isinstance(run, Run) else run for run in runs]

        # remove runs from dirty dangling runs
        for run in runs:
            if run.id in self._dirty_dangling_runs:
                del self._dirty_dangling_runs[run.id]

        req = ReqWriteSessionPayload(
            module_id=self.module_id,
            session=session_data,
            runs=runs_data,
            client=self.node.client,
        )
        rep: NMessage[RepWriteSessionPayload] = await request(
            NMessageType.WRITE_SESSION, req, RepWriteSessionPayload, retry=3
        )
        return rep.p.success

    async def notify_logs_changed(self, logs: list[wire.LogEntryData]) -> None:
        await publish(
            NMessageType.LOGS_CHANGED,
            LogsChangedPayload(project_id=self.project_id, module_id=self.module_id, logs=logs),
        )

    async def download_blob(self, blob: "Blob") -> str:
        rep: NMessage[RepDownloadBlobPayload] = await request(
            NMessageType.DOWNLOAD_BLOB,
            ReqDownloadBlobPayload(blobs=[wire.pack_data(blob)]),
            reply_t=RepDownloadBlobPayload,
            timeout=5,
        )
        get_url = rep.p.get_urls[0] if rep.p.get_urls else None
        if get_url is None:
            raise ValueError(f"unable to GET {blob}")
        return get_url

    async def prepare_upload_blob(self, blob: "Blob") -> tuple["Blob", str | None]:
        self.log.debug("blob.prepare_upload", blob=blob)
        # first get POST url to upload the object
        rep: NMessage[RepUploadBlobPayload] = await request(
            NMessageType.UPLOAD_BLOB,
            ReqUploadBlobPayload(module_id=self.module.id, blobs=[wire.pack_data(blob)]),
            reply_t=RepUploadBlobPayload,
        )
        blob_data = rep.p.blobs[0]
        post_url = rep.p.post_urls[0] if rep.p.post_urls else None
        blob = wire.unpack_data(blob_data, blob.module)
        return blob, post_url

    async def mark_uploaded_blob(self, blob: "Blob") -> None:
        logger.debug("blob.mark_uploaded", object=self)
        rep: NMessage[RepMarkUploadedBlobPayload] = await request(
            NMessageType.MARK_UPLOADED_BLOB,
            ReqMarkUploadedBlobPayload(blobs=[wire.pack_data(blob)]),
            reply_t=RepMarkUploadedBlobPayload,
        )
        if not rep.p.success:
            raise ValueError(f"unable to mark uploaded {blob}")

    async def reveal_secret(self, secret: "Secret") -> None:
        logger.debug("secret.reveal", secret=secret)
        rep: NMessage[RepRevealSecretPayload] = await request(
            NMessageType.REVEAL_SECRET,
            ReqRevealSecretPayload(secrets=[wire.pack_data(secret)]),
            reply_t=RepRevealSecretPayload,
            timeout=10,
        )
        if not rep.p.secrets:
            raise ValueError(f"could not reveal {secret!r}")
        return rep.p.secrets[0].value

    async def run_proxy_statement(self, statement: Statement, inputs: dict) -> dict:
        req = ReqRunStatementPayload(
            project_id=statement.session.module.project_id,
            module_name=statement.session.module.path,
            statement=statement.path,
            inputs=inputs,
        )
        rep: NMessage[RepRunStatementPayload] = await request(
            NMessageType.RUN_PROXY_STATEMENT,
            req,
            RepRunStatementPayload,
            retry=3,
            retry_delay=10,
        )
        if rep.p.error:
            raise RuntimeError(rep.p.error)
        return rep.p.outputs

    async def run_proxy_inference(self, statement: Statement, inputs: dict, timeout: float) -> dict:
        req = ReqRunInferencePayload(
            project_id=statement.module.project_id,
            model_path=statement.path,
            inputs=inputs,
            timeout=timeout,
            run_id=statement.session.current_run.id,
        )
        rep: NMessage[RepRunInferencePayload] = await request(
            NMessageType.RUN_PROXY_INFERENCE,
            req,
            RepRunInferencePayload,
            timeout=timeout + 3,
        )
        if rep.p.error is not None:
            raise ModelError(rep.p.error, statement, f"remote {self} failed")
        return rep.p.outputs
