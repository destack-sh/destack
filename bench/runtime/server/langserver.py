import asyncio
import json
from datetime import datetime, timedelta
from functools import partial
from itertools import chain
from typing import Optional
from uuid import UUID

import pytz
import structlog
from asgiref.sync import sync_to_async

from bench import language, models
from bench.language import build, wire
from bench.language.const import InterpScope
from bench.language.inference import (
    SETTINGS_CLS_BY_MODALITY,
    Modality,
    get_inference_cache_key,
    run_inference,
)
from bench.language.mutate import MMT, ModuleMutation, ModuleMutator
from bench.language.wire import InterpData
from bench.models import Execution, ExecutionStatus, ProjectVersion, packer
from bench.models.execution import PENDING_EXECUTION_STATUSES
from bench.msg import NMessage
from bench.msg.core import handle_reply, message_handler, nc_init, publish, subscribe
from bench.msg.messages import (
    ClientOrigin,
    ExecutionChangedPayload,
    ExecutionMarkedDeadPayload,
    ExecutionSavedPayload,
    ModuleChangedPayload,
    ModuleInternalChangedPayload,
    NMessageType,
    RepLangserverPayload,
    RepReadModulePayload,
    RepReadObjectPayload,
    RepReadSecretPayload,
    RepRegisterWorkerPayload,
    RepRunInferencePayload,
    RepWriteModulePayload,
    ReqLangserverPayload,
    ReqReadModulePayload,
    ReqReadObjectPayload,
    ReqReadSecretPayload,
    ReqRegisterWorkerPayload,
    ReqRunInferencePayload,
    ReqSearchDatasetPayload,
    ReqWriteModulePayload,
    WorkerHeartbeatPayload,
)
from bench.runtime.common.interp import (
    InterpModule,
    LanguageInterpreter,
    ModuleFetcher,
    get_requirements,
    interp_module,
)
from bench.runtime.common.models import get_inference_endpoint, get_model_key_from_env
from bench.runtime.common.mutate import map_mutation_to_public
from bench.runtime.common.type import ExecutionFrameData
from bench.runtime.server.mutate import read_packed_module, write_mutations
from bench.utils.cache import redis
from bench.utils.func import wrap_task
from bench.utils.utils import sentry_capture_if_enabled
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
        module = await sync_to_async(read_packed_module)(
            project_version, exclude_non_semantic=False
        )
        if self.cache_committed and project_version.committed:
            self._cached_modules[module_id] = module, project_version.id
        return module, project_version.project_id

    async def fetch(self, module_id: UUID) -> wire.ModuleData:
        return (await self.get_module(module_id))[0]


