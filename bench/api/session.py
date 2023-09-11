from datetime import datetime
from typing import TYPE_CHECKING, Annotated, AsyncGenerator, Optional
from uuid import UUID

import django.db.models
import strawberry
import strawberry_django
import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from django.db.models import OuterRef, Subquery
from nats.errors import NoRespondersError
from strawberry import auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_project_access
from bench.api.statement import Statement
from bench.api.utils import (
    ListConnectionWithTotalCount,
    QueryOp,
    SearchQuery,
    SearchSort,
    asafe_mutation,
    asafe_subscription,
    get_user_from_info,
    to_global_id,
    to_uuid,
    to_uuids,
)
from bench.language import Q, Query, Sort, SortOrder, wire
from bench.language.const import RUNNABLE_STATEMENT_TYPES
from bench.language.session import PENDING_RUN_STATUSES
from bench.models import ProjectAccessLevel, packer
from bench.msg.core import MessagingError, NMessage, request, subscribe, subscribe_many
from bench.msg.messages import (
    LogsChangedPayload,
    NMessageType,
    RepGetEnvironmentPayload,
    RepKillRunPayload,
    RepRestartWorkerSetPayload,
    RepStartRunPayload,
    RepWakeRuntimePayload,
    RepWakeWorkerSetPayload,
    ReqGetEnvironmentPayload,
    ReqKillRunPayload,
    ReqRestartWorkerSetPayload,
    ReqStartRunPayload,
    ReqWakeRuntimePayload,
    ReqWakeWorkerSetPayload,
    RunsChangedGlobalPayload,
    SessionChangedPayload,
    StartRunErrorType,
    WorkersChangedPayload,
)
from bench.opensearch import mirror
from bench.opensearch.client import os_client
from bench.opensearch.core import IndexType
from bench.opensearch.query import encode_cursor, prepare_search

if TYPE_CHECKING:
    from bench.api.project import Project, ProjectVersion
    from bench.api.statement import Trigger
    from bench.api.token import AccessToken
    from bench.api.user import User

logger = structlog.get_logger(__name__)

RunStatus = strawberry.enum(models.RunStatus)
TriggerType = strawberry.enum(models.TriggerType)


@strawberry.type
class RunCodeFrame:
    filename: str
    lineno: int
    name: str
    line: str = None
    locals: Optional[JSON] = None

    @staticmethod
    def from_dict(data: dict) -> "RunCodeFrame":
        return RunCodeFrame(
            filename=data["filename"],
            lineno=data["lineno"],
            name=data["name"],
            line=data.get("line"),
            locals=data.get("locals"),
        )


@strawberry.type
class RunError:
    """Wire-able representation of an exception."""

    kind: str
    type: str
    message: str
    statement_id: Optional[GlobalID]
    traceback: Optional[list[RunCodeFrame]]

    @staticmethod
    def from_dict(data: dict) -> "RunError":
        statement_id = (
            to_global_id("Statement", data.get("statement_id"))
            if data.get("statement_id")
            else None
        )
        traceback = [RunCodeFrame.from_dict(frame) for frame in data.get("traceback") or []] or None
        return RunError(
            kind=data["kind"],
            type=data["type"],
            message=data["message"],
            statement_id=statement_id,
            traceback=traceback,
        )


def get_error_nice(root: "Run") -> Optional[RunError]:
    if root.error:
        return RunError.from_dict(data=root.error)
    else:
        return None


WorkerProfile = strawberry.enum(models.WorkerProfile)
WorkerRegion = strawberry.enum(models.WorkerRegion)
WorkerSetStatus = strawberry.enum(models.WorkerSetStatus)


@strawberry_django.type(models.WorkerSet)
class WorkerSet(relay.Node):
    project: Annotated["Project", lazy(".project")]
    created_at: auto
    updated_at: auto
    profile: WorkerProfile
    region: WorkerRegion
    sleeping: bool
    status: WorkerSetStatus
    desired_replicas: int
    target_replicas: int
    available_replicas: int
    ready_replicas: int
    last_bumped_at: auto
    last_active_at: auto


