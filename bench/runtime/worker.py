import asyncio
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import Any, Callable, ClassVar, Optional, cast
from uuid import UUID

import pytz
import structlog
from more_itertools import first

from bench import language
from bench.language import wire
from bench.language.parse import REFERENCE_REGEX, ErrorCollector, interp, resolve, sort
from bench.language.type import (
    SYMBOL_CLASS_BY_TYPE,
    Build,
    InterpSymbol,
    LiteralValue,
    StatementPath,
    SymbolType,
    Task,
    TypeTag,
)
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType, ModuleReference
from bench.msg import NMessage, NMessageType
from bench.msg.core import handle_reply, message_handler, nc_init, publish, request, subscribe
from bench.msg.messages import (
    InterpModuleChangedPayload,
    ModuleBuildErrorType,
    ModuleChangedPayload,
    ModuleRunErrorType,
    RepInterpModulePayload,
    RepModuleBuildPayload,
    RepModuleRunPayload,
    RepReadModulePayload,
    RepRegisterWorkerPayload,
    RepWriteBuildCandidatePayload,
    RepWriteBuildPayload,
    RepWriteEvaluationPayload,
    RepWriteJobPayload,
    RepWriteModulePayload,
    ReqInterpModulePayload,
    ReqModuleBuildPayload,
    ReqModuleRunPayload,
    ReqReadModulePayload,
    ReqRegisterWorkerPayload,
    ReqWriteBuildCandidatePayload,
    ReqWriteBuildPayload,
    ReqWriteEvaluationPayload,
    ReqWriteJobPayload,
    ReqWriteModulePayload,
    WorkerHeartbeatPayload,
)
from bench.runtime.build import (
    BuildCandidate,
    BuildResult,
    BuildTracker,
    build,
    get_build_files_for,
    get_builds_for,
)
from bench.runtime.evaluate import EvaluationResult, lint
from bench.runtime.map import map_to_file
from bench.runtime.reactivity import RevisionMap, get_stale_symbols
from bench.runtime.run import DEFAULT_TRACER, RunError, instantiate, run
from bench.runtime.tracing import PubTrackerContext, WorkerContext, pub_tracker_context, worker
from bench.runtime.type import (
    BuildCandidateData,
    CodeInstance,
    EvaluationResultData,
    Job,
    JobData,
    JobStatus,
    JobType,
    TaskInstance,
    WorkerType,
)
from bench.utils.func import debounce, wrap_task
from bench.utils.utils import get_from_env, required_field, sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT

WORKER_HEARTBEAT_INTERVAL = get_from_env("WORKER_HEARTBEAT_INTERVAL", 5, type_cast=int)

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


logger = structlog.get_logger(__name__)


@dataclass(repr=False, slots=True)
class InterpModule:
    module_idx: Optional[language.ModuleIndex]
    errors: list[language.Error]
    dependencies: list[language.ModuleIndex]

    @property
    def has_user_errors(self):
        """Whether any non-generated errors are present."""
        return any(
            error.statement is None or not error.statement.generated for error in self.errors
        )

    def symbol(self, path: str):
        if path.startswith("."):
            return self.module_idx.symbol(path)
        else:
            match = REFERENCE_REGEX.match(path)
            module_name = match.group("module_owner") + "." + match.group("module_name")
            dependency = next((d for d in self.dependencies if d.module.name == module_name), None)
            if dependency is None:
                raise LookupError(f"could not find dependency {module_name}")
            return dependency.symbol("." + match.group("path") + ":" + match.group("name"))


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
class GenerateJob(Job):
    type: ClassVar[JobType] = JobType.GENERATE
    generator: Optional[InterpSymbol] = None
    success: bool = False


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
class EvaluateJob(Job):
    type: ClassVar[JobType] = JobType.EVALUATE
    evaluation_result: Optional[EvaluationResult] = None

    @property
    def success(self) -> bool:
        return self.evaluation_result is not None


