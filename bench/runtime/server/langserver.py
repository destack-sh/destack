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
from django.db.models import Q

from bench import bench, models
from bench.bench import HasType, Issue, ResolvedField, wire
from bench.bench.core import MOT, Module, ModuleReference, parse_absolute_statement_reference
from bench.bench.libs import DEFAULT_MODULES
from bench.bench.model import get_execution_cache_key
from bench.bench.mutate import ModuleMutation, ModuleMutator
from bench.bench.query import QueryOp
from bench.bench.type import instantiate_py_value, strip_py_value
from bench.bench.wire import ExecutionFrameData, ModuleTree
from bench.models import Execution, ExecutionStatus, Project, ProjectVersion, packer
from bench.models.execution import PENDING_EXECUTION_STATUSES
from bench.models.packer import write_mutations
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
    RepSearchDatasetPayload,
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
from bench.opensearch import mirror
from bench.opensearch.client import os_client
from bench.opensearch.core import IndexType
from bench.opensearch.query import CompilationInfo, compile_to_os
from bench.runtime.common.mutate import get_api_mutation_from_internal, trim_record_mutations
from bench.utils.cache import redis
from bench.utils.func import wrap_task
from bench.utils.utils import sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)


def create_wrapped_task(coro, task_id: str = None):
    asyncio.create_task(wrap_task(coro, task_id))


WORKER_HEARTBEAT_TIMEOUT = 30
MAX_SEARCH_DATASET_LIMIT = 500


