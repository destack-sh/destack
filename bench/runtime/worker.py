import asyncio
from dataclasses import dataclass, field
from typing import Any, Optional
from uuid import UUID

import structlog

from bench import language
from bench.language import wire
from bench.language.type import SYMBOL_CLASS_BY_TYPE, Build, LiteralValue, SymbolType
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.msg import NMessage, NMessageType
from bench.msg.core import handle_reply, message_handler, nc_init, publish, request, subscribe
from bench.msg.messages import (
    ModuleChangedPayload,
    ModuleRunErrorType,
    RepModuleRunPayload,
    RepReadModulePayload,
    RepRegisterWorkerPayload,
    ReqModuleRunPayload,
    ReqReadModulePayload,
    ReqRegisterWorkerPayload,
    WorkerHeartbeatPayload,
)
from bench.runtime.build import BuildCandidate
from bench.runtime.interp import InterpModule, LanguageInterpreter
from bench.runtime.run import DEFAULT_TRACER, RunError, instantiate, run
from bench.runtime.tracing import (
    ExecutionTrackerContext,
    WorkerContext,
    pub_tracker_ctx,
    worker_ctx,
)
from bench.runtime.type import CodeInstance, TaskInstance, WorkerType
from bench.utils.func import wrap_task
from bench.utils.utils import get_from_env, sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT

WORKER_HEARTBEAT_INTERVAL = get_from_env("WORKER_HEARTBEAT_INTERVAL", 5, type_cast=int)

logger = structlog.get_logger(__name__)


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


@dataclass(repr=False, slots=True)
class RunJob:
    priority: int = 1
    cancelled: bool = False
    runnable: TaskInstance | CodeInstance = None
    arguments: dict[str, LiteralValue] = None
    error: RunError | None = None
    error_details: dict[str, Any] = None
    output: LiteralValue | None = None
    ctx: Optional[ExecutionTrackerContext] = None
    terminated: asyncio.Event = field(default_factory=asyncio.Event)
    id: UUID = field(default_factory=UUIDT)


class ModuleWorker:
    """A worker that processes all jobs for a single module (incl. to maintain its state)"""

    def __init__(self, module_id: UUID, master: "SandboxedWorker", deployment_id: UUID):
        self.master = master
        self.module_id = module_id
        self.project_id: Optional[UUID] = None  # set in init (requires langserver fetch)
        # TODO @Broken: track module worker deployment id, make Execution.deployment non-nullable
        self.deployment_id: UUID = deployment_id
        self.ctx: Optional[WorkerContext] = None
        self.ready = asyncio.Event()

        self.interpreter = LanguageInterpreter(master.fetch)
        self.interp: InterpModule | None = None
        self.run_jobs: asyncio.Queue[tuple[int, RunJob]] = asyncio.PriorityQueue()
        self.log = logger.bind(worker_id=self.master.worker_id, module_id=self.module_id)

    @property
    def interpreted(self) -> bool:
        return self.interp is not None

    @property
    def idx(self) -> language.ModuleIndex:
        return self.interp.module_idx

    async def do_interp(self, source: wire.ModuleData):
        self.log.debug("module.changed")
        self.interp = await self.interpreter.interp(source)

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
            if runnable_type:
                runnable_type = SYMBOL_CLASS_BY_TYPE[SymbolType(runnable_type)]
            else:
                runnable_type = None
            runnable = self.idx.symbol(runnable, symbol_t=runnable_type)
            if build:
                build = self.idx.symbol(
                    build,
                    Build,
                    filter=lambda b: any(t.definition.id == runnable.id for t in b.tasks),
                )
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
            runnable=runnable_instance,
            arguments=arguments,
            ctx=ExecutionTrackerContext(
                tracing_level=tracing_level,
                trigger_type=trigger_type,
                trigger_id=trigger_id,
                root_id=root_id,
            ),
        )
        self.run_jobs.put_nowait((job.priority, job))
        qpos = self.run_jobs.qsize()
        # emit queued status immediately
        if isinstance(runnable_instance, TaskInstance):
            code_instance = runnable_instance.implementation
        else:
            code_instance = runnable_instance
        # notify tracer about queue enter
        # there are nicer ways to do this, but essentially we need access to the
        # current root tracer, which defaults to DEFAULT_TRACER
        run_ctx_token = pub_tracker_ctx.set(job.ctx)
        DEFAULT_TRACER.queue_enter(code_instance, arguments, qpos)
        pub_tracker_ctx.reset(run_ctx_token)
        return job

    async def do_run(
        self,
        runnable: CodeInstance,
        arguments: dict[str, LiteralValue],
        ctx: ExecutionTrackerContext,
    ):
        run_ctx_token = pub_tracker_ctx.set(ctx)
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
            pub_tracker_ctx.reset(run_ctx_token)

    def provide_context(self):
        """Sets the worker context var."""
        if self.ctx is None:
            raise RuntimeError(f"worker context not set: {self}")
        worker_ctx.set(self.ctx)

    async def run(self):
        """Runs the module worker main processing loop"""

        # first interp
        self.log.info("module.start")
        source, self.project_id = await self.master.get_module(self.module_id)
        try:
            await self.do_interp(source)
        except Exception as e:
            self.log.error("worker_init_failed", exc_info=e)
            raise RuntimeError(f"failed to initialize module worker {self}")

        # provide general worker context
        self.ctx = WorkerContext(
            deployment_id=self.deployment_id,
            worker_id=self.master.worker_id,
            module_id=self.module_id,
            project_id=self.project_id,
        )
        self.provide_context()

        self.ready.set()

        # process run tasks ad infinitum
        while True:
            _, job = await self.run_jobs.get()
            if job.cancelled:
                continue

            job_context = ExecutionTrackerContext(
                tracing_level=self.master.default_tracing_level,
                trigger_type=ExecutionTriggerType.JOB,
                trigger_id=job.id,
            )
            job_context_token = pub_tracker_ctx.set(job_context)
            try:
                self.log.debug("run", job=job)
                error, ret = await self.do_run(job.runnable, job.arguments, job.ctx)
                if error is None:
                    job.output = ret
                else:
                    job.error = error
                    job.error_details = ret
                self.log.debug("run.completed", job=job)
            except asyncio.CancelledError:
                self.log.info("run.cancelled", job=job)
                # keep the queue running?
            except Exception as e:
                sentry_enabled = sentry_capture_if_enabled(e)
                job.error = str(e)
                self.log.exception("run.failed", job=job, sentry_enabled=sentry_enabled)
            finally:
                job.terminated.set()
                pub_tracker_ctx.reset(job_context_token)
                self.run_jobs.task_done()


