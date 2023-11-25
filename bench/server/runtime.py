import asyncio
import json
import time
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import Optional
from uuid import UUID, uuid4

import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import ValidationError
from django.db import transaction
from more_itertools import first

from bench import models, settings
from bench.language import (
    HasDatabase,
    Module,
    SortOp,
    Statement,
    Trigger,
    TriggerType,
    libs,
    wire,
)
from bench.language.builtin import symbolx_lib
from bench.language.cache import CacheAsync
from bench.language.const import (
    INTERP_NODE_TYPES,
    MNT,
    ConditionalOp,
    ModuleReference,
    RunStatus,
    SessionAccessLevel,
    StatementType,
    parse_absolute_node_reference,
)
from bench.language.database import RecordQuery
from bench.language.edit import EditData, EditKind, NodeTreeEditor
from bench.language.expression import SCORE_KEY, C, S
from bench.language.libs import DEFAULT_MODULES
from bench.language.model import ModelError, ModelErrorType
from bench.language.packer import pack_value, unpack_value
from bench.language.run import get_run_cache_subkey
from bench.language.trigger import HasTriggers, TriggerScheduleIterator, is_time_trigger_equal
from bench.language.validation import on_issue_raise
from bench.models import Project, ProjectVersion, packer
from bench.models.packer import write_host_db_edits, write_session
from bench.models.user import loops_request
from bench.msg import NMessage
from bench.msg.core import VERSION, handle_reply, message_handler, nc_init, publish, request
from bench.msg.messages import (
    ClientOrigin,
    ModuleChangedPayload,
    NMessageType,
    ProjectChangedPayload,
    RepDownloadBlobPayload,
    RepMarkUploadedBlobPayload,
    RepPasteNodesPayload,
    RepPullWorkerRunsPayload,
    RepReadModulePayload,
    RepRevealSecretPayload,
    RepRunInferencePayload,
    RepRunStatementPayload,
    RepSearchRecordsPayload,
    RepSnapshotModulePayload,
    RepStartRunPayload,
    RepUploadBlobPayload,
    RepWakeRuntimePayload,
    RepWriteEditsPayload,
    RepWriteSessionPayload,
    ReqDownloadBlobPayload,
    ReqMarkUploadedBlobPayload,
    ReqPasteNodesPayload,
    ReqPullWorkerRunsPayload,
    ReqReadModulePayload,
    ReqRevealSecretPayload,
    ReqRunInferencePayload,
    ReqRunStatementPayload,
    ReqSearchRecordsPayload,
    ReqSnapshotModulePayload,
    ReqStartRunPayload,
    ReqUploadBlobPayload,
    ReqWakeRuntimePayload,
    ReqWriteEditsPayload,
    ReqWriteSessionPayload,
    RunsChangedGlobalPayload,
    SessionChangedPayload,
    StartRunErrorType,
)
from bench.search import mirror
from bench.search.engine import update_os_schema
from bench.server import search
from bench.server.k8 import WorkerObserver
from bench.server.search import write_edits_to_os
from bench.sql.client import async_pg_cursor
from bench.sql.engine import (
    SqlUndefinedConstruct,
    duplicate_records_in_pg,
    update_pg_schema,
    write_local_edits_to_pg,
)
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import partition
from bench.utils.monitoring import Monitored
from bench.utils.task import TaskManager
from bench.utils.utils import sentry_capture
from bench.utils.uuidt import UUIDT
from bench.worker.edit import get_api_edit_from_internal

logger = structlog.get_logger(__name__)

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
            raise RuntimeError("versioned module fetch not supported (must be head)")
        project_version = (
            await Project.objects.filter(
                slug=project,
            )
            .filter(
                models.Q(organization__owner_slug_id=owner) | models.Q(user__owner_slug_id=owner)
            )
            .select_related("head", "user", "organization")
            .aget()
        )
        project_version = project_version.head
    module = await sync_to_async(packer.pack_module)(project_version, excluded=INTERP_NODE_TYPES)
    if project_version.committed:
        _cached_modules[ref] = module, project_version.project
    return module, project_version.project


