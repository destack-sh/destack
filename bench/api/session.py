from datetime import datetime
from typing import TYPE_CHECKING, Annotated, AsyncGenerator, Optional
from uuid import UUID

import django.db.models
import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied, ValidationError
from django.db.models import OuterRef, Subquery
from strawberry import auto, lazy
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo
from strawberry_django_plus.utils.resolvers import async_safe

from bench import models
from bench.api.auth import check_can_read_project, check_can_view_project_by_id
from bench.api.statement import Statement
from bench.api.utils import (
    QueryOp,
    SearchQuery,
    SearchSort,
    asafe_subscription,
    get_user_from_info,
    to_global_id,
    to_uuid,
    to_uuids,
)
from bench.bench import Q, session, wire, Query
from bench.bench.const import RUNNABLE_STATEMENT_TYPES
from bench.bench.session import PENDING_RUN_STATUSES
from bench.models import packer
from bench.msg.core import NMessage, subscribe
from bench.msg.messages import LogsChangedPayload, NMessageType, SessionChangedPayload
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
    def from_data(data: session.RunCodeFrame) -> "RunCodeFrame":
        return RunCodeFrame(
            filename=data.filename,
            lineno=data.lineno,
            name=data.name,
            line=data.line,
            locals=data.locals,
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
        error: session.RunError = session.RunError.instantiate_from(data)
        statement_id = to_global_id("Statement", error.statement_id) if error.statement_id else None
        traceback = (
            [RunCodeFrame.from_data(frame) for frame in error.traceback]
            if error.traceback
            else None
        )
        return RunError(
            kind=error.kind,
            type=error.type,
            message=error.message,
            statement_id=statement_id,
            traceback=traceback,
        )


def get_error_nice(root: "Run") -> Optional[RunError]:
    if root.error:
        return RunError.from_dict(data=root.error)
    else:
        return None


@gql.django.type(models.Session)
class Session(gql.Node):
    project: Annotated["Project", lazy(".project")]
    created_at: auto
    updated_at: auto
    opened_at: auto
    closed_at: auto
    metadata: Optional[JSON]
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
    cached_generated_at: auto
    cached_duration: auto


@gql.type
class LogEntry:
    module_id: GlobalID
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
            module_id=to_global_id("Module", log_entry.module_id),
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
            module_id=to_global_id("Module", log_entry.module_id),
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
    runs: list[Run]


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
        return SessionState(runs=latest_run_instances)

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
        effective_limit = min(limit or LOGS_LIMIT, LOGS_LIMIT)
        search = prepare_search(
            type=mirror.DocumentType.RUN,
            project_version_id=str(project_version_id) if project_version_id else None,
            limit=effective_limit + 1,  # +1 to determine if there is a next page
            count=count or False,
            after=after,
            sort=[s.to_dsl() for s in sort] if sort else None,
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

        query = query.to_dsl() if query else None
        if session_id:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "session_id", session_id))
        if run_id:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "run_id", run_id))
        if runnable_ids:
            query = Query.and_if_set(query, Q(QueryOp.EQUALS, "runnable_id", runnable_ids))
        effective_limit = min(limit or LOGS_LIMIT, LOGS_LIMIT)
        search = prepare_search(
            type=mirror.DocumentType.LOG_ENTRY,
            project_version_id=str(project_version_id) if project_version_id else None,
            limit=effective_limit + 1,  # +1 to determine if there is a next page
            count=count or False,
            after=after,
            sort=[s.to_dsl() for s in sort] if sort else None,
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
class LogChange:
    logs: list[LogEntry]


@gql.type
class SessionChange:
    session: Session
    runs: list[Run]


@gql.type
class SessionSubscription:
    @asafe_subscription
    async def sessions_changed(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID],
    ) -> AsyncGenerator[SessionChange, None]:
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
        sessions_sub = await subscribe(
            f"{NMessageType.SESSION_CHANGED}.{project_version_id}", payload_t=SessionChangedPayload
        )
        while True:
            msg: NMessage[SessionChangedPayload] = await sessions_sub.next_msg()
            log.debug("sessions.update", msg=msg)
            yield SessionChange(
                session=(packer.unpack_data(msg.p.session)),
                runs=[packer.unpack_data(r) for r in msg.p.runs],
            )

    @asafe_subscription
    async def logs_changed(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID],
        session_id: Optional[GlobalID],
        run_id: Optional[GlobalID],
        runnable_ids: Optional[list[GlobalID]],
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
            if session_id and log.session_id != session_id:
                return False
            if run_id and log.run_id != run_id:
                return False
            if runnable_ids and log.runnable_id not in runnable_ids:
                return False
            return True

        log.info("logs.subscribe")
        logs_sub = await subscribe(
            f"{NMessageType.LOGS_CHANGED}.{project_version_id}", payload_t=LogsChangedPayload
        )
        while True:
            msg: NMessage[LogsChangedPayload] = await logs_sub.next_msg()
            logs = [LogEntry.from_data(l) for l in msg.p.logs if _filter_log(l)]
            if not logs:
                continue
            log.debug("logs.update", msg=msg, logs=len(logs))
            yield LogChange(logs=logs)
