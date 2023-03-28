import asyncio
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import Any, Callable, ClassVar, NamedTuple, Optional, cast
from uuid import UUID

import pytz
import structlog

from bench import language
from bench.language import wire
from bench.language.parse import ErrorCollector, interp, resolve, sort
from bench.language.type import SYMBOL_CLASS_BY_TYPE, Build, LiteralValue, StatementPath, SymbolType
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType, ModuleReference
from bench.msg import NMessage, NMessageType
from bench.msg.core import (
    handle_reply,
    message_handler,
    nc_init,
    publish,
    publish_soon,
    request,
    subscribe,
)
from bench.msg.messages import (
    ExecutionChangedPayload,
    ModuleBuildErrorType,
    ModuleChangedPayload,
    ModuleRunErrorType,
    ModuleRuntimeChangedPayload,
    RepModuleBuildPayload,
    RepModuleRunPayload,
    RepModuleRuntimePayload,
    RepReadModulePayload,
    RepWriteEvaluationPayload,
    RepWriteJobPayload,
    RepWriteModulePayload,
    ReqModuleBuildPayload,
    ReqModuleRunPayload,
    ReqModuleRuntimePayload,
    ReqReadModulePayload,
    ReqWriteBuildPayload,
    ReqWriteEvaluationPayload,
    ReqWriteJobPayload,
)
from bench.runtime.build import (
    BuildCandidate,
    BuildResult,
    build,
    get_build_files_for,
    get_builds_for,
)
from bench.runtime.evaluate import EvaluationResult, lint
from bench.runtime.reactivity import RevisionMap, get_stale_symbols
from bench.runtime.run import Proxy, RunError, instantiate, run
from bench.runtime.tracing import ExecutionTracer, MultiTracer, ValidationTracer
from bench.runtime.type import (
    CodeInstance,
    EvaluationResultData,
    ExecutionFrame,
    ExecutionFrameData,
    Job,
    JobData,
    JobStatus,
    JobType,
    TaskInstance,
)
from bench.utils.func import wrap_task
from bench.utils.utils import required_field, sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)

InterpModule = NamedTuple(
    "InterpModule",
    module_idx=Optional[language.ModuleIndex],
    errors=list[language.Error],
    dependencies=list[language.ModuleIndex],
)


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


async def cancel_job(job: Job):
    job.status = JobStatus.Cancelling
    job.task.cancel()
    try:
        await job.task
    finally:
        job.status = JobStatus.Cancelled


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
class BuildJob(Job):
    type: ClassVar[JobType] = JobType.BUILD
    buildable_id: UUID = None
    builds: list[Build] = None
    revmap: RevisionMap = None
    build_results: list[BuildResult] = None

    @property
    def success(self) -> bool:
        return self.build_results is not None


@dataclass(repr=False, slots=True)
class RunJob(Job):
    type: ClassVar[JobType] = JobType.RUN
    runnable: TaskInstance | CodeInstance = None
    arguments: dict[str, LiteralValue] = None
    error: Optional[ModuleRunErrorType] = None
    error_details: Optional[Any] = None
    output: Optional[LiteralValue] = None
    tracing_level: ExecutionTracingLevel = required_field()
    trigger_type: str = required_field()
    trigger_id: Optional[UUID] = None

    @property
    def success(self) -> bool:
        return self.error is None


def rmap_job(job: Job) -> JobData:
    data = JobData(
        type=job.type,
        project_id=job.project_id,
        project_version_id=job.project_version_id,
        deployment_id=job.deployment_id,
        id=job.id,
        status=job.status,
        started_at=job.started_at,
        terminated_at=job.terminated_at,
    )
    if isinstance(job, RunJob):
        data.error = job.error
        data.statement_id = job.runnable.id
    elif isinstance(job, BuildJob):
        data.statement_id = job.buildable_id
    return data


