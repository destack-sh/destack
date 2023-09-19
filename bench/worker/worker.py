import asyncio
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from datetime import timedelta
from typing import Optional, Union
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async

from bench.language import HasRun, LogEntry, Module, Run, RunError, wire
from bench.language.const import ModuleReference, RunStatus, SessionMode
from bench.language.mapping import map_value, unpack_value_flat
from bench.language.mutate import ModuleMutation, ModuleMutator
from bench.language.run import RunErrorKind
from bench.language.session import ModuleWriter, Session, SessionContext
from bench.language.wire import RunData
from bench.msg.core import NMessage, handle_reply, message_handler, nc_init, request, subscribe
from bench.msg.messages import (
    ClientOrigin,
    ModuleInternalChangedPayload,
    NMessageType,
    RepGetEnvironmentPayload,
    RepGetModuleHeadPayload,
    RepKillRunPayload,
    RepPingWorkerSetPayload,
    RepReadModulePayload,
    RepStartRunPayload,
    RepWriteModulePayload,
    RepWriteSessionPayload,
    ReqGetEnvironmentPayload,
    ReqGetModuleHeadPayload,
    ReqKillRunPayload,
    ReqPingWorkerSetPayload,
    ReqReadModulePayload,
    ReqStartRunPayload,
    ReqWriteModulePayload,
    ReqWriteSessionPayload,
    StartRunErrorType,
)
from bench.utils.cache import redis
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import wrap_task
from bench.utils.monitoring import Monitored
from bench.utils.task import TaskManager
from bench.utils.utils import get_from_env, sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT
from bench.worker.environment import WORKER_ENVIRONMENT_DATA

WORKER_RUN_TIMEOUT = get_from_env("WORKER_RUN_TIMEOUT", 300, type_cast=int)
WORKER_ACTIVE_TIMEOUT = timedelta(seconds=30)
WORKER_ACTIVE_PUBLISH_INTERVAL = 10
WORKER_SCHEDULE_BLOCK_AHEAD = 0.5

logger = structlog.get_logger(__name__)


