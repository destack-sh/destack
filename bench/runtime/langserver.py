import asyncio
from collections import defaultdict, deque
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from functools import partial
from itertools import chain
from typing import Callable, ClassVar, Deque, Optional
from uuid import UUID

import pytz
import structlog
from asgiref.sync import sync_to_async

from bench import language, models
from bench.language import ModuleIndex, wire
from bench.language.mutate import ModuleMutation, ModuleMutator
from bench.language.type import Build, BuildSettings, InterpSymbol
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.models import Execution, ExecutionStatus, ProjectVersion, mapper
from bench.models.execution import PENDING_EXECUTION_STATUSES
from bench.models.job import PENDING_JOB_STATUSES, JobStatus, JobType
from bench.models.mapper import read_module, write_mutations
from bench.msg import NMessage
from bench.msg.core import handle_reply, message_handler, nc_init, publish, publish_soon, subscribe
from bench.msg.messages import (
    ClientOrigin,
    EvaluationSavedPayload,
    ExecutionChangedPayload,
    ExecutionMarkedDeadPayload,
    ExecutionSavedPayload,
    InterpChangedPayload,
    JobSavedPayload,
    ModuleChangedPayload,
    ModuleInternalChangedPayload,
    NMessageType,
    RepInterpPayload,
    RepReadModulePayload,
    RepReadObjectPayload,
    RepRegisterWorkerPayload,
    RepWriteModulePayload,
    ReqInterpPayload,
    ReqReadModulePayload,
    ReqReadObjectPayload,
    ReqRegisterWorkerPayload,
    ReqWriteModulePayload,
    WorkerHeartbeatPayload,
)
from bench.runtime.evaluate import lint
from bench.runtime.interp import (
    InterpModule,
    LanguageInterpreter,
    ModuleFetcher,
    get_or_create_file,
    get_requirements,
    interp_module,
)
from bench.runtime.map import map_to_file
from bench.runtime.mutate import map_mutation_to_public
from bench.runtime.reactivity import RevisionMap, get_stale_symbols, tree_from_module
from bench.runtime.tracing import (
    ExecutionTrackerContext,
    WorkerContext,
    pub_tracker_ctx,
    worker_ctx,
)
from bench.runtime.type import (
    EvaluationMetric,
    EvaluationResult,
    EvaluationResultData,
    ExecutionFrameData,
    JobData,
)
from bench.utils.cache import redis
from bench.utils.func import debounce, wrap_task
from bench.utils.utils import get_from_env, required_field, sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


WORKER_HEARTBEAT_TIMEOUT = 30


class ModuleDB:
    def __init__(self, cache_committed: bool = True):
        self.cache_committed = cache_committed
        self._cached_modules: dict[UUID, tuple[wire.ModuleData, UUID]] = {}

    async def get_module(self, module_id: UUID) -> tuple[wire.ModuleData, UUID]:
        if module_id in self._cached_modules:
            return self._cached_modules[module_id]
        project_version = await ProjectVersion.objects.aget(id=module_id)
        module = await sync_to_async(read_module)(project_version, exclude_non_semantic=False)
        if self.cache_committed and project_version.committed:
            self._cached_modules[module_id] = module, project_version.id
        return module, project_version.project_id

    async def fetch(self, module_id: UUID) -> wire.ModuleData:
        return (await self.get_module(module_id))[0]