def get_requirements(source: wire.ModuleData) -> set[ModuleReference]:
    """Returns the set of module ids required by the given module source (not transitive)"""
    requirements_ids: set[ModuleReference] = set()
    for statement in chain.from_iterable(file.statements for file in source.files):
        if statement.symbol_type == SymbolType.REQUIREMENT:
            if not isinstance(statement.reference_module.id, UUID):
                raise ValueError(f"requirement must specify reference module id: {statement}")
            requirements_ids.add(statement.reference_module)
    return requirements_ids


def lookup_in_dependencies(dependencies: list[language.ModuleIndex]):
    # assumes no conflicting names (checked in resolve)
    dependencies_by_name = {m.module.name: m for m in dependencies}

    def lookup(
        requirement: language.RequirementContent, path: StatementPath
    ) -> language.Scope | None:
        idx: language.ModuleIndex = dependencies_by_name.get(requirement.module_name)
        if not idx:
            return None
        return idx.get_scope(path)

    return lookup


def interp_module(
    source: wire.ModuleData, dependencies: list[language.ModuleIndex]
) -> InterpModule:
    """Interprets the given module source with the given dependencies"""
    logger.debug("module.interp", module=source)
    module = wire.wmap_module(source)
    # TODO @Language: revert explicit statement references to StatementPath to lookup refs properly?
    collector = ErrorCollector()
    sort(module)  # for nicer debugging and automatically sorted module index
    module_idx = resolve(
        module, lookup_in_module=lookup_in_dependencies(dependencies), on_error=collector
    )
    interp(module_idx, on_error=collector)
    errors = [e.to_error() for e in collector.errors]

    return InterpModule(module_idx=module_idx, errors=errors, dependencies=dependencies)


