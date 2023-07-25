from datetime import datetime
from typing import TYPE_CHECKING, Annotated, AsyncGenerator, Optional
from uuid import UUID

import django.db.models
import posthog
import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied, ValidationError
from django.db.models import OuterRef, Subquery
from nats.errors import NoRespondersError
from strawberry import auto, lazy
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo
from strawberry_django_plus.utils.resolvers import async_safe

from bench import models
from bench.api.auth import (
    CanViewProject,
    check_can_read_project,
    check_can_view_project_by_id,
    check_can_write_project,
)
from bench.api.statement import Statement
from bench.api.utils import (
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
from bench.models import packer
from bench.msg.core import NMessage, request, subscribe, subscribe_many
from bench.msg.messages import (
    LogsChangedPayload,
    NMessageType,
    RepCancelRunPayload,
    RepGetEnvironmentPayload,
    RepRestartWorkerSetPayload,
    RepStartRunPayload,
    RepWakeLangserverPayload,
    RepWakeWorkerSetPayload,
    ReqCancelRunPayload,
    ReqGetEnvironmentPayload,
    ReqRestartWorkerSetPayload,
    ReqStartRunPayload,
    ReqWakeLangserverPayload,
    ReqWakeWorkerSetPayload,
    RunErrorType,
    SessionChangedPayload,
    WorkersChangedPayload,
)
from bench.opensearch import mirror
from bench.opensearch.client import os_client
from bench.opensearch.core import IndexType
from bench.opensearch.query import encode_cursor, prepare_search

if TYPE_CHECKING:
    from bench.api.project import Project, ProjectVersion
    from bench.api.token import AccessToken
    from bench.api.user import User

logger = structlog.get_logger(__name__)

RunStatus = gql.enum(models.RunStatus)
RunTriggerType = gql.enum(models.RunTriggerType)


@gql.type
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


@gql.type
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
        traceback = [RunCodeFrame.from_dict(frame) for frame in data.get("traceback", [])] or None
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


WorkerProfile = gql.enum(models.WorkerProfile)
WorkerRegion = gql.enum(models.WorkerRegion)
WorkerSetStatus = gql.enum(models.WorkerSetStatus)


@gql.django.type(models.WorkerSet)
class WorkerSet(gql.Node):
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
    last_active_at: auto


@gql.django.type(models.Session)
class Session(gql.Node):
    project: Annotated["Project", lazy(".project")]
    created_at: auto
    updated_at: auto
    opened_at: auto
    closed_at: auto
    metadata: Optional[JSON]
    runs: list["Run"]
    # trigger
    trigger_type: RunTriggerType
    user: Optional[Annotated["User", lazy(".user")]]
    access_token: Optional[Annotated["AccessToken", lazy(".token")]]


@gql.django.type(models.Run)
class Run(gql.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    session: Session
    root: Optional["Run"]
    parent: Optional["Run"]
    children: list["Run"]
    descendants: list["Run"]
    runnable: Optional["Statement"]
    created_at: auto
    updated_at: auto
    started_at: auto
    terminated_at: auto
    duration: auto
    status: RunStatus
    inputs: auto
    outputs: auto
    error: auto
    error_nice: Optional[RunError] = gql.django.field(only=["error"], resolver=get_error_nice)
    metadata: auto


@gql.type
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
            run_id=to_global_id("Run", log_entry.run_id),
            metadata=log_entry.metadata,
        )


async def _expand_filter(
    project_version_id: UUID,
    include_ancestor_versions: bool,
    runnable_ids: list[UUID] | None,
    ancestor_depth: int = 8,
):
    if include_ancestor_versions:
        if project_version_id is None:
            raise ValidationError(
                "project_version_id must be specified if include_ancestor_versions"
            )
            # TODO @Performance: implement symbol version id expansion in SQL
        project_versions = await sync_to_async(models.ProjectVersion.objects.get_ancestors)(
            version_id=project_version_id, depth=ancestor_depth
        )
        project_version_ids = [pv.id for pv in project_versions]
        # expand symbols using RefMapping.source_id/target_id up to ancestor_depth
        expanded_ids = await sync_to_async(models.RefMapping.objects.expand_target_ids)(
            runnable_ids or [], ancestor_depth
        )
    else:
        project_version_ids = [project_version_id]
        expanded_ids = runnable_ids or []
    return expanded_ids, project_version_ids


RUNS_LIMIT = 100
LOGS_LIMIT = 250


@gql.type
class SessionState:
    worker_set: Optional[WorkerSet]
    runs: list[Run]


@gql.input
class WakeLangserverInput:
    project_version_id: GlobalID


@gql.type
class WakeLangserverPayload:
    success: bool


@gql.input
class WakeWorkerSetInput:
    project_id: GlobalID


@gql.type
class WakeWorkerSetPayload:
    worker_set: Optional[WorkerSet]
    success: bool


@gql.input
class RestartWorkerSetInput:
    project_id: GlobalID


@gql.type
class RestartWorkerSetPayload:
    worker_set: Optional[WorkerSet]
    success: bool


@gql.input
class RunInput:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID] = None
    run_id: Optional[GlobalID] = None
    session_id: Optional[GlobalID] = None
    arguments: Optional[JSON] = None
    block: bool = True
    keyed: bool = False
    timeout_seconds: Optional[int] = None


