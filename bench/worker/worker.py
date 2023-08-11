import asyncio
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Optional
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async

from bench.language import LogEntry, wire
from bench.language.code_ import run
from bench.language.core import Module, ModuleReference, ModuleWriter, Session, SessionContext
from bench.language.mutate import ModuleMutation, ModuleMutator
from bench.language.session import Run, RunError
from bench.language.type import instantiate_py_value_flat, map_value
from bench.language.wire import RunData
from bench.msg.core import NMessage, handle_reply, message_handler, nc_init, request, subscribe
from bench.msg.messages import (
    ClientOrigin,
    ModuleInternalChangedPayload,
    NMessageType,
    RepCancelRunPayload,
    RepGetEnvironmentPayload,
    RepPingWorkerSetPayload,
    RepReadModulePayload,
    RepStartRunPayload,
    RepWriteModulePayload,
    RepWriteSessionPayload,
    ReqCancelRunPayload,
    ReqGetEnvironmentPayload,
    ReqPingWorkerSetPayload,
    ReqReadModulePayload,
    ReqStartRunPayload,
    ReqWriteModulePayload,
    ReqWriteSessionPayload,
    StartRunErrorType,
)
from bench.utils.cache import redis
from bench.utils.func import describe_type, wrap_task
from bench.utils.monitoring import Monitored
from bench.utils.task import TaskManager
from bench.utils.utils import get_from_env, sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT
from bench.worker.environment import WORKER_ENVIRONMENT_DATA

WORKER_RUN_TIMEOUT = get_from_env("WORKER_RUN_TIMEOUT", 300, type_cast=int)
WORKER_ACTIVE_TIMEOUT = timedelta(seconds=30)
WORKER_ACTIVE_PUBLISH_INTERVAL = 10

logger = structlog.get_logger(__name__)


class WorkerNode(Monitored):
    """
    A user worker to run user code, generally one worker process per project (Bench).
    For local development a node can host multiple Benches.
    """

    def __init__(self, worker_node_id: str, worker_set_id: UUID | None, project_id: UUID):
        self.worker_set_id = worker_set_id
        self.worker_node_id = worker_node_id
        self.project_id = project_id
        self.workers: dict[UUID, ModuleWorkerProcess] = {}
        self.subs = []
        self.tasks = TaskManager()
        self.cached_committed_modules: dict[ModuleReference, tuple[wire.ModuleTreeData, UUID]] = {}
        self._ready = asyncio.Event()

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
        logger.info(
            "start",
            worker_node=self.worker_node_id,
            workset_set=self.worker_set_id,
            project_id=self.project_id,
        )
        # topics for .project.module or just .project
        m_routing = f"{self.project_id}.*" if self.project_id else ">"
        p_routing = f"{self.project_id}" if self.project_id else "*"
        self.subs = [
            await subscribe(
                f"{NMessageType.MODULE_INTERNAL_CHANGED}.{m_routing}", cb=self.module_changed
            ),
            await handle_reply(f"{NMessageType.START_RUN}.{m_routing}", self.start_run),
            await handle_reply(f"{NMessageType.CANCEL_RUN}.{m_routing}", self.cancel_run),
            await handle_reply(f"{NMessageType.GET_ENVIRONMENT}.{p_routing}", self.get_environment),
            await handle_reply(f"{NMessageType.PING_WORKER_SET}.{p_routing}", self.ping),
        ]
        if self.worker_set_id is not None:  # only mark as active if not a local worker
            self.tasks.start(
                self.mark_as_active_if_active_forever(interval=WORKER_ACTIVE_PUBLISH_INTERVAL)
            )

        if self.project_id is not None:
            # preload worker for project (assumes it's at head)
            await self._prepare_worker(self.project_id)

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
            # nocheckin: module_id could be a project_id, which means self.workers key is wrong
            worker = ModuleWorkerProcess(module_id, self, timeout=WORKER_RUN_TIMEOUT)
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
        logger.debug("worker.mark_active", worker_set=self.worker_set_id)
        await redis.set(
            f"worker_set.{self.worker_set_id}.{self.worker_node_id}.last_active_at",
            value=str(time.time()),
            ex=24 * 60 * 60,  # keep for 1 day
        )

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
        logger.debug("run.start", msg=msg)
        worker = await self._prepare_worker(msg.p.module_id)
        run_job = worker.start_run(
            runnable=msg.p.runnable,
            arguments=msg.p.arguments,
            run_id=msg.p.run_id,
            session_id=msg.p.session_id,
            trigger_type=msg.p.trigger_type,
            trigger_id=msg.p.trigger_id,
            keyed=msg.p.keyed,
        )
        if isinstance(run_job, StartRunErrorType):  # couldn't queue run
            await msg.reply(RepStartRunPayload(error=run_job))
        else:
            if msg.p.block is not None:
                try:
                    await run_job.started.wait()
                    await asyncio.wait_for(run_job.terminated.wait(), timeout=msg.p.block)
                except asyncio.TimeoutError:
                    run_job.run = run_job.session.tracer.run.runs[run_job.id]
            run = wire.pack_data(run_job.run) if run_job.run else None
            logs = [wire.pack_data(log) for log in run_job.logs] if run_job.logs else None
            rep = RepStartRunPayload(error=run_job.error, run=run, run_id=run_job.id, logs=logs)
            await msg.reply(rep)

    @message_handler
    async def cancel_run(self, msg: NMessage[ReqCancelRunPayload]):
        logger.debug("run.cancel", msg=msg)
        worker = await self._prepare_worker(msg.p.module_id)
        success = await worker.cancel_run(msg.p.run_id)
        await msg.reply(RepCancelRunPayload(success=success))

    @message_handler
    async def get_environment(self, msg: NMessage[ReqGetEnvironmentPayload]):
        await msg.reply(RepGetEnvironmentPayload(environment=WORKER_ENVIRONMENT_DATA))

    @message_handler
    async def ping(self, msg: NMessage[ReqPingWorkerSetPayload]):
        await msg.reply(RepPingWorkerSetPayload(success=self.healthy))

    async def get_module(self, ref: ModuleReference | UUID) -> tuple[wire.ModuleTreeData, UUID]:
        """Gets a modules wire data"""
        log = logger.bind(ref=ref)
        cached = self.cached_committed_modules.get(ref)
        if cached is not None:
            log.debug("module.fetch", cached=True)
            return cached
        module_rep = await request(
            NMessageType.READ_MODULE, ReqReadModulePayload(ref), RepReadModulePayload
        )
        if module_rep.p.module.committed:
            self.cached_committed_modules[ref] = module_rep.p.module, module_rep.p.project_id
        log.debug("module.fetch", cached=False)
        return module_rep.p.module, module_rep.p.project_id

    async def fetch(self, ref: ModuleReference) -> wire.ModuleTreeData:
        return (await self.get_module(ref))[0]

    async def stop(self):
        logger.info("stop", worker_node=self.worker_node_id, workset_set=self.worker_set_id)
        await self.tasks.stop()
        await asyncio.gather(*[sub.unsubscribe() for sub in self.subs])
        self._ready.clear()