@strawberry.interface
class HasTriggeredBy:
    trigger_type: Optional[TriggerType]
    trigger_user: Optional[Annotated["User", lazy(".user")]]
    trigger_access_token: Optional[Annotated["AccessToken", lazy(".token")]]
    trigger: Optional[Annotated["Trigger", lazy(".statement")]]


@strawberry_django.type(models.Session)
class Session(HasTriggeredBy, relay.Node):
    project: Annotated["Project", lazy(".project")]
    created_at: auto
    updated_at: auto
    opened_at: auto
    closed_at: auto
    metadata: Optional[JSON]
    runs: list["Run"]
    # trigger
    trigger_type: Optional[TriggerType]
    trigger_user: Optional[Annotated["User", lazy(".user")]]
    trigger_access_token: Optional[Annotated["AccessToken", lazy(".token")]]
    trigger: Optional[Annotated["Trigger", lazy(".statement")]]


@strawberry_django.type(models.Run)
class Run(HasTriggeredBy, relay.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    session: Optional[Session]
    root: Optional["Run"]
    parent: Optional["Run"]
    children: list["Run"]
    descendants: list["Run"]
    runnable: Optional["Statement"]
    runnable_ck: auto
    created_at: auto
    updated_at: auto
    started_at: auto
    terminated_at: auto
    duration: auto
    status: RunStatus
    inputs: auto
    outputs: auto
    error: auto
    error_nice: Optional[RunError] = strawberry_django.field(
        only=["error"], resolver=get_error_nice
    )
    metadata: auto
    # trigger
    trigger_type: Optional[TriggerType]
    trigger_user: Optional[Annotated["User", lazy(".user")]]
    trigger_access_token: Optional[Annotated["AccessToken", lazy(".token")]]
    trigger: Optional[Annotated["Trigger", lazy(".statement")]]


@strawberry.type
class LogEntry:
    id: GlobalID
    project_version_id: GlobalID
    created_at: datetime
    stream: str
    level: Optional[str]
    logger: Optional[str]
    message: Optional[str]
    session_id: Optional[GlobalID]
    runnable_id: Optional[GlobalID]
    runnable_ck: Optional[UUID]
    run_id: Optional[GlobalID]
    metadata: Optional[JSON]

    @staticmethod
    def from_os(log_entry: mirror.LogEntry) -> "LogEntry":
        return LogEntry(
            id=to_global_id("LogEntry", log_entry.id),
            project_version_id=to_global_id("ProjectVersion", log_entry.project_version_id),
            created_at=log_entry.created_at,
            stream=log_entry.stream,
            level=log_entry.level,
            logger=log_entry.logger,
            message=log_entry.message,
            session_id=to_global_id("Session", log_entry.session_id),
            runnable_id=to_global_id("Statement", log_entry.runnable_id),
            runnable_ck=log_entry.runnable_ck,
            run_id=to_global_id("Run", log_entry.run_id),
            metadata=log_entry.metadata,
        )

    @staticmethod
    def from_data(log_entry: wire.LogEntryData) -> "LogEntry":
        return LogEntry(
            id=to_global_id("LogEntry", log_entry.id),
            project_version_id=to_global_id("ProjectVersion", log_entry.module_id),
            created_at=log_entry.created_at,
            stream=log_entry.stream,
            level=log_entry.level,
            logger=log_entry.logger,
            message=log_entry.message,
            session_id=to_global_id("Session", log_entry.session_id),
            runnable_id=to_global_id("Statement", log_entry.runnable_id),
            runnable_ck=log_entry.runnable_ck,
            run_id=to_global_id("Run", log_entry.run_id),
            metadata=log_entry.metadata,
        )


RUNS_LIMIT = 50
LOGS_LIMIT = 200


@strawberry.type
class SessionState:
    worker_set: Optional[WorkerSet]
    runs: list[Run]


@strawberry.input
class WakeRuntimeInput:
    project_version_id: GlobalID


@strawberry.type
class WakeRuntimePayload:
    success: bool


@strawberry.input
class WakeWorkerSetInput:
    project_id: GlobalID


@strawberry.type
class WakeWorkerSetPayload:
    worker_set: Optional[WorkerSet]
    success: bool


@strawberry.input
class RestartWorkerSetInput:
    project_id: GlobalID


@strawberry.type
class RestartWorkerSetPayload:
    worker_set: Optional[WorkerSet]
    success: bool


@strawberry.input
class RunInput:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID] = None
    run_id: Optional[GlobalID] = None
    session_id: Optional[GlobalID] = None
    inputs: Optional[JSON] = None
    block: float = 1.0
    keyed: bool = False
    timeout_seconds: Optional[int] = None