class WorkerNode(Monitored):
    """
    A user worker to run user code, generally one worker process per project (Bench).
    For local development a node can host multiple Benches.
    TODO @Architecture: merge WorkerNode/WorkerHost,  processes should be 1:1 with ModuleWorkerProcess
     (see :BE-213)
    """

    def __init__(self, worker_node_id: str, worker_set_id: UUID | None, project_id: UUID | None):
        self.worker_set_id = worker_set_id
        self.worker_node_id = worker_node_id
        self.project_id = project_id
        self.workers: dict[UUID, ModuleWorkerProcess] = {}
        self.subs = []
        self.tasks = TaskManager()
        self.cached_committed_modules: dict[ModuleReference, tuple[wire.ModuleTreeData, UUID]] = {}
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
        # topics for .project.module or just .project
        m_routing = f"{self.project_id}.*" if self.project_id else ">"
        p_routing = f"{self.project_id}" if self.project_id else "*"
        self.subs = [
            await subscribe(
                f"{NMessageType.MODULE_INTERNAL_CHANGED}.{m_routing}", cb=self.module_changed
            ),
            await handle_reply(f"{NMessageType.START_RUN}.{m_routing}", self.start_run),
            await handle_reply(f"{NMessageType.KILL_RUN}.{m_routing}", self.kill_run),
            await handle_reply(f"{NMessageType.GET_ENVIRONMENT}.{p_routing}", self.get_environment),
            await handle_reply(f"{NMessageType.PING_WORKER_SET}.{p_routing}", self.ping),
        ]
        if self.worker_set_id is not None:  # only mark as active if not a local worker
            self.tasks.start(
                self.mark_as_active_if_active_forever(interval=WORKER_ACTIVE_PUBLISH_INTERVAL)
            )

        if self.project_id is not None:
            # preload worker for project (assumes it's at head)
            rep: NMessage[RepGetModuleHeadPayload] = await request(
                NMessageType.GET_MODULE_HEAD,
                ReqGetModuleHeadPayload(project_id=self.project_id),
                retry=3,
                timeout=3,
                reply_t=RepGetModuleHeadPayload,
            )
            await self._prepare_worker(rep.p.module_id)

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
    async def module_changed(self, msg: NMessage[ModuleInternalChangedPayload]):
        if msg.p.module_id not in self.workers:
            # ignore if we don't have a worker for this module
            return
        if not msg.p.has_origin(self.client.id):
            is_semantic = any(m.type.semantic for m in msg.p.mutations)
            if not is_semantic:
                # ignore non-semantic changes (will have to be smarter when we :BumpProperly)
                return
            worker = await self._prepare_worker(msg.p.module_id)
            await worker.interp_on_change(msg.p.mutations)

    @message_handler
    async def start_run(self, msg: NMessage[ReqStartRunPayload]):
        self.log.debug("run.start", msg=msg, block=msg.p.block)
        worker = await self._prepare_worker(msg.p.module_id)
        run_id = msg.p.run_id or UUIDT()

        try:
            # get runnable
            runnable = worker.module.lookup_or_error(msg.p.runnable)
            if runnable is None:
                pass

            # key inputs if needed
            if not msg.p.keyed:
                inputs = map_value(msg.p.inputs, runnable, map_k=lambda f: (f.key, f.py_ident))
            else:
                inputs = msg.p.inputs

            # create run data
            now = utcnow_with_tz()
            status = RunStatus.Queued if msg.p.scheduled_at is None else RunStatus.Scheduled
            # only create session id if not scheduled
            run_data = RunData(
                id=run_id,
                project_id=self.project_id,
                module_id=msg.p.module_id,
                worker_node_id=self.worker_node_id,
                worker_process_id=None,
                runnable_id=runnable.id,
                runnable_type=runnable.type,
                runnable_ck=runnable.ck,
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
                metadata=None,
            )

            # store session id separately from run because we write the run data directly
            #  (and the session doesn't actually exist until the run starts)
            session_id = msg.p.session_id or (UUIDT() if not msg.p.scheduled_at else None)
            job = worker.add_run(run_data, session_id)
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

        # update job's run_data from session
        # (this doesn't feel like the right place for this, but we always need to do it to reply)
        if job.session and job.session.tracer.run.runs:
            run = job.session.tracer.run.runs[job.run_data.id]
            job.run_data = wire.pack_data(run)
            last_logs = job.session.tracer.cached_logs[:50]
        else:
            last_logs = None

        logs = [wire.pack_data(log) for log in last_logs] if last_logs else None
        rep = RepStartRunPayload(
            error=error, run=job.run_data if job else None, run_id=run_id, logs=logs
        )
        await msg.reply(rep)

    @message_handler
    async def kill_run(self, msg: NMessage[ReqKillRunPayload]):
        logger.debug("run.kill", msg=msg)
        worker = await self._prepare_worker(msg.p.module_id)
        success = await worker.kill_run(msg.p.run_id)
        await msg.reply(RepKillRunPayload(success=success))

    @message_handler
    async def get_environment(self, msg: NMessage[ReqGetEnvironmentPayload]):
        await msg.reply(RepGetEnvironmentPayload(environment=WORKER_ENVIRONMENT_DATA))

    @message_handler
    async def ping(self, msg: NMessage[ReqPingWorkerSetPayload]):
        await msg.reply(RepPingWorkerSetPayload(success=self.healthy))

    async def get_module(self, ref: ModuleReference | UUID) -> tuple[wire.ModuleTreeData, UUID]:
        """Gets a modules wire data"""
        log = self.log.bind(ref=ref)
        cached = self.cached_committed_modules.get(ref)
        if cached is not None:
            log.debug("module.fetch", cached=True)
            return cached
        module_rep = await request(
            NMessageType.READ_MODULE, ReqReadModulePayload(ref), RepReadModulePayload, retry=3
        )
        if module_rep.p.module.committed:
            self.cached_committed_modules[ref] = module_rep.p.module, module_rep.p.project_id
        log.debug("module.fetch", cached=False)
        return module_rep.p.module, module_rep.p.project_id

    async def fetch(self, ref: ModuleReference) -> wire.ModuleTreeData:
        return (await self.get_module(ref))[0]

    async def stop(self):
        self.log.info("stop")
        await self.tasks.stop()
        await asyncio.gather(*[sub.unsubscribe() for sub in self.subs])
        self._ready.clear()


@dataclass(repr=False, slots=True)
class RunJob:
    run_data: RunData
    session_id: UUID
    priority: int = 10  # default
    task: asyncio.Task | None = None
    session: Optional[Session] = None
    started: asyncio.Event = field(default_factory=asyncio.Event)
    terminated: asyncio.Event = field(default_factory=asyncio.Event)

    def __lt__(self, other: "RunJob"):
        return self.priority < other.priority

    def __gt__(self, other):
        return self.priority > other.priority

    def __str__(self):
        return f"{self.run_data.id}"

    def __repr__(self):
        return f"<RunJob {self}>"