@dataclass(repr=False, slots=True)
class RunJob(Job):
    type: ClassVar[JobType] = JobType.RUN
    runnable: TaskInstance | CodeInstance = None
    arguments: dict[str, LiteralValue] = None
    error: Optional[ModuleRunErrorType] = None
    error_details: Optional[Any] = None
    output: Optional[LiteralValue] = None
    tracing_level: ExecutionTracingLevel = required_field()
    trigger_type: ExecutionTriggerType = required_field()
    trigger_id: Optional[UUID] = None
    ctx: Optional[PubTrackerContext] = None

    @property
    def success(self) -> bool:
        return self.error is None


def rmap_job(job: Job) -> JobData:
    data = JobData.from_job(
        job,
        project_id=job.project_id,
        project_version_id=job.project_version_id,
        deployment_id=job.deployment_id,
        worker_id=job.worker_id,
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


def diff_implicit_builds(interp: InterpModule) -> tuple[list[Build], list[Build]]:
    """
    Gets missing and extraneous implicit builds
    """

    builds = [symbol for symbol in interp.module_idx.symbols.values() if isinstance(symbol, Build)]
    implicit_builds = [build for build in builds if build.is_generated]
    explicit_builds = [build for build in builds if not build.is_generated]
    tasks = [symbol for symbol in interp.module_idx.symbols.values() if isinstance(symbol, Task)]

    tasks_with_implicit_builds_ids = set(
        chain.from_iterable(
            [(task.definition.id for task in build.tasks) for build in implicit_builds]
        )
    )
    tasks_with_explicit_builds_ids = set(
        chain.from_iterable(
            [(task.definition.id for task in build.tasks) for build in explicit_builds]
        )
    )

    # missing implicit builds for tasks without any builds
    default_model = interp.symbol("openai.std.text.gpt-3-5-turbo")
    missing_implicit_builds = []
    for task in tasks:
        if (
            task.definition.id in tasks_with_explicit_builds_ids
            or task.definition.id in tasks_with_implicit_builds_ids
        ):
            continue
        build = Build(
            name="auto_" + task.id.hex[:6],
            tasks=[task.to_ref()],
            models=[default_model.to_ref()],
            source_mappings=[],
        )
        missing_implicit_builds.append(build)

    # extraneous implicit builds if an explicit build exists or the task no longer exists
    extraneous_implicit_builds = []
    for build in implicit_builds:
        if not build.tasks or any(
            task.definition.id in tasks_with_explicit_builds_ids for task in build.tasks
        ):
            extraneous_implicit_builds.append(build)

    return missing_implicit_builds, extraneous_implicit_builds


class ModuleBuildTracker(BuildTracker):
    def __init__(self, worker: "ModuleWorker", build: Build, job_id: UUID):
        self.worker = worker
        self.build = build
        self.job_id = job_id
        self.seen_evaluation_ids: set[UUID] = set()

    def candidates_planned(self, candidates: list[BuildCandidate]):
        write = self.worker.master.write_build_candidates(
            self.worker, self.build.id, candidates, job_id=self.job_id
        )
        create_wrapped_task(write)

    def candidates_built(self, candidates: list[BuildCandidate]):
        write = self.worker.master.write_build_candidates(
            self.worker, self.build.id, candidates, job_id=self.job_id
        )
        create_wrapped_task(write)

    async def _write_evaluated(self, candidates: list[BuildCandidate]):
        # candidates are only evaluated once, but other candidates may be updated depending
        # on another candidates' evaluation (e.g. to update it from won to abandoned)
        evaluations = [
            candidate.evaluation
            for candidate in candidates
            if candidate.evaluation.id not in self.seen_evaluation_ids
        ]
        self.seen_evaluation_ids.update(evaluation.id for evaluation in evaluations)
        if evaluations:
            await self.worker.master.write_evaluations(self.worker, evaluations, job_id=self.job_id)
        await self.worker.master.write_build_candidates(
            self.worker, self.build.id, candidates, job_id=self.job_id
        )

    def candidates_evaluated(self, candidates: list[BuildCandidate]):
        create_wrapped_task(self._write_evaluated(candidates))


class ModuleWorker:
    """A worker that processes all jobs for a single module (incl. to maintain its state)"""

    def __init__(self, module_id: UUID, master: "Worker", deployment_id: UUID):
        self.master = master
        self.module_id = module_id
        self.project_id: Optional[UUID] = None  # set in init (requires intserver fetch)
        # TODO @Broken: track module worker deployment id, make Execution.deployment non-nullable
        self.deployment_id: UUID = deployment_id
        self.ctx: Optional[WorkerContext] = None
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
        # mark pending jobs cancelled in queue
        for (prio, job) in self.stateful_jobs._queue:
            if predicate(job):
                job.status = JobStatus.Cancelled

    def on_module_changed(self, source: wire.ModuleData) -> InterpJob:
        job = InterpJob(
            new_source=source,
            project_id=self.project_id,
            project_version_id=self.module_id,
            deployment_id=self.deployment_id,
            worker_id=self.master.worker_id,
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
        # reactively trigger (debounced) reactors
        create_wrapped_task(self._fire_reactive_generate())
        create_wrapped_task(self._fire_reactive_lint())
        create_wrapped_task(self._fire_reactive_build())
        # notify master
        await self.master.notify_module_changed(self)

    @debounce(GENERATE_DEBOUNCE, max_wait=GENERATE_DEBOUNCE_MAX_WAIT)
    async def _fire_reactive_generate(self) -> None:
        self.log.debug("module.react.generate")
        self.queue_generate(cancel_running=True)

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
            stale_builds = [
                symbol for symbol in self.stale_symbols if symbol.symbol_type == SymbolType.BUILD
            ]
            for b in stale_builds:
                self.queue_build(b.id, cancel_running=True)

    def queue_lint(self, cancel_running: bool) -> LintJob:
        job = LintJob(
            project_id=self.project_id,
            project_version_id=self.module_id,
            deployment_id=self.deployment_id,
            worker_id=self.master.worker_id,
        )
        if cancel_running:
            self._cancel_jobs_like(lambda j: isinstance(j, LintJob))
        self._queue_job(job)
        return job

    async def do_lint(self, job_id: UUID) -> EvaluationResult:
        evaluation = await lint(self.idx)
        await self.master.write_evaluations(self, [evaluation], job_id)
        return evaluation

    def queue_generate(self, cancel_running: bool) -> GenerateJob:
        job = GenerateJob(
            project_id=self.project_id,
            project_version_id=self.module_id,
            deployment_id=self.deployment_id,
            worker_id=self.master.worker_id,
        )
        if cancel_running:
            self._cancel_jobs_like(lambda j: isinstance(j, GenerateJob))
        self._queue_job(job)
        return job

    async def do_generate(self, generator: Optional[InterpSymbol]) -> bool:
        # generate (missing) implicit builds
        missing_builds, extraneous_builds = diff_implicit_builds(self.interp)
        if missing_builds or extraneous_builds:
            # TODO @Cleanup @Robustness: implicit build generation seems fragile (all in one file, overwrites)
            _path = "__implicit_builds__"
            implicit_build_file = first(
                (f for f in self.interp.module_idx.module.files if f.path == _path), None
            )
            if implicit_build_file is None:
                implicit_build_file = language.File(
                    path=_path,
                    generated=True,
                    module=self.interp.module_idx.module,
                )
            else:
                # filter out extraneous builds (and their children)
                # extraneous builds must already exist (have a source), so filtering by source is okay
                extraneous_symbols_ids = set()
                for b in extraneous_builds:
                    extraneous_symbols_ids.add(b.id)
                    for statement in self.idx.scopes[b.id].statements.values():
                        extraneous_symbols_ids.add(statement.id)
                implicit_build_file.statements = [
                    s for s in implicit_build_file.statements if s.id not in extraneous_symbols_ids
                ]
            # append missing builds
            implicit_build_file = map_to_file(
                missing_builds, weak_references=[], file=implicit_build_file
            )
            await self.master.write_module(self, files=[wire.rmap_file(implicit_build_file)])

        return True

    def queue_build(
        self, buildable_id: UUID, cancel_running: bool
    ) -> BuildJob | ModuleBuildErrorType:
        # get the builds to run
        if not self.interpreted or self.interp.has_user_errors:
            return ModuleBuildErrorType.NOT_READY
        buildable = self.interp.module_idx.symbol_by_id(buildable_id)
        if isinstance(buildable, language.Task):
            # collect any builds that reference this task
            builds = get_builds_for(buildable, self.interp.module_idx)
            if not builds:
                return ModuleBuildErrorType.INVALID_BUILDABLE
            if buildable.type.output.tag == TypeTag.NULL:
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
            worker_id=self.master.worker_id,
        )
        if cancel_running:
            self._cancel_jobs_like(
                lambda j: isinstance(j, BuildJob) and j.buildable_id == buildable_id
            )
        self._queue_job(job)
        return job

    async def do_build(self, revmap: RevisionMap, builds: list[language.Build], job_id: UUID):
        # instruct model should be configurable maybe? but we'll likely use our own
        instruct_model = self.interp.symbol("openai.std.text.gpt-3-5-turbo")
        build_processes = [
            wrap_task(
                build(b, instruct_model, tracker=ModuleBuildTracker(self, b, job_id)),
                f"build_{b.id}",
            )
            for b in builds
        ]
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
        try:
            # TODO @Performance: share/cache run instances across runs
            runnable_instance = instantiate(
                runnable,
                build=build,
                buildmap=lambda source: self.idx.get_symbol_by_id(build.get_target(source.id)),
            )
            if not isinstance(runnable_instance, (TaskInstance, CodeInstance)):
                raise TypeError(f"invalid runnable type: {type(runnable_instance)}")
        except Exception as e:
            self.log.exception("module.run.instantiate.failed", exc_info=e)
            return ModuleRunErrorType.INVALID_RUNCONFIG

        root_id = UUIDT()
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
            worker_id=self.master.worker_id,
            ctx=PubTrackerContext(
                tracing_level=tracing_level,
                trigger_type=trigger_type,
                trigger_id=trigger_id,
                root_id=root_id,
            ),
        )
        qpos = self._queue_job(job)
        # emit queued status immediately
        if isinstance(runnable_instance, TaskInstance):
            code_instance = runnable_instance.implementation
        else:
            code_instance = runnable_instance
        # notify tracer about queue enter
        # there are nicer ways to do this, but essentially we need access to the
        # current root tracer, which defaults to DEFAULT_TRACER
        run_ctx_token = pub_tracker_context.set(job.ctx)
        DEFAULT_TRACER.queue_enter(code_instance, arguments, qpos)
        pub_tracker_context.reset(run_ctx_token)
        return job

    async def do_run(
        self,
        runnable: CodeInstance,
        arguments: dict[str, LiteralValue],
        ctx: PubTrackerContext,
    ):
        run_ctx_token = pub_tracker_context.set(ctx)
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
        finally:
            pub_tracker_context.reset(run_ctx_token)

    async def _process_queue(self, queue: asyncio.Queue[tuple[int, Job]], track: bool) -> None:
        """Process module jobs sequentially"""
        while True:
            _, job = await queue.get()
            if job.status == JobStatus.Cancelled:
                continue

            create_task = asyncio.create_task
            job_context = PubTrackerContext(
                tracing_level=self.master.default_tracing_level,
                trigger_type=ExecutionTriggerType.JOB,
                trigger_id=job.id,
            )
            job_context_token = pub_tracker_context.set(job_context)
            try:
                job.status = JobStatus.Running
                job.started_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                self.log.debug("module.job.start", job=job)
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
                elif isinstance(job, GenerateJob):
                    job.task = create_task(self.do_generate(job.generator))
                    job.success = await job.task
                elif isinstance(job, RunJob):
                    job.task = create_task(self.do_run(job.runnable, job.arguments, job.ctx))
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
                pub_tracker_context.reset(job_context_token)
                queue.task_done()

    def provide_context(self):
        """Sets the worker context var."""
        if self.ctx is None:
            raise RuntimeError(f"worker context not set: {self}")
        worker.set(self.ctx)

    async def run(self):
        """Runs the module worker main processing loop"""

        # first get the source
        self.log.info("module.start")
        source, self.project_id = await self.master.get_module(self.module_id)
        interp_job = self.on_module_changed(source)

        # provide general worker context
        self.ctx = WorkerContext(
            deployment_id=self.deployment_id,
            worker_id=self.master.worker_id,
            module_id=self.module_id,
            project_id=self.project_id,
        )
        self.provide_context()

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
    include_module: bool = False,
    include_dependencies: bool = False,
):
    """Builds a complete runtime change message from the module worker's state"""
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
        stale_symbols=stale_symbols if include_module else None,
    )