class ModuleWorker:
    """A worker that processes all jobs for a single module (incl. to maintain its state)"""

    def __init__(self, module_id: UUID, master: "Worker", deployment_id: UUID):
        self.master = master
        self.module_id = module_id
        self.project_id: Optional[UUID] = None  # set in init (requires intserver fetch)
        # TODO @Broken: track module worker deployment id, make Execution.deployment non-nullable
        self.deployment_id: UUID = deployment_id
        self.ready = asyncio.Event()
        self.interp_dependencies_cached: dict[UUID, InterpModule] = {}

        # module-specific state that must be synchronized
        self.source: wire.ModuleData | None = None
        self.interp = InterpModule(module_idx=None, errors=[], dependencies=[])
        self.revmap: RevisionMap | None = None
        self.stale_symbols: list[language.Statement] | None = None
        self.wire_module: wire.ModuleData | None = None
        self.wire_errors: list[wire.ErrorData] | None = None
        self.wire_dependencies: dict[UUID, wire.ModuleData] | None = None

        self.stateful_jobs: asyncio.Queue[tuple[int, Job]] = asyncio.PriorityQueue()
        self.running_jobs: dict[UUID, Job] = {}
        self.run_jobs: asyncio.Queue[tuple[int, RunJob]] = asyncio.PriorityQueue()
        self.log = logger.bind(worker_id=self.master.worker_id, module_id=self.module_id)

    @property
    def interpreted(self) -> bool:
        return self.interp.module_idx is not None

    @property
    def idx(self) -> language.ModuleIndex:
        return self.interp.module_idx

    def _queue_job(self, job: Job, priority: int = None) -> int:
        priority = priority or job.default_priority
        if isinstance(job, RunJob):
            self.run_jobs.put_nowait((priority, job))
            qpos = self.run_jobs.qsize()
        else:
            self.stateful_jobs.put_nowait((priority, job))
            qpos = self.stateful_jobs.qsize()
        return qpos

    def _cancel_jobs_like(self, predicate: Callable[[Job], bool]):
        for job in self.running_jobs.values():
            if predicate(job):
                asyncio.create_task(cancel_job(job))

    def on_module_changed(self, source: wire.ModuleData) -> InterpJob:
        job = InterpJob(
            new_source=source,
            project_id=self.project_id,
            project_version_id=self.module_id,
            deployment_id=self.deployment_id,
        )
        self._queue_job(job)
        return job

    async def _interp_requirement_rec(self, module_id: UUID) -> InterpModule:
        """Fetch and interpret the requirement module (incl. transitive deps)"""
        if module_id in self.interp_dependencies_cached:
            return self.interp_dependencies_cached[module_id]
        self.log.debug("module.requirement.interp", module_id=module_id)
        source, _ = await self.master.get_module(module_id)
        requirements = get_requirements(source)
        dependencies = await asyncio.gather(
            *[self._interp_requirement_rec(req.id) for req in requirements]
        )
        interp = interp_module(source, [m.module_idx for m in dependencies])
        if interp.errors:
            # not good, but we can still try to use the module?
            self.log.warn("module.requirement.failed", interp=interp)
        self.interp_dependencies_cached[module_id] = interp
        return interp

    async def _interp_requirements(self, requirements: set[ModuleReference]) -> list[InterpModule]:
        # return immediately if all cached (saves context switching)
        all_cached = all(r.id in self.interp_dependencies_cached for r in requirements)
        if all_cached:
            return [self.interp_dependencies_cached[r.id] for r in requirements]
        dependencies = await asyncio.gather(
            *[self._interp_requirement_rec(r.id) for r in requirements], return_exceptions=False
        )
        return cast(list[InterpModule], dependencies)

    async def do_interp(self, new_source: wire.ModuleData) -> None:
        """Interprets the new module source, fetching deps and firing reactivity jobs"""
        requirements = get_requirements(new_source)
        dependencies = await self._interp_requirements(requirements)

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

        # reactively trigger reactors for new stale symbols
        self._fire_reactive_jobs(self.stale_symbols)
        # fire lint job
        self.queue_lint(cancel_running=True)

    def _fire_reactive_jobs(self, stale_symbols: list[language.Statement]) -> None:
        # if no errors, queue new builds for any stale builds
        if not self.interp.errors:
            stale_builds = [
                symbol for symbol in stale_symbols if symbol.symbol_type == SymbolType.BUILD
            ]
            for b in stale_builds:
                self.queue_build(b.id, cancel_running=True)

    def queue_lint(self, cancel_running: bool) -> LintJob:
        job = LintJob(
            project_id=self.project_id,
            project_version_id=self.module_id,
            deployment_id=self.deployment_id,
        )
        if cancel_running:
            self._cancel_jobs_like(lambda j: isinstance(j, LintJob))
        self._queue_job(job)
        return job

    async def do_lint(self, job_id: UUID) -> EvaluationResult:
        evaluation = await lint(self.idx)
        await self.master.write_evaluations(self, [evaluation], job_id)
        return evaluation

    def queue_build(
        self, buildable_id: UUID, cancel_running: bool
    ) -> BuildJob | ModuleBuildErrorType:
        # get the builds to run
        if not self.interpreted or self.interp.errors:
            return ModuleBuildErrorType.NOT_READY
        buildable = self.interp.module_idx.symbol_by_id(buildable_id)
        if isinstance(buildable, language.Task):
            # collect any builds that reference this task
            builds = get_builds_for(buildable, self.interp.module_idx)
            if not builds:
                return ModuleBuildErrorType.INVALID_BUILDABLE
        elif isinstance(buildable, language.Build):
            builds = [buildable]
        else:
            return ModuleBuildErrorType.INVALID_BUILDABLE

        job = BuildJob(
            revmap=self.revmap,
            buildable_id=buildable_id,
            builds=builds,
            project_id=self.project_id,
            project_version_id=self.module_id,
            deployment_id=self.deployment_id,
        )
        if cancel_running:
            self._cancel_jobs_like(
                lambda j: isinstance(j, BuildJob) and j.buildable_id == buildable_id
            )
        self._queue_job(job)
        return job

    async def do_build(self, revmap: RevisionMap, builds: list[language.Build], job_id: UUID):
        build_processes = [wrap_task(build(b), f"build_{b.id}") for b in builds]
        # TODO @Incomplete: track builds and write candidates & evaluations
        build_results = await asyncio.gather(*build_processes, return_exceptions=False)
        previous_builds_files: set[UUID] = set()
        for b in builds:
            build_files = get_build_files_for(b, self.idx)
            previous_builds_files.update({file.id for file in build_files})
        await self.master.write_build_results(
            self,
            build_ids=[b.id for b in builds],
            build_results=build_results,
            revmap=revmap,
            previous_build_files=list(previous_builds_files),
        )
        return build_results

    def queue_run(
        self,
        *,
        runnable: str | UUID,
        runnable_type: str | None,
        build: str | UUID,
        arguments: dict[str, Any],
        tracing_level: ExecutionTracingLevel,
        trigger_type: ExecutionTriggerType,
        trigger_id: Optional[UUID],
    ) -> RunJob | ModuleRunErrorType:
        if not self.interpreted:
            return ModuleRunErrorType.NOT_READY

        # get the runconfig
        try:
            build = self.idx.symbol(build, Build) if build else None
            if runnable_type:
                runnable_type = SYMBOL_CLASS_BY_TYPE[SymbolType(runnable_type)]
            else:
                runnable_type = None
            runnable = self.idx.symbol(runnable, symbol_t=runnable_type)
        except (TypeError, KeyError) as e:
            self.log.exception("module.run.failed", exc_info=e)
            return ModuleRunErrorType.INVALID_RUNCONFIG

        # instantiate
        # root execution id is pre-set for tracking (run job gets the same id)
        root_id = UUIDT()
        try:
            # TODO @Performance: instantiate runs once and use context vars for tracking :ReusableInstances
            # trace level filtering happens in this tracker
            tracker = pub_filtered_execution_tracker(
                root_id,
                project_id=self.project_id,
                tracing_level=tracing_level,
                deployment_id=self.deployment_id,
                trigger_type=trigger_type,
                trigger_id=trigger_id,
            )
            tracer = ExecutionTracer(self.module_id, tracker)
            runnable_instance = instantiate(
                runnable,
                build=build,
                buildmap=lambda source: self.idx.get_symbol_by_id(build.get_target(source.id)),
                proxy=Proxy(
                    tracer=MultiTracer([tracer, ValidationTracer()]),
                    cache_inferences=True,
                    inference_timeout=15,
                    inference_retries=2,
                ),
            )
            if not isinstance(runnable_instance, (TaskInstance, CodeInstance)):
                raise TypeError(f"invalid runnable type: {type(runnable_instance)}")
        except Exception as e:
            self.log.exception("module.run.instantiate.failed", exc_info=e)
            return ModuleRunErrorType.INVALID_RUNCONFIG

        job = RunJob(
            id=root_id,
            runnable=runnable_instance,
            arguments=arguments,
            tracing_level=tracing_level,
            trigger_type=trigger_type,
            trigger_id=trigger_id,
            project_id=self.project_id,
            project_version_id=self.module_id,
            deployment_id=self.deployment_id,
        )
        qpos = self._queue_job(job)
        # emit queued status immediately
        if isinstance(runnable_instance, TaskInstance):
            code_instance = runnable_instance.implementation
        else:
            code_instance = runnable_instance
        tracer.queue_enter(code_instance, arguments, qpos)
        return job

    async def do_run(self, runnable: CodeInstance, arguments: dict[str, LiteralValue]):
        try:
            if isinstance(runnable, TaskInstance):
                code_instance = runnable.implementation
            else:
                code_instance = runnable
            self.log.info("module.run", code_instance=code_instance)
            ret = await run(code_instance, arguments)
            return None, ret
        except RunError as e:
            self.log.exception("module.run.failed", exc_info=e)
            details = dict(type=e.type.name, symbol=str(e.symbol), message=str(e.cause))
            return ModuleRunErrorType.RUNTIME_ERROR, details
        except Exception as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            self.log.exception("module.run.failed", exc_info=e, sentry_enabled=sentry_enabled)
            return ModuleRunErrorType.INTERNAL_ERROR, None

    async def _process_queue(self, queue: asyncio.Queue[tuple[int, Job]], track: bool) -> None:
        """Process module jobs sequentially"""
        while True:
            _, job = await queue.get()
            create_task = asyncio.create_task
            try:
                job.status = JobStatus.Running
                job.started_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                self.log.info("module.job.start", job=job)
                if track:
                    self.running_jobs[job.id] = job
                    # must wait for job to be saved since we reference job ids
                    await self.master.notify_job_status(self, job)
                if isinstance(job, InterpJob):
                    job.task = create_task(self.do_interp(job.new_source))
                    await job.task
                    job.success = True
                elif isinstance(job, LintJob):
                    job.task = create_task(self.do_lint(job.id))
                    job.evaluation = await job.task
                elif isinstance(job, RunJob):
                    job.task = create_task(self.do_run(job.runnable, job.arguments))
                    error, ret = await job.task
                    if error is None:
                        job.output = ret
                    else:
                        job.error = error
                        job.error_details = ret
                elif isinstance(job, BuildJob):
                    job.task = create_task(self.do_build(job.revmap, job.builds, job.id))
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
                if track:
                    # no need to await this update since we don't need the result
                    create_wrapped_task(self.master.notify_job_status(self, job))
                    del self.running_jobs[job.id]
                queue.task_done()

    async def run(self):
        """Runs the module worker main processing loop"""

        # first get the source
        self.log.info("module.start")
        source, self.project_id = await self.master.get_module(self.module_id)
        interp_job = self.on_module_changed(source)

        # start running both queues (for stateful and run)
        main = asyncio.gather(
            wrap_task(
                self._process_queue(self.stateful_jobs, track=True),
                f"worker_run_stateful_{self.module_id}",
            ),
            wrap_task(
                self._process_queue(self.run_jobs, track=False), f"worker_run_{self.module_id}"
            ),
        )
        # wait for the initial interp job to complete
        await interp_job.terminated.wait()
        if interp_job.status != JobStatus.Completed:
            self.log.error("module_worker_init_failed", job=interp_job)
            raise RuntimeError(f"failed to initialize module worker: {interp_job}")

        self.ready.set()
        # wait for the main loop to complete
        await main  # (this will never return)


