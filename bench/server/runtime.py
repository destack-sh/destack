import asyncio
import json
import typing
from dataclasses import dataclass
from datetime import datetime, timedelta
from functools import partial
from itertools import chain
from typing import Optional
from uuid import UUID

import pytz
import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import ValidationError
from django.db import transaction

from bench import models
from bench.language import (
    HasType,
    Issue,
    Q,
    Query,
    QueryOp,
    ResolvedField,
    Trigger,
    TriggerType,
    wire,
)
from bench.language.cache import CacheAsync
from bench.language.core import MOT, Module, ModuleReference, parse_absolute_statement_reference
from bench.language.flow import IsFlowable, TriggerScheduleIterator, is_time_trigger_equal
from bench.language.libs import DEFAULT_MODULES
from bench.language.mutate import ModuleMutation, ModuleMutator
from bench.language.session import RunStatus
from bench.language.type import instantiate_py_value, strip_py_value
from bench.language.utils import get_run_cache_subkey
from bench.language.wire import ModuleTree
from bench.models import Project, ProjectVersion, packer
from bench.models.packer import write_mutations, write_session
from bench.msg import NMessage
from bench.msg.core import handle_reply, message_handler, nc_init, publish, request, subscribe
from bench.msg.messages import (
    ClientOrigin,
    LogsChangedPayload,
    ModuleChangedPayload,
    ModuleInternalChangedPayload,
    NMessageType,
    RepMarkUploadedObjectPayload,
    RepReadModulePayload,
    RepReadObjectPayload,
    RepReadSecretPayload,
    RepRunInferencePayload,
    RepSearch,
    RepSearchLogPayload,
    RepSearchRecordPayload,
    RepSearchRunPayload,
    RepStartRunPayload,
    RepWakeRuntimePayload,
    RepWriteModulePayload,
    RepWriteObjectPayload,
    RepWriteSessionPayload,
    ReqMarkUploadedObjectPayload,
    ReqReadModulePayload,
    ReqReadObjectPayload,
    ReqReadSecretPayload,
    ReqRunInferencePayload,
    ReqSearch,
    ReqSearchLogPayload,
    ReqSearchRecordPayload,
    ReqSearchRunPayload,
    ReqStartRunPayload,
    ReqWakeRuntimePayload,
    ReqWriteModulePayload,
    ReqWriteObjectPayload,
    ReqWriteSessionPayload,
    RunsChangedGlobalPayload,
    SessionChangedPayload,
)
from bench.opensearch import mirror
from bench.opensearch.client import os_client
from bench.opensearch.core import IndexType
from bench.opensearch.query import encode_cursor, prepare_search
from bench.server.observer import WorkerSetObserver
from bench.utils.func import wrap_task
from bench.utils.monitoring import Monitored
from bench.utils.task import TaskManager
from bench.utils.utils import sentry_capture_if_enabled
from bench.utils.uuidt import UUIDT
from bench.worker.mutate import get_api_mutation_from_internal, trim_record_mutations

logger = structlog.get_logger(__name__)

MAX_SEARCH_RECORD_LIMIT = 500
MAX_SEARCH_LOG_LIMIT = 1000
MAX_SEARCH_RUN_LIMIT = 1000

_cached_modules: dict[ModuleReference | UUID, tuple[wire.ModuleTreeData, models.Project]] = {}


async def get_module(ref: ModuleReference | UUID) -> tuple[wire.ModuleTreeData, models.Project]:
    # TODO @Cleanup @Architecture: ModuleDB fetch is suspiciously similar to interpreter fetch
    if ref in _cached_modules:
        return _cached_modules[ref]
    id = ref if isinstance(ref, UUID) else ref.id
    if id:
        try:
            project_version = await ProjectVersion.objects.aget(id=id)
        except ProjectVersion.DoesNotExist:
            project_version = (await Project.objects.select_related("head").aget(id=id)).head
    else:
        owner, project = ref.name.split(".")
        if ref.version != "x":
            raise NotImplementedError("TODO: versioned module fetch")
        project_version = (
            await Project.objects.filter(
                slug=project,
            )
            .filter(
                models.Q(organization__owner_slug_id=owner) | models.Q(user__owner_slug_id=owner)
            )
            .select_related("head")
            .aget()
        )
        project_version = project_version.head
    module = await sync_to_async(packer.pack_module)(project_version)
    if project_version.committed:
        _cached_modules[ref] = module, project_version.project
    return module, project_version.project