ModuleRunErrorType = gql.enum(RunErrorType)


@gql.type
class RunState:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID]
    success: bool
    error: Optional[ModuleRunErrorType]
    run: Optional[Run]
    logs: Optional[list[LogEntry]]


@gql.input
class CancelRunInput:
    project_version_id: GlobalID
    run_id: GlobalID


@gql.type
class CancelRunPayload:
    success: bool
    run: Optional[Run]


@gql.type
class Package:
    name: str
    version: str


@gql.type
class Environment:
    language: str
    version: str
    platform: str
    packages: list[Package]


@gql.type
class SessionQuery:
    @gql.field
    @async_safe
    def current_runs(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: GlobalID,
    ) -> SessionState | OperationInfo:
        project = models.Project.objects.get(id=to_uuid(project_id))
        project_version_id = to_uuid(project_version_id)
        check_can_read_project(info, project)

        runnable_statements_ids = models.Statement.objects.filter(
            deleted_at=None,
            project_version_id=project_version_id,
            type__in=RUNNABLE_STATEMENT_TYPES,
        ).values_list("id", flat=True)

        # subquery to get the latest run per runnable_id
        latest_runs = models.Run.objects.filter(
            runnable_id=OuterRef("pk"), project_version_id=project_version_id
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
            | django.db.models.Q(status__in=PENDING_RUN_STATUSES)
        )
        return SessionState(worker_set=project.worker_set, runs=latest_run_instances)

    session: Optional[Session] = gql.relay.node(directives=[CanViewProject()])
    run: Optional[Run] = gql.relay.node(directives=[CanViewProject()])

    @gql.field
    @async_safe
    async def environment(self, info: Info, project_id: GlobalID) -> Environment | OperationInfo:
        project_id = UUID(project_id.node_id)
        project = await models.Project.objects.select_related("worker_set").aget(id=project_id)
        await sync_to_async(check_can_read_project)(info, project)
        try:
            rep: NMessage[RepGetEnvironmentPayload] = await request(
                NMessageType.GET_ENVIRONMENT,
                ReqGetEnvironmentPayload(project_id=project_id, node_id=None),
                reply_t=RepGetEnvironmentPayload,
            )
            environment_data = rep.p.environment
        except (TimeoutError, RuntimeError, NoRespondersError) as e:
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

    @gql.relay.connection
    @async_safe
    def runs(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: GlobalID,
        session_id: Optional[GlobalID] = None,
        run_id: Optional[GlobalID] = None,
        runnable_ids: Optional[list[GlobalID]] = None,
        root_only: Optional[bool] = None,
        query: Optional[SearchQuery] = None,
        sort: Optional[list[SearchSort]] = None,
        after: Optional[str] = None,
        limit: Optional[int] = None,
        count: Optional[bool] = None,
    ) -> gql.relay.Connection[Run]:
        project = models.Project.objects.get(id=to_uuid(project_id))
        project_version_id = to_uuid(project_version_id)
        session_id = to_uuid(session_id)
        run_id = to_uuid(run_id)
        runnable_ids = to_uuids(runnable_ids)
        check_can_read_project(info, project)

        query = query.to_dsl() if query else None
        if session_id:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "session_id", session_id))
        if run_id:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "run_id", run_id))
        if runnable_ids:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "runnable_id", runnable_ids))
        if root_only:
            query = Query.and_if_set(query, Q(QueryOp.DOES_NOT_EXIST, "parent_id"))
        effective_limit = min(limit or RUNS_LIMIT, RUNS_LIMIT)
        sort = [s.to_dsl() for s in sort] if sort else [Sort("created_at", SortOrder.DESCENDING)]
        search = prepare_search(
            type=mirror.DocumentType.RUN,
            project_version_id=str(project_version_id) if project_version_id else None,
            limit=effective_limit + 1,  # +1 to determine if there is a next page
            count=count or False,
            after=after,
            sort=sort,
            query=query,
        )

        results = os_client.search(
            index=IndexType.BENCH.get_index_name(project_id=project.id), body=search
        )

        edges = []
        for i, r in enumerate(results["hits"]["hits"][0:effective_limit]):
            doc = mirror.Run.from_dict(r["_source"], r["_id"], r["_version"])
            run = mirror.unmirror_node(doc)
            cursor = encode_cursor(r, after, i)
            edge = gql.relay.Edge(node=run, cursor=cursor)
            edges.append(edge)
        page_info = gql.relay.PageInfo(
            start_cursor=edges[0].cursor if edges else None,
            end_cursor=edges[-1].cursor if edges else None,
            has_next_page=len(results["hits"]["hits"]) > effective_limit,
            has_previous_page=False,
        )
        total_count = results["hits"]["total"]["value"] if count else None
        return gql.relay.Connection(edges=edges, page_info=page_info, total_count=total_count)

    @gql.relay.connection
    @async_safe
    def logs(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID] = None,
        session_id: Optional[GlobalID] = None,
        run_id: Optional[GlobalID] = None,
        runnable_ids: Optional[list[GlobalID]] = None,
        query: Optional[SearchQuery] = None,
        sort: Optional[list[SearchSort]] = None,
        after: Optional[str] = None,
        limit: Optional[int] = None,
        count: Optional[bool] = None,
    ) -> gql.relay.Connection[LogEntry]:
        project = models.Project.objects.get(id=to_uuid(project_id))
        project_version_id = to_uuid(project_version_id)
        session_id = to_uuid(session_id)
        run_id = to_uuid(run_id)
        runnable_ids = to_uuids(runnable_ids)
        check_can_read_project(info, project)

        sort = [s.to_dsl() for s in sort] if sort else [Sort("created_at", SortOrder.DESCENDING)]
        query = query.to_dsl() if query else None
        if session_id:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "session_id", str(session_id)))
        if run_id:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "run_id", str(run_id)))
        if runnable_ids:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "runnable_id", str(runnable_ids)))
        effective_limit = min(limit or LOGS_LIMIT, LOGS_LIMIT)
        search = prepare_search(
            type=mirror.DocumentType.LOG_ENTRY,
            project_version_id=str(project_version_id) if project_version_id else None,
            limit=effective_limit + 1,  # +1 to determine if there is a next page
            count=count or False,
            after=after,
            sort=sort,
            query=query,
        )

        results = os_client.search(
            index=IndexType.BENCH.get_index_name(project_id=project.id), body=search
        )

        edges = []
        for i, r in enumerate(results["hits"]["hits"][0:effective_limit]):
            doc = mirror.LogEntry.from_dict(r["_source"], r["_id"], r["_version"])
            node = LogEntry.from_os(doc)
            cursor = encode_cursor(r, after, i)
            edge = gql.relay.Edge(node=node, cursor=cursor)
            edges.append(edge)
        page_info = gql.relay.PageInfo(
            start_cursor=edges[0].cursor if edges else None,
            end_cursor=edges[-1].cursor if edges else None,
            has_next_page=len(results["hits"]["hits"]) > effective_limit,
            has_previous_page=False,
        )
        total_count = results["hits"]["total"]["value"] if count else None
        return gql.relay.Connection(edges=edges, page_info=page_info, total_count=total_count)