async def interp_module(ref: ModuleReference | UUID) -> tuple[Module, models.Project]:
    module, project = await read_module(ref)
    module = wire.unpack_module(module.nodes, exclude=INTERP_NODE_TYPES, session=None)
    for dependency in libs.DEFAULT_MODULES.values():
        module.add_dependency(dependency)
    module.add_builtin(symbolx_lib.files.get("builtins"))
    module._interp_rec()
    return module, project


async def fetch(ref: ModuleReference) -> wire.ModuleTreeData:
    return (await read_module(ref))[0]


def _unpack_run(run: mirror.Run):
    doc = mirror.Run.from_dict(run["_source"], run["_id"])
    return search.unpack_node_flat(doc)


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


class RuntimeSupervisor(Monitored):
    """
    Bench runtime server to host runtime hosts for each Bench.
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
            # ideally most of these should be delegated to the RuntimeHost directly
            await handle_reply(NMessageType.READ_MODULE, self.read_module),
            await handle_reply(NMessageType.WRITE_EDITS, self.write_edits),
            await handle_reply(NMessageType.PASTE_NODES, self.paste_nodes),
            await handle_reply(NMessageType.WRITE_SESSION, self.write_session),
            await handle_reply(NMessageType.PULL_WORKER_RUNS, self.pull_runs),
            await handle_reply(NMessageType.WAKE_RUNTIME, self.wake_runtime),
            await handle_reply(NMessageType.SNAPSHOT_MODULE, self.snapshot),
            await handle_reply(NMessageType.SEARCH_RECORDS, self.search_records),
            await handle_reply(NMessageType.DOWNLOAD_BLOB, self.read_blob),
            await handle_reply(NMessageType.UPLOAD_BLOB, self.write_blob),
            await handle_reply(NMessageType.MARK_UPLOADED_BLOB, self.mark_uploaded_blob),
            await handle_reply(NMessageType.REVEAL_SECRET, self.reveal_secret),
            await handle_reply(NMessageType.RUN_PROXY_INFERENCE, self.run_inference),
            await handle_reply(NMessageType.RUN_PROXY_STATEMENT, self.run_statement),
        ]

        logger.info("load_modules")
        projects = await sync_to_async(_get_projects_to_manage)()
        await asyncio.gather(*[self._prepare_runtime_host(project.head_id) for project in projects])

        await self.workers.start()

        logger.info("ready")
        self._ready = True

    @property
    def ready(self) -> bool:
        return self._ready

    @property
    def healthy(self):
        return self.ready and self.tasks.healthy

    async def _prepare_runtime_host(self, module_id: UUID) -> "RuntimeHost":
        runtime = self.runtimes.get(module_id)
        if runtime is None:
            # start language worker if not already started
            project_version = await ProjectVersion.objects.select_related(
                "project", "project__user", "project__organization"
            ).aget(id=module_id)
            project = project_version.project
            runtime = RuntimeHost(self.id, self.tasks, self.workers, project, project_version)
            self.runtimes[module_id] = runtime
            self.tasks.start(runtime.run(), f"worker-{module_id}")
        if not runtime.ready.is_set():
            await runtime.ready.wait()
        return runtime

    @message_handler
    async def read_module(self, msg: NMessage[ReqReadModulePayload]) -> None:
        logger.debug("module.read", msg=msg)
        module, project = await read_module(msg.p.ref)
        logger.debug("module.read.done", msg=msg, module=module, project=project)
        await msg.reply(
            RepReadModulePayload(
                module=module,
                project_id=project.id,
                os_name=project.os_name,
                pg_name=project.pg_name,
            )
        )

    @message_handler
    async def write_edits(self, msg: NMessage[ReqWriteEditsPayload]) -> None:
        # TODO @Security!: check if msg origin has write access to module
        runtime = await self._prepare_runtime_host(msg.p.module_id)
        try:
            edited_nodes = await runtime.write_edits(msg.p.edits, origins=(msg.p.client,))
            success = True
            error = None
        except Exception as e:
            sentry_capture(e)
            logger.error("module.write.failed", msg=msg, exc_info=True)
            edited_nodes = []
            error = str(e)
            success = False
        await msg.reply(RepWriteEditsPayload(nodes=edited_nodes, success=success, error=error))

    @message_handler
    async def paste_nodes(self, msg: NMessage[ReqPasteNodesPayload]) -> None:
        # TODO @Security!: check if msg origin has read/write access
        runtime = await self._prepare_runtime_host(msg.p.target_module_id)
        try:
            edited_nodes = await runtime.paste_nodes(
                source_module_id=msg.p.source_module_id,
                source_ids=msg.p.source_ids,
                target_ids=msg.p.target_ids,
                target_cks=msg.p.target_cks,
                target_parent_ids=msg.p.target_parent_ids,
                target_order_keys=msg.p.target_order_keys,
                origins=(msg.p.client,),
            )
            success = True
            error = None
        except Exception as e:
            sentry_capture(e)
            logger.error("module.paste.failed", msg=msg, exc_info=True)
            edited_nodes = []
            error = str(e)
            success = False
        await msg.reply(RepPasteNodesPayload(nodes=edited_nodes, success=success, error=error))

    @message_handler
    async def write_session(self, msg: NMessage[ReqWriteSessionPayload]) -> None:
        runtime = await self._prepare_runtime_host(msg.p.module_id)
        try:
            await runtime.write_session(
                session=msg.p.session, runs=msg.p.runs, origins=(msg.p.client,)
            )
            success = True
            error = None
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
            error = str(e)
        await msg.reply(RepWriteSessionPayload(success=success, error=error))

    @message_handler
    async def pull_runs(self, msg: NMessage[ReqPullWorkerRunsPayload]) -> None:
        logger.debug("run.pull", msg=msg)
        runtime = await self._prepare_runtime_host(msg.p.module_id)
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

    @message_handler
    async def snapshot(self, msg: NMessage[ReqSnapshotModulePayload]) -> None:
        logger.debug("module.snapshot", msg=msg)
        runtime = await self._prepare_runtime_host(msg.p.module_id)
        try:
            await runtime.snapshot(name=msg.p.name, tag=msg.p.tag, description=msg.p.description)
            success = True
            error = None
        except Exception as e:
            sentry_capture(e)
            logger.error("module.snapshot.failed", msg=msg, exc_info=True)
            success = False
            error = str(e)
        await msg.reply(RepSnapshotModulePayload(success=success, error=error))

    @message_handler
    async def search_records(self, msg: NMessage[ReqSearchRecordsPayload]) -> None:
        runtime = await self._prepare_runtime_host(msg.p.module_id)
        await runtime.search_records(msg)

    @message_handler
    async def read_blob(self, msg: NMessage[ReqDownloadBlobPayload]) -> None:
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
        await msg.reply(RepDownloadBlobPayload(get_urls=get_urls))

    @message_handler
    async def write_blob(self, msg: NMessage[ReqUploadBlobPayload]) -> None:
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
        await msg.reply(RepUploadBlobPayload(blobs=msg.p.blobs, post_urls=post_urls))

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
    async def reveal_secret(self, msg: NMessage[ReqRevealSecretPayload]) -> None:
        logger.debug("secret.read", msg=msg)
        # TODO @Security!!: check if msg origin has read access to secret
        secrets = []
        async for secret in models.Secret.objects.filter(id__in=(s.id for s in msg.p.secrets)):
            secret_data = packer.pack_data(secret)
            secret_data.value = json.loads(secret_data.value)  # :SecretJson
            secrets.append(secret_data)
        await msg.reply(RepRevealSecretPayload(secrets=secrets))

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
            outputs = pack_value(outputs, model, is_output=True, ignore_outer=True)
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
    async def wake_runtime(self, msg: NMessage[ReqWakeRuntimePayload]):
        logger.debug("runtime.wake", msg=msg)
        await self._prepare_runtime_host(msg.p.module_id)
        await msg.reply(RepWakeRuntimePayload(module_id=msg.p.module_id))

    async def stop(self):
        logger.info("stop")
        self._ready = False
        await asyncio.gather(*[sub.unsubscribe() for sub in self.subs])


# :MinTriggerInterval (because less than pre send window won't work)
TIME_TRIGGER_PRE_SEND_WINDOW = 45  # seconds
TIME_TRIGGER_LOOKAHEAD = 2  # occurrences


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
        project: models.Project,
        project_version: models.ProjectVersion,
    ):
        self.server_id = host_id
        self.tasks = tasks
        self.workers = workers
        self.project = project
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
    def committed(self):
        return self.project_version.committed

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
        return self.project.id

    def _new_editor(self) -> NodeTreeEditor:
        return NodeTreeEditor(
            self.module._source.deepcopy(), project_id=self.project_id, module_id=self.module_id
        )

    async def run(self) -> None:
        # fetch and interp module
        assert not self.ready.is_set(), "runtime already started"
        source, project = await read_module(self.module_ref)
        self.module = await sync_to_async(Module.make)(
            source=source.nodes,
            project_id=project.id,
            os_name=project.os_name,
            pg_name=project.pg_name,
        )
        await self._reset_interp_state()
        await update_os_schema(self.project.os_name, self.module)
        await update_pg_schema(self.project.pg_name, self.module)
        if not self.committed:  # module is only active at head...?
            await self._update_local_triggers()
            self.tasks.start(self.process_time_triggers_forever())
            self.ready.set()

    async def _publish_edits(self, edits: list[EditData], origins: tuple[ClientOrigin, ...] = None):
        edits = [get_api_edit_from_internal(e) for e in edits]
        self.log.debug("module.publish_edits", edits=edits)
        await publish(
            NMessageType.MODULE_CHANGED,
            ModuleChangedPayload(
                project_id=self.project_id,
                module_id=self.module_id,
                origins=origins,
                edits=edits,
            ),
        )

    async def _reset_interp_state(self):
        """Resets, stores and broadcasts the module's interp nodes."""
        self.log.debug("module.reset_interp_state")
        # gather interp changes (reset to 0)
        editor = self._new_editor()
        module_data = wire.pack_node_flat(self.module)
        for mnt in INTERP_NODE_TYPES:
            editor.truncate(module_data, mnt, apply=False)
        for node in self.module._nodes:
            if node.mnt in INTERP_NODE_TYPES:
                editor.create(node, apply=False)
        # write
        await sync_to_async(write_host_db_edits)(
            self.project_version,
            source=self.module._source,
            edits=editor.edits,
            raise_on_apply_error=False,
        )
        await write_edits_to_os(self.module, edits=editor.edits)
        # broadcast
        await self._publish_edits(editor.edits, origins=(self.client,))

    async def write_edits(
        self,
        edits: list[EditData],
        origins: tuple[ClientOrigin] = None,
    ) -> list[wire.NodeData]:
        """
        Writes the edits to the source of truth (DB) and locally,
         publishes the complete changes and then mirrors them into the search index.
        This is the main point of entry for ALL edits (frontend, workers, etc.);
         but record edits may bypass this and write directly to the local DB via the worker.
        TODO @Robustness: support two-phase commit for record edit :TwoPhaseCommit
        """
        if not edits:
            return []  # bail

        start_time = time.time()
        host_edits, local_edits = partition(lambda e: e.mnt == MNT.RECORD, edits)
        del edits  # refer explicitly to host/local edits
        log = self.log.bind(host_edits=host_edits, local_edits=local_edits, origins=origins)
        log.debug("module.write_edits")

        cascade_edits = []
        # expand restore edits to include all descendants from DB (where soft deleted nodes retire)
        if any(e.kind == EditKind.RESTORE for e in host_edits):
            restored_roots = packer.unpack_nodes(
                self.project_version,
                self.module._source,
                [e.node for e in host_edits if e.kind == EditKind.RESTORE],
            )
            restored = await sync_to_async(packer.pack_node)(
                *restored_roots, excluded=INTERP_NODE_TYPES
            )
            restore_edits = self._new_editor().create_many(*restored.nodes_list()).edits
            cascade_edits.extend(
                e for e in restore_edits if not any(e.node.id == r.id for r in restored_roots)
            )
            log.debug("module.write_edits.restore", restored=restore_edits)
        elif any(e.kind == EditKind.SOFT_DELETE for e in host_edits):
            pass  # TODO @Robustness: cascade soft delete to all descendants (incl. local)

        # apply TODO @Performance: don't deepcopy module on edit?
        edited_nodes: list[wire.NodeData] = []
        old_source = self.module._source.deepcopy()
        change = self.module._apply_edits(host_edits + cascade_edits, old_source=old_source)
        schema_changed = change.includes(MNT.FIELD, MNT.RESOLVED_FIELD, StatementType.DATABASE)
        try:
            # apply host edits
            # for DB, turn restore 'CREATE' edits into 'RESTORE' (since they are already in DB)
            cascaded_node_ids = {e.node.id for e in cascade_edits}
            db_edits = [
                e.to_kind(EditKind.RESTORE)
                if e.node.id in cascaded_node_ids and e.kind == EditKind.CREATE
                else e
                for e in change.all_edits
            ]
            log.debug("module.write_edits.apply", db_edits=db_edits)
            if db_edits:
                edited_host_nodes = await sync_to_async(write_host_db_edits)(
                    self.project_version,
                    source=old_source,
                    edits=db_edits,
                    raise_on_apply_error=False,  # ignore missing interp nodes (until better edits)
                )
                edited_nodes.extend(edited_host_nodes)
            # apply local edits
            if schema_changed:
                await update_pg_schema(self.project.pg_name, self.module)
            if local_edits:
                async with async_pg_cursor(self.module.pg_name) as pg_cur:
                    edited_local_nodes = await write_local_edits_to_pg(
                        pg_cur, self.module, local_edits, return_nodes=True
                    )
                edited_nodes.extend(edited_local_nodes)
        except Exception:
            # reset source & module from db on failure
            log.error("module.write_edits.failed", exc_info=True)
            old_source = await sync_to_async(packer.pack_module)(
                self.project_version, excluded=INTERP_NODE_TYPES
            )
            self.module._reset_from_source(wire.NodeTree(old_source.nodes))
            raise

        if change.includes(MNT.TRIGGER):
            await self._update_local_triggers()

        # broadcast (source from user, interp from runtime)
        await self._publish_edits(change.source_edits, origins=(*(origins or ()), self.client))
        if change.interp_edits:
            await self._publish_edits(change.interp_edits, origins=(self.client,))

        # mirror
        if schema_changed:
            await update_os_schema(self.project.os_name, self.module)
        await write_edits_to_os(self.module, edits=change.all_edits + local_edits)

        duration = time.time() - start_time
        self.log.debug("module.write_edits.done", duration=duration, edited_nodes=len(edited_nodes))
        return edited_nodes

    async def paste_nodes(
        self,
        source_module_id: UUID,
        source_ids: list[UUID],
        target_ids: dict[UUID, UUID],
        target_cks: dict[UUID, UUID],
        target_parent_ids: dict[UUID, UUID],
        target_order_keys: dict[UUID, str],
        origins: tuple[ClientOrigin] = None,
    ):
        # get copy
        same_module = source_module_id == self.module_id
        if same_module:
            source_project_v = self.project_version
        else:
            source_project_v = await ProjectVersion.objects.select_related("project").aget(
                id=source_module_id
            )
        # extend default filter to exclude template tags
        template_key = symbolx_lib.resolve(".builtins.template").key
        filter = packer.DEFAULT_PACK_FILTER.extend(
            (models.Tagging, lambda qs: qs.exclude(key=template_key))
        )
        copy = await sync_to_async(models.ProjectVersion.objects.pack_copy)(
            source=source_project_v,
            target=self.project_version,
            nodes=models.Statement.objects.filter(id__in=source_ids),
            keep_cks=False,
            excluded=packer.INTERP_MODEL_TYPES,
            target_ids=target_ids,
            target_cks=target_cks,
            copy_revisions=False,
            filter=filter,
        )
        for node in copy.nodes_by_id.values():  # patch parent and order keys
            if node.id in target_parent_ids:
                node.parent_id = target_parent_ids[node.id]
            elif node.parent_id in target_ids:
                node.parent_id = copy.target_ids[node.parent_id]
            if isinstance(node, wire.HasOrder):
                node.order_key = target_order_keys.get(node.id, node.order_key)

        # apply copy as edits
        editor = self._new_editor()
        for node in copy.nodes_by_id.values():
            editor.create(node)
        await self.write_edits(editor.edits)
        # we don't include origins because we use the edit publishing to get the results
        #  (and if we include the origin, the frontend will auto-ignore its own edits;
        #   this is faster and easier with the current API edit/load mechanism)

        # paste versioned databases (with new cks)
        target_databases: list["HasDatabase"] = [
            s
            for s in self.module._nodes
            if isinstance(s, Statement)
            and s.type == StatementType.DATABASE
            and s.versioned
            and s.id in copy.target_ids_reversed
        ]
        if not target_databases:
            return  # nothing to do
        if same_module:
            source_module = self.module
        else:
            # unfortunately we need the whole source module even though we just need a small part
            source_module, _ = await read_module(source_module_id)
            source_module = await sync_to_async(Module.make)(
                source=source_module.nodes,
                project_id=source_project_v.project_id,
                os_name=source_project_v.project.os_name,
                pg_name=source_project_v.project.pg_name,
            )
        async with async_pg_cursor(source_project_v.project.pg_name) as source_cur, async_pg_cursor(
            self.project.pg_name
        ) as target_cur:
            for target_database in target_databases:
                source_database_ck = copy.target_cks_reversed[target_database.ck]
                source_database = source_module.resolve(source_database_ck)
                await duplicate_records_in_pg(
                    source_cur=source_cur,
                    source_database=source_database,
                    target_cur=target_cur,
                    target_database=target_database,
                    where=C(ConditionalOp.NOT_EXISTS, "deleted_at"),
                    keep_cks=False,
                    copy_revisions=False,
                )

    async def write_session(
        self,
        session: Optional[wire.SessionData],
        runs: list[wire.RunData] | None,
        origins: tuple[ClientOrigin] = None,
    ) -> None:
        """Write a session and runs to the database, and publish it to the client"""
        self.log.debug("session.write", session=session, runs=len(runs))
        await sync_to_async(write_session)(self.project_version, session, runs)

        await publish(
            NMessageType.SESSION_CHANGED,
            SessionChangedPayload(
                project_id=self.project_id, module_id=self.module_id, session=session, runs=runs
            ),
        )

    def _do_snapshot_host(
        self, name: str | None, tag: str | None, description: str | None
    ) -> models.ProjectVersion:
        """Snapshots all host Bench nodes (excluding local records)."""
        snapshot = models.ProjectVersion.objects.create(
            id=uuid4(),
            ck=self.project.id,
            project=self.project,
            name=name,
            tag=tag,
            description=description,
            committed_at=utcnow_with_tz(),
        )

        # actually copy into new version
        models.ProjectVersion.objects.copy(
            source=self.project_version,
            target=snapshot,
            keep_cks=True,
            copy_revisions=True,
            include_interp=True,
        )
        return snapshot

    def _do_insert_snapshot(self, snapshot: models.ProjectVersion):
        # insert new head between parents and head
        snapshot.parents.set(self.project_version.parents.all())
        self.project_version.parents.set([snapshot])

    async def snapshot(self, name: str | None, tag: str | None, description: str | None) -> None:
        """
        Snapshot the module and publish a corresponding project change.
        Records from versioned local databases are also copied into the snapshot.
        TODO @UX: warn if large databases are snapshotted implicitly ('versioned')
        """

        # make snapshot (host)
        logger.info("module.snapshot", module=self.module, name=name, tag=tag)
        snapshot = await sync_to_async(self._do_snapshot_host)(
            name=name, tag=tag, description=description
        )

        try:
            # get snapshot module (to get Database instances, probably can be more efficient...)
            target_module, _ = await read_module(snapshot.id)
            target_module = await sync_to_async(Module.make)(
                source=target_module.nodes,
                project_id=self.project.id,
                os_name=self.project.os_name,
                pg_name=self.project.pg_name,
            )
            logger.debug("module.snapshot.clone", snapshot=snapshot, target_module=target_module)

            # copy local records from source databases into target (the snapshot)
            # (later we'll probably also add non-versioned ids to 'back up' here)
            target_databases: list["HasDatabase"] = [
                s
                for s in target_module._nodes
                if isinstance(s, Statement) and s.type == StatementType.DATABASE and s.versioned
            ]
            async with async_pg_cursor(self.module.pg_name, autocommit=False) as pg_cursor:
                for target_database in target_databases:
                    source_database = self.module.resolve(target_database.ck)
                    await duplicate_records_in_pg(
                        source_cur=pg_cursor,
                        source_database=source_database,
                        target_cur=pg_cursor,
                        target_database=target_database,
                        where=C(ConditionalOp.NOT_EXISTS, "deleted_at"),
                        keep_cks=True,
                        copy_revisions=True,
                    )
                await pg_cursor.connection.commit()

            # insert snapshot into project
            await sync_to_async(self._do_insert_snapshot)(snapshot)
        except Exception:
            # rollback
            logger.error("module.snapshot.failed", snapshot=snapshot, exc_info=True)
            await models.ProjectVersion.objects.filter(id=snapshot.id).adelete()
            raise
        finally:
            # always notify (we did create a snapshot, so it's possible someone read it)
            await publish(
                NMessageType.PROJECT_CHANGED,
                ProjectChangedPayload(project_id=self.project.id, origins=[self.client]),
            )

    async def search_records(self, msg: NMessage[ReqSearchRecordsPayload]) -> None:
        """
        Search records in this module (for the frontend client).
        (Full module state is needed to query the local database).
        """
        try:
            filter = wire.unpack_data(msg.p.query, self.module) if msg.p.query else None
            sort = [wire.unpack_data(s, self.module) for s in msg.p.sort] if msg.p.sort else None
            if not sort and filter is not None and filter.is_scored:
                sort = [S(SortOp.DESCENDING, field=SCORE_KEY)]

            database = self.module.resolve(msg.p.statement_ck)
            query = RecordQuery(
                database=database,
                filter=filter,
                sort=sort,
                first=msg.p.limit,
            )
            query._interp_self(database, on_issue=on_issue_raise)
            self.log.debug("records.search", query=query)
            async with async_pg_cursor(self.module.pg_name) as pg_cursor:
                fetched = await query._do_fetch(
                    pg_cursor=pg_cursor, count=msg.p.count, after=msg.p.after
                )
            rep = RepSearchRecordsPayload(
                records=fetched.records,
                cursors=fetched.cursors,
                total=fetched.total,
                limit=msg.p.limit,
                engine=fetched.engine,
            )
            self.log.debug(
                "records.search.done", records=len(fetched.records), engine=fetched.engine
            )
        except SqlUndefinedConstruct as e:
            # not created yet... should wait in UI?
            self.log.warn("records.search.undefined", database=msg.p.statement_ck, e=e)
            rep = RepSearchRecordsPayload(
                records=[], cursors=[], total=0, limit=msg.p.limit, engine=None
            )
        except Exception as e:
            sentry_capture(e)
            self.log.error("records.search.failed", req=msg.p, exc_info=True)
            rep = RepSearchRecordsPayload(
                records=None, cursors=None, total=None, limit=msg.p.limit, error=str(e), engine=None
            )
        await msg.reply(rep)

    #
    # Triggers
    #

    async def pull_runs(
        self, worker_set_id: UUID, worker_node_id: Optional[str], worker_process_id: Optional[str]
    ) -> list[wire.RunData]:
        """Pull any runs potentially missed by the worker."""
        prescheduled_runs = [
            r
            async for r in models.Run.objects.filter(
                status=RunStatus.Scheduled, project_id=self.project_id
            ).order_by("scheduled_at")
        ]
        prescheduled_runs = [packer.pack_data(r) for r in prescheduled_runs]
        return prescheduled_runs

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