class SandboxedWorker:
    """A sandboxed runtime worker to execute arbitrary code (community or dedicated)."""

    def __init__(self, worker_id: UUID, deployment_id: UUID | None, project_id: UUID | None):
        self.worker_id = worker_id
        self.deployment_id = deployment_id
        self.project_id = project_id
        self.type = WorkerType.COMMUNITY if deployment_id is None else WorkerType.DEDICATED
        self.workers: dict[UUID, ModuleWorker] = {}
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

    def _get_worker(self, module_id: UUID) -> ModuleWorker:
        if module_id not in self.workers:
            # start module worker if not already started
            # TODO @Broken: assign workers to deployments
            worker = ModuleWorker(module_id, self, self.deployment_id)
            self.workers[module_id] = worker
            asyncio.create_task(wrap_task(worker.run(), "worker_run_" + str(module_id)))
        return self.workers[module_id]

    async def _get_ready_worker(self, module_id: UUID) -> ModuleWorker:
        worker = self._get_worker(module_id)
        if not worker.ready.is_set():
            await worker.ready.wait()
        return worker

    @message_handler
    async def module_changed(self, msg: NMessage[ModuleChangedPayload]):
        if msg.p.module_id not in self.workers:
            # ignore if we don't have a worker for this module
            return
        worker = await self._get_ready_worker(msg.p.module_id)
        worker.provide_context()
        await worker.do_interp(msg.p.module)
        # module worker will trigger any follow-ups

    @message_handler
    async def request_module_run(self, msg: NMessage[ReqModuleRunPayload]):
        worker = await self._get_ready_worker(msg.p.module_id)
        worker.provide_context()
        run_job = worker.queue_run(
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

    async def fetch(self, module_id: UUID) -> wire.ModuleData:
        return (await self.get_module(module_id))[0]

    async def stop(self):
        logger.info("stop", worker_id=self.worker_id)
        await asyncio.gather(task.cancel() for task in self.tasks)
        await asyncio.gather(sub.unsubscribe() for sub in self.subs)


class Sandbox:
    # TODO @Broken: ship build_candidate data to sandbox
    def __init__(self, build_candidate: BuildCandidate, module_id: UUID, job_id: UUID):
        self.module_id = module_id
        self.job_id = job_id
        self.build_candidate = build_candidate

    async def run(
        self, code: CodeInstance, arguments: dict[str, LiteralValue] | None = None
    ) -> dict[str, LiteralValue]:
        req = ReqModuleRunPayload(
            module_id=self.module_id,
            runnable=code.fqn,
            runnable_type="code",
            build=self.build_candidate.build,
            arguments=arguments,
            block=True,
            tracing_level=ExecutionTracingLevel.ALL_FRAMES_WITH_DATA,
            trigger_type=ExecutionTriggerType.JOB,
            trigger_id=self.job_id,
        )
        rep: NMessage[RepModuleRunPayload] = await request(
            NMessageType.REQUEST_MODULE_RUN, req, RepModuleRunPayload
        )
        if rep.p.error is not None:
            raise RunError(rep.p.error, code)
        return rep.p.output
