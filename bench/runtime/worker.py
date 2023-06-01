import asyncio
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from typing import Any, Optional
from uuid import UUID

import structlog

from bench import language
from bench.language import wire
from bench.language.mutate import ModuleMutation, ModuleMutator
from bench.language.type import SYMBOL_CLASS_BY_TYPE, Build, LiteralValue, SymbolType
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.msg import NMessage, NMessageType
from bench.msg.core import handle_reply, message_handler, nc_init, publish, request, subscribe
from bench.msg.messages import (
    ClientOrigin,
    ExecutionMarkedDeadPayload,
    ModuleInternalChangedPayload,
    RepCancelRunPayload,
    RepReadModulePayload,
    RepRegisterWorkerPayload,
    RepRunPayload,
    RepWriteModulePayload,
    ReqCancelRunPayload,
    ReqReadModulePayload,
    ReqRegisterWorkerPayload,
    ReqRunPayload,
    ReqWriteModulePayload,
    RunErrorType,
    WorkerHeartbeatPayload,
)
from bench.runtime.instance import CodeInstance, Session, SessionMode, TaskInstance, instantiate
from bench.runtime.interp import InterpModule, LanguageInterpreter
from bench.runtime.run import RunError, run
from bench.runtime.tracing import (
    ExecutionTrackerContext,
    WorkerContext,
    in_memory_traces,
    pub_tracker_ctx,
    worker_ctx,
)
from bench.runtime.type import ExecutionFrame, ExecutionFrameData, WorkerType
from bench.utils.func import describe_type, wrap_task
from bench.utils.utils import get_from_env, sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT

WORKER_HEARTBEAT_INTERVAL = get_from_env("WORKER_HEARTBEAT_INTERVAL", 5, type_cast=int)
WORKER_RUN_TIMEOUT = get_from_env("WORKER_RUN_TIMEOUT", 300, type_cast=int)

logger = structlog.get_logger(__name__)


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


@dataclass(repr=False, slots=True)
class RunJob:
    priority: int = 1
    cancelled: bool = False
    default_build: Build = None
    runnable: TaskInstance | CodeInstance = None
    arguments: dict[str, LiteralValue] = None
    execution: Optional[ExecutionFrame] = None
    error: Optional[RunError] = None
    ctx: Optional[ExecutionTrackerContext] = None
    terminated: asyncio.Event = field(default_factory=asyncio.Event)
    id: UUID = field(default_factory=UUIDT)

    def __str__(self):
        return f"{self.id} {self.runnable}"

    def __repr__(self):
        return f"<RunJob {self}>"


