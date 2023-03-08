from __future__ import annotations

import asyncio
import enum
from dataclasses import dataclass, field
from datetime import datetime
from itertools import chain
from typing import Any, ClassVar, NamedTuple, Optional, cast
from uuid import UUID

import pytz
import structlog
import zmq
import zmq.asyncio

from bench import language
from bench.language import wire
from bench.language.parse import ErrorCollector, interp, resolve
from bench.language.type import SYMBOL_CLASS_BY_TYPE, Build, LiteralValue, StatementPath, SymbolType
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType, ModuleReference
from bench.msg import (
    ZMessage,
    ZMessageType,
    recv_message_poll,
    recv_message_with,
    send_message,
    zmq_ctx,
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
    RepWriteModulePayload,
    ReqModuleBuildPayload,
    ReqModuleRunPayload,
    ReqModuleRuntimePayload,
    ReqReadModulePayload,
    ReqWriteModulePayload,
    as_key,
)
from bench.runtime.build import BuildResult, make_build
from bench.runtime.execute import Proxy, RunError, instantiate, run
from bench.runtime.reactivity import RevisionMap, diff_trees, tree_from_mappings, tree_from_module
from bench.runtime.tracing import ExecutionTracer, MultiTracer, ValidationTracer
from bench.runtime.type import CodeInstance, ExecutionFrame, ExecutionFrameData, TaskInstance
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


class JobType(enum.StrEnum):
    INTERP = "interp"
    GENERATE = "generate"
    BUILD = "build"
    EVALUATE = "evaluate"
    LINT = "lint"
    RUN = "run"


# lower is higher
JOB_PRIORITY = {
    JobType.INTERP: 0,
    JobType.RUN: 0,
    JobType.GENERATE: 1,
    JobType.BUILD: 2,
    JobType.EVALUATE: 3,
    JobType.LINT: 4,
}


class JobStatus(enum.StrEnum):
    QUEUED = "queued"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"


@dataclass(repr=False)
class Job:
    type: ClassVar[JobType]
    id: UUID = field(default_factory=UUIDT)
    status: JobStatus = JobStatus.QUEUED
    started_at: Optional[datetime] = None
    terminated_at: Optional[datetime] = None
    terminated: asyncio.Event = field(default_factory=asyncio.Event)

    def __str__(self):
        return f"{self.type} {self.id} ({self.status})"

    def __repr__(self):
        return f"<Job {self}>"

    def __lt__(self, other):
        return self.default_priority < other.default_priority

    @property
    def success(self) -> bool:
        raise NotImplementedError

    @property
    def default_priority(self) -> int:
        return JOB_PRIORITY[self.type]


@dataclass(repr=False)
class InterpJob(Job):
    type: ClassVar[JobType] = JobType.INTERP
    new_source: wire.ModuleData = None
    success: bool = False


@dataclass(repr=False)
class BuildJob(Job):
    type: ClassVar[JobType] = JobType.BUILD
    buildable_id: UUID = None
    builds: list[Build] = None
    revmap: RevisionMap = None
    build_results: list[BuildResult] = None

    @property
    def success(self) -> bool:
        return self.build_results is not None


@dataclass(repr=False)
class RunJob(Job):
    type: ClassVar[JobType] = JobType.RUN
    runnable: TaskInstance | CodeInstance = None
    arguments: dict[str, LiteralValue] = None
    error: Optional[ModuleRunErrorType] = None
    error_details: Optional[Any] = None
    output: Optional[LiteralValue] = None
    tracing_level: ExecutionTracingLevel = required_field()
    deployment_id: UUID = required_field()
    trigger_type: str = required_field()
    trigger_id: Optional[UUID] = None

    @property
    def success(self) -> bool:
        return self.error is None