ModuleRunErrorType = strawberry.enum(StartRunErrorType)


@strawberry.type
class RunState:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID]
    success: bool
    error: Optional[ModuleRunErrorType]
    run: Optional[Run]
    logs: Optional[list[LogEntry]]


@strawberry.input
class KillRunInput:
    project_version_id: GlobalID
    run_id: GlobalID


@strawberry.type
class KillRunPayload:
    success: bool
    run: Optional[Run]


@strawberry.type
class Package:
    name: str
    version: str


@strawberry.type
class Environment:
    language: str
    version: str
    platform: str
    packages: list[Package]


@strawberry.type
class SessionQuery:
    @strawberry_django.field
    def current_runs(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: GlobalID,
    ) -> SessionState | OperationInfo:
        project_id = to_uuid(project_id)
        project_version_id = to_uuid(project_version_id)
        project = models.Project.objects.get(id=project_id)
        check_project_access(info, project, ProjectAccessLevel.Read)

        runnable_statements_ids = models.Statement.objects.filter(
            deleted_at=None,
            project_version_id=project_version_id,
            type__in=RUNNABLE_STATEMENT_TYPES,
        ).values_list("id", flat=True)

        # subquery to get the latest run per runnable_id
        latest_runs = models.Run.objects.filter(
            runnable_id=OuterRef("pk"), project_id=project_id
        ).order_by("-updated_at")

        # get ids of the latest runs for each runnable statement
        latest_run_ids = (
            models.Statement.objects.filter(id__in=runnable_statements_ids)
            .annotate(
                latest_run_id=Subquery(latest_runs.values("id")[:1]),
            )
            .values_list("latest_run_id", flat=True)
        )
        latest_run_instances = models.Run.objects.filter(
            django.db.models.Q(id__in=latest_run_ids)
            | (
                django.db.models.Q(status__in=PENDING_RUN_STATUSES)
                & django.db.models.Q(project_id=project_id)
            )
        )
        return SessionState(worker_set=project.worker_set, runs=latest_run_instances)

    session: Optional[Session] = strawberry_django.node(extensions=[])
    run: Optional[Run] = strawberry_django.node(extensions=[])

    @strawberry_django.field
    async def environment(self, info: Info, project_id: GlobalID) -> Environment | OperationInfo:
        project_id = UUID(project_id.node_id)
        project = await models.Project.objects.select_related("worker_set").aget(id=project_id)
        await sync_to_async(check_project_access)(info, project, ProjectAccessLevel.Read)
        try:
            rep: NMessage[RepGetEnvironmentPayload] = await request(
                NMessageType.GET_ENVIRONMENT,
                ReqGetEnvironmentPayload(project_id=project_id),
                reply_t=RepGetEnvironmentPayload,
            )
            environment_data = rep.p.environment
        except (TimeoutError, RuntimeError, MessagingError, NoRespondersError) as e:
            # worker unavailable, return default environment
            logger.debug("environment.failed", project_id=project_id, exc_info=e)
            from bench.worker.environment import WORKER_ENVIRONMENT_DATA

            environment_data = WORKER_ENVIRONMENT_DATA

        return Environment(
            language=environment_data.language,
            version=environment_data.version,
            platform=environment_data.platform,
            packages=[
                Package(name=name, version=version)
                for name, version in environment_data.packages.items()
            ],
        )

    @strawberry_django.field
    def search_runs(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: GlobalID,
        session_id: Optional[GlobalID] = None,
        run_id: Optional[GlobalID] = None,
        runnable_ids: Optional[list[GlobalID]] = None,
        runnable_cks: Optional[list[UUID]] = None,
        root_only: Optional[bool] = None,
        query: Optional[SearchQuery] = None,
        sort: Optional[list[SearchSort]] = None,
        after: Optional[str] = None,
        limit: Optional[int] = None,
        count: Optional[bool] = None,
    ) -> ListConnectionWithTotalCount[Run]:
        project = models.Project.objects.get(id=to_uuid(project_id))
        project_version_id = to_uuid(project_version_id)
        session_id = to_uuid(session_id)
        run_id = to_uuid(run_id)
        runnable_ids = to_uuids(runnable_ids)
        check_project_access(info, project, ProjectAccessLevel.Read)

        query = query.to_dsl() if query else None
        if session_id:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "session_id", session_id))
        if run_id:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "run_id", run_id))
        if runnable_ids:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "runnable_id", runnable_ids))
        if runnable_cks:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "runnable_ck", runnable_cks))
        if root_only:
            query = Query.and_if_set(query, Q(QueryOp.DOES_NOT_EXIST, "parent_id"))
        effective_limit = min(limit or RUNS_LIMIT, RUNS_LIMIT)
        sort = [s.to_dsl() for s in sort] if sort else [Sort("created_at", SortOrder.DESCENDING)]

        logger.debug("runs.search", project_id=project_id, query=query, sort=sort)
        # query id only and then fetch full run from DB
        search = prepare_search(
            type=mirror.DocumentType.RUN,
            project_version_id=str(project_version_id) if project_version_id else None,
            limit=effective_limit + 1,  # +1 to determine if there is a next page
            count=count or False,
            after=after,
            sort=sort,
            query=query,
            fields=[],
            source=False,
        )
        os_results = os_client.search(
            index=IndexType.BENCH.get_index_name(project_id=project.id), body=search
        )

        logger.debug("runs.search.db", project_id=project_id, hits=len(os_results["hits"]["hits"]))
        edges = []
        run_ids = [r["_id"] for r in os_results["hits"]["hits"]]
        runs = models.Run.objects.filter(id__in=run_ids).prefetch_related(
            "trigger", "trigger_user", "trigger_access_token"
        )
        if len(runs) != len(run_ids):
            logger.warning("runs.search.db.missing", project_id=project_id, runs=run_ids)
            # some runs have been deleted, we need to filter them out
            runs = [r for r in runs if r.id in run_ids]
        logger.debug("runs.search.resolve", project_id=project_id, hits=len(runs))
        for i, r in enumerate(os_results["hits"]["hits"][0:effective_limit]):
            cursor = encode_cursor(r, after, i)
            edges.append(relay.Edge(node=runs[i], cursor=cursor))
            # pres-set related fields where we know we only need the id
            run = runs[i]
            run.project_version = models.ProjectVersion(id=run.project_version_id)
            run.session = models.Session(id=run.session_id) if run.session_id else None
            run.runnable = models.Statement(id=run.runnable_id)
            run.root = models.Run(id=run.root_id) if run.root_id else None
            run.parent = models.Run(id=run.parent_id) if run.parent_id else None

        page_info = relay.PageInfo(
            start_cursor=edges[0].cursor if edges else None,
            end_cursor=edges[-1].cursor if edges else None,
            has_next_page=len(os_results["hits"]["hits"]) > effective_limit,
            has_previous_page=False,
        )
        total_count = os_results["hits"]["total"]["value"] if count else None
        logger.debug("runs.search.done", project_id=project_id, total_count=total_count)
        return ListConnectionWithTotalCount(
            edges=edges, page_info=page_info, total_count=total_count
        )

    @strawberry_django.field
    def search_logs(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID] = None,
        session_id: Optional[GlobalID] = None,
        run_id: Optional[GlobalID] = None,
        runnable_ids: Optional[list[GlobalID]] = None,
        runnable_cks: Optional[list[UUID]] = None,
        query: Optional[SearchQuery] = None,
        sort: Optional[list[SearchSort]] = None,
        after: Optional[str] = None,
        limit: Optional[int] = None,
        count: Optional[bool] = None,
    ) -> ListConnectionWithTotalCount[LogEntry]:
        project = models.Project.objects.get(id=to_uuid(project_id))
        project_version_id = to_uuid(project_version_id)
        session_id = to_uuid(session_id)
        run_id = to_uuid(run_id)
        runnable_ids = to_uuids(runnable_ids)
        check_project_access(info, project, ProjectAccessLevel.Read)

        sort = [s.to_dsl() for s in sort] if sort else [Sort("created_at", SortOrder.DESCENDING)]
        query = query.to_dsl() if query else None
        if session_id:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "session_id", str(session_id)))
        if run_id:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "run_id", str(run_id)))
        if runnable_ids:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "runnable_id", str(runnable_ids)))
        if runnable_cks:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "runnable_ck", runnable_cks))
        effective_limit = min(limit or LOGS_LIMIT, LOGS_LIMIT)
        search = prepare_search(
            type=mirror.DocumentType.LOG_ENTRY,
            project_version_id=str(project_version_id) if project_version_id else None,
            limit=effective_limit + 1,  # +1 to determine if there is a next page
            count=count or False,
            after=after,
            sort=sort,
            query=query,
            version=True,
        )

        results = os_client.search(
            index=IndexType.BENCH.get_index_name(project_id=project.id), body=search
        )

        edges = []
        for i, r in enumerate(results["hits"]["hits"][0:effective_limit]):
            doc = mirror.LogEntry.from_dict(r["_source"], r["_id"], r["_version"])
            node = LogEntry.from_os(doc)
            cursor = encode_cursor(r, after, i)
            edge = relay.Edge(node=node, cursor=cursor)
            edges.append(edge)
        page_info = relay.PageInfo(
            start_cursor=edges[0].cursor if edges else None,
            end_cursor=edges[-1].cursor if edges else None,
            has_next_page=len(results["hits"]["hits"]) > effective_limit,
            has_previous_page=False,
        )
        total_count = results["hits"]["total"]["value"] if count else None
        return ListConnectionWithTotalCount(
            edges=edges, page_info=page_info, total_count=total_count
        )


