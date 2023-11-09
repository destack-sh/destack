import asyncio
import json
import typing
from dataclasses import dataclass
from datetime import datetime, timedelta
from itertools import chain
from typing import Optional
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import ValidationError
from django.db import transaction
from more_itertools import first

from bench import models, settings
from bench.language import Module, Q, Query, QueryOp, Trigger, TriggerType, wire
from bench.language.builtin import symbolx_lib
from bench.language.cache import CacheAsync
from bench.language.const import (
    INTERP_NODE_TYPES,
    ModuleReference,
    RunStatus,
    SessionAccessLevel,
    parse_absolute_node_reference,
)
from bench.language.edit import EditData, ModuleEditor
from bench.language.libs import DEFAULT_MODULES
from bench.language.model import ModelError, ModelErrorType
from bench.language.module import ModuleChange
from bench.language.packer import pack_value, unpack_value
from bench.language.run import get_run_cache_subkey
from bench.language.trigger import HasTriggers, TriggerScheduleIterator, is_time_trigger_equal
from bench.models import Project, ProjectVersion, packer
from bench.models.packer import write_edits, write_session
from bench.models.user import loops_request
from bench.msg import NMessage
from bench.msg.core import (
    VERSION,
    handle_reply,
    message_handler,
    nc_init,
    publish,
    request,
    subscribe,
)
from bench.msg.messages import (
    ClientOrigin,
    LogsChangedPayload,
    ModuleChangedPayload,
    ModuleInternalChangedPayload,
    NMessageType,
    RepGetModuleHeadPayload,
    RepMarkUploadedBlobPayload,
    RepPullWorkerRunsPayload,
    RepReadBlobPayload,
    RepReadModulePayload,
    RepReadSecretPayload,
    RepRunInferencePayload,
    RepRunStatementPayload,
    RepSearch,
    RepSearchLogPayload,
    RepSearchRecordsPayload,
    RepSearchRunPayload,
    RepStartRunPayload,
    RepWakeRuntimePayload,
    RepWriteModulePayload,
    RepWriteObjectPayload,
    RepWriteSessionPayload,
    ReqGetModuleHeadPayload,
    ReqMarkUploadedBlobPayload,
    ReqPullWorkerRunsPayload,
    ReqReadBlobPayload,
    ReqReadModulePayload,
    ReqReadSecretPayload,
    ReqRunInferencePayload,
    ReqRunStatementPayload,
    ReqSearch,
    ReqSearchLogPayload,
    ReqSearchRecordsPayload,
    ReqSearchRunsPayload,
    ReqStartRunPayload,
    ReqWakeRuntimePayload,
    ReqWriteBlobPayload,
    ReqWriteModulePayload,
    ReqWriteSessionPayload,
    RunsChangedGlobalPayload,
    SessionChangedPayload,
    StartRunErrorType,
)
from bench.search import mirror
from bench.search.client import os_client
from bench.search.mapping import encode_cursor, prepare_search
from bench.server.observer import WorkerObserver
from bench.utils.dt import utcnow_with_tz
from bench.utils.monitoring import Monitored
from bench.utils.task import TaskManager
from bench.utils.utils import sentry_capture
from bench.utils.uuidt import UUIDT
from bench.worker.edit import get_api_edit_from_internal, trim_record_edits

logger = structlog.get_logger(__name__)

MAX_SEARCH_RECORDS_LIMIT = 500
MAX_SEARCH_LOG_LIMIT = 1000
MAX_SEARCH_RUN_LIMIT = 1000

_cached_modules: dict[ModuleReference | UUID, tuple[wire.ModuleTreeData, models.Project]] = {}


async def read_module(ref: ModuleReference | UUID) -> tuple[wire.ModuleTreeData, models.Project]:
    if ref in _cached_modules:
        return _cached_modules[ref]
    id = ref if isinstance(ref, UUID) else ref.id
    if id:
        project_version = await ProjectVersion.objects.aget(id=id)
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
    module = await sync_to_async(packer.pack_module)(
        project_version, excluded=[models.Record, models.ResolvedField, models.Issue]
    )
    if project_version.committed:
        _cached_modules[ref] = module, project_version.project
    return module, project_version.project


async def fetch(ref: ModuleReference) -> wire.ModuleTreeData:
    return (await read_module(ref))[0]


record_packer = mirror.get_node_packer(mirror.Record)
run_packer = mirror.get_node_packer(mirror.Run)
log_packer = mirror.get_node_packer(mirror.LogEntry)


def _unpack_record(record: mirror.Record):
    doc = mirror.Record.from_dict(record["_source"], record["_id"])
    return record_packer.pack(doc)


