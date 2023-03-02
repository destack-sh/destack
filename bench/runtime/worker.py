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
from bench.language.wire import ModuleReference
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
from bench.runtime.tracing import ExecutionTracer, MultiTracer, ValidationTracer
from bench.runtime.type import CodeInstance, ExecutionFrame, ExecutionFrameData, TaskInstance
from bench.utils.func import wrap_task
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)

InterpModule = NamedTuple(
    "InterpModule",
    module_idx=Optional[language.ModuleIndex],
    errors=list[language.Error],
    dependencies=list[language.ModuleIndex],
)


class JobType(enum.Enum):
    RUN = "run"
    BUILD = "build"
    GENERATE = "generate"
    EVALUATE = "evaluate"


class JobStatus(enum.Enum):
    QUEUED = "queued"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"


@dataclass
class Job:
    type: ClassVar[JobType]
    id: UUID = field(default_factory=UUIDT)
    status: JobStatus = JobStatus.QUEUED
    started_at: Optional[datetime] = None
    terminated_at: Optional[datetime] = None
    terminated: asyncio.Event = field(default_factory=asyncio.Event)


@dataclass
class BuildJob(Job):
    type: ClassVar[JobType] = JobType.BUILD
    buildable_id: UUID = None
    builds: list[Build] = None
    build_results: list[BuildResult] = None


@dataclass
class RunJob(Job):
    type: ClassVar[JobType] = JobType.RUN
    runnable: TaskInstance | CodeInstance = None
    arguments: dict[str, LiteralValue] = None
    error: Optional[ModuleRunErrorType] = None
    error_details: Optional[Any] = None
    output: Optional[LiteralValue] = None


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
    elif isinstance(job, BuildJob):
        data.buildable_id = job.buildable_id
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
    # TODO @Accuracy: revert explicit statement references to StatementPath to lookup refs properly
    collector = ErrorCollector()
    module_idx = resolve(
        module, lookup_in_module=lookup_in_dependencies(dependencies), on_error=collector
    )
    interp(module_idx, on_error=collector)
    errors = [e.to_error() for e in collector.errors]

    return InterpModule(module_idx=module_idx, errors=errors, dependencies=dependencies)


RECENT_JOBS_BUFFER_SIZE = 64