class LanguageServer:
    """
    Bench language & runtime server for LSP and runtime DB access.
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
            await handle_reply(NMessageType.REQUEST_LANGSERVER, self.request_langserver),
            await handle_reply(NMessageType.REQUEST_SEARCH_DATASET, self.search_dataset),
            await handle_reply(NMessageType.REQUEST_READ_OBJECT, self.read_object),
            await handle_reply(NMessageType.REQUEST_READ_SECRET, self.read_secret),
            await handle_reply(NMessageType.REQUEST_RUN_INFERENCE, self.run_inference),
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
                project_id=msg.p.project_id,
                tenancy=msg.p.tenancy,
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
    async def search_dataset(self, msg: NMessage[ReqSearchDatasetPayload]) -> None:
        logger.debug("dataset.search", msg=msg)
        raise NotImplementedError  # nocheckin: datasets

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
    async def read_secret(self, msg: NMessage[ReqReadSecretPayload]) -> None:
        logger.debug("secret.read", msg=msg)
        # TODO @Security: check if msg origin has read access to secret
        secrets = []
        async for secret in models.Secret.objects.filter(id__in=(s.id for s in msg.p.secrets)):
            secret_data = wire.pack_data(secret)
            secret_data.value = json.loads(secret_data.value)  # :SecretJson
            secrets.append(secret_data)
        await msg.reply(RepReadSecretPayload(secrets=secrets))

    @message_handler
    async def run_inference(self, msg: NMessage[ReqRunInferencePayload]) -> None:
        modality = Modality(msg.p.modality)
        log = logger.bind(model=msg.p.model_fqn, modality=modality, msg=msg)
        log.debug("inference.run")
        key = get_model_key_from_env(msg.p.model_fqn)
        inference = get_inference_endpoint(
            model=msg.p.model_fqn,
            modality=modality,
            external_name=msg.p.model_external_name,
            key=key,
        )
        endpoint = getattr(inference, modality.value)
        try:
            settings = SETTINGS_CLS_BY_MODALITY[msg.p.modality](**msg.p.settings)
            xblocks = [build.unpack_xblock(xblock) for xblock in msg.p.blocks]
            cache_key = get_inference_cache_key(msg.p.model_fqn, modality, xblocks, settings)
            output = await asyncio.wait_for(
                asyncio.shield(run_inference(endpoint, xblocks, settings, cache_key, log)),
                msg.p.timeout,
            )
            timeout = False
        except Exception as e:
            log.error("inference.exception", exc_info=True, sentry=sentry_capture_if_enabled(e))
            output = None
            timeout = isinstance(e, asyncio.TimeoutError)
        await msg.reply(RepRunInferencePayload(output=output, timeout=timeout))

    @message_handler
    async def request_langserver(self, msg: NMessage[ReqLangserverPayload]):
        logger.debug("langserver.wake", msg=msg)
        await self._get_ready_worker(msg.p.module_id)
        await msg.reply(RepLangserverPayload(module_id=msg.p.module_id))

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
                module_id=msg.p.module_id, frames=[packer.pack_execution_frame(execution)]
            ),
        )

    @message_handler
    async def module_changed(self, msg: NMessage[ModuleInternalChangedPayload]) -> None:
        if msg.p.has_origin(self.id):
            return  # ignore own changes
        # update language worker
        worker = await self._get_ready_worker(msg.p.module_id)
        await worker.on_module_changed(msg.p.mutations)

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
            await Execution.objects.filter(
                status__in=PENDING_EXECUTION_STATUSES,
                started_at__lt=start_cutoff,
            ).aupdate(status=ExecutionStatus.Failed)
            await asyncio.sleep(interval_seconds)

    async def stop(self):
        logger.info("stop")
        await asyncio.gather(sub.unsubscribe() for sub in self.subs)
        # update self as worker
        await models.Worker.objects.filter(id=self.id).aupdate(
            status=models.WorkerStatus.TERMINATED
        )


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
        # module data
        self.source: wire.ModuleData | None = None
        self.last_interp_by_statement: dict[UUID, InterpData] = {}
        self.interp: Optional[InterpModule] = None

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
    def module(self):
        return self.interp.module

    def mutate(self) -> ModuleMutator:
        return ModuleMutator(self.interp.module)

    @property
    def interpreted(self) -> bool:
        return self.interp.module is not None

    async def on_module_changed(self, mutator: list[ModuleMutation] | ModuleMutator):
        if not isinstance(mutator, ModuleMutator):
            mutator = ModuleMutator(self.module, mutator, source=self.source)
        new_source = mutator.apply()
        await self.do_interp(new_source)

    async def write_module(
        self, mutations: list[ModuleMutation] | ModuleMutator, origins: tuple[ClientOrigin] = None
    ):
        if isinstance(mutations, ModuleMutator):
            mutations = mutations.mutations
        await sync_to_async(write_mutations)(project_v=self.project_version, mutations=mutations)
        await self.on_module_changed(mutations)
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

    def _do_interp_sync(
        self, new_source: wire.ModuleData, dependencies: list[InterpModule]
    ) -> dict[UUID, InterpData]:
        self.source = new_source
        self.interp = interp_module(new_source)

        # interpret
        interp_by_scope: dict[UUID, InterpData] = {}
        for symbol in self.interp.module.symbols_by_id.values():
            if isinstance(symbol, language.HasType) and symbol.resolved_fields is not None:
                resolved_fields = [wire.pack_node_flat(field) for field in symbol.resolved_fields]
            else:
                resolved_fields = None
            interp_by_scope[symbol.id] = InterpData(
                scope=InterpScope.STATEMENT,
                file_id=symbol.source.file.id,
                statement_id=symbol.source.id,
                issues=None,
                resolved_fields=resolved_fields,
            )
        # add issues to interp scope
        for issue in self.interp.issues:
            if issue.subject is not None and issue.subject.id in interp_by_scope:
                interp = interp_by_scope[issue.subject.id]
            else:
                continue  # not sure what to do here
            if interp.issues is None:
                interp.issues = []
            interp.issues.append(wire.pack_data(issue))
        return interp_by_scope

    async def do_interp(self, new_source: wire.ModuleData) -> None:
        """Interprets the new module source, fetching deps and firing reactivity jobs"""
        requirements = get_requirements(new_source)
        dependencies = await self.interpreter.interp_requirements(requirements)
        interp_by_scope = await asyncio.get_event_loop().run_in_executor(
            None, partial(self._do_interp_sync, new_source, dependencies)
        )

        # track any interp changes
        mutations = []
        for interp_data in interp_by_scope.values():
            last_interp = self.last_interp_by_statement.get(interp_data.statement_id)
            if last_interp is not None and last_interp.hash_content() == interp_data.hash_content():
                continue
            mutation = ModuleMutation(
                type=MMT.UPDATE_INTERP,
                project_version_id=self.module_id,
                file_id=interp_data.file_id,
                statement_id=interp_data.statement_id,
            )
            mutation.data = interp_data
            mutations.append(mutation)
        self.last_interp_by_statement = interp_by_scope
        # save and notify
        if mutations:
            await sync_to_async(write_mutations)(
                project_v=self.project_version, mutations=mutations
            )
            await publish(
                NMessageType.MODULE_CHANGED,
                ModuleChangedPayload(
                    module_id=self.module_id, origins=(self.client,), mutations=mutations
                ),
            )

    async def run(self) -> None:
        source = await self.fetcher(self.module_id)
        await self.do_interp(source)
        self.ready.set()


def save_execution_frames(frames: list[ExecutionFrameData]) -> bool:
    model_executions: list[Execution] = []
    seen_ids = set()  # dedup by id, keep last (assumes chronological order)
    for frame in reversed(frames):
        if frame.id in seen_ids:
            continue
        seen_ids.add(frame.id)
        execution = packer.pack_execution_frame(frame)
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