def _unpack_run(run: mirror.Run):
    doc = mirror.Run.from_dict(run["_source"], run["_id"])
    return run_packer.pack(doc)


def _unpack_log(log: mirror.LogEntry):
    doc = mirror.LogEntry.from_dict(log["_source"], log["_id"])
    return log_packer.pack(doc)


def _get_projects_to_manage() -> list[models.Project]:
    projects = list(Project.objects.all())  # obviously will shard this later

    # sanity check if default libs in code match those in DB
    for lib in DEFAULT_MODULES.values():
        project = first((p for p in projects if p.id == lib.ck), None)
        if project is None:
            raise RuntimeError(f"default lib {lib} not found in DB")
        if project.head_id != lib.id:
            raise RuntimeError(f"default lib {lib} head mismatch with {project}: {project.head}")
    # and then exclude default projects since there's nothing to manage
    projects = [p for p in projects if p.path not in DEFAULT_MODULES]

    return projects


class RuntimeServer(Monitored):
    """
    Bench runtime server to host per-module runtime workers that
     proxy user worker module access (read/write) and process triggers.
    """

    def __init__(self):
        self.id = UUIDT()
        self.runtimes: dict[UUID, RuntimeHost] = {}
        self.subs = []
        self.tasks = TaskManager()
        self.workers = WorkerObserver()
        self._ready = False

    async def run(self):
        await nc_init.wait()
        logger.info("start")
        self.subs = [
            await handle_reply(f"{NMessageType.GET_MODULE_HEAD}.>", self.get_module_head),
            await handle_reply(NMessageType.READ_MODULE, self.read_module),
            await handle_reply(NMessageType.WRITE_MODULE, self.write_module),
            await handle_reply(NMessageType.WRITE_SESSION, self.write_session),
            await handle_reply(NMessageType.PULL_WORKER_RUNS, self.pull_runs),
            await handle_reply(NMessageType.WAKE_RUNTIME, self.request_runtime),
            await handle_reply(NMessageType.SEARCH_RECORDS, self.search_records),
            await handle_reply(NMessageType.SEARCH_RUNS, self.search_runs),
            await handle_reply(NMessageType.SEARCH_LOGS, self.search_log),
            await handle_reply(NMessageType.READ_BLOB, self.read_blob),
            await handle_reply(NMessageType.WRITE_BLOB, self.write_blob),
            await handle_reply(NMessageType.MARK_UPLOADED_BLOB, self.mark_uploaded_blob),
            await handle_reply(NMessageType.READ_SECRET, self.read_secret),
            await handle_reply(NMessageType.RUN_PROXY_INFERENCE, self.run_inference),
            await handle_reply(NMessageType.RUN_PROXY_STATEMENT, self.run_statement),
            await subscribe(f"{NMessageType.MODULE_INTERNAL_CHANGED}.>", cb=self.module_changed),
        ]

        logger.info("load_modules")
        projects = await sync_to_async(_get_projects_to_manage)()
        await asyncio.gather(*[self._prepare_runtime(project.head_id) for project in projects])

        await self.workers.start()

        logger.info("ready")
        self._ready = True

    @property
    def ready(self) -> bool:
        return self._ready

    @property
    def healthy(self):
        return self.ready and self.tasks.healthy

    async def _prepare_runtime(self, module_id: UUID) -> "RuntimeHost":
        runtime = self.runtimes.get(module_id)
        if runtime is None:
            # start language worker if not already started
            project_version = await ProjectVersion.objects.select_related(
                "project", "project__user", "project__organization"
            ).aget(id=module_id)
            runtime = RuntimeHost(self.id, self.tasks, self.workers, project_version)
            self.runtimes[module_id] = runtime
            self.tasks.start(runtime.run(), f"worker-{module_id}")
        if not runtime.ready.is_set():
            await runtime.ready.wait()
        return runtime

    @message_handler
    async def get_module_head(self, msg: NMessage[ReqGetModuleHeadPayload]) -> None:
        logger.debug("module.head", msg=msg)
        project = await Project.objects.select_related("user", "organization").aget(
            id=msg.p.project_id
        )
        await msg.reply(RepGetModuleHeadPayload(module_id=project.head_id))

    @message_handler
    async def read_module(self, msg: NMessage[ReqReadModulePayload]) -> None:
        logger.debug("module.read", msg=msg)
        module, project = await read_module(msg.p.ref)
        logger.debug("module.read.done", msg=msg, module=module, project=project)
        await msg.reply(RepReadModulePayload(module=module, project_id=project.id))

    @message_handler
    async def write_module(self, msg: NMessage[ReqWriteModulePayload]) -> None:
        logger.debug("module.write", msg=msg)
        # TODO @Security!: check if msg origin has write access to module
        runtime = await self._prepare_runtime(msg.p.module_id)
        try:
            await runtime.write_module(
                msg.p.edits, origins=(msg.p.client,), refresh_index=msg.p.refresh_index
            )
            logger.debug("module.write.done", msg=msg)
            success = True
        except Exception as e:
            sentry_capture(e)
            logger.error("module.write.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepWriteModulePayload(success=success))

    @message_handler
    async def write_session(self, msg: NMessage[ReqWriteSessionPayload]) -> None:
        logger.debug("session.write", msg=msg, client=msg.p.client)
        runtime = await self._prepare_runtime(msg.p.module_id)
        try:
            await runtime.write_session(
                msg.p.session, msg.p.runs, msg.p.logs, origins=(msg.p.client,)
            )
            logger.debug("session.write.done", msg=msg)
            success = True
        except Exception as e:
            sentry_capture(e)
            logger.error(
                "session.write.failed",
                msg=msg,
                session=msg.p.session,
                runs=msg.p.runs,
                exc_info=True,
            )
            success = False
        await msg.reply(RepWriteSessionPayload(success=success))

    @message_handler
    async def pull_runs(self, msg: NMessage[ReqPullWorkerRunsPayload]) -> None:
        logger.debug("run.pull", msg=msg)
        runtime = await self._prepare_runtime(msg.p.module_id)
        try:
            runs = await runtime.pull_runs(
                worker_set_id=msg.p.worker_set_id,
                worker_node_id=msg.p.worker_node_id,
                worker_process_id=msg.p.worker_process_id,
            )
            logger.debug("run.pull.done", msg=msg, runs=runs)
            success = True
        except Exception as e:
            sentry_capture(e)
            logger.error("run.pull.failed", msg=msg, exc_info=True)
            success = False
            runs = []
        await msg.reply(RepPullWorkerRunsPayload(runs=runs, success=success))

    def _do_search(
        self,
        project_v: models.ProjectVersion,
        type: Optional[mirror.DocumentType],
        extra_query: Optional[Query],
        max_limit: int,
        req: ReqSearch,
        unpack: typing.Callable,
        rep_cls: typing.Type[RepSearch],
    ) -> RepSearch:
        effective_limit = min(req.limit, max_limit)
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
            results = os_client.search(index=project_v.project.os_name, body=search)
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
            sentry_capture(e)
            logger.error("database.search.failed", req=req, exc_info=True)
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
    async def search_records(self, msg: NMessage[ReqSearchRecordsPayload]) -> None:
        logger.debug("search.record", msg=msg)
        extra_queries = []
        if msg.p.statement_keys:
            extra_queries.append(Q(QueryOp.EQUALS, "statement_key", value=msg.p.statement_keys))
        # TODO @Security!: check if msg origin has read access to database
        project_v = await ProjectVersion.objects.aget(id=msg.p.module_id)
        rep = await sync_to_async(self._do_search)(
            project_v=project_v,
            extra_query=Q(QueryOp.AND, extra_queries) if extra_queries else None,
            type=mirror.DocumentType.RECORD,
            max_limit=MAX_SEARCH_RECORDS_LIMIT,
            req=msg.p,
            unpack=_unpack_record,
            rep_cls=RepSearchRecordsPayload,
        )
        await msg.reply(rep)

    @message_handler
    async def search_runs(self, msg: NMessage[ReqSearchRunsPayload]) -> None:
        logger.debug("search.run", msg=msg)
        # TODO @Security!: check if msg origin has read access to database
        extra_queries = []
        if msg.p.statements_ids:
            extra_queries.append(Q(QueryOp.EQUALS, "statement_id", value=msg.p.statements_ids))
        if msg.p.statements_cks:
            extra_queries.append(Q(QueryOp.EQUALS, "statement_ck", value=msg.p.statements_cks))
        project_v = await ProjectVersion.objects.aget(id=msg.p.module_id)
        rep = await sync_to_async(self._do_search)(
            project_v=project_v,
            extra_query=Q(QueryOp.AND, extra_queries) if extra_queries else None,
            type=mirror.DocumentType.RUN,
            max_limit=MAX_SEARCH_RUN_LIMIT,
            req=msg.p,
            unpack=_unpack_run,
            rep_cls=RepSearchRunPayload,
        )
        await msg.reply(rep)

    @message_handler
    async def search_log(self, msg: NMessage[ReqSearchLogPayload]) -> None:
        logger.debug("search.log", msg=msg)
        extra_queries = []
        if msg.p.statements_ids:
            extra_queries.append(Q(QueryOp.EQUALS, "statement_id", value=msg.p.statements_ids))
        if msg.p.statements_cks:
            extra_queries.append(Q(QueryOp.EQUALS, "statement_ck", value=msg.p.statements_cks))
        # TODO @Security!: check if msg origin has read access to database
        project_v = await ProjectVersion.objects.aget(id=msg.p.module_id)
        rep = await sync_to_async(self._do_search)(
            project_v=project_v,
            extra_query=Q(QueryOp.AND, extra_queries) if extra_queries else None,
            type=mirror.DocumentType.LOG_ENTRY,
            max_limit=MAX_SEARCH_LOG_LIMIT,
            req=msg.p,
            unpack=_unpack_log,
            rep_cls=RepSearchLogPayload,
        )
        await msg.reply(rep)

    @message_handler
    async def read_blob(self, msg: NMessage[ReqReadBlobPayload]) -> None:
        logger.debug("blob.read", msg=msg)
        # TODO @Security!: check if msg origin has read access to object
        get_urls: list[str | None] = []
        async for model_obj in models.Blob.objects.filter(id__in=(obj.id for obj in msg.p.blobs)):
            model_obj: models.Blob
            obj_data = msg.p.blobs[len(get_urls)]
            if obj_data.sha512 != model_obj.sha512:
                logger.warning(
                    "blob.read.sha512_mismatch", msg=msg, obj=model_obj, obj_data=obj_data
                )
                get_urls.append(None)
            else:
                get_urls.append(model_obj.presigned_get)
        logger.debug("blob.read.rep", msg=msg, get_urls=[url is not None for url in get_urls])
        await msg.reply(RepReadBlobPayload(get_urls=get_urls))

    @message_handler
    async def write_blob(self, msg: NMessage[ReqWriteBlobPayload]) -> None:
        logger.debug("blob.write", msg=msg)
        # TODO @Security!: check if msg origin has write access to object
        project_v = await ProjectVersion.objects.select_related("project").aget(id=msg.p.module_id)
        post_urls: list[str | None] = []
        for obj_data in msg.p.blobs:
            model_blob: models.Blob = packer.unpack_data(obj_data)
            model_blob.project_id = project_v.project_id
            existing_blob = await project_v.project.blobs.filter(sha512=model_blob.sha512).afirst()
            if existing_blob is not None:
                obj_data.id = existing_blob.id
                if existing_blob.status == models.BlobStatus.AVAILABLE:
                    obj_data.status = models.BlobStatus.AVAILABLE
                    post_urls.append(None)
                else:
                    existing_blob.generate_presigned_post()
                    post_urls.append(existing_blob.presigned_post)
            else:
                model_blob.generate_presigned_post()
                post_urls.append(model_blob.presigned_post)
                await model_blob.asave()  # create
        logger.debug("blob.write.rep", msg=msg, post_urls=[url is not None for url in post_urls])
        await msg.reply(RepWriteObjectPayload(blobs=msg.p.blobs, post_urls=post_urls))

    @message_handler
    async def mark_uploaded_blob(self, msg: NMessage[ReqMarkUploadedBlobPayload]) -> None:
        logger.debug("blob.mark_uploaded", msg=msg)
        try:
            for obj_data in msg.p.blobs:
                blob: models.Blob = await models.Blob.objects.aget(id=obj_data.id)
                blob.mark_available_if_exists_in_s3()
                await blob.asave()
            success = True
        except ValidationError:
            logger.error("blob.mark_uploaded.failed", msg=msg, exc_info=True)
            success = False
        await msg.reply(RepMarkUploadedBlobPayload(success=success))

    @message_handler
    async def read_secret(self, msg: NMessage[ReqReadSecretPayload]) -> None:
        logger.debug("secret.read", msg=msg)
        # TODO @Security!!: check if msg origin has read access to secret
        secrets = []
        async for secret in models.Secret.objects.filter(id__in=(s.id for s in msg.p.secrets)):
            secret_data = packer.pack_data(secret)
            secret_data.value = json.loads(secret_data.value)  # :SecretJson
            secrets.append(secret_data)
        await msg.reply(RepReadSecretPayload(secrets=secrets))

    @message_handler
    async def run_inference(self, msg: NMessage[ReqRunInferencePayload]) -> None:
        module_name, localized_path = parse_absolute_node_reference(msg.p.model_path)
        log = logger.bind(model=msg.p.model_path, msg=msg)
        try:
            log.debug("inference.run")
            # remotely proxied inference if the worker doesn't have the required model api key
            # we call the underlying model implementation directly (the worker does the tracing)
            # :LibImplementation
            module = DEFAULT_MODULES[module_name]
            model = module.resolve(localized_path)
            cache_subkey = get_run_cache_subkey(inputs_raw=msg.p.inputs)
            log = log.bind(cache_subkey=cache_subkey)
            cache = CacheAsync(module=None, subkey=model.ck.hex, project_id=msg.p.project_id)
            inputs = unpack_value(msg.p.inputs, model, is_output=False)
            inference = model._inference(
                inputs=inputs,
                cache_subkey=cache_subkey,
                log=log,
                cache=cache,
                run_id=msg.p.run_id,
            )
            outputs = await asyncio.wait_for(asyncio.shield(inference), msg.p.timeout)
            outputs = pack_value(outputs, model, is_output=True, ignore_outer_map=True)
            error = None
        except Exception as e:
            log.error("inference.exception", exc_info=True, sentry=sentry_capture(e))
            outputs = None
            if isinstance(e, asyncio.TimeoutError):
                error = ModelErrorType.Timeout
            elif isinstance(e, ModelError):
                error = e.type
            else:
                error = ModelErrorType.Unknown
        await msg.reply(RepRunInferencePayload(outputs=outputs, error=error))

    @message_handler
    async def run_statement(self, msg: NMessage[ReqRunStatementPayload]) -> None:
        statement = symbolx_lib.resolve(msg.p.statement)
        log = logger.bind(statement=msg.p.statement, msg=msg)
        try:
            log.debug("statement.run")
            inputs = unpack_value(msg.p.inputs, statement, is_output=False)
            if statement.name == "send email":
                if not await models.User.objects.filter(email=inputs["to"]).aexists():
                    raise RuntimeError(f"{inputs['to']} is not a Bench user")
                byline = f"<br><br><i>Sent via {msg.p.module_name} (Bench {VERSION})</i>"
                inputs["body"] = inputs["body"] + byline
                if not settings.LOCAL:
                    loops_request(
                        "POST",
                        "transactional",
                        {
                            "email": inputs["to"],
                            "transactionalId": settings.LOOPS_USER_TRANSACTIONAL_ID,
                            "dataVariables": {
                                "subject": inputs["subject"],
                                "body": inputs["body"],
                            },
                        },
                    )
                else:
                    log.warning("statement.run.local", inputs=inputs)
                outputs = {}
            else:
                raise RuntimeError(f"unknown proxy statement {statement}")
            error = None
        except Exception as e:
            log.error("statement.exception", exc_info=True, sentry=sentry_capture(e))
            outputs = None
            error = f"{e.__class__.__name__}: {e}"
        await msg.reply(RepRunStatementPayload(outputs=outputs, error=error))

    @message_handler
    async def request_runtime(self, msg: NMessage[ReqWakeRuntimePayload]):
        logger.debug("runtime.wake", msg=msg)
        await self._prepare_runtime(msg.p.module_id)
        await msg.reply(RepWakeRuntimePayload(module_id=msg.p.module_id))

    @message_handler
    async def module_changed(self, msg: NMessage[ModuleInternalChangedPayload]) -> None:
        logger.debug("module.changed", msg=msg)
        if msg.p.has_origin(self.id):
            return  # ignore own changes
        runtime = await self._prepare_runtime(msg.p.module_id)
        await runtime.apply_edits(msg.p.edits)

    async def stop(self):
        logger.info("stop")
        self._ready = False
        await asyncio.gather(*[sub.unsubscribe() for sub in self.subs])


# :MinTriggerInterval (because less than pre send window won't work)
TIME_TRIGGER_PRE_SEND_WINDOW = 45  # seconds
TIME_TRIGGER_LOOKAHEAD = 2  # occurrences


async def run_at(func: typing.Callable[[], typing.Awaitable[None]], at: datetime) -> None:
    """Run a function at a given time."""
    delay = (at - utcnow_with_tz()).total_seconds()
    if delay < 0:
        delay = 0
    await asyncio.sleep(delay)
    await func()


@dataclass
class ActiveTrigger:
    trigger: Trigger
    processed_up_to: Optional[datetime]
    next_occurrence: Optional[datetime]
    iter: TriggerScheduleIterator


class RuntimeHost:
    """
    Runtime host for a single module.
    Assumed to run as a singleton per module, mainly to ensure time triggers are processed
     with hopefully exactly once / definitely at least once semantics (later also OTs).
     We may separate those parts into some elected 'main' worker later for scalability.
    """

    def __init__(
        self,
        host_id: UUID,
        tasks: TaskManager,
        workers: WorkerObserver,
        project_version: models.ProjectVersion,
    ):
        self.server_id = host_id
        self.tasks = tasks
        self.workers = workers
        self.project_version = project_version
        self.ready = asyncio.Event()
        self.log = logger.bind(
            module_id=str(self.module_id),
            project_id=str(self.project_id),
            worker_id=str(self.server_id),
            module=self.project_version,
        )
        self.module: Optional[Module] = None
        # time triggers
        self.active_triggers: dict[UUID, ActiveTrigger] = {}
        self.active_trigger_process_wait: asyncio.Event = asyncio.Event()

    def __str__(self):
        return f"{self.project_version.project.path} {self.project_version.id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}"

    @property
    def client(self) -> ClientOrigin:
        return ClientOrigin("runtime-host", self.server_id, None)

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
        assert not self.ready.is_set(), "runtime already started"
        source, project = await read_module(self.module_ref)
        self.module = await sync_to_async(Module.interp)(source.nodes, project.id)
        await self._on_module_changed(change=None)
        self.tasks.start(self.process_time_triggers_forever())
        self.ready.set()

    async def pull_runs(
        self, worker_set_id: UUID, worker_node_id: Optional[str], worker_process_id: Optional[str]
    ) -> list[wire.RunData]:
        prescheduled_runs = [
            r
            async for r in models.Run.objects.filter(
                status=RunStatus.Scheduled, project_id=self.project_id
            ).order_by("scheduled_at")
        ]
        prescheduled_runs = [packer.pack_data(r) for r in prescheduled_runs]
        return prescheduled_runs

    async def process_time_triggers_forever(self) -> None:
        """
        Process all time triggers for this module forever.
        As noted above, this is assumed to run once per module.
        """

        async def set_wait(after: float) -> None:
            await asyncio.sleep(after)
            self.active_trigger_process_wait.set()

        triggers_to_fire: set[UUID] = set()
        runs_to_start: dict[UUID, wire.RunData] = {}
        timed_wait_task: Optional[asyncio.Task] = None

        # backfill time triggers on first go (coalescing to at most one per trigger)
        for trigger in self.active_triggers.values():
            if (
                trigger.processed_up_to is not None
                and trigger.iter.last_occurrence_initial > trigger.processed_up_to
                and trigger.iter.last_occurrence_initial > trigger.trigger.updated_at
            ):
                # should we ignore backfill here if just edited (updated_at > processed_up_to)?
                triggers_to_fire.add(trigger.trigger.id)

        # backfill previously 'scheduled' runs that failed to start (also coalescing)
        prescheduled_runs = [
            r
            async for r in models.Run.objects.filter(
                status=RunStatus.Scheduled, project_id=self.project_id
            ).order_by("scheduled_at")
        ]
        latest_prescheduled_run_by_trigger: dict[UUID, UUID] = {
            r.trigger_id: r.id for r in prescheduled_runs
        }
        coalesced_runs: list[models.Run] = []
        for run in prescheduled_runs:
            if (
                run.trigger_id not in triggers_to_fire
                and run.id == latest_prescheduled_run_by_trigger[run.trigger_id]
            ):
                runs_to_start[run.id] = packer.pack_data(run)
            else:
                run.mark_dead()
                coalesced_runs.append(run)
        await models.Run.objects.abulk_update(coalesced_runs, fields=("status", "terminated_at"))
        coalesced_runs = [packer.pack_data(r) for r in coalesced_runs]
        await publish(NMessageType.RUNS_CHANGED, RunsChangedGlobalPayload(runs=coalesced_runs))
        logger.debug(
            "time_triggers.backfill", runs_to_start=runs_to_start, coalesced_runs=coalesced_runs
        )

        self.log.debug("time_triggers.process_forever", initial_triggers_to_fire=triggers_to_fire)

        # enter process triggers forever loop
        while True:
            if timed_wait_task is not None:
                timed_wait_task.cancel()

            process_up_to = utcnow_with_tz() + timedelta(seconds=TIME_TRIGGER_PRE_SEND_WINDOW)
            self.log.debug(
                "time_triggers.check",
                process_up_to=process_up_to,
                active_triggers=self.active_triggers.values(),
            )

            # TODO @Robustness @UX: cancel pre-scheduled runs that no longer have an active trigger

            # collect triggers that are due to fire within the send window
            for trigger in self.active_triggers.values():
                while (
                    trigger.next_occurrence is None
                    or trigger.processed_up_to is not None
                    and trigger.next_occurrence <= trigger.processed_up_to
                ):
                    trigger.next_occurrence = trigger.iter.next()
                if trigger.next_occurrence <= process_up_to:
                    triggers_to_fire.add(trigger.trigger.id)

            # "process" triggers (write to DB and create runs atomically)
            next_runs_to_start: list[wire.RunData] = await sync_to_async(self._process_triggers)(
                triggers=self.active_triggers,
                triggers_to_fire=triggers_to_fire,
                processed_up_to=process_up_to,
            )
            for run in next_runs_to_start:
                runs_to_start[run.id] = run

            # publish scheduled runs (should be project scoped later, but we don't have a session)
            await publish(
                NMessageType.RUNS_CHANGED,
                RunsChangedGlobalPayload(runs=list(runs_to_start.values())),
            )

            if runs_to_start:
                # start worker set if not already started
                if not self.workers.is_healthy(self.project_id):
                    # not sure what to do after timeout here... retry? panic?
                    await self.workers.wake_until_healthy(self.project_id, timeout=300)
                logger.debug(
                    "time_triggers.fire",
                    runs_to_start=runs_to_start,
                    fired_triggers=[self.active_triggers[id].trigger for id in triggers_to_fire],
                    worker_set=self.workers.get(self.project_id),
                )

            # send out run requests (could do this in parallel but doesn't matter for now)
            for run in runs_to_start.values():
                req = ReqStartRunPayload(
                    project_id=run.project_id,
                    module_id=run.module_id,
                    session_id=run.session_id,
                    run_id=run.id,
                    statement=run.statement_id,
                    inputs=run.inputs,
                    block=None,
                    keyed=True,
                    trigger_type=run.trigger_type,
                    trigger_id=run.trigger_id,
                    scheduled_at=run.scheduled_at,
                    root_value=run.value,
                    access_level=run.access_level,
                )
                try:
                    rep: NMessage[RepStartRunPayload] = await request(
                        NMessageType.START_RUN, req, reply_t=RepStartRunPayload, retry=3
                    )
                    if rep.p.error and rep.p.error != StartRunErrorType.ALREADY_PREPARED:
                        # already prepared is okay
                        raise RuntimeError(rep.p.error)
                except Exception as e:
                    sentry_capture(e)
                    logger.error("time_triggers.start_run.error", run=run, exc_info=e)
                    # mark run as cancelled
                    run_model = await models.Run.objects.aget(id=run.id)
                    run_model.mark_dead()
                    await run_model.asave()
                    await publish(NMessageType.RUNS_CHANGED, RunsChangedGlobalPayload(runs=[run]))

            # reset next occurrence for all triggers that fired
            for trigger_id in triggers_to_fire:
                trigger = self.active_triggers[trigger_id]
                trigger.next_occurrence = trigger.iter.next()
            triggers_to_fire.clear()
            runs_to_start.clear()

            # wait for earliest next trigger occurrence (or trigger change)
            if self.active_triggers:
                first_occurrence = min(t.next_occurrence for t in self.active_triggers.values())
                timeout = (
                    first_occurrence - utcnow_with_tz()
                ).total_seconds() - TIME_TRIGGER_PRE_SEND_WINDOW
                if timeout < 0:
                    logger.warning("time_triggers.wait.overdue", timeout=timeout)
                    continue  # immediately go to next iteration
                asyncio.create_task(set_wait(timeout))
                self.log.debug("time_triggers.wait", timeout=timeout)
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
        now = utcnow_with_tz()
        for trigger_id in triggers_to_fire:
            fired_trigger = triggers[trigger_id]
            statement = fired_trigger.trigger.parent
            run = wire.RunData(
                id=UUIDT(),
                project_id=self.project_id,
                module_id=self.module.id,
                worker_node_id=None,
                worker_process_id=None,
                statement_id=statement.id,
                statement_type=statement.type,
                statement_ck=statement.ck,
                statement_path=None,
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
                access_level=SessionAccessLevel.Full,
                inputs={},
                outputs=None,
                error=None,
                value=None,
            )
            runs.append(run)
        models.Run.objects.bulk_create([packer.unpack_data(run) for run in runs])

        return runs

    async def _update_local_triggers(self):
        """Update active time triggers when the module changes."""

        # collect new (i.e. current) module's triggers
        new_active_triggers = {}
        for node in self.module._nodes:
            if HasTriggers in node._components and not node.errors:
                for trigger in node.triggers:
                    if trigger.active and trigger.type == TriggerType.TIME:
                        new_active_triggers[trigger.id] = trigger

        # upsert triggers (if new or changed)
        new_now = utcnow_with_tz()
        for new_trigger in new_active_triggers.values():
            existing_trigger = self.active_triggers.get(new_trigger.id)
            if not existing_trigger or not is_time_trigger_equal(
                existing_trigger.trigger, new_trigger
            ):
                processed_up_to = await (
                    models.Trigger.objects.filter(id=new_trigger.id)
                    .values_list("processed_up_to", flat=True)
                    .afirst()
                )
                self.active_triggers[new_trigger.id] = ActiveTrigger(
                    trigger=new_trigger,
                    iter=TriggerScheduleIterator(new_trigger, new_now),
                    next_occurrence=None,
                    processed_up_to=processed_up_to,
                )
                logger.debug("time_triggers.upsert", trigger=new_trigger)
            elif existing_trigger:
                # preserve existing active trigger, just update trigger reference
                self.active_triggers[new_trigger.id].trigger = new_trigger

        # remove triggers that are no longer active
        for old_trigger_id in set(self.active_triggers.keys()) - set(new_active_triggers.keys()):
            removed_trigger = self.active_triggers[old_trigger_id]
            del self.active_triggers[old_trigger_id]
            logger.debug("time_triggers.remove", trigger=removed_trigger.trigger)

        # re-trigger active trigger processing
        self.active_trigger_process_wait.set()

    async def _on_module_changed(self, change: Optional[ModuleChange]):
        """Handle module changes to store interp state, update triggers, etc."""

        # interp state
        start_time = utcnow_with_tz()
        if change is None:  # reset completely
            interp_mut = ModuleEditor(self.module._source, self.project_id, self.module_id)
            module_data = wire.pack_node_flat(self.module)
            for mnt in INTERP_NODE_TYPES:
                interp_mut.truncate(module_data, mnt, apply=False)
            for node in self.module._nodes:
                # config fields are only used internally for now
                # :InterpEditFilter
                if node.mnt in INTERP_NODE_TYPES:
                    interp_mut.create(node, apply=False)
            interp_edits = interp_mut.edits
        else:
            interp_edits = change.interp_edits
        if interp_edits:
            await sync_to_async(write_edits)(
                self.project_version,
                self.module._source,
                interp_edits,
                validate=False,
                refresh_index=False,
                apply=False,
            )
            await publish(
                NMessageType.MODULE_CHANGED,
                ModuleChangedPayload(
                    project_id=self.project_id,
                    module_id=self.module_id,
                    origins=(self.client,),
                    edits=interp_edits,
                ),
            )
            duration = (utcnow_with_tz() - start_time).total_seconds()
            self.log.debug("runtime.interp", total=len(interp_edits), duration=duration)

        # triggers
        if change is None or any(isinstance(n, Trigger) for n in change.touched):
            await self._update_local_triggers()

    async def apply_edits(self, edits: list[EditData]) -> None:
        """Apply external edits to the module."""
        start_time = utcnow_with_tz()
        change = self.module._apply_edits(edits)
        duration = (utcnow_with_tz() - start_time).total_seconds()
        self.log.debug("runtime.apply_edits", total=len(edits), duration=duration)
        await self._on_module_changed(change)

    async def write_module(
        self,
        edits: list[EditData],
        origins: tuple[ClientOrigin] = None,
        refresh_index: bool = False,
    ):
        self.log.debug(
            "module.write",
            edits=edits[:5],
            total=len(edits),
            origins=origins,
            refresh_index=refresh_index,
        )

        # apply in DB/OS
        await sync_to_async(write_edits)(
            self.project_version,
            self.module._source,
            edits,
            validate=True,
            refresh_index=refresh_index,
        )
        if not edits:
            # edits may be empty if we just want to trigger an index refresh
            # e.g. on record search preflight in session after a non-refresh flush happened
            return

        # broadcast
        trimmed_edits = trim_record_edits(edits)
        origins = (*(origins or ()), self.client)
        api_edits = list(chain.from_iterable(get_api_edit_from_internal(e) for e in trimmed_edits))
        await publish(
            NMessageType.MODULE_INTERNAL_CHANGED,
            ModuleInternalChangedPayload(
                project_id=self.project_id,
                module_id=self.module_id,
                origins=origins,
                edits=trimmed_edits,
            ),
        )
        await publish(
            NMessageType.MODULE_CHANGED,
            ModuleChangedPayload(
                project_id=self.project_id,
                module_id=self.module_id,
                origins=origins,
                edits=api_edits,
            ),
        )

        # apply locally (after broadcast to ensure interp edits are delivered after source edits)
        await self.apply_edits(edits)

    async def write_session(
        self,
        session: Optional[wire.SessionData],
        runs: list[wire.RunData] | None,
        logs: list[wire.LogEntryData] | None,
        origins: tuple[ClientOrigin] = None,
    ) -> None:
        """Write a session to the database, and publish it to the client"""
        self.log.debug("session.write", session=session, runs=len(runs), logs=len(logs))
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