@dataclass(repr=False, slots=True)
class RunJob:
    run_data: RunData
    run: Optional[Run] = None
    task: asyncio.Task | None = None
    started: asyncio.Event = field(default_factory=asyncio.Event)
    terminated: asyncio.Event = field(default_factory=asyncio.Event)

    def __str__(self):
        return f"{self.run_data.id}"

    def __repr__(self):
        return f"<RunJob {self}>"


class ModuleWorkerProcess(ModuleWriter):
    """
    A user worker that helps run a specific module.
    Generally, a worker process is intended to process one run at a time (for now).
    """

    def __init__(self, module_id: UUID, node: "WorkerNode", timeout: float):
        self.node = node
        self.module_id = module_id
        self.project_id: Optional[UUID] = None  # set in init (requires runtime fetch)
        self.timeout = timeout
        self.ready = asyncio.Event()

        self.source: wire.ModuleTreeData | None = None
        self.module: Module | None = None
        self.queue: asyncio.Queue[tuple[int, RunJob]] = asyncio.PriorityQueue()
        self.pending_runs: dict[UUID, RunJob] = {}
        self.last_run: Optional[RunJob] = None
        self.executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="worker")
        self.log = logger.bind(
            worker_set=self.node.worker_set_id,
            worker_node=self.node.worker_node_id,
            module_id=self.module_id,
        )

    @property
    def interpreted(self) -> bool:
        return self.module is not None

    @property
    def active(self) -> bool:
        last_run_recent = (
            self.last_run is not None
            and self.last_run.created_at > datetime.utcnow() - WORKER_ACTIVE_TIMEOUT
        )
        return last_run_recent or not self.queue.empty()

    async def run(self):
        """Runs the module worker main processing loop"""

        # first interp
        self.log.info("module.start")
        self.source, self.project_id = await self.node.get_module(self.module_id)
        try:
            self.module = await sync_to_async(Module.interp_from)(self.source, session=None)
        except Exception as e:
            self.log.error("module.init.failed", exc_info=e)
            raise RuntimeError(f"failed to initialize module worker {self}")

        self.ready.set()

        # process run tasks ad infinitum
        await self._process_runs_forever()

    async def _process_runs_forever(self):
        while True:
            _, job = await self.queue.get()
            if job.cancelled:
                continue

            try:
                self.log.debug("run", job=job, timeout=self.timeout)
                job.started.set()
                await self._do_run(job, self.timeout)
                if job.session.tracer.run.runs:
                    job.run = job.session.tracer.run.runs[job.id]
                    job.logs = job.session.tracer.cached_logs[:50]
                self.log.debug("run.completed", job=job)
            except asyncio.CancelledError:
                self.log.info("run.cancelled", job=job)
                # keep the queue running?
            except Exception as e:
                job.error = StartRunErrorType.RUNTIME_ERROR
                self.log.exception("run.failed", job=job, sentry=sentry_capture_if_enabled(e))
            finally:
                job.terminated.set()
                self.queue.task_done()

    async def interp_on_change(self, mutations: list[ModuleMutation]):
        self.log.debug("module.interp", mutations=len(mutations))
        new_source = ModuleMutator(self.source, mutations).to_module()
        self.source = new_source
        self.module = await sync_to_async(Module.interp_from)(new_source, session=None)

    def start_run(self, run_data: RunData, keyed: bool) -> RunJob | StartRunErrorType:
        self.log.exception("worker.start_run", run_data=run_data, keyed=keyed)
        # nocheckin: simplify run and respect scheduled_at
        if not self.interpreted:
            return StartRunErrorType.NOT_READY

        # get the runnable
        runnable = self.module.lookup(runnable)
        if runnable is None:
            return StartRunErrorType.INVALID_RUNCONFIG

        first_run_id = run_id or UUIDT()
        # instantiate
        try:
            session_ctx = SessionContext(
                module_id=self.module_id,
                project_id=self.project_id,
                worker_node_id=self.node.worker_node_id,
                trigger_type=trigger_type,
                trigger_id=trigger_id,
                first_run_id=first_run_id,
            )
        except Exception as e:
            self.log.exception("worker.start_run.failed", exc_info=e)
            return StartRunErrorType.INVALID_RUNCONFIG

        if keyed:
            arguments = map_value(
                arguments,
                runnable,
                map_k=lambda f: (f.typed_key, f.py_ident),
                map_v=instantiate_py_value_flat,
                is_output=False,
            )
        job = RunJob(id=first_run_id, session=session, runnable=runnable, arguments=arguments)
        if run_data.scheduled_at:
            pass  # nocheckin queue at scheduled_at
        else:
            # immediately add to queue
            self.queue.put_nowait((job.priority, job))
            session.tracer.run_queue(runnable, arguments, queue_position=self.queue.qsize())
        return job

    async def _do_run(self, job: RunJob, timeout: float) -> Optional[StartRunErrorType]:
        """Actually runs the job, and updates the job with the result"""
        try:
            self.log.info(
                "module.run",
                runnable=job.runnable,
                arguments=describe_type(job.arguments),
                timeout=timeout,
            )
            # TODO @Architecture @Robustness: handle module instantiation & session linking better
            #  esp. with contexts, dependencies, parallelism, etc.
            job.session.module.activate_in(job.session)
            arguments = map_value(
                job.arguments,
                job.runnable,
                map_k=lambda f: (f.py_ident, f.py_ident),
                map_v=instantiate_py_value_flat,
                is_output=False,
            )
            job.task = asyncio.create_task(run(job.runnable, arguments, job.session))
            self.pending_runs[job.id] = job
            self.last_run = job
            await asyncio.wait_for(job.task, timeout=timeout)
            return None
        except RunError as e:
            self.log.exception("module.run.failed", exc_info=e)
            return StartRunErrorType.RUNTIME_ERROR
        except Exception as e:
            self.log.exception("module.run.failed", exc_info=e, sentry=sentry_capture_if_enabled(e))
            return StartRunErrorType.INTERNAL_ERROR
        finally:
            job.session.module.deactivate()
            if job.id in self.pending_runs:
                del self.pending_runs[job.id]

    async def cancel_run(self, run_id: UUID) -> bool:
        job = self.pending_runs.get(run_id)
        if not job:
            for j in self.queue._queue:
                if j.id == run_id:
                    job = j
                    break
        if not job:
            return False

        job.cancelled = True
        job.run.session.tracer.run_cancel(job.runnable, job.arguments)

        if job.task:
            # this doesn't really work, but we're not using this cancellation yet
            # we need some better mechanism for targeted :RunCancellation
            # maybe another thread? multiprocessing?
            job.task.cancel()
            # send keybord interrupt to self (this causes all sorts of unintended cancels)
            # os.kill(os.getpid(), signal.SIGINT)
        return True

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
            NMessageType.WRITE_MODULE, req, RepWriteModulePayload
        )
        return rep.p.success

    async def write_session(
        self, session: "Session", runs: list["Run"], logs: list["LogEntry"]
    ) -> bool:
        self.log.debug("session.write", session=session, runs=len(runs), logs=len(logs))

        session_data = wire.pack_data(session)
        runs_data = [wire.pack_data(run) for run in runs]
        logs_data = [wire.pack_data(log) for log in logs]

        req = ReqWriteSessionPayload(
            module_id=self.module_id,
            session=session_data,
            runs=runs_data,
            logs=logs_data,
            client=self.node.client,
        )
        rep: NMessage[RepWriteSessionPayload] = await request(
            NMessageType.WRITE_SESSION, req, RepWriteSessionPayload
        )
        return rep.p.success