class LanguageServer:
    """
    Bench language & runtime server for interpretation and managing runtime state.
    Also manages sandboxed worker lifecycle (for now?).
    """

    def __init__(self):
        self.id = UUIDT()
        self.lang_workers: dict[UUID, LanguageWorker] = {}
        self.subs = []
        self.tasks = []
        self.module_db = ModuleDB()

    async def run(self):
        await nc_init.wait()
        logger.info("start")
        self.subs = [
            await handle_reply(NMessageType.REQUEST_REGISTER_WORKER, self.register_worker),
            await subscribe(NMessageType.WORKER_HEARTBEAT, cb=self.worker_heartbeat),
            await handle_reply(NMessageType.REQUEST_READ_MODULE, self.read_module),
            await handle_reply(NMessageType.REQUEST_WRITE_MODULE, self.write_module),
            await handle_reply(NMessageType.REQUEST_INTERP, self.request_module_interp),
            await handle_reply(NMessageType.REQUEST_READ_OBJECT, self.read_object),
            await subscribe(f"{NMessageType.EXECUTION_CHANGED}.*", cb=self.execution_changed),
            await subscribe(
                f"{NMessageType.EXECUTION_MARKED_DEAD}.*", cb=self.execution_marked_dead
            ),
            await subscribe(f"{NMessageType.MODULE_INTERNAL_CHANGED}.*", cb=self.module_changed),
        ]
        self.tasks = [
            create_wrapped_task(self.manage_sandboxed_workers(interval_seconds=10)),
            create_wrapped_task(self.manage_timeouts(interval_seconds=10, timeout_seconds=60)),
        ]
        # register self as worker
        await models.Worker.objects.acreate(
            id=self.id,
            status=models.WorkerStatus.ACTIVE,
            type=models.WorkerType.LANGUAGE,
            started_at=datetime.utcnow().replace(tzinfo=pytz.utc),
        )

    async def _get_ready_worker(self, module_id: UUID) -> "LanguageWorker":
        worker = self.lang_workers.get(module_id)
        if worker is None:
            # start language worker if not already started
            # TODO @Broken: assign workers to deployments
            project_version = await ProjectVersion.objects.aget(id=module_id)
            worker = LanguageWorker(self.id, project_version, self.module_db.fetch)
            self.lang_workers[module_id] = worker
            asyncio.create_task(wrap_task(worker.run(), "worker_run_" + str(module_id)))
        if not worker.ready.is_set():
            await worker.ready.wait()
        return worker

    @message_handler
    async def register_worker(self, msg: NMessage[ReqRegisterWorkerPayload]) -> None:
        try:
            worker = await models.Worker.objects.acreate(
                id=msg.payload.worker_id,
                status=models.WorkerStatus.ACTIVE,
                deployment_id=msg.p.deployment_id,
                project_id=msg.p.project_id,
                type=msg.p.type,
                started_at=datetime.utcnow().replace(tzinfo=pytz.utc),
            )
            success = True
            logger.info("register_worker", worker=worker)
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("register_worker.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepRegisterWorkerPayload(success=success))

    @message_handler
    async def worker_heartbeat(self, msg: NMessage[WorkerHeartbeatPayload]) -> None:
        last_seen = datetime.utcnow().replace(tzinfo=pytz.utc)
        await redis.set(
            f"worker.{msg.payload.worker_id}.heartbeat", str(last_seen), ex=WORKER_HEARTBEAT_TIMEOUT
        )

    @message_handler
    async def read_module(self, msg: NMessage[ReqReadModulePayload]) -> None:
        logger.debug("module.read", msg=msg)
        module, project_id = await self.module_db.get_module(msg.p.module_id)
        await msg.reply(RepReadModulePayload(module=module, project_id=project_id))

    @message_handler
    async def write_module(self, msg: NMessage[ReqWriteModulePayload]) -> None:
        logger.debug("module.write", msg=msg)
        # TODO @Security: check if msg origin has write access to module
        worker = await self._get_ready_worker(msg.p.module_id)
        try:
            await worker.write_module(msg.p.mutations, origins=(msg.p.client,))
            logger.debug("module.write.done", msg=msg)
            success = True
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("write_module.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepWriteModulePayload(success=success))

    @message_handler
    async def read_object(self, msg: NMessage[ReqReadObjectPayload]) -> None:
        logger.debug("object.read", msg=msg)
        # TODO @Security: check if msg origin has read access to object
        get_urls: list[str | None] = []
        async for model_obj in models.RemoteObject.objects.filter(
            id__in=(obj.id for obj in msg.p.objects)
        ):
            model_obj: models.RemoteObject
            obj_data = msg.p.objects[len(get_urls)]
            if obj_data.sha512 != model_obj.sha512:
                logger.warning(
                    "object.read.sha512_mismatch", msg=msg, obj=model_obj, obj_data=obj_data
                )
                get_urls.append(None)
            else:
                get_urls.append(model_obj.presigned_get)
        logger.debug("object.read.rep", msg=msg, get_urls=[url is not None for url in get_urls])
        await msg.reply(RepReadObjectPayload(get_urls=get_urls))

    @message_handler
    async def request_module_interp(self, msg: NMessage[ReqInterpPayload]):
        logger.debug("module.interp", msg=msg)
        worker = await self._get_ready_worker(msg.p.module_id)
        payload = make_full_change_payload(worker, RepInterpPayload)
        await msg.reply(payload)

    @message_handler
    async def execution_changed(self, msg: NMessage[ExecutionChangedPayload]) -> None:
        save_success = await sync_to_async(save_execution_frames)(msg.payload.frames)
        if save_success:
            # forward to API clients now that DB frames are saved
            await publish(
                NMessageType.EXECUTION_SAVED,
                ExecutionSavedPayload(module_id=msg.p.module_id, frames=msg.p.frames),
            )

    @message_handler
    async def execution_marked_dead(self, msg: NMessage[ExecutionMarkedDeadPayload]) -> None:
        execution = await models.Execution.objects.filter(id=msg.p.execution_id).afirst()
        if execution is None:
            logger.warning(
                "execution_marked_dead.not_found", msg=msg, execution_id=msg.p.execution_id
            )
            return
        if execution.terminated_at is not None:
            return
        execution.status = models.ExecutionStatus.Aborted
        execution.terminated_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        await execution.asave()
        await publish(
            NMessageType.EXECUTION_SAVED,
            ExecutionSavedPayload(
                module_id=msg.p.module_id, frames=[mapper.wmap_execution_frame(execution)]
            ),
        )

    @message_handler
    async def module_changed(self, msg: NMessage[ModuleInternalChangedPayload]) -> None:
        if msg.p.has_origin(self.id):
            return  # ignore own changes
        # update language worker
        if msg.p.module_id in self.lang_workers:
            worker = self.lang_workers[msg.p.module_id]
            worker.on_module_changed(msg.p.mutations)

    async def manage_sandboxed_workers(self, interval_seconds: int):
        """Update last seens and mark any unresponsive workers as inactive."""
        while True:
            # get last seen for all workers
            live_worker_keys = [
                worker_id async for worker_id in redis.scan_iter("worker.*.heartbeat")
            ]
            live_worker_ids = [UUID(key.decode().split(".")[1]) for key in live_worker_keys]
            live_worker_ids.append(self.id)  # we're a worker too
            last_seen = await redis.mget(keys=live_worker_keys)
            if len(live_worker_ids) != len(last_seen):
                continue  # try again?
            last_seen = [datetime.fromisoformat(ts.decode()) for ts in last_seen]

            # batch update last seen for live workers
            live_workers = [w async for w in models.Worker.objects.filter(id__in=live_worker_ids)]
            for ls, worker in zip(last_seen + [datetime.utcnow()], live_workers):
                worker.last_seen_at = ls
            await models.Worker.objects.abulk_update(live_workers, ["last_seen_at"])

            # check if there are any dead workers
            liveness_cutoff = datetime.utcnow().replace(tzinfo=pytz.utc) - timedelta(
                seconds=WORKER_HEARTBEAT_TIMEOUT
            )
            dead_workers = [
                worker
                async for worker in models.Worker.objects.filter(
                    status=models.WorkerStatus.ACTIVE, last_seen_at__lt=liveness_cutoff
                )
            ]
            logger.debug("manage_workers", live_workers=live_workers, dead_workers=dead_workers)

            if dead_workers:
                # mark all relevant jobs and executions as failed
                dead_ids = [worker.id for worker in dead_workers]
                await Execution.objects.filter(
                    status__in=PENDING_EXECUTION_STATUSES, worker_id__in=dead_ids
                ).aupdate(status=ExecutionStatus.Failed)
                dead_jobs = [
                    job
                    async for job in models.Job.objects.filter(
                        status__in=PENDING_JOB_STATUSES, worker_id__in=dead_ids
                    )
                ]
                # publish job updates, then update
                await self.kill_jobs(dead_jobs)
                for worker in dead_workers:
                    worker.status = models.WorkerStatus.TERMINATED
                    worker.terminated_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                await models.Worker.objects.abulk_update(dead_workers, ["status", "terminated_at"])

            await asyncio.sleep(interval_seconds)

    async def manage_timeouts(self, interval_seconds: int, timeout_seconds: int):
        """Mark any timed out jobs or executions as failed."""
        while True:
            start_cutoff = datetime.utcnow().replace(tzinfo=pytz.utc) - timedelta(
                seconds=timeout_seconds
            )
            dead_jobs = [
                job
                async for job in models.Job.objects.filter(
                    status__in=PENDING_JOB_STATUSES, started_at__lt=start_cutoff
                )
            ]
            await self.kill_jobs(dead_jobs)
            await Execution.objects.filter(
                status__in=PENDING_EXECUTION_STATUSES,
                started_at__lt=start_cutoff,
            ).aupdate(status=ExecutionStatus.Failed)
            await asyncio.sleep(interval_seconds)

    async def kill_jobs(self, dead_jobs: list[models.Job]):
        for job in dead_jobs:
            job_data = mapper.wmap_job(job)
            job.status = models.JobStatus.Failed
            await publish(
                NMessageType.JOB_SAVED,
                JobSavedPayload(job=job_data, module_id=job.project_version_id),
            )
        await models.Job.objects.abulk_update(dead_jobs, ["status"])

    async def stop(self):
        logger.info("stop")
        await asyncio.gather(sub.unsubscribe() for sub in self.subs)
        # update self as worker
        await models.Worker.objects.filter(id=self.id).aupdate(
            status=models.WorkerStatus.TERMINATED
        )


# TODO @UX: reduce/avoid debounce for reactive module jobs
#  If too frequent, reactors lead to lots of unnecessary work and can run into rate limits.
GENERATE_DEBOUNCE = get_from_env("RUNTIME_REACTIVE_GENERATE_DEBOUNCE", 5, type_cast=float)
GENERATE_DEBOUNCE_MAX_WAIT = get_from_env(
    "RUNTIME_REACTIVE_GENERATE_DEBOUNCE_MAX_WAIT", 20, type_cast=float
)
LINT_DEBOUNCE = get_from_env("RUNTIME_REACTIVE_LINT_DEBOUNCE", 3, type_cast=float)
LINT_DEBOUNCE_MAX_WAIT = get_from_env(
    "RUNTIME_REACTIVE_LINT_DEBOUNCE_MAX_WAIT", 10, type_cast=float
)
BUILD_DEBOUNCE = get_from_env("RUNTIME_REACTIVE_BUILD_DEBOUNCE", 5, type_cast=float)
BUILD_DEBOUNCE_MAX_WAIT = get_from_env(
    "RUNTIME_REACTIVE_BUILD_DEBOUNCE_MAX_WAIT", 20, type_cast=float
)

# lower is higher
JOB_DEFAULT_PRIORITY = {
    JobType.INTERP: 0,
    JobType.GENERATE: 1,
    JobType.BUILD: 2,
    JobType.EVALUATE: 3,
    JobType.LINT: 4,
}


@dataclass(repr=False)
class Job:
    type: ClassVar[JobType]
    worker_ctx: "WorkerContext"
    module_hash: str | None
    reactive: bool
    id: UUID = field(default_factory=UUIDT)
    status: JobStatus = JobStatus.Queued
    started_at: Optional[datetime] = None
    terminated_at: Optional[datetime] = None
    task: Optional[asyncio.Task] = None
    terminated: asyncio.Event = field(default_factory=asyncio.Event)
    _created: bool = False

    def __str__(self):
        return f"{self.type} {self.id} ({self._content_str}, {self.status})"

    @property
    def _content_str(self):
        return "<blank>"

    @property
    def _content_symbols(self) -> list[UUID]:
        return []

    def __repr__(self):
        return f"<Job {self}>"

    def __lt__(self, other):
        return self.default_priority < other.default_priority

    def to_data(self) -> JobData:
        return JobData(
            id=self.id,
            type=self.type,
            project_id=self.worker_ctx.project_id,
            project_version_id=self.worker_ctx.module_id,
            deployment_id=self.worker_ctx.deployment_id,
            worker_id=self.worker_ctx.worker_id,
            status=self.status,
            started_at=self.started_at,
            terminated_at=self.terminated_at,
        )

    @property
    def worth_saving(self) -> bool:
        return self.type in [JobType.GENERATE, JobType.BUILD, JobType.EVALUATE]

    async def save_and_notify(self):
        if not self.worth_saving:
            return
        if not self._created:
            await mapper.rmap_job(self.to_data()).asave()
            self._created = True
        else:
            await models.Job.objects.filter(id=self.id).aupdate(
                status=self.status,
                started_at=self.started_at,
                terminated_at=self.terminated_at,
            )
        publish_soon(
            NMessageType.JOB_SAVED,
            JobSavedPayload(module_id=self.worker_ctx.module_id, jobs=[self.to_data()]),
        )

    async def cancel(self):
        self.status = JobStatus.Cancelling
        asyncio.create_task(self.save_and_notify())
        self.task.cancel()
        try:
            await self.task
        finally:
            self.status = JobStatus.Cancelled
            asyncio.create_task(self.save_and_notify())

    @property
    def success(self) -> bool:
        raise NotImplementedError

    @property
    def default_priority(self) -> int:
        return JOB_DEFAULT_PRIORITY[self.type]


@dataclass(repr=False, slots=True)
class InterpJob(Job):
    type: ClassVar[JobType] = JobType.INTERP
    new_source: wire.ModuleData = None
    success: bool = False


@dataclass(repr=False, slots=True)
class LintJob(Job):
    # TODO @Performance: scope lint job to specific files/symbols (cc :PartialModuleUpdates)
    type: ClassVar[JobType] = JobType.LINT
    evaluation: Optional[EvaluationResult] = None

    @property
    def success(self) -> bool:
        return self.evaluation is not None


@dataclass(repr=False, slots=True)
class GenerateJob(Job):
    type: ClassVar[JobType] = JobType.GENERATE
    revmap: RevisionMap = required_field()
    generator: Optional[InterpSymbol] = None
    success: bool = False

    @property
    def _content_str(self):
        return f"{self.generator}" if self.generator else "<blank>"

    @property
    def _content_symbols(self) -> list[UUID]:
        return [self.generator.id] if self.generator else []


def get_default_builds(interp):
    """
    Creates the default starter builds for a project.
    Note that these aren't used right now :BuildEvaluate
    """
    gpt35 = interp.symbol("openai.std.text.gpt-3-5-turbo")
    davinci3 = interp.symbol("openai.std.text.text-davinci-003")
    claude_instant = interp.symbol("anthropic.std.text.claude-instant")
    claude = interp.symbol("anthropic.std.text.claude")
    default_builds = [
        Build(
            name="balanced",
            comment="As all things should be.",
            settings=BuildSettings(
                reactive=False,
                weights={EvaluationMetric.Performance: 0.5, EvaluationMetric.Speed: 0.5},
            ),
            models=[gpt35, claude_instant],
        ),
        Build(
            name="fast",
            comment="Speedy and economic.",
            settings=BuildSettings(
                reactive=False,
                weights={EvaluationMetric.Performance: 0.2, EvaluationMetric.Speed: 0.8},
            ),
            models=[gpt35, claude_instant],
        ),
        Build(
            name="accurate",
            comment="The best at any cost.",
            settings=BuildSettings(
                reactive=False,
                weights={EvaluationMetric.Performance: 0.8, EvaluationMetric.Speed: 0.2},
            ),
            models=[davinci3, claude],
        ),
    ]
    return default_builds


COMPLETED_JOBS_BUFFER_SIZE = 128


class LanguageWorker:
    """Language server worker for a single module"""

    def __init__(
        self, worker_id: UUID, project_version: models.ProjectVersion, fetcher: ModuleFetcher
    ):
        self.worker_id = worker_id
        self.project_version = project_version
        self.ready = asyncio.Event()
        self.log = logger.bind(
            module_id=self.module_id, project_id=self.project_id, worker_id=self.worker_id
        )
        self.fetcher = fetcher
        self.interpreter = LanguageInterpreter(fetcher)
        self.worker_ctx = WorkerContext(
            worker_id=self.worker_id,
            module_id=self.module_id,
            project_id=self.project_id,
            deployment_id=None,
        )
        self.jobs_queue: asyncio.Queue[tuple[int, Job]] = asyncio.PriorityQueue()
        self.completed_jobs: Deque[Job] = deque(maxlen=COMPLETED_JOBS_BUFFER_SIZE)
        # module data
        self.source: wire.ModuleData | None = None
        self.interp: Optional[InterpModule] = None
        self.revmap: RevisionMap | None = None
        self.stale_symbols: list[language.Statement] | None = None
        self.module_hash: str | None = None
        # wire-able data of interpreted module
        self.wire_module: wire.ModuleData | None = None
        self.wire_errors: list[wire.ErrorData] | None = None
        self.wire_dependencies: dict[UUID, wire.ModuleData] | None = None

    @property
    def client(self) -> ClientOrigin:
        return ClientOrigin("worker", self.worker_id, None)

    @property
    def module_id(self) -> UUID:
        return self.project_version.id

    @property
    def project_id(self) -> UUID:
        return self.project_version.project_id

    @property
    def idx(self) -> ModuleIndex:
        if self.interp is None:
            raise RuntimeError("not interpreted yet")
        return self.interp.module_idx

    def mutate(self) -> ModuleMutator:
        return ModuleMutator(self.idx)

    @property
    def instruct_model(self) -> language.Model:
        return self.interp.symbol("openai.std.text.gpt-3-5-turbo")

    @property
    def interpreted(self) -> bool:
        return self.interp.module_idx is not None

    def is_stale(self, id: UUID):
        return any(s.id == id for s in self.stale_symbols)

    def _provide_context(self, job: Job):
        worker_ctx.set(self.worker_ctx)
        pub_ctx = ExecutionTrackerContext(
            tracing_level=ExecutionTracingLevel.ALL_FRAMES_WITH_DATA,
            trigger_type=ExecutionTriggerType.JOB,
            trigger_id=job.id,
        )
        pub_tracker_ctx.set(pub_ctx)

    def _queue_job(self, job: Job, priority: int = None) -> int:
        # prevent reactive loops
        if job.reactive:
            # we detect loops by checking for an unbroken string of same-state stateful reactive jobs
            # note that this doesn't detect mutation loops yet
            same_state_jobs = [
                j
                for j in self.completed_jobs
                if j.type == job.type and j.reactive and j._content_symbols == job._content_symbols
            ]
            found_break = len(same_state_jobs) == 0
            for j in self.completed_jobs:
                if j.type in (JobType.GENERATE, JobType.BUILD) and (
                    not j.reactive or j.module_hash != job.module_hash
                ):
                    found_break = True
                    break
                if j.type == job.type and j.reactive and j._content_symbols == job._content_symbols:
                    # found ourselves again
                    break
            if not found_break:
                job.status = JobStatus.Cancelled
                logger.warning("react.break", job=job)
                return -1

        priority = priority or job.default_priority
        self.jobs_queue.put_nowait((priority, job))
        qpos = self.jobs_queue.qsize()
        return qpos

    def _cancel_jobs_like(self, predicate: Callable[[Job], bool]):
        for job in self.jobs_queue._queue:
            if predicate(job) and job.status != JobStatus.Queued:
                asyncio.create_task(job.cancel())
        # mark pending jobs cancelled in queue
        for prio, job in self.jobs_queue._queue:
            if predicate(job):
                job.status = JobStatus.Cancelled

    def on_module_changed(self, mutator: list[ModuleMutation] | ModuleMutator) -> InterpJob:
        if not isinstance(mutator, ModuleMutator):
            mutator = ModuleMutator(self.idx, mutator)
        new_source = mutator.apply()
        job = InterpJob(
            # not sure if reactive=False is always correct?
            worker_ctx=self.worker_ctx,
            reactive=False,
            new_source=new_source,
            module_hash=None,
        )
        self._queue_job(job)
        return job

    async def write_module(
        self, mutations: list[ModuleMutation] | ModuleMutator, origins: tuple[ClientOrigin] = None
    ):
        if isinstance(mutations, ModuleMutator):
            mutations = mutations.mutations
        await sync_to_async(write_mutations)(project_v=self.project_version, mutations=mutations)
        self.on_module_changed(mutations)
        origins = (*(origins or ()), self.client)
        public_mutations = list(chain.from_iterable(map_mutation_to_public(m) for m in mutations))
        await publish(
            NMessageType.MODULE_INTERNAL_CHANGED,
            ModuleInternalChangedPayload(
                module_id=self.module_id, origins=origins, mutations=mutations
            ),
        )
        await publish(
            NMessageType.MODULE_CHANGED,
            ModuleChangedPayload(
                module_id=self.module_id, origins=origins, mutations=public_mutations
            ),
        )

    def _do_interp_sync(self, new_source: wire.ModuleData, dependencies) -> None:
        self.source = new_source
        self.interp = interp_module(new_source, [m.module_idx for m in dependencies])
        self.revmap = RevisionMap.from_module(self.source)
        logger.debug("module.interp.stale", module_id=self.module_id)
        self.stale_symbols = get_stale_symbols(self.revmap, self.interp.module_idx)
        logger.debug("module.interp.treehash", module_id=self.module_id)
        self.module_hash = tree_from_module(self.revmap, self.idx).stable_hash()
        logger.debug("module.interp.wire", module_id=self.module_id)
        if self.interp.module_idx:
            self.wire_module = wire.rmap_module(
                self.interp.module_idx.module, impute_type_references=True
            )
        else:  # re-use source (if failed to parse or not yet parsed)
            self.wire_module = self.source
        self.wire_errors = [wire.rmap_error(e) for e in self.interp.errors]
        self.wire_dependencies = {
            m.module.id: wire.rmap_module(m.module) for m in self.interp.dependencies
        }

    async def do_interp(self, new_source: wire.ModuleData) -> None:
        """Interprets the new module source, fetching deps and firing reactivity jobs"""
        prev_hash = self.module_hash
        requirements = get_requirements(new_source)
        dependencies = await self.interpreter.interp_requirements(requirements)
        await asyncio.get_event_loop().run_in_executor(
            None, partial(self._do_interp_sync, new_source, dependencies)
        )
        if prev_hash != self.module_hash:
            # reactively trigger (debounced) reactors
            if not self.interp.committed:
                create_wrapped_task(self._trigger_reactive_generate())
                # create_wrapped_task(self._trigger_reactive_lint()) (disabled for now)
            # notify clients
            payload = make_full_change_payload(self, InterpChangedPayload)
            await publish(NMessageType.INTERP_CHANGED, payload)

    @debounce(GENERATE_DEBOUNCE, max_wait=GENERATE_DEBOUNCE_MAX_WAIT)
    async def _trigger_reactive_generate(self) -> None:
        self.log.debug("module.react.generate")
        # default generate job for unbound/misc generation tasks
        self.queue_generate(generator=None, reactive=True, cancel_running=True)
        if not self.interp.has_user_errors:
            pass  # reactive task generation disabled for now

    @debounce(LINT_DEBOUNCE, max_wait=LINT_DEBOUNCE_MAX_WAIT)
    async def _trigger_reactive_lint(self) -> None:
        """Triggers all reactive lints for this module (as needed)"""
        self.log.debug("module.react.lint")
        self.queue_lint(cancel_running=True)

    def queue_lint(self, cancel_running: bool) -> LintJob:
        job = LintJob(worker_ctx=self.worker_ctx, reactive=False, module_hash=self.module_hash)
        if cancel_running:
            self._cancel_jobs_like(lambda j: isinstance(j, LintJob))
        self._queue_job(job)
        return job

    async def do_lint(self, job_id: UUID) -> EvaluationResult:
        evaluation = await lint(self.idx)
        await self.write_evaluation_results([evaluation], job_id)
        return evaluation

    def queue_generate(
        self, generator: Optional[InterpSymbol], reactive: bool, cancel_running: bool
    ) -> GenerateJob:
        job = GenerateJob(
            worker_ctx=self.worker_ctx,
            revmap=self.revmap,
            module_hash=self.module_hash,
            reactive=reactive,
            generator=generator,
        )
        if cancel_running:
            self._cancel_jobs_like(
                lambda j: isinstance(j, GenerateJob) and j.generator == generator
            )
        self._queue_job(job)
        return job

    async def do_generate(self, generator: Optional[InterpSymbol], revmap: RevisionMap) -> bool:
        if generator is None:
            await self._do_generate_autobuilds()
        else:
            raise NotImplementedError(f"unexpected generator type: {generator}")
        return True

    async def _do_generate_autobuilds(self):
        # special case: create default builds if they don't exist yet
        # sync the file name!  :AutobuildTasks
        autobuild_file, created = get_or_create_file(self.interp.module_idx, "instructors")
        if created:
            # gpt4 = self.interp.symbol("openai.std.text.gpt-4")
            default_builds = get_default_builds(self.interp)
            autobuild_file = map_to_file(default_builds, autobuild_file)
            await self.write_module(self.mutate().create(wire.rmap_file(autobuild_file)))

    async def run(self) -> None:
        source = await self.fetcher(self.module_id)
        await self.do_interp(source)
        self.ready.set()

        # process run jobs
        # TODO @Performance: run langserver jobs of same type in parallel?
        while True:
            _, job = await self.jobs_queue.get()
            if job.status == JobStatus.Cancelled:
                continue

            self._provide_context(job)
            create_task = asyncio.create_task
            try:
                job.status = JobStatus.Running
                job.started_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                self.log.debug("module.job.start", job=job)
                await job.save_and_notify()
                if isinstance(job, InterpJob):
                    job.task = create_task(self.do_interp(job.new_source))
                    await job.task
                    job.success = True
                elif isinstance(job, LintJob):
                    job.task = create_task(self.do_lint(job.id))
                    job.evaluation = await job.task
                elif isinstance(job, GenerateJob):
                    job.task = create_task(self.do_generate(job.generator, job.revmap))
                    job.success = await job.task
                else:
                    raise RuntimeError(f"unexpected job type: {job}")
                self.log.info("module.job.completed", job=job)
            except asyncio.CancelledError:
                self.log.info("module.job.cancelled", job=job)
                # keep the queue running?
            except Exception as e:
                sentry_enabled = sentry_capture_if_enabled(e)
                job.error = str(e)
                self.log.exception("module.job.failed", job=job, sentry_enabled=sentry_enabled)
            finally:
                job.status = JobStatus.Completed if job.success else JobStatus.Failed
                job.terminated_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                job.terminated.set()
                await job.save_and_notify()
                self.jobs_queue.task_done()
                self.completed_jobs.appendleft(job)

    async def write_evaluation_results(self, results: list[EvaluationResult], job_id: UUID) -> None:
        """Writes (upserts) the given evaluation results"""
        evaluations_data = []
        for evaluation in results:
            evaluations_data.extend(
                EvaluationResultData.from_result(
                    evaluation,
                    project_id=self.project_id,
                    project_version_id=self.module_id,
                    job_id=job_id,
                )
            )
        model_evaluations: list[models.EvaluationResult] = [
            mapper.rmap_evaluation_result(evaluation_data) for evaluation_data in evaluations_data
        ]
        await models.EvaluationResult.objects.abulk_create(
            model_evaluations,
            update_conflicts=True,
            unique_fields=["id"],
            update_fields=["updated_at", "job_id", "self_metrics", "aggregated_metrics"],
        )
        await publish(
            NMessageType.EVALUATION_SAVED,
            EvaluationSavedPayload(
                module_id=self.module_id,
                evaluations=evaluations_data,
            ),
        )


def make_change_payload(
    worker: LanguageWorker,
    cls,
    include_module: bool = False,
    include_dependencies: bool = False,
):
    """Builds a complete runtime change message from the module worker's state"""
    dependencies = (
        list(worker.wire_dependencies.values()) if worker.wire_dependencies is not None else None
    )
    stale_symbols = (
        [s.id for s in worker.stale_symbols] if worker.stale_symbols is not None else None
    )
    builds_by_symbol = defaultdict(list)
    for b in worker.idx.symbols_of_type(Build):
        for task in b.tasks:
            builds_by_symbol[task.definition.id].append(b.id)
    return cls(
        module_id=worker.module_id,
        updated_at=datetime.utcnow().replace(tzinfo=pytz.utc),
        module=worker.wire_module if include_module else None,
        dependencies=dependencies if include_dependencies else None,
        errors=worker.wire_errors if include_module else None,
        stale_symbols=stale_symbols if include_module else None,
        builds_by_symbol=builds_by_symbol if include_module else None,
    )


def make_full_change_payload(worker: LanguageWorker, cls):
    return make_change_payload(worker, cls, include_module=True, include_dependencies=True)


def save_execution_frames(frames: list[ExecutionFrameData]) -> bool:
    model_executions: list[Execution] = []
    seen_ids = set()  # dedup by id, keep last (assumes chronological order)
    for frame in reversed(frames):
        if frame.id in seen_ids:
            continue
        seen_ids.add(frame.id)
        execution = mapper.rmap_execution_frame(frame)
        model_executions.append(execution)

    try:
        # upsert frames
        Execution.objects.bulk_create(
            model_executions,
            update_conflicts=True,
            unique_fields=["id"],
            update_fields=[
                "status",
                "terminated_at",
                "cached_generated_at",
                "cached_duration",
                "outputs",
                "error",
            ],
        )
        return True
    except Exception as e:
        logger.error("save_execution_frames_failed", exc_info=e, executions=model_executions)
        return False