@strawberry.type
class SessionMutation:
    @asafe_mutation
    async def wake_runtime(
        self, info: Info, input: WakeRuntimeInput
    ) -> WakeRuntimePayload | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        await sync_to_async(check_project_access)(info, project_version, ProjectAccessLevel.Read)
        await request(
            NMessageType.WAKE_RUNTIME,
            ReqWakeRuntimePayload(module_id=project_version_id),
            reply_t=RepWakeRuntimePayload,
            retry=3,
        )
        return WakeRuntimePayload(success=True)

    @asafe_mutation
    async def wake_worker_set(
        self, info: Info, input: WakeWorkerSetInput
    ) -> WakeWorkerSetPayload | OperationInfo:
        project_id = UUID(input.project_id.node_id)
        project = await models.Project.objects.aget(id=project_id)
        await sync_to_async(check_project_access)(info, project, ProjectAccessLevel.Use)
        rep: NMessage[RepWakeWorkerSetPayload] = await request(
            NMessageType.WAKE_WORKER_SET,
            ReqWakeWorkerSetPayload(project_id=project_id),
            reply_t=RepWakeWorkerSetPayload,
            retry=3,
        )
        if rep.payload.success:
            worker_set = await models.WorkerSet.objects.aget(id=rep.p.worker_set_id)
        else:
            worker_set = None
        return WakeWorkerSetPayload(worker_set=worker_set, success=rep.p.success)

    @asafe_mutation
    async def restart_worker_set(
        self, info: Info, input: RestartWorkerSetInput
    ) -> RestartWorkerSetPayload | OperationInfo:
        project_id = UUID(input.project_id.node_id)
        project = await models.Project.objects.aget(id=project_id)
        await sync_to_async(check_project_access)(info, project, ProjectAccessLevel.Use)
        rep: NMessage[RepRestartWorkerSetPayload] = await request(
            NMessageType.RESTART_WORKER_SET,
            ReqRestartWorkerSetPayload(project_id=project_id),
            reply_t=RepRestartWorkerSetPayload,
            retry=3,
        )
        if rep.payload.success and rep.p.worker_set_id:
            worker_set = await models.WorkerSet.objects.aget(id=rep.p.worker_set_id)
        else:
            worker_set = None
        return RestartWorkerSetPayload(worker_set=worker_set, success=rep.p.success)

    @asafe_mutation
    async def run(self, info: Info, input: RunInput) -> RunState | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = get_user_from_info(info)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        await sync_to_async(check_project_access)(info, project_version, ProjectAccessLevel.Use)

        run = ReqStartRunPayload(
            project_id=project_version.project_id,
            module_id=project_version_id,
            runnable=to_uuid(input.runnable_id),
            inputs=input.inputs,
            scheduled_at=None,
            block=input.block,
            trigger_type=TriggerType.USER,
            trigger_id=user.id,
            run_id=to_uuid(input.run_id),
            session_id=to_uuid(input.session_id),
            keyed=input.keyed,
        )
        try:
            rep: NMessage[RepStartRunPayload] = await request(
                NMessageType.START_RUN,
                run,
                reply_t=RepStartRunPayload,
                timeout=input.timeout_seconds,
                retry=2,
            )
            success = rep.p.error is None
            error = rep.p.error
        except MessagingError as e:
            rep = None
            success = False
            if isinstance(e.__cause__, TimeoutError):
                error = StartRunErrorType.TIMEOUT
            else:
                error = StartRunErrorType.UNAVAILABLE
        run = packer.unpack_data(rep.p.run) if rep and rep.p.run else None
        logs = [LogEntry.from_data(log) for log in rep.p.logs] if rep and rep.p.logs else None
        return RunState(
            project_version_id=input.project_version_id,
            runnable_id=input.runnable_id,
            success=success,
            error=error,
            run=run,
            logs=logs,
        )

    @asafe_mutation
    async def kill_run(self, info: Info, input: KillRunInput) -> KillRunPayload | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        await sync_to_async(check_project_access)(info, project_version, ProjectAccessLevel.Use)
        kill = ReqKillRunPayload(
            project_id=project_version.project_id,
            module_id=project_version_id,
            run_id=to_uuid(input.run_id),
        )
        try:
            rep: NMessage[RepKillRunPayload] = await request(
                NMessageType.KILL_RUN, kill, reply_t=RepKillRunPayload, retry=2
            )
            run = await models.Run.objects.filter(id=kill.run_id).afirst()
            success = rep.p.success
        except (NoRespondersError, TimeoutError, MessagingError):
            run = None
            success = False
        return KillRunPayload(success=success, run=run)