async def fetch(ref: ModuleReference) -> wire.ModuleTreeData:
    return (await get_module(ref))[0]


record_packer = mirror.get_node_packer(mirror.Record)
run_packer = mirror.get_node_packer(mirror.Run)
log_packer = mirror.get_node_packer(mirror.LogEntry)


def _unpack_record(record: mirror.Record):
    doc = mirror.Record.from_dict(record["_source"], record["_id"], record["_version"])
    return record_packer.pack(doc)


def _unpack_run(run: mirror.Run):
    doc = mirror.Run.from_dict(run["_source"], run["_id"], run["_version"])
    return run_packer.pack(doc)


def _unpack_log(log: mirror.LogEntry):
    doc = mirror.LogEntry.from_dict(log["_source"], log["_id"], log["_version"])
    return log_packer.pack(doc)


class RuntimeServer(Monitored):
    """
    Bench runtime server to host per-module runtime workers that
     proxy user worker module access (read/write) and process triggers.
    """

    def __init__(self):
        self.id = UUIDT()
        self.runtime_workers: dict[UUID, RuntimeWorker] = {}
        self.subs = []
        self.tasks = TaskManager()
        self.user_worker_observer = WorkerSetObserver()
        self._ready = False

    async def run(self):
        await nc_init.wait()
        logger.info("start")
        self.subs = [
            await handle_reply(NMessageType.READ_MODULE, self.read_module),
            await handle_reply(NMessageType.WRITE_MODULE, self.write_module),
            await handle_reply(NMessageType.WRITE_SESSION, self.write_session),
            await handle_reply(NMessageType.WAKE_RUNTIME, self.request_runtime),
            await handle_reply(NMessageType.SEARCH_RECORD, self.search_record),
            await handle_reply(NMessageType.SEARCH_RUN, self.search_run),
            await handle_reply(NMessageType.SEARCH_LOG, self.search_log),
            await handle_reply(NMessageType.READ_OBJECT, self.read_object),
            await handle_reply(NMessageType.WRITE_OBJECT, self.write_object),
            await handle_reply(NMessageType.MARK_UPLOADED_OBJECT, self.mark_uploaded_object),
            await handle_reply(NMessageType.READ_SECRET, self.read_secret),
            await handle_reply(NMessageType.RUN_PROXY_INFERENCE, self.run_inference),
            await subscribe(f"{NMessageType.MODULE_INTERNAL_CHANGED}.>", cb=self.module_changed),
        ]

        logger.info("load_modules")
        projects = [project async for project in Project.objects.all()]
        await asyncio.gather(*[self._prepare_worker(project.head_id) for project in projects])

        await self.user_worker_observer.start()

        logger.info("ready")
        self._ready = True

    @property
    def ready(self) -> bool:
        return self._ready

    @property
    def healthy(self):
        return self.ready and self.tasks.healthy

    async def _prepare_worker(self, module_id: UUID) -> "RuntimeWorker":
        worker = self.runtime_workers.get(module_id)
        if worker is None:
            # start language worker if not already started
            project_version = await ProjectVersion.objects.select_related(
                "project", "project__user", "project__organization"
            ).aget(id=module_id)
            worker = RuntimeWorker(self.id, self.tasks, self.user_worker_observer, project_version)
            self.runtime_workers[module_id] = worker
            asyncio.create_task(wrap_task(worker.run(), f"worker-{module_id}"))
        if not worker.ready.is_set():
            await worker.ready.wait()
        return worker

    @message_handler
    async def read_module(self, msg: NMessage[ReqReadModulePayload]) -> None:
        logger.debug("module.read", msg=msg)
        module, project = await get_module(msg.p.ref)
        await msg.reply(RepReadModulePayload(module=module, project_id=project.id))

    @message_handler
    async def write_module(self, msg: NMessage[ReqWriteModulePayload]) -> None:
        logger.debug("module.write", msg=msg)
        # TODO @Security: check if msg origin has write access to module
        worker = await self._prepare_worker(msg.p.module_id)
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
    async def write_session(self, msg: NMessage[ReqWriteSessionPayload]) -> None:
        logger.debug("session.write", msg=msg, client=msg.p.client)
        worker = await self._prepare_worker(msg.p.module_id)
        try:
            await worker.write_session(
                msg.p.session, msg.p.runs, msg.p.logs, origins=(msg.p.client,)
            )
            logger.debug("session.write.done", msg=msg)
            success = True
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("write_session.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepWriteSessionPayload(success=success))

    def _do_search(
        self,
        project_v: models.ProjectVersion,
        type: Optional[mirror.DocumentType],
        extra_query: Optional[Query],
        limit: int,
        req: ReqSearch,
        unpack: typing.Callable,
        rep_cls: typing.Type[RepSearch],
    ) -> RepSearch:
        effective_limit = min(req.limit, limit)
        try:
            search = prepare_search(
                type=type,
                project_version_id=str(project_v.id),
                limit=effective_limit,
                count=req.count,
                after=req.after,
                sort=req.sort,
                query=Query.and_if_set(req.query, extra_query),
            )
            results = os_client.search(
                index=IndexType.BENCH.get_index_name(project_id=project_v.project_id),
                body=search,
            )
            elements: list[typing.Any] = []
            for r in results["hits"]["hits"]:
                elements.append(unpack(r))
            if elements:
                start_cursor = encode_cursor(results["hits"]["hits"][0], req.after, i=0)
                end_cursor = encode_cursor(
                    results["hits"]["hits"][-1], req.after, i=len(elements) - 1
                )
            else:
                start_cursor = None
                end_cursor = None
            rep = rep_cls(
                elements=elements,
                total=(results["hits"]["total"]["value"] if req.count else None),
                limit=effective_limit,
                start_cursor=start_cursor,
                end_cursor=end_cursor,
            )
        except Exception as e:
            sentry_capture_if_enabled(e)
            logger.error("dataset.search.failed", req=req, exc_info=True)
            rep = rep_cls(
                elements=None,
                total=-1,
                limit=effective_limit,
                start_cursor=None,
                end_cursor=None,
                error=str(e),
            )
        return rep

    @message_handler
    async def search_record(self, msg: NMessage[ReqSearchRecordPayload]) -> None:
        logger.debug("search.record", msg=msg)
        if msg.p.keys:
            extra_query = Q(QueryOp.EQUALS, "dataset_id", msg.p.keys)
        else:
            extra_query = None
        # TODO @Security: check if msg origin has read access to dataset
        project_v = await ProjectVersion.objects.aget(id=msg.p.module_id)
        rep = await sync_to_async(self._do_search)(
            project_v=project_v,
            extra_query=extra_query,
            type=mirror.DocumentType.RECORD,
            limit=MAX_SEARCH_RECORD_LIMIT,
            req=msg.p,
            unpack=_unpack_record,
            rep_cls=RepSearchRecordPayload,
        )
        await msg.reply(rep)

    @message_handler
    async def search_run(self, msg: NMessage[ReqSearchRunPayload]) -> None:
        logger.debug("search.dataset", msg=msg)
        # TODO @Security: check if msg origin has read access to dataset
        if msg.p.runnables_ids:
            extra_query = Q(QueryOp.EQUALS, "runnable_id", msg.p.runnables_ids)
        else:
            extra_query = None
        project_v = await ProjectVersion.objects.aget(id=msg.p.module_id)
        rep = await sync_to_async(self._do_search)(
            project_v=project_v,
            extra_query=extra_query,
            type=mirror.DocumentType.RUN,
            limit=MAX_SEARCH_RUN_LIMIT,
            req=msg.p,
            unpack=_unpack_run,
            rep_cls=RepSearchRunPayload,
        )
        await msg.reply(rep)

    @message_handler
    async def search_log(self, msg: NMessage[ReqSearchLogPayload]) -> None:
        logger.debug("search.log", msg=msg)
        if msg.p.runnables_ids:
            extra_query = Q(QueryOp.EQUALS, "runnable_id", msg.p.runnables_ids)
        else:
            extra_query = None
        # TODO @Security: check if msg origin has read access to dataset
        project_v = await ProjectVersion.objects.aget(id=msg.p.module_id)
        rep = await sync_to_async(self._do_search)(
            project_v=project_v,
            extra_query=extra_query,
            type=mirror.DocumentType.LOG_ENTRY,
            limit=MAX_SEARCH_LOG_LIMIT,
            req=msg.p,
            unpack=_unpack_log,
            rep_cls=RepSearchLogPayload,
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
    async def write_object(self, msg: NMessage[ReqWriteObjectPayload]) -> None:
        logger.debug("object.write", msg=msg)
        # TODO @Security: check if msg origin has write access to object
        project_v = await ProjectVersion.objects.select_related("project").aget(id=msg.p.module_id)
        post_urls: list[str | None] = []
        for obj_data in msg.p.objects:
            model_object: models.RemoteObject = packer.unpack_data(obj_data)
            model_object.project_id = project_v.project_id
            existing_object = await project_v.project.remote_objects.filter(
                sha512=model_object.sha512
            ).afirst()
            if existing_object is not None:
                obj_data.id = existing_object.id
                if existing_object.status == models.RemoteObjectStatus.AVAILABLE:
                    obj_data.status = models.RemoteObjectStatus.AVAILABLE
                    post_urls.append(None)
                else:
                    existing_object.generate_presigned_post()
                    post_urls.append(existing_object.presigned_post)
            else:
                model_object.generate_presigned_post()
                post_urls.append(model_object.presigned_post)
                await model_object.asave()  # create
        logger.debug("object.write.rep", msg=msg, post_urls=[url is not None for url in post_urls])
        await msg.reply(RepWriteObjectPayload(objects=msg.p.objects, post_urls=post_urls))

    @message_handler
    async def mark_uploaded_object(self, msg: NMessage[ReqMarkUploadedObjectPayload]) -> None:
        logger.debug("object.mark_uploaded", msg=msg)
        try:
            for obj_data in msg.p.objects:
                remote_object: models.RemoteObject = await models.RemoteObject.objects.aget(
                    id=obj_data.id
                )
                remote_object.mark_available_if_exists_in_s3()
                await remote_object.asave()
            success = True
        except ValidationError:
            logger.error("object.mark_uploaded.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepMarkUploadedObjectPayload(success=success))

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
            cache_subkey = get_run_cache_subkey(inputs_raw=msg.p.inputs)
            log = log.bind(cache_subkey=cache_subkey)
            cache = CacheAsync(module=None, subkey=model.id.hex, project_id=msg.p.project_id)
            inputs = instantiate_py_value(msg.p.inputs, model, is_output=False)
            output = await asyncio.wait_for(
                asyncio.shield(
                    model._inference(inputs=inputs, cache_subkey=cache_subkey, log=log, cache=cache)
                ),
                msg.p.timeout,
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
    async def request_runtime(self, msg: NMessage[ReqWakeRuntimePayload]):
        logger.debug("runtime.wake", msg=msg)
        await self._prepare_worker(msg.p.module_id)
        await msg.reply(RepWakeRuntimePayload(module_id=msg.p.module_id))

    @message_handler
    async def module_changed(self, msg: NMessage[ModuleInternalChangedPayload]) -> None:
        logger.debug("module.changed", msg=msg)
        if msg.p.has_origin(self.id):
            return  # ignore own changes
        # update language worker
        worker = await self._prepare_worker(msg.p.module_id)
        await worker.on_module_changed(msg.p.mutations)

    async def stop(self):
        logger.info("stop")
        self._ready = False
        await asyncio.gather(*[sub.unsubscribe() for sub in self.subs])


TIME_TRIGGER_PRE_SEND_WINDOW = timedelta(minutes=1)  # 1 minute before
TIME_TRIGGER_LOOKAHEAD = 2  # occurrences


async def run_at(func: typing.Callable[[], typing.Awaitable[None]], at: datetime) -> None:
    """Run a function at a given time."""
    delay = (at - datetime.utcnow()).total_seconds()
    if delay < 0:
        delay = 0
    await asyncio.sleep(delay)
    await func()


def schedule_run_at(func: typing.Callable[[], typing.Awaitable[None]], at: datetime) -> None:
    """Schedule a function to run at a given time."""
    asyncio.create_task(run_at(func, at))


@dataclass
class ActiveTrigger:
    trigger: Trigger
    processed_up_to: Optional[datetime]
    next_occurrence: Optional[datetime]
    iter: TriggerScheduleIterator


class RuntimeWorker:
    """
    Runtime worker for a single module.
    Assumed to run as a singleton per module, mainly to ensure time triggers are processed
     with hopefully exactly once / definitely at least once semantics (later also OTs).
     We may separate those parts into some elected 'main' worker later for scalability.
    """

    def __init__(
        self,
        host_id: UUID,
        tasks: TaskManager,
        workers_watcher: WorkerSetObserver,
        project_version: models.ProjectVersion,
    ):
        self.server_id = host_id
        self.tasks = tasks
        self.workers_watcher = workers_watcher
        self.project_version = project_version
        self.ready = asyncio.Event()
        self.log = logger.bind(
            module_id=self.module_id,
            project_id=self.project_id,
            worker_id=self.server_id,
            module=self.project_version,
        )
        # module data
        self.source: wire.ModuleTreeData | None = None
        self.module: Optional[Module] = None
        self.module_tree: Optional[ModuleTree] = None
        # time triggers
        self.active_triggers: dict[UUID, ActiveTrigger] = {}
        self.active_trigger_process_wait: asyncio.Event = asyncio.Event()

    @property
    def client(self) -> ClientOrigin:
        return ClientOrigin("runtime-worker", self.server_id, None)

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

    async def run(self) -> None:
        # fetch and interp module
        source, project = await get_module(self.module_ref)
        await self.interp(source)

        self.tasks.start(self.process_time_triggers_forever())

        self.ready.set()

    async def process_time_triggers_forever(self) -> None:
        """
        Process all time triggers for this module forever.
        As noted above, this is assumed to run once per module.
        """

        async def set_wait(after: float) -> None:
            await asyncio.sleep(after)
            self.active_trigger_process_wait.set()

        triggers_to_fire: set[UUID] = set()
        runs_to_start: list[wire.RunData] = []
        timed_wait_task: Optional[asyncio.Task] = None

        # backfill time triggers on first go (coalescing to at most one per trigger)
        for trigger in self.active_triggers.values():
            if (
                trigger.processed_up_to is not None
                and trigger.iter.last_occurrence_initial > trigger.processed_up_to
            ):
                # should we ignore backfill here if just edited (updated_at > processed_up_to)?
                triggers_to_fire.add(trigger.trigger.id)

        # nocheckin: backfill scheduled runs into runs_to_start

        self.log.debug("time_triggers.process_forever", initial_triggers_to_fire=triggers_to_fire)

        # enter forever loop
        while True:
            if timed_wait_task is not None:
                timed_wait_task.cancel()

            process_up_to = (
                datetime.utcnow().replace(tzinfo=pytz.UTC) + TIME_TRIGGER_PRE_SEND_WINDOW
            )
            self.log.debug(
                "time_triggers.check",
                process_up_to=process_up_to,
                active_triggers=self.active_triggers,
            )

            # TODO @Robustness @UX: cancel pre-scheduled runs that no longer have an active trigger

            # collect triggers that are due to fire within the send window
            for trigger in self.active_triggers.values():
                if trigger.next_occurrence is None:
                    trigger.next_occurrence = trigger.iter.next()
                if trigger.next_occurrence <= process_up_to:
                    triggers_to_fire.add(trigger.trigger.id)

            # "process" triggers (write to DB and create runs atomically)
            next_runs_to_start: list[wire.RunData] = await sync_to_async(self._process_triggers)(
                triggers=self.active_triggers,
                triggers_to_fire=triggers_to_fire,
                processed_up_to=process_up_to,
            )
            runs_to_start.extend(next_runs_to_start)

            # publish scheduled runs (should be project scoped later, but we don't have a session)
            await publish(NMessageType.RUNS_CHANGED, RunsChangedGlobalPayload(runs=runs_to_start))

            # start worker set if not already started
            if runs_to_start:
                if not self.workers_watcher.is_healthy(self.project_id):
                    # not sure what to do after timeout here... retry? panic?
                    await self.workers_watcher.wake_until_healthy(self.project_id, timeout=300)

                logger.debug(
                    "time_triggers.fire",
                    runs_to_start=runs_to_start,
                    fired_triggers=[self.active_triggers[id].trigger for id in triggers_to_fire],
                )

                # send out run requests (could do this in parallel but doesn't matter for now)
                for run in runs_to_start:
                    req = ReqStartRunPayload(
                        project_id=run.project_id,
                        module_id=run.module_id,
                        session_id=run.session_id,
                        run_id=run.id,
                        runnable=run.runnable_id,
                        inputs=run.inputs,
                        block=None,
                        keyed=True,
                        trigger_type=run.trigger_type,
                        trigger_id=run.trigger_id,
                        scheduled_at=run.scheduled_at,
                    )
                    try:
                        rep: NMessage[RepStartRunPayload] = await request(
                            NMessageType.START_RUN, req, reply_t=RepStartRunPayload, retry=3
                        )
                        if rep.p.error:
                            logger.error(
                                "time_triggers.start_run.error", run=run, error=rep.p.error
                            )
                            continue
                    except Exception as e:
                        logger.error("time_triggers.start_run.error", run=run, exc_info=e)
                        continue

            # reset next occurrence for all triggers that fired
            for trigger_id in triggers_to_fire:
                trigger = self.active_triggers[trigger_id]
                trigger.next_occurrence = trigger.iter.next()
            triggers_to_fire.clear()
            runs_to_start.clear()

            # wait for earliest next trigger occurrence (or trigger change)
            if self.active_triggers:
                earliest_next_occurrence = min(
                    t.next_occurrence for t in self.active_triggers.values()
                )
                new_now = datetime.utcnow().replace(tzinfo=pytz.UTC)
                timeout_till_next = (earliest_next_occurrence - new_now).total_seconds()
                assert timeout_till_next >= 0, f"negative timeout: {timeout_till_next}"
                timed_wait_task = asyncio.create_task(set_wait(timeout_till_next))
                self.log.debug("time_triggers.wait", timeout=timeout_till_next)
            else:
                # no triggers, wait forever until next change
                self.log.debug("time_triggers.wait", timeout=None)

            # wait until change or until next earliest trigger
            self.active_trigger_process_wait.clear()
            await self.active_trigger_process_wait.wait()

    @transaction.atomic
    def _process_triggers(
        self,
        triggers: dict[UUID, ActiveTrigger],
        processed_up_to: datetime,
        triggers_to_fire: set[UUID],
    ) -> list[wire.RunData]:
        """
        Atomically update processed_up_to for all triggers and create Runs for firing triggers.
        """

        self.log.debug(
            "time_triggers.process",
            triggers=list(triggers.values()),
            triggers_to_fire=triggers_to_fire,
        )

        # update processed_up_to for all triggers
        for trigger in triggers.values():
            trigger.processed_up_to = processed_up_to
        models.Trigger.objects.filter(id__in=triggers.keys()).update(
            processed_up_to=processed_up_to
        )

        # create runs for fired triggers
        runs: list[wire.RunData] = []
        now = datetime.utcnow().replace(tzinfo=pytz.UTC)
        for trigger_id in triggers_to_fire:
            fired_trigger = triggers[trigger_id]
            runnable = fired_trigger.trigger.parent
            run = wire.RunData(
                id=UUIDT(),
                project_id=self.project_id,
                module_id=self.module.id,
                worker_node_id=None,
                worker_process_id=None,
                runnable_id=runnable.id,
                runnable_type=runnable.type,
                session_id=None,
                trigger_type=fired_trigger.trigger.type,
                trigger_id=fired_trigger.trigger.id,
                root_id=None,
                parent_id=None,
                created_at=now,
                updated_at=now,
                scheduled_at=fired_trigger.next_occurrence,
                started_at=None,
                terminated_at=None,
                status=RunStatus.Scheduled,
                inputs={},
                outputs=None,
                error=None,
                metadata=None,
            )
            runs.append(run)
        models.Run.objects.bulk_create([packer.unpack_data(run) for run in runs])

        return runs

    def _update_local_triggers(self):
        """Update active time triggers when the module changes."""

        # collect new (i.e. current) module's triggers
        new_active_triggers = {}
        for statement in self.module._statements_by_id.values():
            if isinstance(statement, IsFlowable) and not statement.errors:
                for trigger in statement.triggers:
                    if trigger.active and trigger.type == TriggerType.TIME:
                        new_active_triggers[trigger.id] = trigger

        # upsert triggers (if new or changed)
        new_now = datetime.utcnow()
        for new_trigger in new_active_triggers.values():
            existing_trigger = self.active_triggers.get(new_trigger.id)
            if not existing_trigger or not is_time_trigger_equal(
                existing_trigger.trigger, new_trigger
            ):
                self.active_triggers[new_trigger.id] = ActiveTrigger(
                    trigger=new_trigger,
                    iter=TriggerScheduleIterator(new_trigger, new_now),
                    next_occurrence=None,
                    processed_up_to=None,
                )
                logger.debug("time_triggers.upsert", trigger=new_trigger)

        # remove triggers that are no longer active
        for old_trigger_id in set(self.active_triggers.keys()) - set(new_active_triggers.keys()):
            removed_trigger = self.active_triggers[old_trigger_id]
            del self.active_triggers[old_trigger_id]
            logger.debug("time_triggers.remove", trigger=removed_trigger.trigger)

        # re-trigger active trigger processing
        self.active_trigger_process_wait.set()

    async def interp(self, new_source: wire.ModuleTreeData) -> None:
        """Interprets the new module source, updating the interpreted state."""

        interp_mut = await asyncio.get_event_loop().run_in_executor(
            None, partial(self._do_interp, new_source)
        )
        # save and notify interp changes
        if interp_mut.mutations:
            # TODO @Performance: ensure write resolved fields only happens if module changed
            #  This is especially important on startup because we load all the modules.
            await sync_to_async(write_mutations)(
                self.project_version, self.module_tree, interp_mut.mutations, wait_for_os=False
            )
            await publish(
                NMessageType.MODULE_CHANGED,
                ModuleChangedPayload(
                    project_id=self.project_id,
                    module_id=self.module_id,
                    origins=(self.client,),
                    mutations=interp_mut.mutations,
                ),
            )

    def _do_interp(self, new_source: wire.ModuleTreeData) -> ModuleMutator:
        """
        Re-interpret the module from the given source in place.
        Return any interpreted module state changes.
        """

        self.source = new_source
        old_module = self.module
        old_tree = self.module_tree
        self.module = Module.interp_from(module=new_source, session=None)
        self.module_tree = ModuleTree(wire.pack_module(self.module).nodes)

        # check for any interp changes
        interp_mut = ModuleMutator(new_source)
        # resolved fields
        interp_mut.tree.prune(wire.ResolvedFieldData)  # replace all resolved fields
        if old_module is None:
            interp_mut.truncate(new_source.module, MOT.RESOLVED_FIELD)
        for statement in self.module._statements_by_id.values():
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
        new_issues: dict[UUID, Issue] = {issue.id: issue for issue in self.module.issues}
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

        # update triggers
        self._update_local_triggers()

        return interp_mut

    async def on_module_changed(self, mutations: list[ModuleMutation]):
        is_semantic = any(m.type.semantic for m in mutations)
        if not is_semantic:
            # ignore non-semantic changes (will have to be smarter when we :BumpProperly)
            return
        mutator = ModuleMutator(self.source, mutations)
        new_source = mutator.to_module()
        await self.interp(new_source)

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
        await self.on_module_changed(mutations)

        # trim mutations to remove overhead from large dataset updates
        trimmed_mutations = trim_record_mutations(mutations)
        origins = (*(origins or ()), self.client)
        api_mutations = list(
            chain.from_iterable(get_api_mutation_from_internal(m) for m in trimmed_mutations)
        )
        await publish(
            NMessageType.MODULE_INTERNAL_CHANGED,
            ModuleInternalChangedPayload(
                project_id=self.project_id,
                module_id=self.module_id,
                origins=origins,
                mutations=trimmed_mutations,
            ),
        )
        await publish(
            NMessageType.MODULE_CHANGED,
            ModuleChangedPayload(
                project_id=self.project_id,
                module_id=self.module_id,
                origins=origins,
                mutations=api_mutations,
            ),
        )

    async def write_session(
        self,
        session: Optional[wire.SessionData],
        runs: list[wire.RunData] | None,
        logs: list[wire.LogEntryData] | None,
        origins: tuple[ClientOrigin] = None,
    ) -> None:
        """Write a session to the database, and publish it to the client"""
        logger.debug("write_session", session=session, runs=len(runs), logs=len(logs))
        await sync_to_async(write_session)(self.project_version, session, runs, logs)

        await publish(
            NMessageType.SESSION_CHANGED,
            SessionChangedPayload(
                project_id=self.project_id, module_id=self.module_id, session=session, runs=runs
            ),
        )
        if logs:
            await publish(
                NMessageType.LOGS_CHANGED,
                LogsChangedPayload(project_id=self.project_id, module_id=self.module_id, logs=logs),
            )