def make_change_payload(
    module_worker: ModuleWorker,
    cls,
    include_jobs: bool = False,
    include_module: bool = False,
    include_dependencies: bool = False,
):
    """Builds a complete runtime change message from the module worker's state"""
    relevant_jobs = [
        job
        for job in module_worker.running_jobs.values()
        if job.type in (JobType.INTERP, JobType.BUILD, JobType.GENERATE, JobType.EVALUATE)
    ]
    dependencies = (
        list(module_worker.wire_dependencies.values())
        if module_worker.wire_dependencies is not None
        else None
    )
    stale_symbols = (
        [s.id for s in module_worker.stale_symbols]
        if module_worker.stale_symbols is not None
        else None
    )
    return cls(
        module_id=module_worker.module_id,
        updated_at=datetime.utcnow().replace(tzinfo=pytz.utc),
        module=module_worker.wire_module if include_module else None,
        dependencies=dependencies if include_dependencies else None,
        errors=module_worker.wire_errors if include_module else None,
        jobs=[rmap_job(job) for job in relevant_jobs] if include_jobs else None,
        stale_symbols=stale_symbols if include_module else None,
    )


def make_full_change_payload(module_worker: ModuleWorker, cls):
    return make_change_payload(
        module_worker,
        cls,
        include_jobs=True,
        include_module=True,
        include_dependencies=True,
    )