def make_full_change_payload(module_worker: ModuleWorker, cls):
    return make_change_payload(module_worker, cls, include_module=True, include_dependencies=True)


class Worker:
    """A community worker or single deployment worker."""

    def __init__(self, worker_id: UUID, deployment_id: UUID | None, project_id: UUID | None):
        self.worker_id = worker_id
        self.deployment_id = deployment_id
        self.project_id = project_id
        self.type = WorkerType.COMMUNITY if deployment_id is None else WorkerType.DEDICATED
        self.module_workers: dict[UUID, ModuleWorker] = {}
        self.subs = []
        self.tasks = []
        self.cached_committed_modules: dict[UUID, tuple[wire.ModuleData, UUID]] = {}

    @property
    def default_tracing_level(self) -> ExecutionTracingLevel:
        return ExecutionTracingLevel.ALL_FRAMES_WITH_DATA

    async def run(self):
        await nc_init.wait()
        logger.info(
            "start", worker_id=self.worker_id, deployment_id=self.deployment_id, type=self.type
        )
        register_rep: NMessage[RepRegisterWorkerPayload] = await request(
            NMessageType.REQUEST_REGISTER_WORKER,
            ReqRegisterWorkerPayload(
                worker_id=self.worker_id,
                deployment_id=self.deployment_id,
                project_id=self.project_id,
                type=self.type,
            ),
            RepRegisterWorkerPayload,
        )
        if not register_rep.p.success:
            raise RuntimeError("failed to register worker")
        self.subs = [
            await subscribe(f"{NMessageType.MODULE_CHANGED}.*", cb=self.module_changed),
            await handle_reply(NMessageType.REQUEST_INTERP_MODULE, self.request_module_runtime),
            await handle_reply(NMessageType.REQUEST_MODULE_BUILD, self.request_module_build),
            await handle_reply(NMessageType.REQUEST_MODULE_RUN, self.request_module_run),
        ]
        self.tasks = [
            create_wrapped_task(self.send_heartbeats(interval_seconds=WORKER_HEARTBEAT_INTERVAL))
        ]

    async def run_forever(self):
        # run forever until cancelled
        try:
            asyncio.create_task(self.run())
            await asyncio.Event().wait()
        finally:
            await self.stop()

    async def send_heartbeats(self, interval_seconds: float):
        # of course, eventually this should be done on / synced with the k8s level
        while True:
            await publish(
                NMessageType.WORKER_HEARTBEAT, WorkerHeartbeatPayload(worker_id=self.worker_id)
            )
            await asyncio.sleep(interval_seconds)

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
        module_worker.provide_context()
        module_worker.on_module_changed(msg.p.module)
        # module worker will trigger any follow-ups

    @message_handler
    async def request_module_runtime(self, msg: NMessage[ReqInterpModulePayload]):
        module_worker = await self._get_ready_module_worker(msg.p.module_id)
        module_worker.provide_context()
        payload = make_full_change_payload(module_worker, RepInterpModulePayload)
        await msg.reply(payload)

    @message_handler
    async def request_module_build(self, msg: NMessage[ReqModuleBuildPayload]):
        module_worker = await self._get_ready_module_worker(msg.p.module_id)
        module_worker.provide_context()
        build_job = module_worker.queue_build(msg.p.buildable_id, cancel_running=True)
        error = build_job if isinstance(build_job, ModuleBuildErrorType) else None
        await msg.reply(RepModuleBuildPayload(error=error))

    @message_handler
    async def request_module_run(self, msg: NMessage[ReqModuleRunPayload]):
        module_worker = await self._get_ready_module_worker(msg.p.module_id)
        module_worker.provide_context()
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
        change = make_full_change_payload(module_worker, InterpModuleChangedPayload)
        await publish(NMessageType.INTERP_MODULE_CHANGED, change)

    async def write_evaluations(
        self, module_worker: ModuleWorker, evaluations: list[EvaluationResult], job_id: UUID
    ):
        """Writes evaluation results back to the internal server"""
        evaluations_data = []
        for evaluation in evaluations:
            evaluations_data.extend(
                EvaluationResultData.from_result(
                    evaluation,
                    project_id=module_worker.project_id,
                    project_version_id=module_worker.module_id,
                    job_id=job_id,
                )
            )
        write = ReqWriteEvaluationPayload(
            module_id=module_worker.module_id, evaluations=evaluations_data
        )
        rep = await request(NMessageType.REQUEST_WRITE_EVALUATION, write, RepWriteEvaluationPayload)
        if not rep.p.success:
            logger.error("evaluation.write.failed", evaluations=evaluations_data)

    async def write_build_candidates(
        self,
        module_worker: ModuleWorker,
        build_id: UUID,
        build_candidates: list[BuildCandidate],
        job_id: UUID,
    ):
        """Writes build candidates back to the internal server"""
        build_candidates_data = []
        for build_candidate in build_candidates:
            build_candidate_data = BuildCandidateData(
                id=build_candidate.id,
                build_id=build_id,
                status=build_candidate.status,
                name=build_candidate.name,
                evaluation_id=build_candidate.evaluation.id if build_candidate.evaluation else None,
                instruct_model_id=build_candidate.instruct_model.id
                if build_candidate.instruct_model
                else None,
                order_key=build_candidate.order_key,
                job_id=job_id,
                file_id=None,  # intermediate results are not written (yet)
                project_id=module_worker.project_id,
                project_version_id=module_worker.module_id,
            )
            build_candidates_data.append(build_candidate_data)
        write = ReqWriteBuildCandidatePayload(
            module_id=module_worker.module_id,
            build_id=build_id,
            build_candidates=build_candidates_data,
        )
        rep = await request(
            NMessageType.REQUEST_WRITE_BUILD_CANDIDATE, write, RepWriteBuildCandidatePayload
        )
        if not rep.p.success:
            logger.error(
                "build_candidate.write.failed", build_candidates_data=build_candidates_data
            )

    async def write_module(self, module_worker: ModuleWorker, files: list[wire.FileData]):
        """Writes module files back to the internal server"""
        # TODO @Performance @UX: module writes should immediately apply locally :ImmediateModuleWrites

        write = ReqWriteModulePayload(
            module_id=module_worker.module_id, generated_mappings=[], files=files
        )
        rep = await request(NMessageType.REQUEST_WRITE_MODULE, write, RepWriteModulePayload)
        if not rep.p.success:
            logger.error("module.write.failed", files=files)

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
        #  :ImmediateModuleWrites
        rep = await request(NMessageType.REQUEST_WRITE_BUILD, write, RepWriteBuildPayload)
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
        await asyncio.gather(task.cancel() for task in self.tasks)
        await asyncio.gather(sub.unsubscribe() for sub in self.subs)