@gql.type
class SessionMutation:
    @asafe_mutation
    async def wake_langserver(
        self, info: Info, input: WakeLangserverInput
    ) -> WakeLangserverPayload | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        await sync_to_async(check_can_read_project)(info, project_version)
        await request(
            NMessageType.WAKE_LANGSERVER,
            ReqWakeLangserverPayload(module_id=project_version_id),
            reply_t=RepWakeLangserverPayload,
        )
        return WakeLangserverPayload(success=True)

    @asafe_mutation
    async def wake_worker_set(
        self, info: Info, input: WakeWorkerSetInput
    ) -> WakeWorkerSetPayload | OperationInfo:
        project_id = UUID(input.project_id.node_id)
        project = await models.Project.objects.aget(id=project_id)
        await sync_to_async(check_can_read_project)(info, project)
        rep: NMessage[RepWakeWorkerSetPayload] = await request(
            NMessageType.WAKE_WORKER_SET,
            ReqWakeWorkerSetPayload(project_id=project_id),
            reply_t=RepWakeWorkerSetPayload,
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
        await sync_to_async(check_can_read_project)(info, project)
        rep: NMessage[RepRestartWorkerSetPayload] = await request(
            NMessageType.RESTART_WORKER_SET,
            ReqRestartWorkerSetPayload(project_id=project_id),
            reply_t=RepRestartWorkerSetPayload,
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
        # TODO @Auth: should run be a guest-level permission for projects?
        await sync_to_async(check_can_write_project)(info, project_version)

        run = ReqStartRunPayload(
            project_id=project_version.project_id,
            module_id=project_version_id,
            runnable=to_uuid(input.runnable_id),
            runnable_type=None,
            arguments=input.arguments,
            block=input.block,
            trigger_type=RunTriggerType.UI,
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
            )
            success = rep.p.error is None
            error = rep.p.error
        except TimeoutError:
            rep = None
            success = False
            error = ModuleRunErrorType.TIMEOUT
        except NoRespondersError:
            rep = None
            success = False
            error = ModuleRunErrorType.UNAVAILABLE
        posthog.capture(
            str(user.id),
            "run",
            {"project_version_id": str(project_version_id), "success": success, "error": error},
        )
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
    async def cancel_run(
        self, info: Info, input: CancelRunInput
    ) -> CancelRunPayload | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = get_user_from_info(info)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        await sync_to_async(check_can_write_project)(info, project_version)
        cancel = ReqCancelRunPayload(
            project_id=project_version.project_id,
            module_id=project_version_id,
            run_id=to_uuid(input.run_id),
        )
        try:
            rep: NMessage[RepCancelRunPayload] = await request(
                NMessageType.CANCEL_RUN,
                cancel,
                reply_t=RepCancelRunPayload,
            )
            success = rep.p.success
        except (NoRespondersError, TimeoutError):
            success = False
        posthog.capture(
            str(user.id),
            "cancel_run",
            {"project_version_id": str(project_version_id), "success": success},
        )
        return CancelRunPayload(success=success, run=None)


@gql.type
class LogChange:
    logs: list[LogEntry]


@gql.type
class SessionChange:
    session: Session
    runs: list[Run]


@gql.type
class WorkerChange:
    worker_sets: list[WorkerSet]


@gql.type
class SessionSubscription:
    @asafe_subscription
    async def sessions_changed(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID] = None,
    ) -> AsyncGenerator[SessionChange | WorkerChange, None]:
        project_id = UUID(project_id.node_id)
        project_version_id = UUID(project_version_id.node_id)
        user = get_user_from_info(info)
        log = logger.bind(project_id=project_id, project_version_id=project_version_id, user=user)
        try:
            await sync_to_async(check_can_view_project_by_id)(
                user, project_id=project_id, project_version_id=project_version_id
            )
        except PermissionDenied:
            log.debug("sessions.subscribe_denied", exc_info=True)
            return

        log.info("sessions.subscribe")
        routing_id = f"{project_id}.{project_version_id or '*'}".replace("-", "")
        sub = await subscribe_many(
            {
                f"{NMessageType.SESSION_CHANGED}.{routing_id}": SessionChangedPayload,
                f"{NMessageType.WORKERS_CHANGED}.{routing_id}": WorkersChangedPayload,
                f"{NMessageType.WORKERS_CHANGED}.all": WorkersChangedPayload,
            },
        )
        while True:
            msg: NMessage[SessionChangedPayload | WorkersChangedPayload] = await sub.next_msg()
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
                    session=packer.unpack_data(msg.p.session),
                    runs=[packer.unpack_data(r) for r in msg.p.runs],
                )
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
    ) -> AsyncGenerator[LogChange, None]:
        project_id = to_uuid(project_id)
        project_version_id = to_uuid(project_version_id)
        session_id = to_uuid(session_id)
        run_id = to_uuid(run_id)
        runnable_ids = to_uuids(runnable_ids)
        user = get_user_from_info(info)
        log = logger.bind(project_id=project_id, project_version_id=project_version_id, user=user)
        try:
            await sync_to_async(check_can_view_project_by_id)(
                user, project_id=project_id, project_version_id=project_version_id
            )
        except PermissionDenied:
            log.debug("sessions.subscribe_denied", exc_info=True)
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
            return True

        log.info("logs.subscribe")
        routing_id = f"{project_id}.{project_version_id or '*'}".replace("-", "")
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