class Worker:
    """A community worker or single deployment worker."""

    def __init__(self, worker_id: str | UUID, deployment_id: UUID | None):
        self.worker_id = worker_id
        self.deployment_id = deployment_id
        self.module_workers: dict[UUID, ModuleWorker] = {}
        self.subs = []
        self.cached_committed_modules: dict[UUID, tuple[wire.ModuleData, UUID]] = {}

    async def run(self):
        await nc_init.wait()
        logger.info("start", worker_id=self.worker_id)
        self.subs = [
            await subscribe(f"{NMessageType.MODULE_CHANGED}.*", cb=self.module_changed),
            await handle_reply(NMessageType.REQUEST_MODULE_RUNTIME, self.request_module_runtime),
            await handle_reply(NMessageType.REQUEST_MODULE_BUILD, self.request_module_build),
            await handle_reply(NMessageType.REQUEST_MODULE_RUN, self.request_module_run),
        ]

    async def run_forever(self):
        # run forever until cancelled
        try:
            asyncio.create_task(self.run())
            await asyncio.Event().wait()
        finally:
            await self.stop()

    def _get_module_worker(self, module_id: UUID) -> ModuleWorker:
        if module_id not in self.module_workers:
            # start module worker if not already started
            # TODO @Broken: assign workers to deployments
            worker = ModuleWorker(module_id, self, self.deployment_id)
            self.module_workers[module_id] = worker
            asyncio.create_task(wrap_task(worker.run(), "worker_run_" + str(module_id)))
        return self.module_workers[module_id]

    async def _get_ready_module_worker(self, module_id: UUID) -> ModuleWorker:
        module_worker = self._get_module_worker(module_id)
        if not module_worker.ready.is_set():
            await module_worker.ready.wait()
        return module_worker

    @message_handler
    async def module_changed(self, msg: NMessage[ModuleChangedPayload]):
        if msg.p.module_id not in self.module_workers:
            # ignore if we don't have a worker for this module
            return
        module_worker = await self._get_ready_module_worker(msg.p.module_id)
        module_worker.on_module_changed(msg.p.module)
        # module worker will trigger any follow-ups

    @message_handler
    async def request_module_runtime(self, msg: NMessage[ReqModuleRuntimePayload]):
        module_worker = await self._get_ready_module_worker(msg.p.module_id)
        payload = make_full_change_payload(module_worker, RepModuleRuntimePayload)
        await msg.reply(payload)

    @message_handler
    async def request_module_build(self, msg: NMessage[ReqModuleBuildPayload]):
        module_worker = await self._get_ready_module_worker(msg.p.module_id)
        build_job = module_worker.queue_build(msg.p.buildable_id, cancel_running=True)
        error = build_job if isinstance(build_job, ModuleBuildErrorType) else None
        await msg.reply(RepModuleBuildPayload(error=error))

    @message_handler
    async def request_module_run(self, msg: NMessage[ReqModuleRunPayload]):
        module_worker = await self._get_ready_module_worker(msg.p.module_id)
        run_job = module_worker.queue_run(
            runnable=msg.p.runnable,
            runnable_type=msg.p.runnable_type,
            build=msg.p.build,
            arguments=msg.p.arguments,
            tracing_level=msg.p.tracing_level,
            trigger_type=msg.p.trigger_type,
            trigger_id=msg.p.trigger_id,
        )
        if isinstance(run_job, ModuleRunErrorType):
            await msg.reply(RepModuleRunPayload(None, run_job, None, None))
        else:
            if msg.p.block:
                await run_job.terminated.wait()
            rep = RepModuleRunPayload(
                execution_id=run_job.id,
                error=run_job.error,
                error_details=run_job.error_details,
                output=run_job.output,
            )
            await msg.reply(rep)

    async def notify_job_status(self, module_worker: ModuleWorker, job: Job):
        """Publishes the new job status"""
        _ = await request(
            NMessageType.REQUEST_WRITE_JOB,
            ReqWriteJobPayload(module_worker.module_id, rmap_job(job)),
            RepWriteJobPayload,
        )

    async def notify_module_changed(self, module_worker: ModuleWorker):
        """Publishes the new module runtime"""
        change = make_full_change_payload(module_worker, ModuleRuntimeChangedPayload)
        await publish(NMessageType.MODULE_RUNTIME_CHANGED, change)

    async def write_evaluations(
        self, module_worker: ModuleWorker, evaluations: list[EvaluationResult], job_id: UUID
    ):
        """Writes evaluation results back to the internal server"""
        evaluations_data = []
        for evaluation in evaluations:
            evaluations_data += EvaluationResultData.from_result(
                evaluation,
                project_id=module_worker.project_id,
                project_version_id=module_worker.module_id,
                job_id=job_id,
            )
        write = ReqWriteEvaluationPayload(
            module_id=module_worker.module_id, evaluations=evaluations_data
        )
        rep = await request(NMessageType.REQUEST_WRITE_EVALUATION, write, RepWriteEvaluationPayload)
        if not rep.p.success:
            logger.error("evaluation.write.failed", evaluations_data=evaluations_data)

    async def write_build_candidates(
        self,
        module_worker: ModuleWorker,
        build_candidates: list[BuildCandidate],
        revmap: RevisionMap,
    ):
        """Writes build candidates back to the internal server"""
        raise NotImplementedError

    async def write_build_results(
        self,
        module_worker: ModuleWorker,
        build_ids: list[UUID],
        build_results: list[BuildResult],
        previous_build_files: list[UUID],
        revmap: RevisionMap,
    ):
        """Writes build results back to the internal server"""

        # convert build results into writes with the revisions that were used
        generated_files = []
        generated_mappings = []
        for build_result in build_results:
            generated_file = build_result.to_file(module_worker.idx.module)
            generated_files.append(wire.rmap_file(generated_file))
            mappings = [revmap.map_mapping(m) for m in build_result.source_mappings]
            generated_mappings.append((build_result.build.id, mappings))

        # actually write to the internal server
        write = ReqWriteBuildPayload(
            module_id=module_worker.module_id,
            build_ids=build_ids,
            files=generated_files,
            generated_mappings=generated_mappings,
            delete_files=previous_build_files,
        )
        rep = await request(NMessageType.REQUEST_WRITE_BUILD, write, RepWriteModulePayload)
        if not rep.p.success:
            # TODO @Robustness: panic if we can't write back builds?
            logger.error("module.write.failed", write=write, write_result=rep)

    async def get_module(self, module_id: UUID) -> tuple[wire.ModuleData, UUID]:
        """Gets a modules wire data"""
        log = logger.bind(module_id=module_id)
        cached = self.cached_committed_modules.get(module_id)
        if cached is not None:
            log.debug("module.fetch", cached=True)
            return cached
        module_rep = await request(
            NMessageType.REQUEST_READ_MODULE, ReqReadModulePayload(module_id), RepReadModulePayload
        )
        if module_rep.p.module.committed:
            self.cached_committed_modules[module_id] = module_rep.p.module, module_rep.p.project_id
        log.debug("module.fetch", cached=False)
        return module_rep.p.module, module_rep.p.project_id

    async def stop(self):
        logger.info("stop", worker_id=self.worker_id)
        await asyncio.gather(sub.unsubscribe() for sub in self.subs)