@strawberry.type
class LogChange:
    logs: list[LogEntry]


@strawberry.type
class SessionChange:
    session: Optional[Session]
    runs: list[Run]


@strawberry.type
class RunsChange:
    runs: list[Run]


@strawberry.type
class WorkerChange:
    worker_sets: list[WorkerSet]


@strawberry.type
class SessionSubscription:
    @asafe_subscription
    async def sessions_changed(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID] = None,
    ) -> AsyncGenerator[SessionChange | RunsChange | WorkerChange, None]:
        project_id = UUID(project_id.node_id)
        project_version_id = UUID(project_version_id.node_id)
        user = get_user_from_info(info)
        log = logger.bind(project_id=project_id, project_version_id=project_version_id, user=user)
        try:
            await sync_to_async(check_project_access)(info, project_id, ProjectAccessLevel.Read)
        except PermissionDenied:
            log.warn("sessions.subscribe_denied", exc_info=True)
            return

        def _filter_run(run: wire.RunData) -> bool:
            if run.project_id != project_id:
                return False
            if project_version_id is not None and run.module_id != project_version_id:
                return False
            return True

        log.info("sessions.subscribe")
        routing_id = f"{project_id}.{project_version_id or '*'}"
        sub = await subscribe_many(
            {
                f"{NMessageType.SESSION_CHANGED}.{routing_id}": SessionChangedPayload,
                f"{NMessageType.SESSION_CHANGED}.all": SessionChangedPayload,
                f"{NMessageType.RUNS_CHANGED}": SessionChangedPayload,
                f"{NMessageType.WORKERS_CHANGED}.{project_id}": WorkersChangedPayload,
                f"{NMessageType.WORKERS_CHANGED}.all": WorkersChangedPayload,
            },
        )
        while True:
            msg: NMessage[
                SessionChangedPayload | RunsChangedGlobalPayload | WorkersChangedPayload
            ] = await sub.next_msg()
            if isinstance(msg.p, WorkersChangedPayload):
                if msg.p.project_id is not None:
                    worker_sets = [
                        packer.unpack_data(ws)
                        for ws in msg.p.worker_sets
                        if ws.project_id == project_id
                    ]
                elif msg.p.project_id == project_id:
                    worker_sets = [packer.unpack_data(ws) for ws in msg.p.worker_sets]
                else:
                    continue
                log.debug("workers.update", msg=msg)
                yield WorkerChange(worker_sets=worker_sets)
            elif isinstance(msg.p, SessionChangedPayload):
                log.debug("sessions.update", msg=msg)
                yield SessionChange(
                    session=packer.unpack_data(msg.p.session) if msg.p.session else None,
                    runs=[packer.unpack_data(r) for r in msg.p.runs],
                )
            elif isinstance(msg.p, RunsChangedGlobalPayload):
                log.debug("runs.update", msg=msg)
                # filter runs to only those in the project
                runs = [packer.unpack_data(r) for r in msg.p.runs if _filter_run(r)]
                if runs:
                    yield RunsChange(runs=runs)
            else:
                raise RuntimeError(f"unexpected message type {msg}")

    @asafe_subscription
    async def logs_changed(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID] = None,
        session_id: Optional[GlobalID] = None,
        run_id: Optional[GlobalID] = None,
        runnable_ids: Optional[list[GlobalID]] = None,
        runnable_cks: Optional[list[UUID]] = None,
    ) -> AsyncGenerator[LogChange, None]:
        project_id = to_uuid(project_id)
        project_version_id = to_uuid(project_version_id)
        session_id = to_uuid(session_id)
        run_id = to_uuid(run_id)
        runnable_ids = to_uuids(runnable_ids)
        user = get_user_from_info(info)
        log = logger.bind(project_id=project_id, project_version_id=project_version_id, user=user)
        try:
            await sync_to_async(check_project_access)(info, project_id, ProjectAccessLevel.Read)
        except PermissionDenied:
            log.warn("sessions.subscribe_denied", exc_info=True)
            return

        def _filter_log(log: wire.LogEntryData) -> bool:
            if project_version_id and log.module_id != project_version_id:
                return False
            if session_id and log.session_id != session_id:
                return False
            if run_id and log.run_id != run_id:
                return False
            if runnable_ids and log.runnable_id not in runnable_ids:
                return False
            if runnable_cks and log.runnable_ck not in runnable_cks:
                return False
            return True

        log.info("logs.subscribe")
        routing_id = f"{project_id}.{project_version_id or '*'}"
        logs_sub = await subscribe(
            f"{NMessageType.LOGS_CHANGED}.{routing_id}", payload_t=LogsChangedPayload
        )
        while True:
            msg: NMessage[LogsChangedPayload] = await logs_sub.next_msg()
            logs = [LogEntry.from_data(e) for e in msg.p.logs if _filter_log(e)]
            if not logs:
                continue
            log.debug("logs.update", msg=msg, logs=len(logs))
            yield LogChange(logs=logs)