class ModuleWorker:
    """A worker that processes all jobs for a single module (incl. to maintain its state)"""

    def __init__(
        self, module_id: UUID, master: "SandboxedWorker", deployment_id: UUID, timeout: float
    ):
        self.master = master
        self.module_id = module_id
        self.project_id: Optional[UUID] = None  # set in init (requires langserver fetch)
        # TODO @Broken: track module worker deployment id, make Execution.deployment non-nullable
        self.deployment_id: UUID = deployment_id
        self.timeout = timeout
        self.ctx: Optional[WorkerContext] = None
        self.ready = asyncio.Event()

        self.interpreter = LanguageInterpreter(master.fetch)
        self.interp: InterpModule | None = None
        self.queue: asyncio.Queue[tuple[int, RunJob]] = asyncio.PriorityQueue()
        self.pending_runs: dict[UUID, asyncio.Task] = {}
        self.executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="worker")
        self.log = logger.bind(worker_id=self.master.worker_id, module_id=self.module_id)

    @property
    def interpreted(self) -> bool:
        return self.interp is not None

    @property
    def idx(self) -> language.ModuleIndex:
        return self.interp.module_idx

    async def do_interp(self, source: wire.ModuleData):
        self.log.debug("module.init")
        self.interp = await self.interpreter.interp(source)

    async def do_interp_on_change(self, mutations: list[ModuleMutation]):
        self.log.debug("module.changed")
        new_source = ModuleMutator(self.interp.module_idx, mutations).apply()
        self.interp = await self.interpreter.interp(new_source)

    async def do_write(self, mutations: list[ModuleMutation]) -> bool:
        self.log.debug("module.write")
        new_source = ModuleMutator(self.interp.module_idx, mutations).apply()
        # interp and write in parallel
        self.interp, rep = await asyncio.gather(
            self.interpreter.interp(new_source),
            request(
                NMessageType.REQUEST_WRITE_MODULE,
                ReqWriteModulePayload(
                    module_id=self.module_id, mutations=mutations, client=self.master.client
                ),
                RepWriteModulePayload,
            ),
        )
        return rep.p.success

    def queue_run(
        self,
        *,
        runnable: str | UUID,
        runnable_type: str | None,
        arguments: dict[str, Any],
        tracing_level: ExecutionTracingLevel,
        trigger_type: ExecutionTriggerType,
        trigger_id: Optional[UUID],
        execution_id: Optional[UUID],
    ) -> RunJob | RunErrorType:
        if not self.interpreted:
            return RunErrorType.NOT_READY

        # get the runconfig
        try:
            if runnable_type:
                runnable_type = SYMBOL_CLASS_BY_TYPE[SymbolType(runnable_type)]
            else:
                runnable_type = None
            runnable = self.idx.symbol(runnable, symbol_t=runnable_type)
        except (TypeError, KeyError) as e:
            self.log.exception("module.run.failed", exc_info=e)
            return RunErrorType.INVALID_RUNCONFIG

        # instantiate
        try:
            from bench.runtime.build import get_default_builds

            builds = {b.name: b for b in get_default_builds(self.interp)}
            session = Session(
                idx=self.idx,
                mode=SessionMode.WRITE_GLOBAL,
                write=self.do_write,
                builds=builds,
                executor=self.executor,
            )
            runnable_instance = instantiate(runnable, session=session)
            if not isinstance(runnable_instance, (TaskInstance, CodeInstance)):
                raise TypeError(f"invalid runnable type: {type(runnable_instance)}")
        except Exception as e:
            self.log.exception("module.run.instantiate.failed", exc_info=e)
            return RunErrorType.INVALID_RUNCONFIG

        root_id = execution_id or UUIDT()
        job = RunJob(
            id=root_id,
            runnable=runnable_instance,
            arguments=arguments,
            ctx=ExecutionTrackerContext(
                tracing_level=tracing_level,
                trigger_type=trigger_type,
                trigger_id=trigger_id,
                root_id=root_id,
            ),
        )
        self.queue.put_nowait((job.priority, job))
        qpos = self.queue.qsize()
        # emit queued status immediately
        # notify tracer about queue enter
        run_ctx_token = pub_tracker_ctx.set(job.ctx)
        session.tracer.queue_enter(runnable_instance, arguments, qpos)
        pub_tracker_ctx.reset(run_ctx_token)
        return job

    async def do_run(
        self,
        id: UUID,
        runnable: CodeInstance | TaskInstance,
        arguments: dict[str, LiteralValue],
        ctx: ExecutionTrackerContext,
        timeout: float,
    ) -> Optional[RunErrorType]:
        run_ctx_token = pub_tracker_ctx.set(ctx)
        try:
            self.log.info(
                "module.run", runnable=runnable, arguments=describe_type(arguments), timeout=timeout
            )
            task = asyncio.create_task(run(runnable, arguments))
            self.pending_runs[id] = task
            await asyncio.wait_for(task, timeout=timeout)
            return None
        except RunError as e:
            self.log.exception("module.run.failed", exc_info=e)
            return RunErrorType.RUNTIME_ERROR
        except Exception as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            self.log.exception("module.run.failed", exc_info=e, sentry_enabled=sentry_enabled)
            return RunErrorType.INTERNAL_ERROR
        finally:
            pub_tracker_ctx.reset(run_ctx_token)
            if id in self.pending_runs:
                del self.pending_runs[id]

    async def cancel_run(self, execution_id: UUID) -> bool:
        if execution_id in self.pending_runs:
            self.pending_runs[execution_id].cancel()
            return True
        # maybe check if it's in the queue?
        for job in self.queue._queue:
            if job.id == execution_id:
                job.cancelled = True
        # mark it as dead for everyone
        # (just in case it's still bugging around in some frontend)
        await publish(
            NMessageType.EXECUTION_MARKED_DEAD,
            ExecutionMarkedDeadPayload(self.module_id, execution_id),
        )
        return False

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
            _, job = await self.queue.get()
            if job.cancelled:
                continue

            context_token = pub_tracker_ctx.set(job.ctx)
            try:
                self.log.debug("run", job=job)
                with in_memory_traces() as traces:
                    await self.do_run(
                        job.id, job.runnable, job.arguments, job.ctx, timeout=self.timeout
                    )
                    job.execution = traces.frames[0]
                self.log.debug("run.completed", job=job)
            except asyncio.CancelledError:
                self.log.info("run.cancelled", job=job)
                # keep the queue running?
            except Exception as e:
                sentry_enabled = sentry_capture_if_enabled(e)
                job.error = RunErrorType.RUNTIME_ERROR
                self.log.exception("run.failed", job=job, sentry_enabled=sentry_enabled)
            finally:
                job.terminated.set()
                pub_tracker_ctx.reset(context_token)
                self.queue.task_done()


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

    @property
    def client(self):
        return ClientOrigin(type="worker", id=self.worker_id, nonce=None)

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
            await subscribe(f"{NMessageType.MODULE_INTERNAL_CHANGED}.*", cb=self.module_changed),
            await handle_reply(NMessageType.REQUEST_RUN, self.request_run),
            await handle_reply(NMessageType.REQUEST_CANCEL_RUN, self.request_cancel),
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
            worker = ModuleWorker(module_id, self, self.deployment_id, timeout=WORKER_RUN_TIMEOUT)
            self.workers[module_id] = worker
            asyncio.create_task(wrap_task(worker.run(), "worker_run_" + str(module_id)))
        return self.workers[module_id]

    async def _get_ready_worker(self, module_id: UUID) -> ModuleWorker:
        worker = self._get_worker(module_id)
        if not worker.ready.is_set():
            await worker.ready.wait()
        return worker

    @message_handler
    async def module_changed(self, msg: NMessage[ModuleInternalChangedPayload]):
        if msg.p.module_id not in self.workers:
            # ignore if we don't have a worker for this module
            return
        if not msg.p.has_origin(self.client.id):
            worker = await self._get_ready_worker(msg.p.module_id)
            worker.provide_context()
            await worker.do_interp_on_change(msg.p.mutations)

    @message_handler
    async def request_run(self, msg: NMessage[ReqRunPayload]):
        worker = await self._get_ready_worker(msg.p.module_id)
        worker.provide_context()
        run_job = worker.queue_run(
            runnable=msg.p.runnable,
            runnable_type=msg.p.runnable_type,
            arguments=msg.p.arguments,
            tracing_level=msg.p.tracing_level,
            trigger_type=msg.p.trigger_type,
            trigger_id=msg.p.trigger_id,
            execution_id=msg.p.execution_id,
        )
        if isinstance(run_job, RunErrorType):  # couldn't queue run
            await msg.reply(RepRunPayload(error=run_job))
        else:
            if msg.p.block:
                await run_job.terminated.wait()
            if run_job.execution is not None:  # may be cancelled
                execution = ExecutionFrameData.from_frame(
                    run_job.execution,
                    project_id=worker.project_id,
                    tracing_level=msg.p.tracing_level,
                    deployment_id=worker.deployment_id,
                    worker_id=self.worker_id,
                    trigger_type=msg.p.trigger_type,
                    trigger_id=msg.p.trigger_id,
                )
            else:
                execution = None
            rep = RepRunPayload(
                error=run_job.error,
                execution=execution,
                execution_id=run_job.id,
            )
            await msg.reply(rep)

    @message_handler
    async def request_cancel(self, msg: NMessage[ReqCancelRunPayload]):
        worker = await self._get_ready_worker(msg.p.module_id)
        success = await worker.cancel_run(msg.p.execution_id)
        await msg.reply(RepCancelRunPayload(success=success))

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