@dataclass
class ModuleWorker:
    def __init__(self, module_id: UUID, master: Worker):
        self.master = master
        self.module_id = module_id
        self.ready = asyncio.Event()
        self.source: wire.ModuleData | None = None

        self.interp = InterpModule(module_idx=None, errors=[], dependencies=[])
        self.stateful_jobs: asyncio.Queue[Job] = field(default_factory=asyncio.Queue)
        self.recent_jobs: list[Job] = []
        self.run_jobs: asyncio.Queue[RunJob] = field(default_factory=asyncio.Queue)

        self.wire_module: wire.ModuleData | None = None
        self.wire_errors: list[wire.ErrorData] | None = None
        self.wire_dependencies: dict[UUID, wire.ModuleData] = {}

        self.log = logger.bind(worker_id=self.master.worker_id, module_id=self.module_id)

    @property
    def interpreted(self) -> bool:
        return self.interp.module_idx is not None

    @property
    def idx(self) -> language.ModuleIndex:
        return self.interp.module_idx

    def _derive_wire(self):
        """Re-derives wire state from interpreted state"""
        if self.interp.module_idx:
            self.wire_module = wire.rmap_module(self.interp.module_idx.module)
        else:  # re-use source (if failed to parse or not yet parsed)
            self.wire_module = self.source
        self.wire_errors = [wire.rmap_error(e) for e in self.interp.errors]
        self.wire_dependencies = {
            m.module.id: wire.rmap_module(m.module) for m in self.interp.dependencies
        }

    def _on_new_job(self, job: Job) -> None:
        self.recent_jobs.append(job)
        if len(self.recent_jobs) > RECENT_JOBS_BUFFER_SIZE:
            self.recent_jobs.pop(0)

    def on_module_changed(self, source: wire.ModuleData):
        self.source = source
        # re-fetch dependencies as needed (recursively)
        requirements = get_requirements(source)

        if self.stateful_jobs.qsize() > 0:
            # not sure what to do here?
            pass

        raise NotImplementedError

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

        job = BuildJob(buildable_id=buildable_id, builds=builds)
        self.stateful_jobs.put_nowait(job)
        self.on_new_job(job)
        return job

    async def do_build(self, builds: list[language.Build]):
        build_processes = [make_build(build) for build in builds]
        build_results = await asyncio.gather(*build_processes, return_exceptions=False)
        return cast(list[BuildResult], build_results)

    def queue_run(
        self, runnable: str | UUID, runnable_type: str | None, build: str | UUID
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
            logger.exception("run_fail", exc_info=e)
            return ModuleRunErrorType.INVALID_RUNCONFIG

        # instantiate
        # root execution id is pre-set for tracking (run job gets the same id)
        root_id = UUIDT()
        try:
            tracker = forward_execution_capture(root_id, self.master.pub_sock)
            tracer = MultiTracer([ExecutionTracer(self.module_id, tracker), ValidationTracer()])
            runnable_instance = instantiate(
                runnable, idx=self.idx, build=build, proxy=Proxy(tracer=tracer)
            )
            if not isinstance(runnable_instance, (TaskInstance, CodeInstance)):
                raise TypeError(f"invalid runnable type: {type(runnable_instance)}")
        except Exception as e:
            logger.exception("queue_run_fail_instantiate", exc_info=e)
            return ModuleRunErrorType.INVALID_RUNCONFIG

        job = RunJob(id=root_id, runnable=runnable_instance)
        self.run_jobs.put_nowait(job)
        self._on_new_job(job)
        return job

    async def do_run(self, runnable: CodeInstance, arguments: dict[str, LiteralValue]):
        try:
            if isinstance(runnable, TaskInstance):
                code_instance = runnable.code
            else:
                code_instance = runnable
            logger.info("run", code_instance=code_instance)
            ret = await run(code_instance, arguments)
            return None, ret
        except RunError as e:
            logger.exception("run_failed", exc_info=e)
            details = dict(type=e.type.name, symbol=str(e.symbol), message=str(e.cause))
            return ModuleRunErrorType.RUNTIME_ERROR, details
        except Exception as e:
            logger.exception("run_failed", exc_info=e)
            return ModuleRunErrorType.INTERNAL_ERROR, None

    async def _run_queue(self, queue: asyncio.Queue[Job]) -> None:
        """Process module jobs sequentially"""
        while True:
            job = await queue.get()
            try:
                job.status = JobStatus.RUNNING
                job.started_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                self.log.info("module_worker_job_start", job=job)
                self.master.notify_job_status(self, job)

                if isinstance(job, RunJob):
                    error, ret = await self.do_run(job.runnable, job.arguments)
                    if error is None:
                        job.output = ret
                    else:
                        job.error = error
                        job.error_details = ret

                elif isinstance(job, BuildJob):
                    build_results = await self.do_build(job.builds)
                    job.build_results = build_results

                else:
                    raise RuntimeError(f"unexpected job type: {job}")
                self.log.info("module_worker_job_completed", job=job)
            except Exception as e:
                job.error = str(e)
                self.log.exception("module_worker_job_failed", job=job)
            finally:
                job.status = JobStatus.COMPLETED if job.error is None else JobStatus.FAILED
                job.terminated_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                job.terminated.set()
                self.master.notify_job_status(self, job)
                queue.task_done()

    async def run(self):
        """Runs the module worker main processing loop"""

        # first get the source
        self.log.info("module_worker_start")
        source = await self.master.get_module(self.module_id)
        self.on_module_changed(source)
        self.ready.set()

        # start running both queues (for stateful and run)
        await asyncio.gather(self._run_queue(self.stateful_jobs), self._run_queue(self.run_jobs))
        # (this will never return)


def module_worker_to_rep(module_worker: ModuleWorker) -> ModuleRuntimeChangedPayload:
    # don't need to report run jobs since they're only for internal bookkeeping
    relevant_jobs = [job for job in module_worker.recent_jobs if job.type != JobType.RUN]
    return ModuleRuntimeChangedPayload(
        module_id=module_worker.module_id,
        updated_at=datetime.utcnow().replace(tzinfo=pytz.utc),
        module=module_worker.wire_module,
        dependencies=list(module_worker.wire_dependencies.values()),
        errors=module_worker.wire_errors,
        jobs=[rmap_job(job) for job in relevant_jobs],
    )


class Worker:
    def __init__(self, worker_id: str | UUID):
        self.worker_id = worker_id
        self.rep_sock = zmq_ctx.socket(zmq.REP)
        self.intserver_req_sock = zmq_ctx.socket(zmq.REQ)
        self.sub_sock = zmq_ctx.socket(zmq.SUB)
        self.pub_sock = zmq_ctx.socket(zmq.PUB)

        self.module_workers: dict[UUID, ModuleWorker] = {}
        self.interp_module_cache: dict[UUID, InterpModule] = {}

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
            msg = await recv_message_poll(poller)
            try:
                await self.process_message(msg)
            except Exception as e:
                logger.exception("worker.process_message", exc_info=e, msg=msg)

    def _get_module_worker(self, module_id: UUID) -> ModuleWorker:
        if module_id not in self.module_workers:
            worker = ModuleWorker(module_id, self)
            self.module_workers[module_id] = worker
            # init worker
            asyncio.get_running_loop().create_task(
                wrap_task(worker.run(), "worker_" + str(module_id))
            )
        return self.module_workers[module_id]

    async def process_message(self, msg: ZMessage):
        logger.debug("process_message", msg=msg)
        if msg.type == ZMessageType.REQ_MODULE_RUNTIME:
            payload: ReqModuleRuntimePayload = msg.payload_as(ReqModuleRuntimePayload)
            module_worker = self._get_module_worker(payload.module_id)
            await module_worker.ready.wait()
            send_message(
                self.rep_sock, ZMessageType.REP_MODULE_RUNTIME, module_worker_to_rep(module_worker)
            )

        elif msg.type == ZMessageType.MODULE_CHANGED:
            payload: ModuleChangedPayload = msg.payload_as(ModuleChangedPayload)
            module_worker = self._get_module_worker(payload.module_id)
            module_worker.on_module_changed(payload.module)
            # worker will trigger any follow-up messages

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
                payload.runnable, payload.runnable_type, payload.build
            )
            if payload.blocking:
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
        send_message(
            self.pub_sock,
            ZMessageType.MODULE_RUNTIME_CHANGED,
            module_worker_to_rep(module_worker),
        )

        # write back build results to internal server
        if isinstance(job, BuildJob) and job.status == JobStatus.COMPLETED:
            for build_result in job.build_results:
                generated_file = build_result.to_file(module=module_worker.idx.module)
                write = ReqWriteModulePayload(
                    module_id=module_worker.module_id,
                    files=[wire.rmap_file(generated_file)],
                    generated_mappings=[(build_result.build.id, build_result.source_mappings)],
                )
                send_message(self.intserver_req_sock, ZMessageType.REQ_WRITE_MODULE, write)
                _, write_result = await recv_message_with(
                    self.intserver_req_sock, RepWriteModulePayload
                )
                if not write_result.success:
                    # TODO @Robustness: panic if we can't write back builds?
                    logger.error("write_module_failed", write=write, write_result=write_result)

    async def get_module(self, module_id: UUID) -> wire.ModuleData:
        """Gets a modules wire data"""
        logger.debug("fetch_wire_module", module_id=module_id)
        send_message(
            self.intserver_req_sock,
            ZMessageType.REQ_READ_MODULE,
            ReqReadModulePayload(module_id),
        )
        _, payload = await recv_message_with(self.intserver_req_sock, RepReadModulePayload)
        return payload.module

    async def stop(self):
        logger.info("stop", worker_id=self.worker_id)
        self.rep_sock.close()
        self.intserver_req_sock.close()
        self.sub_sock.close()
        self.pub_sock.close()


def forward_execution_capture(root_id: UUID, pub_sock: zmq.Socket):
    def _do_track(frame: ExecutionFrame):
        # TODO @Performance: batch execution frame updates
        if frame.root is None:
            frame.id = root_id  # set root to fixed id (in-place)
        frame_data = ExecutionFrameData.from_frame(frame)
        logger.debug("execution.track", frame=frame_data.id)
        send_message(
            pub_sock,
            ZMessageType.EXECUTION_CHANGED,
            ExecutionChangedPayload(frame.module_id, frames=[frame_data]),
        )

    return _do_track