class RunStartError(Exception):
    def __init__(self, type: StartRunErrorType):
        self.type = type


class ModuleWorkerProcess(ModuleWriter):
    """
    A user worker that helps run a specific module.
    Generally, a worker process is intended to process one run at a time (for now).
    """

    def __init__(self, module_id: UUID, node: "WorkerNode", process_id: Optional[str]):
        self.node = node
        self.module_id = module_id
        self.project_id: Optional[UUID] = None  # set in init (requires runtime fetch)
        self.ready = asyncio.Event()

        self.source: wire.ModuleTreeData | None = None
        self.module: Module | None = None
        self.queue: asyncio.Queue[RunJob] = asyncio.PriorityQueue()
        self.executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="worker")
        self.log = logger.bind(
            worker_set=self.node.worker_set_id,
            worker_node=self.node.worker_node_id,
            worker_process=process_id,
            module_id=self.module_id,
        )

        self._active_runs: dict[UUID, RunJob] = {}
        self._scheduled_runs: dict[UUID, RunJob] = {}
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
        self.source, self.project_id = await self.node.get_module(self.module_id)
        try:
            self.module = await sync_to_async(Module.interp_from)(self.source, session=None)
        except Exception as e:
            self.log.error("module.init.failed", exc_info=e)
            raise RuntimeError(f"failed to initialize module worker {self}")
        self.node.tasks.start(self._flush_dirty_runs_forever(interval=0.1))
        self.ready.set()

    async def run(self):
        """Runs the module worker main processing loop"""

        # first interp
        self.log.info("worker.start")
        await self.start()

        # process run tasks ad infinitum
        await self._process_runs_forever()

    async def _process_runs_forever(self):
        while True:
            job = await self.queue.get()
            if job.run_data.status != RunStatus.Queued:
                continue

            job.started.set()
            try:
                await self._do_run_job(job, WORKER_RUN_TIMEOUT)
            except RunError as e:
                self.log.debug("run.failed", job=job, exc_info=e)
            except Exception as e:
                self.log.error(
                    "run.failed.internal", job=job, sentry=sentry_capture_if_enabled(e), exc_info=e
                )
            finally:
                job.terminated.set()
                self.queue.task_done()

    async def interp_on_change(self, mutations: list[ModuleMutation]):
        self.log.debug("worker.interp", mutations=len(mutations))
        new_source = ModuleMutator(self.source, mutations).to_module()
        self.source = new_source
        self.module = await sync_to_async(Module.interp_from)(new_source, session=None)

    def add_run(self, run_data: RunData, session_id: UUID) -> RunJob:
        """
        Registers a run to be processed by this worker process.
        If scheduled, the run will be queued after the delay.
        """

        if run_data.id in self._scheduled_runs:
            raise RunStartError(StartRunErrorType.ALREADY_SCHEDULED)

        job = RunJob(run_data=run_data, session_id=session_id)

        def _enqueue(priority: int):
            job.priority = priority
            run_data.status = RunStatus.Queued
            self.queue.put_nowait(job)
            self._dirty_dangling_runs[run_data.id] = run_data
            self.log.debug("worker.queue", job=job)
            if job.run_data.id in self._scheduled_runs:
                del self._scheduled_runs[job.run_data.id]

        # add to queue (now or later if scheduled)
        if run_data.scheduled_at:
            run_data.status = RunStatus.Scheduled
            now = utcnow_with_tz()
            delay = (run_data.scheduled_at - now).total_seconds() - WORKER_SCHEDULE_BLOCK_AHEAD
            if delay > 0:
                self._scheduled_runs[run_data.id] = job
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
            runnable = self.module.lookup(job.run_data.runnable_id)
            inputs = map_value(
                job.run_data.inputs,
                runnable,
                map_k=lambda f: (f.typed_key, f.py_ident),
                map_v=unpack_value_flat,
                is_output=False,
            )

            # create session
            session_ctx = SessionContext(
                module_id=self.module_id,
                project_id=self.project_id,
                worker_node_id=self.node.worker_node_id,
                worker_process_id=None,
                trigger_type=job.run_data.trigger_type,
                trigger_id=job.run_data.trigger_id,
                first_run_id=job.run_data.id,
            )
            job.session = Session(
                module=self.module,
                writer=self,
                ctx=session_ctx,
                id=job.session_id,
                mode=SessionMode.WRITE,
            )

            # run in active session
            self.module._activate_in(job.session)

            # wait out remaining schedule delay if needed (should be very short)
            now = utcnow_with_tz()
            if job.run_data.scheduled_at and job.run_data.scheduled_at > now:
                # wait out the schedule delay if needed
                delay = (job.run_data.scheduled_at - now).total_seconds()
                assert delay < WORKER_SCHEDULE_BLOCK_AHEAD, "schedule delay too long"
                self.log.info("worker.run.delay", job=job, delay=delay)
                await asyncio.sleep(delay)

            # run
            job.task = asyncio.create_task(self._do_run_in_session(job.session, runnable, inputs))
            self._active_runs[job.run_data.id] = job
            self._last_run_job = job
            await asyncio.wait_for(job.task, timeout=timeout)
        finally:
            job.terminated.set()
            self.module._deactivate()
            if job.run_data.id in self._active_runs:
                del self._active_runs[job.run_data.id]

    async def _do_run_in_session(self, session: Session, runnable: HasRun, inputs: dict) -> None:
        """Actually runs the runnable in the session"""

        # open session
        await session.aopen()
        if not runnable._is_async:
            runnable = runnable.to_async()

        # run session
        try:
            await runnable(**inputs)
        except BaseException as e:
            raise RunError(
                kind=RunErrorKind.Runtime, type=type(e).__name__, message=str(e), runnable=runnable
            ) from e
        finally:
            # remove root run from our own dirty runs (for queue/schedule) to avoid race condition
            #  (where a queued run may be saved after a fast run has completed and flushed)
            if session.ctx.first_run_id in self._dirty_dangling_runs:
                del self._dirty_dangling_runs[session.ctx.first_run_id]

            # always close session
            await session.aclose()

    async def kill_run(self, run_id: UUID) -> bool:
        # TODO @Broken: implement kill (cancel/abort) properly (interupts don't work)
        #  (currently we only do process restarts)
        return False

    async def _flush_dirty_runs_forever(self, interval: float):
        while True:
            if self._dirty_dangling_runs:
                logger.debug("worker.flush_dirty_runs", runs=len(self._dirty_dangling_runs))
                runs = list(self._dirty_dangling_runs.values())
                self._dirty_dangling_runs = {}
                success = await self.write_session(session=None, runs=runs, logs=[])
                if not success:
                    logger.error("worker.flush_dirty_runs.failed", runs=len(runs))
            await asyncio.sleep(interval)

    async def write_module(self, mutations: list[ModuleMutation]) -> bool:
        # ignore non-semantic changes (will have to be smarter when we :BumpProperly)
        is_semantic = any(m.type.semantic for m in mutations)
        self.log.debug("module.write", mutations=len(mutations), is_semantic=is_semantic)

        # interp
        if is_semantic:
            new_source = ModuleMutator(self.module, mutations).to_module()
            self.source = new_source
            self.module = await sync_to_async(Module.interp_from)(new_source, session=None)

        req = ReqWriteModulePayload(
            module_id=self.module_id,
            mutations=mutations,
            client=self.node.client,
            wait=False,
        )
        rep: NMessage[RepWriteModulePayload] = await request(
            NMessageType.WRITE_MODULE, req, RepWriteModulePayload, retry=3
        )
        return rep.p.success

    async def write_session(
        self,
        session: Optional["Session"],
        runs: list[Union["Run", wire.RunData]],
        logs: list[Union["LogEntry", wire.LogEntryData]],
    ) -> bool:
        self.log.debug("session.write", session=session, runs=len(runs), logs=len(logs))

        session_data = wire.pack_data(session) if session else None
        runs_data = [wire.pack_data(run) if isinstance(run, Run) else run for run in runs]
        logs_data = [wire.pack_data(log) if isinstance(log, LogEntry) else log for log in logs]

        # remove runs from dirty dangling runs
        for run in runs:
            if run.id in self._dirty_dangling_runs:
                del self._dirty_dangling_runs[run.id]

        req = ReqWriteSessionPayload(
            module_id=self.module_id,
            session=session_data,
            runs=runs_data,
            logs=logs_data,
            client=self.node.client,
        )
        rep: NMessage[RepWriteSessionPayload] = await request(
            NMessageType.WRITE_SESSION, req, RepWriteSessionPayload, retry=3
        )
        return rep.p.success