class ModuleDB:
    def __init__(self, cache_committed: bool = True):
        self.cache_committed = cache_committed
        self._cached_modules: dict[
            ModuleReference | UUID, tuple[wire.ModuleTreeData, models.Project]
        ] = {}

    async def get_module(
        self, ref: ModuleReference | UUID
    ) -> tuple[wire.ModuleTreeData, models.Project]:
        # TODO @Cleanup @Architecture: ModuleDB fetch is suspiciously similar to interpreter fetch
        if ref in self._cached_modules:
            return self._cached_modules[ref]
        if isinstance(ref, UUID):
            project_version = await ProjectVersion.objects.aget(id=ref)
        elif ref.id is not None:
            project_version = await ProjectVersion.objects.aget(id=ref.id)
        else:
            owner, project = ref.name.split(".")
            if ref.version != "x":
                raise NotImplementedError("TODO: versioned module fetch")
            project_version = (
                await Project.objects.filter(
                    slug=project,
                )
                .filter(Q(organization__owner_slug_id=owner) | Q(user__owner_slug_id=owner))
                .select_related("head")
                .aget()
            )
            project_version = project_version.head
        module = await sync_to_async(packer.pack_module)(project_version)
        if self.cache_committed and project_version.committed:
            self._cached_modules[ref] = module, project_version.project
        return module, project_version.project

    async def fetch(self, ref: ModuleReference) -> wire.ModuleTreeData:
        return (await self.get_module(ref))[0]


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
            project_version = await ProjectVersion.objects.select_related(
                "project", "project__user", "project__organization"
            ).aget(id=module_id)
            worker = LanguageWorker(self.id, project_version, self.module_db)
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
        module, project = await self.module_db.get_module(msg.p.ref)
        await msg.reply(RepReadModulePayload(module=module, project_id=project.id))

    @message_handler
    async def write_module(self, msg: NMessage[ReqWriteModulePayload]) -> None:
        logger.debug("module.write", msg=msg)
        # TODO @Security: check if msg origin has write access to module
        worker = await self._get_ready_worker(msg.p.module_id)
        try:
            await worker.write_module(msg.p.mutations, origins=(msg.p.client,), wait=msg.p.wait)
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
        # TODO @Security: check if msg origin has read access to dataset
        project_version = await ProjectVersion.objects.aget(id=msg.p.module_id)
        combined_query = bench.Q(
            bench.QueryOp.AND,
            queries=[
                bench.Q(QueryOp.EQUALS, key="dataset_id", value=msg.p.backend_id),
                ~bench.Q(QueryOp.EXISTS, key="deleted_at"),
            ],
        )
        if msg.p.query is not None:
            combined_query &= msg.p.query
        effective_limit = min(msg.p.limit or MAX_SEARCH_DATASET_LIMIT, MAX_SEARCH_DATASET_LIMIT)
        compilation = CompilationInfo(root_limit=effective_limit)
        compiled_query = compile_to_os(compilation, combined_query)
        compiled_sort = compile_to_os(compilation, msg.p.sort) if msg.p.sort else [{"_id": "asc"}]
        search = {
            "size": effective_limit,
            "query": compiled_query,
            "sort": compiled_sort,
            "track_total_hits": msg.p.count,
            "version": True,
        }
        if msg.p.after:
            search["search_after"] = msg.p.after

        try:
            results = os_client.search(
                index=IndexType.BENCH.get_index_name(project_id=project_version.project_id),
                body=search,
            )
            record_packer = mirror.get_node_packer(mirror.Record)
            records: list[wire.RecordData] = []
            for r in results["hits"]["hits"]:
                doc = mirror.Record.from_dict(r["_source"], r["_id"], r["_version"])
                record = record_packer.pack(doc)
                records.append(record)
            rep = RepSearchDatasetPayload(
                records=records,
                total=(results["hits"]["total"]["value"] if msg.p.count else None),
                limit=effective_limit,
                first_sort_key=(results["hits"]["hits"][0]["sort"] if records else None),
                last_sort_key=(results["hits"]["hits"][-1]["sort"] if records else None),
            )
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("dataset.search.failed", msg=msg, exc_info=True)
            rep = RepSearchDatasetPayload(
                records=None,
                total=-1,
                limit=effective_limit,
                first_sort_key=None,
                last_sort_key=None,
                error=str(e),
            )

        await msg.reply(rep)

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
            secret_data = packer.pack_data(secret)
            secret_data.value = json.loads(secret_data.value)  # :SecretJson
            secrets.append(secret_data)
        await msg.reply(RepReadSecretPayload(secrets=secrets))

    @message_handler
    async def run_inference(self, msg: NMessage[ReqRunInferencePayload]) -> None:
        module_name, localized_path = parse_absolute_statement_reference(msg.p.model_path)
        log = logger.bind(model=msg.p.model_path, msg=msg)
        log.debug("inference.run")
        try:
            # remotely proxied inference if the worker doesn't have the required model api key
            # we call the underlying model implementation directly (the worker does the tracing)
            # :LibImplementation
            module = DEFAULT_MODULES[module_name]
            model = module.lookup(localized_path)
            cache_key = get_execution_cache_key(model.path, msg.p.inputs)
            inputs = instantiate_py_value(msg.p.inputs, model, is_output=False)
            output = await asyncio.wait_for(
                asyncio.shield(model._inference(inputs, cache_key, log)), msg.p.timeout
            )
            timeout = False
        except Exception as e:
            log.error("inference.exception", exc_info=True, sentry=sentry_capture_if_enabled(e))
            output = None
            model = None
            timeout = isinstance(e, asyncio.TimeoutError)
        outputs = (
            strip_py_value(output, model, is_output=True, ignore_outer_map=True) if output else None
        )
        await msg.reply(RepRunInferencePayload(outputs=outputs, timeout=timeout))

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
            ExecutionSavedPayload(module_id=msg.p.module_id, frames=[packer.pack_data(execution)]),
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
        self, worker_id: UUID, project_version: models.ProjectVersion, module_db: ModuleDB
    ):
        self.worker_id = worker_id
        self.project_version = project_version
        self.ready = asyncio.Event()
        self.log = logger.bind(
            module_id=self.module_id, project_id=self.project_id, worker_id=self.worker_id
        )
        self.module_db = module_db
        # module data
        self.source: wire.ModuleTreeData | None = None
        self.module: Optional[Module] = None
        self.module_tree: Optional[ModuleTree] = None
        self.last_module: Optional[Module] = None

    @property
    def client(self) -> ClientOrigin:
        return ClientOrigin("worker", self.worker_id, None)

    @property
    def module_id(self) -> UUID:
        return self.project_version.id

    @property
    def module_ref(self) -> ModuleReference:
        return ModuleReference(
            name=self.project_version.project.path, version="x", id=self.module_id
        )

    @property
    def project_id(self) -> UUID:
        return self.project_version.project_id

    def mutate(self) -> ModuleMutator:
        return ModuleMutator(self.module)

    async def on_module_changed(self, mutations: list[ModuleMutation]):
        is_semantic = any(m.type.semantic for m in mutations)
        if not is_semantic:
            # ignore non-semantic changes (will have to be smarter when we :BumpProperly)
            return
        mutator = ModuleMutator(self.source, mutations)
        new_source = mutator.to_module()
        await self.do_interp(new_source)

    async def write_module(
        self,
        mutations: list[ModuleMutation] | ModuleMutator,
        origins: tuple[ClientOrigin] = None,
        wait: bool = False,
    ):
        if isinstance(mutations, ModuleMutator):
            mutations = mutations.mutations
        logger.debug("write_module", mutations=mutations[:5], total=len(mutations), origins=origins)
        await sync_to_async(write_mutations)(
            self.project_version, self.module_tree, mutations, wait_for_os=wait
        )

        # trim mutations to remove overhead from large dataset updates
        trimmed_mutations = trim_record_mutations(mutations)

        await self.on_module_changed(mutations)
        origins = (*(origins or ()), self.client)
        api_mutations = list(
            chain.from_iterable(get_api_mutation_from_internal(m) for m in trimmed_mutations)
        )
        await publish(
            NMessageType.MODULE_INTERNAL_CHANGED,
            ModuleInternalChangedPayload(
                module_id=self.module_id, origins=origins, mutations=trimmed_mutations
            ),
        )
        await publish(
            NMessageType.MODULE_CHANGED,
            ModuleChangedPayload(
                module_id=self.module_id, origins=origins, mutations=api_mutations
            ),
        )

    def _do_interp_sync(self, new_source: wire.ModuleTreeData) -> tuple[Module, ModuleTree, Module]:
        self.source = new_source
        old = self.module
        old_tree = self.module_tree
        self.module = Module.interp_from(module=new_source, session=None)
        self.module_tree = ModuleTree(wire.pack_module(self.module).nodes)
        return old, old_tree, self.module

    async def do_interp(self, new_source: wire.ModuleTreeData) -> None:
        """Interprets the new module source, fetching deps and firing reactivity jobs"""
        old_module, old_tree, new_module = await asyncio.get_event_loop().run_in_executor(
            None, partial(self._do_interp_sync, new_source)
        )

        # check for any interp changes
        interp_mut = ModuleMutator(new_source)

        # resolved fields
        # prune existing interp data from mut tree to track changes
        interp_mut.tree.prune(wire.ResolvedFieldData)
        if old_module is None:
            interp_mut.truncate(new_source.module, MOT.RESOLVED_FIELD)
        for statement in new_module._statements_by_id.values():
            old_statement = old_module._statements_by_id.get(statement.id) if old_module else None
            if not isinstance(statement, HasType):
                continue
            if (
                not isinstance(old_statement, HasType)
                or old_statement.resolved_fields != statement.resolved_fields
            ):
                if old_statement is not None:
                    interp_mut.truncate(statement, MOT.RESOLVED_FIELD)
                for resolved in statement.resolved_fields:
                    if isinstance(resolved, ResolvedField):
                        interp_mut.create(resolved)
        # issues
        new_issues: dict[UUID, Issue] = {issue.id: issue for issue in new_module.issues}
        old_issues: set[UUID] = {issue.id for issue in old_module.issues} if old_module else {}
        if old_module is None:
            interp_mut.truncate(new_source.module, MOT.ISSUE)
        else:
            for issue in old_module.issues:
                if issue.id not in new_issues and issue.parent_id in interp_mut.tree:
                    interp_mut.delete(issue, apply=False)  # only track, doesn't exist
        for issue in new_issues.values():
            if issue.id not in old_issues:
                if old_module and issue.subject_id not in old_tree:
                    interp_mut.truncate(issue.subject, MOT.ISSUE)  # clear in case of restore
                interp_mut.create(issue)

        # save and notify
        if interp_mut.mutations:
            await sync_to_async(write_mutations)(
                self.project_version, self.module_tree, interp_mut.mutations, wait_for_os=False
            )
            await publish(
                NMessageType.MODULE_CHANGED,
                ModuleChangedPayload(
                    module_id=self.module_id, origins=(self.client,), mutations=interp_mut.mutations
                ),
            )

    async def run(self) -> None:
        source, project = await self.module_db.get_module(self.module_ref)
        await self.do_interp(source)
        self.ready.set()


def save_execution_frames(frames: list[ExecutionFrameData]) -> bool:
    model_executions: list[Execution] = []
    seen_ids = set()  # dedup by id, keep last (assumes chronological order)
    for frame in reversed(frames):
        if frame.id in seen_ids:
            continue
        seen_ids.add(frame.id)
        execution = packer.unpack_data(frame)
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
        # TODO @Feature!: write executions to OS
        return True
    except Exception as e:
        logger.error("save_execution_frames_failed", exc_info=e, executions=model_executions)
        return False