def pub_filtered_execution_tracker(
    root_id: UUID,
    *,
    project_id: UUID,
    tracing_level: ExecutionTracingLevel,
    deployment_id: UUID,
    trigger_type: ExecutionTriggerType,
    trigger_id: UUID,
):
    trace_all_frames = tracing_level in (
        ExecutionTracingLevel.ALL_FRAMES,
        ExecutionTracingLevel.ALL_FRAMES_WITH_DATA,
    )
    trace_data = tracing_level in (
        ExecutionTracingLevel.ALL_FRAMES_WITH_DATA,
        ExecutionTracingLevel.ROOT_FRAME_WITH_DATA,
    )

    def _do_track(frame: ExecutionFrame):
        is_root = frame.root is None
        # filter according to trace level
        if not is_root and not trace_all_frames:
            return
        if is_root:
            frame.id = root_id  # set root to fixed id (in-place)

        frame_data = ExecutionFrameData.from_frame(
            frame,
            project_id=project_id,
            tracing_level=tracing_level,
            deployment_id=deployment_id,
            trigger_type=trigger_type,
            trigger_id=trigger_id,
        )

        # wipe data if not tracing it
        # TODO @Cleanup: consider not tracking untracked data at all when creating execution frame
        if not trace_data:
            frame_data.inputs = None
            frame_data.outputs = None

        logger.debug("execution.track", frame=frame_data.id)
        publish_soon(
            NMessageType.EXECUTION_CHANGED,
            ExecutionChangedPayload(frame.module_id, frames=[frame_data]),
        )

    return _do_track
