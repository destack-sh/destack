import asyncio
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from itertools import chain
from typing import Callable, ClassVar, Optional
from uuid import UUID

from more_itertools import first
import pytz
import structlog
from asgiref.sync import sync_to_async

from bench import language, models
from bench.language import ModuleIndex, wire
from bench.language.type import (
    Build,
    BuildSettings,
    EvaluateSettings,
    InterpSymbol,
    TypeTag,
    GeneratedMapping,
    GeneratedMappingType,
)
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.models import Execution, ExecutionStatus, ProjectVersion, mapper
from bench.models.execution import PENDING_EXECUTION_STATUSES
from bench.models.job import PENDING_JOB_STATUSES, JobStatus, JobType
from bench.models.mapper import read_module, write_module, write_statements
from bench.msg import NMessage
from bench.msg.core import handle_reply, message_handler, nc_init, publish, subscribe
from bench.msg.messages import (
    BuildErrorType,
    EvaluationSavedPayload,
    ExecutionChangedPayload,
    ExecutionSavedPayload,
    InterpModuleChangedPayload,
    JobSavedPayload,
    ModuleChangedPayload,
    NMessageType,
    ProjectVersionChangedPayload,
    RepInterpModulePayload,
    RepModuleBuildPayload,
    RepReadModulePayload,
    RepRegisterWorkerPayload,
    ReqInterpModulePayload,
    ReqModuleBuildPayload,
    ReqReadModulePayload,
    ReqRegisterWorkerPayload,
    WorkerHeartbeatPayload,
)
from bench.msg.sync import is_semantic_mutation
from bench.runtime.build import (
    BuildCandidate,
    BuildResult,
    BuildTracker,
    build,
    get_build_files_for,
    get_builds_for,
)
from bench.runtime.evaluate import lint, plan_evaluate_task
from bench.runtime.interp import (
    InterpModule,
    LanguageInterpreter,
    ModuleFetcher,
    get_or_create_file,
    get_requirements,
    interp_module,
)
from bench.runtime.map import map_to_file, map_to_statement
from bench.runtime.reactivity import RevisionMap, get_stale_symbols, tracked_tree_from_symbol
from bench.runtime.tracing import (
    ExecutionTrackerContext,
    WorkerContext,
    pub_tracker_ctx,
    worker_ctx,
)
from bench.runtime.type import (
    BuildScope,
    EvaluationMetric,
    EvaluationResult,
    EvaluationResultData,
    ExecutionFrameData,
    JobData, EvaluationPlan,
)
from bench.utils.cache import redis
from bench.utils.fractional import INTEGER_MINUS_ONE
from bench.utils.func import debounce, wrap_task
from bench.utils.utils import get_from_env, sentry_capture_if_enabled, required_field
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
        module = await sync_to_async(read_module)(project_version, exclude_non_semantic=True)
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
            await handle_reply(NMessageType.REQUEST_INTERP_MODULE, self.request_module_interp),
            await handle_reply(NMessageType.REQUEST_MODULE_BUILD, self.request_module_build),
            await subscribe(f"{NMessageType.EXECUTION_CHANGED}.*", cb=self.execution_changed),
            await subscribe(
                f"{NMessageType.PROJECT_VERSION_CHANGED}.*", cb=self.project_version_changed
            ),
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
    async def request_module_interp(self, msg: NMessage[ReqInterpModulePayload]):
        logger.debug("module.interp", msg=msg)
        worker = await self._get_ready_worker(msg.p.module_id)
        payload = make_full_change_payload(worker, RepInterpModulePayload)
        await msg.reply(payload)

    @message_handler
    async def request_module_build(self, msg: NMessage[ReqModuleBuildPayload]):
        logger.debug("module.build", msg=msg)
        worker = await self._get_ready_worker(msg.p.module_id)
        build_job = worker.queue_build(msg.p.scope, msg.p.buildable_id, cancel_running=True)
        error = build_job if isinstance(build_job, BuildErrorType) else None
        await msg.reply(RepModuleBuildPayload(error=error))

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
    async def project_version_changed(self, msg: NMessage[ProjectVersionChangedPayload]) -> None:
        # reload project version as module
        # TODO @Performance: send partial module updates :PartialModuleUpdates
        if not any(is_semantic_mutation(mutation) for mutation in msg.p.mutations):
            return  # ignore non-semantic changes to modules
        project_v = await ProjectVersion.objects.filter(id=msg.p.project_version_id).afirst()
        if project_v is None:
            return  # just ignore, was probably deleted
        module = await sync_to_async(read_module)(project_v, exclude_non_semantic=True)
        await publish(
            NMessageType.MODULE_CHANGED, ModuleChangedPayload(module_id=module.id, module=module)
        )

        # update language workers
        if module.id in self.lang_workers:
            worker = self.lang_workers[module.id]
            worker.on_module_changed(module)

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
GENERATE_DEBOUNCE = get_from_env("RUNTIME_REACTIVE_GENERATE_DEBOUNCE", 1, type_cast=float)
GENERATE_DEBOUNCE_MAX_WAIT = get_from_env(
    "RUNTIME_REACTIVE_GENERATE_DEBOUNCE_MAX_WAIT", 3, type_cast=float
)
LINT_DEBOUNCE = get_from_env("RUNTIME_REACTIVE_LINT_DEBOUNCE", 2, type_cast=float)
LINT_DEBOUNCE_MAX_WAIT = get_from_env("RUNTIME_REACTIVE_LINT_DEBOUNCE_MAX_WAIT", 5, type_cast=float)
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
    project_id: UUID
    project_version_id: UUID
    deployment_id: Optional[UUID]
    worker_id: UUID
    id: UUID = field(default_factory=UUIDT)
    status: JobStatus = JobStatus.Queued
    started_at: Optional[datetime] = None
    terminated_at: Optional[datetime] = None
    task: Optional[asyncio.Task] = None
    terminated: asyncio.Event = field(default_factory=asyncio.Event)

    def __str__(self):
        return f"{self.type} {self.id} ({self.status})"

    def __repr__(self):
        return f"<Job {self}>"

    def __lt__(self, other):
        return self.default_priority < other.default_priority

    def to_data(self) -> JobData:
        return JobData(
            id=self.id,
            type=self.type,
            project_id=self.project_id,
            project_version_id=self.project_version_id,
            deployment_id=self.deployment_id,
            worker_id=self.worker_id,
            status=self.status,
            started_at=self.started_at,
            terminated_at=self.terminated_at,
        )

    async def save(self):
        await models.Job.objects.abulk_create(
            [mapper.rmap_job(self.to_data())],
            update_conflicts=True,
            unique_fields=["id"],
            update_fields=["status", "started_at", "terminated_at"],
        )

    async def save_and_notify(self):
        await self.save()
        await publish(
            NMessageType.JOB_SAVED,
            JobSavedPayload(module_id=self.project_version_id, job=self.to_data()),
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


@dataclass(repr=False, slots=True)
class BuildJob(Job):
    type: ClassVar[JobType] = JobType.BUILD
    scope: BuildScope = None
    buildable_id: UUID = None
    builds: list[Build] = None
    evals: dict[UUID, EvaluationPlan]= None
    revmap: RevisionMap = None
    build_results: list[BuildResult] = None

    @property
    def success(self) -> bool:
        return self.build_results is not None


@dataclass(repr=False, slots=True)
class EvaluateJob(Job):
    type: ClassVar[JobType] = JobType.EVALUATE
    evalable_id: UUID = None
    builds: list[Build] = None
    evaluation_result: Optional[EvaluationResult] = None

    @property
    def success(self) -> bool:
        return self.evaluation_result is not None


def get_default_builds(interp):
    """Creates the default starter builds for a project."""
    gpt35 = interp.symbol("openai.std.text.gpt-3-5-turbo")
    davinci3 = interp.symbol("openai.std.text.text-davinci-003")
    claude_instant = interp.symbol("anthropic.std.text.claude-instant")
    claude = interp.symbol("anthropic.std.text.claude")
    default_builds = [
        Build(
            name="balanced",
            comment="As all things should be.",
            settings=BuildSettings(reactive=True),
            evaluate_settings=EvaluateSettings(
                reactive=True,
                weights={EvaluationMetric.Performance: 0.5, EvaluationMetric.Speed: 0.5},
            ),
            models=[gpt35, claude_instant],
        ),
        Build(
            name="fast",
            comment="Speedy and economic.",
            settings=BuildSettings(reactive=False),
            evaluate_settings=EvaluateSettings(
                reactive=False,
                weights={EvaluationMetric.Performance: 0.2, EvaluationMetric.Speed: 0.8},
            ),
            models=[gpt35, claude_instant],
        ),
        Build(
            name="accurate",
            comment="The best at any cost.",
            settings=BuildSettings(reactive=False),
            evaluate_settings=EvaluateSettings(
                reactive=False,
                weights={EvaluationMetric.Performance: 0.8, EvaluationMetric.Speed: 0.2},
            ),
            models=[davinci3, claude],
        ),
    ]
    return default_builds


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
        # module data
        self.source: wire.ModuleData | None = None
        self.interp: Optional[InterpModule] = None
        self.revmap: RevisionMap | None = None
        self.stale_symbols: list[language.Statement] | None = None
        self.wire_module: wire.ModuleData | None = None
        self.wire_errors: list[wire.ErrorData] | None = None
        self.wire_dependencies: dict[UUID, wire.ModuleData] | None = None
        self.jobs_queue: asyncio.Queue[tuple[int, Job]] = asyncio.PriorityQueue()

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

    @property
    def instruct_model(self) -> language.Model:
        return self.interp.symbol("openai.std.text.gpt-3-5-turbo")

    @property
    def interpreted(self) -> bool:
        return self.interp.module_idx is not None

    def _provide_context(self, job: Job):
        worker_ctx.set(self.worker_ctx)
        pub_ctx = ExecutionTrackerContext(
            tracing_level=ExecutionTracingLevel.ALL_FRAMES_WITH_DATA,
            trigger_type=ExecutionTriggerType.JOB,
            trigger_id=job.id,
        )
        pub_tracker_ctx.set(pub_ctx)

    def _queue_job(self, job: Job, priority: int = None) -> int:
        priority = priority or job.default_priority
        self.jobs_queue.put_nowait((priority, job))
        qpos = self.jobs_queue.qsize()
        return qpos

    def _cancel_jobs_like(self, predicate: Callable[[Job], bool]):
        for job in self.jobs_queue._queue:
            if predicate(job) and job.status == JobStatus.Queued:
                asyncio.create_task(job.cancel())
        # mark pending jobs cancelled in queue
        for prio, job in self.jobs_queue._queue:
            if predicate(job):
                job.status = JobStatus.Cancelled

    def on_module_changed(self, source: wire.ModuleData) -> InterpJob:
        job = InterpJob(
            new_source=source,
            project_id=self.project_id,
            project_version_id=self.module_id,
            worker_id=self.worker_id,
            deployment_id=None,
        )
        self._queue_job(job)
        return job

    async def write_files_immediate(
        self,
        files: list[wire.FileData],
        overwrite: bool,
        delete_files: set[UUID] = None,
        generated_mappings: list[tuple[UUID, wire.StatementData]] = None,
    ) -> None:
        await sync_to_async(write_module)(
            project_v=self.project_version,
            files=files,
            overwrite=overwrite,
            delete_files=delete_files,
            generated_mappings=generated_mappings,
        )
        # TODO @Performance: apply module writes locally immediately :ImmediateModuleWrites
        module = await sync_to_async(read_module)(self.project_version, exclude_non_semantic=True)
        self.on_module_changed(module)

    async def write_statements_immediate(
        self,
        statements: list[wire.StatementData],
        overwrite: bool,
        generated_mappings: list[tuple[UUID, list[wire.GeneratedMapping]]] = None,
    ) -> None:
        await sync_to_async(write_statements)(
            project_v=self.project_version,
            statements=statements,
            overwrite=overwrite,
            generated_mappings=generated_mappings,
        )
        # TODO @Performance: apply module writes locally immediately :ImmediateModuleWrites
        module = await sync_to_async(read_module)(self.project_version, exclude_non_semantic=True)
        self.on_module_changed(module)

    async def do_interp(self, new_source: wire.ModuleData) -> None:
        """Interprets the new module source, fetching deps and firing reactivity jobs"""
        requirements = get_requirements(new_source)
        dependencies = await self.interpreter.interp_requirements(requirements)

        self.source = new_source
        self.interp = interp_module(new_source, [m.module_idx for m in dependencies])
        self.revmap = RevisionMap.from_module(self.source)
        self.stale_symbols = get_stale_symbols(self.revmap, self.interp.module_idx)
        if self.interp.module_idx:
            self.wire_module = wire.rmap_module(self.interp.module_idx.module)
        else:  # re-use source (if failed to parse or not yet parsed)
            self.wire_module = self.source
        self.wire_errors = [wire.rmap_error(e) for e in self.interp.errors]
        self.wire_dependencies = {
            m.module.id: wire.rmap_module(m.module) for m in self.interp.dependencies
        }
        # reactively trigger (debounced) reactors
        if not self.interp.committed:
            create_wrapped_task(self._fire_reactive_generate())
            create_wrapped_task(self._fire_reactive_lint())
            create_wrapped_task(self._fire_reactive_build())
        # notify
        payload = make_full_change_payload(self, InterpModuleChangedPayload)
        await publish(NMessageType.INTERP_MODULE_CHANGED, payload)

    @debounce(GENERATE_DEBOUNCE, max_wait=GENERATE_DEBOUNCE_MAX_WAIT)
    async def _fire_reactive_generate(self) -> None:
        self.log.debug("module.react.generate")
        # default generate job for unbound/misc generation tasks
        self.queue_generate(generator=None, cancel_running=True)
        if not self.interp.has_user_errors:
            # all reactive stale builds and first timers (that weren't generated yet)
            stale_tasks = [
                task
                for task in self.idx.symbols_of_type(language.Task)
                if (task in self.stale_symbols or task.generated_mappings == [])
                and not task.is_generated
            ]
            for task in stale_tasks:
                self.queue_generate(generator=task, cancel_running=True)

    @debounce(LINT_DEBOUNCE, max_wait=LINT_DEBOUNCE_MAX_WAIT)
    async def _fire_reactive_lint(self) -> None:
        """Triggers all reactive jobs for this module (as needed)"""
        self.log.debug("module.react.lint")
        self.queue_lint(cancel_running=True)

    @debounce(BUILD_DEBOUNCE, max_wait=BUILD_DEBOUNCE_MAX_WAIT)
    async def _fire_reactive_build(self) -> None:
        """Triggers all reactive jobs for this module (as needed)"""
        self.log.debug("module.react.build", stale_symbols=self.stale_symbols)
        if not self.interp.has_user_errors:
            # all reactive stale builds and first timers (that weren't built yet)
            stale_builds = [
                b
                for b in self.idx.symbols_of_type(language.Build)
                if b.settings.reactive and (b in self.stale_symbols or b.generated_mappings == [])
            ]
            for b in stale_builds:
                self.queue_build(BuildScope.SELECTED, b.id, cancel_running=True)

    def queue_lint(self, cancel_running: bool) -> LintJob:
        job = LintJob(
            project_id=self.project_id,
            project_version_id=self.module_id,
            worker_id=self.worker_id,
            deployment_id=None,
        )
        if cancel_running:
            self._cancel_jobs_like(lambda j: isinstance(j, LintJob))
        self._queue_job(job)
        return job

    async def do_lint(self, job_id: UUID) -> EvaluationResult:
        evaluation = await lint(self.idx)
        await self.write_evaluation_results([evaluation], job_id)
        return evaluation

    def queue_generate(
        self, generator: Optional[InterpSymbol], cancel_running: bool
    ) -> GenerateJob:
        job = GenerateJob(
            revmap=self.revmap,
            project_id=self.project_id,
            project_version_id=self.module_id,
            worker_id=self.worker_id,
            deployment_id=None,
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
        elif isinstance(generator, language.Task):
            await self._do_generate_task_evaluation(generator, revmap)
        else:
            raise NotImplementedError(f"unexpected generator type: {generator}")
        return True

    async def _do_generate_autobuilds(self) -> bool:
        # special case: create default builds if they don't exist yet
        #  :AutobuildTasks
        autobuild_file, created = get_or_create_file(self.interp.module_idx, ".instruct")
        if created:
            # gpt4 = self.interp.symbol("openai.std.text.gpt-4")
            default_builds = get_default_builds(self.interp)
            autobuild_file = map_to_file(default_builds, [], autobuild_file)
            await self.write_files_immediate(
                files=[wire.rmap_file(autobuild_file)], overwrite=False
            )
        # could also diff and update here later on

    async def _do_generate_task_evaluation(self, task: language.Task, revmap: RevisionMap):
        # run evaluation plan generator
        # TODO @UX: configure n_samples properly
        plan = await plan_evaluate_task(task=task, n_samples=2, eval_model=self.instruct_model)

        # upsert generated dataset, generated mappings and evaluation plan
        gen_dataset = plan.datasets[0]
        existing_gen_dataset = first(task.generated_expectations, None)
        if existing_gen_dataset is not None:
            gen_dataset.id = existing_gen_dataset.id
            order_key = existing_gen_dataset.order_key
        else:
            order_key = INTEGER_MINUS_ONE

        # map all task dependencies into generated mapping
        task_tree = tracked_tree_from_symbol(task)
        task_dependencies: list[GeneratedMapping] = []
        for node in task_tree:
            dependency = GeneratedMapping(
                type=GeneratedMappingType(node.type.value),
                source_id=node.id,
                source_revision=revmap[node.id],
                target_id=None,
                target_revision=None,
            )
            task_dependencies.append(dependency)

        gen_dataset_statement = map_to_statement(task.source.file, gen_dataset, order_key)
        gen_dataset_statement.parent = task
        gen_dataset_statement.modifier = language.StatementModifier.LIKE
        await self.write_statements_immediate(
            [wire.rmap_statement(gen_dataset_statement)],
            overwrite=True,
            generated_mappings=[(task.id, task_dependencies)],
        )

        # upsert evaluation plan (this is all rather inefficient)
        model_task = await models.Statement.objects.aget(id=task.id)
        model_task.evaluation_plan = models.EvaluationPlan.objects.aget_or_create(id=plan.id)
        await model_task.evaluation_plan.datasets.aset([d.id for d in plan.datasets])
        await model_task.asave()

    def queue_build(
        self, scope: BuildScope, buildable_id: UUID, cancel_running: bool
    ) -> BuildJob | BuildErrorType:
        # get the builds to run
        if not self.interpreted or self.interp.has_user_errors:
            return BuildErrorType.NOT_READY
        if self.interp.committed:
            return BuildErrorType.COMMITTED
        buildable = self.interp.module_idx.symbol_by_id(buildable_id)
        if isinstance(buildable, language.Task):
            builds = get_builds_for(buildable, self.interp.module_idx)
            if scope == BuildScope.REACTIVE:
                builds = [b for b in builds if b.settings.reactive]
            if not builds:
                return BuildErrorType.INVALID_BUILDABLE
            if buildable.type.output.tag == TypeTag.NULL:
                return BuildErrorType.INVALID_BUILDABLE
        elif isinstance(buildable, language.Build):
            builds = [buildable]
        else:
            return BuildErrorType.INVALID_BUILDABLE
        
        tasks = list(chain.from_iterable(b.tasks for b in builds))
        # assemble evaluation plans by task
        for task in tasks:
            pass

        job = BuildJob(
            revmap=self.revmap,
            scope=scope,
            buildable_id=buildable_id,
            builds=builds,
            evals=evals,
            project_id=self.project_id,
            project_version_id=self.module_id,
            worker_id=self.worker_id,
            deployment_id=None,
        )
        if cancel_running:
            self._cancel_jobs_like(
                lambda j: isinstance(j, BuildJob) and j.buildable_id == buildable_id
            )
        self._queue_job(job)
        return job

    async def do_build(
        self, revmap: RevisionMap, builds: list[language.Build], evals: list[EvaluationPlan], job_id: UUID
    ) -> list[BuildResult]:
        # instruct model should be configurable maybe? but we'll likely use our own
        build_processes = [
            wrap_task(
                build(b, self.instruct_model, eval, DbBuildTracker(self, job_id)),
                f"build_{b.id}",
            )
            for b, eval in zip(builds, evals)
        ]
        build_results = await asyncio.gather(*build_processes, return_exceptions=False)
        previous_builds_files: set[UUID] = set()
        for b in builds:
            build_files = get_build_files_for(b, self.idx)
            previous_builds_files.update({file.id for file in build_files})

        # convert build results into writes with the revisions that were used
        generated_files = []
        generated_mappings = []
        for build_result in build_results:
            generated_file = build_result.to_file(self.idx.module)
            generated_files.append(wire.rmap_file(generated_file))
            mappings = [revmap.map_mapping(m) for m in build_result.generated_mappings]
            generated_mappings.append((build_result.build.id, mappings))

        #  :ImmediateModuleWrites
        await self.write_files_immediate(
            files=generated_files,
            generated_mappings=generated_mappings,
            delete_files=previous_builds_files,
            overwrite=True,
        )
        return build_results

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
                elif isinstance(job, BuildJob):
                    job.task = create_task(self.do_build(job.revmap, job.builds∂, job.id))
                    job.build_results = await job.task
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

    async def write_build_candidates(
        self, candidates: list[BuildCandidate], delete_others: bool, job_id: UUID
    ) -> None:
        if delete_others:
            # delete candidates associated with builds not in the list
            build_ids = {c.ctx.build.id for c in candidates}
            candidates_ids = {c.id for c in candidates}
            for build_candidate in models.BuildCandidate.objects.filter(
                build_id__in=build_ids
            ).exclude(id__in=candidates_ids):
                build_candidate.delete()

        model_candidates: list[models.BuildCandidate] = [
            models.BuildCandidate(
                id=candidate.id,
                project_id=self.project_id,
                project_version_id=self.module_id,
                job_id=job_id,
                status=candidate.status,
                name=candidate.name,
                build_id=candidate.ctx.build.id,
                order_key=candidate.order_key,
            )
            for candidate in candidates
        ]
        # upsert candidates (by id)
        await models.BuildCandidate.objects.abulk_create(
            model_candidates,
            update_conflicts=True,
            unique_fields=["id"],
            update_fields=[
                "updated_at",
                "status",
                "job_id",
                "evaluation_id",
                "file_id",
                "order_key",
            ],
        )

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


class DbBuildTracker(BuildTracker):
    def __init__(self, worker: "LanguageWorker", job_id: UUID):
        self.worker = worker
        self.job_id = job_id
        self.seen_evaluation_ids: set[int] = set()

    def candidates_planned(self, candidates: list[BuildCandidate]):
        write = self.worker.write_build_candidates(
            candidates, job_id=self.job_id, delete_others=False
        )
        create_wrapped_task(write)
        # TODO @Robustness: async candidates_planned creates race condition with evaluation save

    def candidates_built(self, candidates: list[BuildCandidate]):
        write = self.worker.write_build_candidates(
            candidates, job_id=self.job_id, delete_others=False
        )
        create_wrapped_task(write)

    async def _write_evaluated(self, candidates: list[BuildCandidate]):
        # candidates are only evaluated once, but other candidates may be updated depending
        # on another candidates' evaluation (e.g. to update it from won to abandoned)
        # this is a bit hacky but the flow will change soon enough
        new_evaluations = [
            candidate.evaluation
            for candidate in candidates
            if id(candidate.evaluation) not in self.seen_evaluation_ids
        ]
        self.seen_evaluation_ids.update(id(evaluation) for evaluation in new_evaluations)
        if new_evaluations:
            await self.worker.write_evaluation_results(new_evaluations, job_id=self.job_id)
        await self.worker.write_build_candidates(
            candidates, job_id=self.job_id, delete_others=False
        )

    def candidates_evaluated(self, candidates: list[BuildCandidate]):
        create_wrapped_task(self._write_evaluated(candidates))


def save_execution_frames(frames: list[ExecutionFrameData]) -> bool:
    model_executions: list[Execution] = []
    for frame in frames:
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