def rmap_job(job: Job) -> wire.JobData:
    data = wire.JobData(
        type=job.type,
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
    logger.info("interp_runtime", module=source)
    module = wire.wmap_module(source)
    # TODO @Language: revert explicit statement references to StatementPath to lookup refs properly?
    collector = ErrorCollector()
    module_idx = resolve(
        module, lookup_in_module=lookup_in_dependencies(dependencies), on_error=collector
    )
    interp(module_idx, on_error=collector)
    errors = [e.to_error() for e in collector.errors]

    return InterpModule(module_idx=module_idx, errors=errors, dependencies=dependencies)


def get_stale_symbols(revmap: RevisionMap, idx: language.ModuleIndex) -> list[language.Statement]:
    """Gets the stale generated or generator symbols in the given module"""

    # A generated/generator symbol is stale if
    #  1) one of its dependencies has changed
    #  2) one of its dependencies is affected by another change
    # These are because 1) checks for changes in known dependencies,
    # while 2) checks for new symbols that affect the dependencies.

    new_tree = tree_from_module(revmap, idx)

    stale_symbols = []
    for symbol in idx.symbols.values():
        if not symbol.is_generator:
            continue
        if isinstance(symbol, Build):
            source_mappings = symbol.source_mappings
        else:
            raise ValueError(f"unexpected generator symbol: {symbol}")

        # rebuild old tree for this generator
        old_tree = tree_from_mappings(source_mappings)
        diff_nodes = list(diff_trees(old_tree, new_tree))
        if not diff_nodes:
            # nothing relevant changed
            continue

        # mark generated statements as stale
        for source_mapping in source_mappings:
            if source_mapping.target_id is None:
                continue
            generated = idx.get_symbol_by_id(source_mapping.target_id)
            if generated is not None:  # ignore if no longer exists
                stale_symbols.append(generated.source)

    return stale_symbols


RECENT_JOBS_BUFFER_SIZE = 64  # won't be necessary with a proper job history in the DB


@dataclass
class ModuleWorker:
    """A worker that processes all jobs for a single module (incl. to maintain its state)"""

    def __init__(self, module_id: UUID, master: Worker):
        self.master = master
        self.module_id = module_id
        self.project_id: Optional[UUID] = None  # set in init (requires intserver fetch)
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
        self.recent_jobs: list[Job] = []
        self.run_jobs: asyncio.Queue[tuple[int, RunJob]] = asyncio.PriorityQueue()
        self.log = logger.bind(worker_id=self.master.worker_id, module_id=self.module_id)

    @property
    def interpreted(self) -> bool:
        return self.interp.module_idx is not None

    @property
    def idx(self) -> language.ModuleIndex:
        return self.interp.module_idx

    def _queue_job(self, job: Job, priority: int = None):
        priority = priority or job.default_priority
        if isinstance(job, RunJob):
            self.run_jobs.put_nowait((priority, job))
        else:
            self.stateful_jobs.put_nowait((priority, job))

        # track recent jobs in a buffer
        self.recent_jobs.append(job)
        if len(self.recent_jobs) > RECENT_JOBS_BUFFER_SIZE:
            self.recent_jobs.pop(0)

    def on_module_changed(self, source: wire.ModuleData) -> InterpJob:
        job = InterpJob(new_source=source)
        self._queue_job(job)
        return job

    async def _interp_requirement_rec(self, module_id: UUID) -> InterpModule:
        """Fetch and interpret the requirement module (incl. transitive deps)"""
        if module_id in self.interp_dependencies_cached:
            return self.interp_dependencies_cached[module_id]
        self.log.info("interp_requirement", module_id=module_id)
        source, _ = await self.master.get_module(module_id)
        requirements = get_requirements(source)
        dependencies = await asyncio.gather(
            *[self._interp_requirement_rec(req.id) for req in requirements]
        )
        interp = interp_module(source, [m.module_idx for m in dependencies])
        if interp.errors:
            # not good, but we can still try to use the module?
            self.log.warn("interp_requirement_failed", interp=interp)
        self.interp_dependencies_cached[module_id] = interp
        return interp

    async def do_interp(self, new_source: wire.ModuleData) -> None:
        """Interprets the new module source, fetching deps and firing reactivity jobs"""
        requirements = get_requirements(new_source)
        dependencies = await asyncio.gather(
            *[self._interp_requirement_rec(req.id) for req in requirements]
        )

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

        # reactively trigger build jobs for all affected builds (and other reactors)
        # TODO @Incomplete: implement reactive job trigger on stale symbol

    def queue_build(self, buildable_id: UUID) -> BuildJob | ModuleBuildErrorType:
        # get the builds to run
        if not self.interpreted:
            return ModuleBuildErrorType.NOT_READY
        buildable = self.interp.module_idx.symbol_by_id(buildable_id)
        if isinstance(buildable, language.Task):
            # collect any builds that reference this task
            builds = []
            for build in self.interp.module_idx.symbols_of_type(Build):
                if not build.is_definition:
                    continue
                if any(t.definition.id == buildable.id for t in build.tasks):
                    builds.append(build)
        elif isinstance(buildable, language.Build):
            builds = [buildable]
        else:
            return ModuleBuildErrorType.INVALID_BUILDABLE

        job = BuildJob(revmap=self.revmap, buildable_id=buildable_id, builds=builds)
        self._queue_job(job)
        return job

    async def do_build(self, revmap: RevisionMap, builds: list[language.Build]):
        self.log.info("build", builds=builds)
        build_processes = [wrap_task(make_build(build), f"build_{build.id}") for build in builds]
        build_results = await asyncio.gather(*build_processes, return_exceptions=False)
        return cast(list[BuildResult], build_results)

    def queue_run(
        self,
        *,
        runnable: str | UUID,
        runnable_type: str | None,
        build: str | UUID,
        arguments: dict[str, Any],
        tracing_level: ExecutionTracingLevel,
        deployment_id: UUID,
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
            self.log.exception("make_run_fail", exc_info=e)
            return ModuleRunErrorType.INVALID_RUNCONFIG

        # instantiate
        # root execution id is pre-set for tracking (run job gets the same id)
        root_id = UUIDT()
        try:
            # trace level filtering happens in this tracker
            tracker = pub_filtered_execution_tracker(
                root_id,
                self.master.pub_sock,
                project_id=self.project_id,
                tracing_level=tracing_level,
                deployment_id=deployment_id,
                trigger_type=trigger_type,
                trigger_id=trigger_id,
            )
            tracer = MultiTracer([ExecutionTracer(self.module_id, tracker), ValidationTracer()])
            runnable_instance = instantiate(
                runnable, idx=self.idx, build=build, proxy=Proxy(tracer=tracer)
            )
            if not isinstance(runnable_instance, (TaskInstance, CodeInstance)):
                raise TypeError(f"invalid runnable type: {type(runnable_instance)}")
        except Exception as e:
            self.log.exception("queue_run_fail_instantiate", exc_info=e)
            return ModuleRunErrorType.INVALID_RUNCONFIG

        job = RunJob(
            id=root_id,
            runnable=runnable_instance,
            arguments=arguments,
            tracing_level=tracing_level,
            deployment_id=deployment_id,
            trigger_type=trigger_type,
            trigger_id=trigger_id,
        )
        self._queue_job(job)
        return job

    async def do_run(self, runnable: CodeInstance, arguments: dict[str, LiteralValue]):
        try:
            if isinstance(runnable, TaskInstance):
                code_instance = runnable.code
            else:
                code_instance = runnable
            self.log.info("run", code_instance=code_instance)
            ret = await run(code_instance, arguments)
            return None, ret
        except RunError as e:
            self.log.exception("run_failed", exc_info=e)
            details = dict(type=e.type.name, symbol=str(e.symbol), message=str(e.cause))
            return ModuleRunErrorType.RUNTIME_ERROR, details
        except Exception as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            self.log.exception("run_failed", exc_info=e, sentry_enabled=sentry_enabled)
            return ModuleRunErrorType.INTERNAL_ERROR, None

    async def _run_queue(self, queue: asyncio.Queue[tuple[int, Job]]) -> None:
        """Process module jobs sequentially"""
        while True:
            _, job = await queue.get()
            try:
                job.status = JobStatus.RUNNING
                job.started_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                self.log.info("module_worker_job_started", job=job)
                self.master.notify_job_status(self, job)

                if isinstance(job, InterpJob):
                    await self.do_interp(job.new_source)
                    job.success = True

                elif isinstance(job, RunJob):
                    error, ret = await self.do_run(job.runnable, job.arguments)
                    if error is None:
                        job.output = ret
                    else:
                        job.error = error
                        job.error_details = ret

                elif isinstance(job, BuildJob):
                    build_results = await self.do_build(job.revmap, job.builds)
                    job.build_results = build_results

                else:
                    raise RuntimeError(f"unexpected job type: {job}")
                self.log.info("module_worker_job_completed", job=job)
            except Exception as e:
                sentry_enabled = sentry_capture_if_enabled(e)
                job.error = str(e)
                self.log.exception(
                    "module_worker_job_failed", job=job, sentry_enabled=sentry_enabled
                )
            finally:
                job.status = JobStatus.COMPLETED if job.success else JobStatus.FAILED
                job.terminated_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                job.terminated.set()
                self.master.notify_job_status(self, job)
                queue.task_done()

    async def run(self):
        """Runs the module worker main processing loop"""

        # first get the source
        self.log.info("module_worker_start")
        source, self.project_id = await self.master.get_module(self.module_id)
        interp_job = self.on_module_changed(source)

        # start running both queues (for stateful and run)
        main = asyncio.gather(
            wrap_task(self._run_queue(self.stateful_jobs), f"worker_run_stateful_{self.module_id}"),
            wrap_task(self._run_queue(self.run_jobs), f"worker_run_{self.module_id}"),
        )
        # wait for the first interp job to complete
        await interp_job.terminated.wait()
        if interp_job.status != JobStatus.COMPLETED:
            self.log.error("module_worker_init_failed", job=interp_job)
            raise RuntimeError(f"failed to initialize module worker: {interp_job}")

        self.ready.set()
        # wait for the main loop to complete
        await main  # (this will never return)


def make_full_change_payload(module_worker: ModuleWorker, cls):
    """Builds a complete runtime change message from the module worker's state"""
    relevant_jobs = [
        job
        for job in module_worker.recent_jobs
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
        module=module_worker.wire_module,
        dependencies=dependencies,
        errors=module_worker.wire_errors,
        jobs=[rmap_job(job) for job in relevant_jobs],
        stale_symbols=stale_symbols,
    )


class Worker:
    def __init__(self, worker_id: str | UUID):
        self.worker_id = worker_id
        self.rep_sock = zmq_ctx.socket(zmq.REP)
        self.intserver_req_sock = zmq_ctx.socket(zmq.REQ)
        self.sub_sock = zmq_ctx.socket(zmq.SUB)
        self.pub_sock = zmq_ctx.socket(zmq.PUB)

        self.module_workers: dict[UUID, ModuleWorker] = {}

    async def run(
        self,
        worker_rep_addr: str,
        worker_pub_addr: str,
        intserver_rep_addr: str,
        intserver_pub_addr: str,
    ):
        logger.info(
            "start",
            worker_id=self.worker_id,
            worker_rep_addr=worker_rep_addr,
            worker_pub_addr=worker_pub_addr,
            intserver_rep_addr=intserver_rep_addr,
            intserver_pub_addr=intserver_pub_addr,
        )
        self.rep_sock.bind(worker_rep_addr)
        self.intserver_req_sock.connect(intserver_rep_addr)
        self.sub_sock.connect(intserver_pub_addr)
        self.sub_sock.setsockopt(zmq.SUBSCRIBE, as_key(ZMessageType.MODULE_CHANGED))
        self.pub_sock.bind(worker_pub_addr)
        poller = zmq.asyncio.Poller()
        poller.register(self.rep_sock, zmq.POLLIN)
        poller.register(self.sub_sock, zmq.POLLIN)

        while True:
            async for msg in recv_message_poll(poller):
                try:
                    await self.process_message(msg)
                except Exception as e:
                    sentry_enabled = sentry_capture_if_enabled(e)
                    logger.exception(
                        "worker.process_message", exc_info=e, msg=msg, sentry_enabled=sentry_enabled
                    )

    def _get_module_worker(self, module_id: UUID) -> ModuleWorker:
        if module_id not in self.module_workers:
            # start module worker if not already started
            worker = ModuleWorker(module_id, self)
            self.module_workers[module_id] = worker
            asyncio.get_running_loop().create_task(
                wrap_task(worker.run(), "worker_run_" + str(module_id))
            )
        return self.module_workers[module_id]

    async def process_message(self, msg: ZMessage):
        logger.debug("process_message", msg=msg)
        if msg.type == ZMessageType.REQ_MODULE_RUNTIME:
            payload: ReqModuleRuntimePayload = msg.payload_as(ReqModuleRuntimePayload)
            module_worker = self._get_module_worker(payload.module_id)
            # shouldn't clog here :AsyncClientServer
            if not module_worker.ready.is_set():
                await module_worker.ready.wait()
            payload = make_full_change_payload(module_worker, RepModuleRuntimePayload)
            send_message(self.rep_sock, ZMessageType.REP_MODULE_RUNTIME, payload)

        elif msg.type == ZMessageType.MODULE_CHANGED:
            payload: ModuleChangedPayload = msg.payload_as(ModuleChangedPayload)
            module_worker = self._get_module_worker(payload.module_id)
            module_worker.on_module_changed(payload.module)
            # worker will trigger any follow-up messages (no reply necessary)

        elif msg.type == ZMessageType.REQ_MODULE_BUILD:
            payload: ReqModuleBuildPayload = msg.payload_as(ReqModuleBuildPayload)
            module_worker = self._get_module_worker(payload.module_id)
            build_job = module_worker.queue_build(payload.buildable_id)
            error = build_job if isinstance(build_job, ModuleBuildErrorType) else None
            send_message(self.rep_sock, ZMessageType.REP_MODULE_BUILD, RepModuleBuildPayload(error))

        elif msg.type == ZMessageType.REQ_MODULE_RUN:
            payload: ReqModuleRunPayload = msg.payload_as(ReqModuleRunPayload)
            module_worker = self._get_module_worker(payload.module_id)
            run_job = module_worker.queue_run(
                runnable=payload.runnable,
                runnable_type=payload.runnable_type,
                build=payload.build,
                arguments=payload.arguments,
                deployment_id=payload.deployment_id,
                tracing_level=payload.tracing_level,
                trigger_type=payload.trigger_type,
                trigger_id=payload.trigger_id,
            )
            if isinstance(run_job, ModuleRunErrorType):
                rep = RepModuleRunPayload(None, run_job, None, None)
            else:
                if payload.blocking:
                    # TODO @Cleanup: don't clog up running queue if blocking :AsyncClientServer
                    await run_job.terminated.wait()
                rep = RepModuleRunPayload(
                    run_job.id, run_job.error, run_job.error_details, run_job.output
                )
            send_message(self.rep_sock, ZMessageType.REP_MODULE_RUN, rep)

        else:
            raise RuntimeError(f"unexpected message type: {msg.type}")

    def notify_job_status(self, module_worker: ModuleWorker, job: Job):
        """Publishes the new job status (sends out module runtime updates)"""
        # publish job status
        change = make_full_change_payload(module_worker, ModuleRuntimeChangedPayload)
        send_message(self.pub_sock, ZMessageType.MODULE_RUNTIME_CHANGED, change)
        # write back build results to internal server
        if isinstance(job, BuildJob) and job.status == JobStatus.COMPLETED:
            asyncio.get_running_loop().create_task(
                wrap_task(self.write_build_job_results(module_worker, job))
            )

    # TODO @Cleanup: switch to async client/server ZMQ flow to avoid sequential locking
    # :AsyncClientServer
    _intserver_rep_lock = asyncio.Lock()

    async def write_build_job_results(self, module_worker: ModuleWorker, job: BuildJob):
        """Writes the build job results back to the internal server"""

        # convert build results into writes with the revisions that were used
        generated_files = []
        generated_mappings = []
        for build_result in job.build_results:
            generated_file = build_result.to_file(module_worker.idx.module)
            generated_files.append(wire.rmap_file(generated_file))
            mappings = [job.revmap.map_mapping(m) for m in build_result.source_mappings]
            generated_mappings.append((build_result.build.id, mappings))

        # actually write to the internal server
        await self._intserver_rep_lock.acquire()
        write = ReqWriteModulePayload(
            module_id=module_worker.module_id,
            files=generated_files,
            generated_mappings=generated_mappings,
        )
        send_message(self.intserver_req_sock, ZMessageType.REQ_WRITE_MODULE, write)
        self._intserver_rep_lock.release()
        _, write_result = await recv_message_with(self.intserver_req_sock, RepWriteModulePayload)
        if not write_result.success:
            # TODO @Robustness: panic if we can't write back builds?
            logger.error("write_module_failed", write=write, write_result=write_result)

    async def get_module(self, module_id: UUID) -> tuple[wire.ModuleData, UUID]:
        """Gets a modules wire data"""
        logger.debug("fetch_wire_module", module_id=module_id)
        await self._intserver_rep_lock.acquire()
        send_message(
            self.intserver_req_sock,
            ZMessageType.REQ_READ_MODULE,
            ReqReadModulePayload(module_id),
        )
        _, payload = await recv_message_with(self.intserver_req_sock, RepReadModulePayload)
        self._intserver_rep_lock.release()
        # TODO @Performance: cache committed modules
        return payload.module, payload.project_id

    async def stop(self):
        logger.info("stop", worker_id=self.worker_id)
        self.rep_sock.close()
        self.intserver_req_sock.close()
        self.sub_sock.close()
        self.pub_sock.close()


def pub_filtered_execution_tracker(
    root_id: UUID,
    pub_sock: zmq.Socket,
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
        # TODO @Cleanup: consider not tracking data at all when creating execution frame
        if not trace_data:
            frame_data.inputs = None
            frame_data.outputs = None

        logger.debug("execution.track", frame=frame_data.id)
        send_message(
            pub_sock,
            ZMessageType.EXECUTION_CHANGED,
            ExecutionChangedPayload(frame.module_id, frames=[frame_data]),
        )

    return _do_track
