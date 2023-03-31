from typing import TYPE_CHECKING, Annotated, AsyncGenerator, Iterable, Optional, cast
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied, ValidationError
from strawberry import auto, lazy
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.auth import CanViewProject, check_can_view_project_by_id
from bench.api.statement import Statement
from bench.api.util import asafe_subscription, to_uuid, to_uuids
from bench.models import mapper
from bench.msg import NMessageType
from bench.msg.core import NMessage, subscribe
from bench.msg.messages import ExecutionSavedPayload

if TYPE_CHECKING:
    from bench.api.deployment import Deployment
    from bench.api.project import Project, ProjectVersion
    from bench.api.token import AccessToken
    from bench.api.user import User

logger = structlog.get_logger(__name__)

ExecutionStatus = gql.enum(models.ExecutionStatus)
ExecutionTriggerType = gql.enum(models.ExecutionTriggerType)


@gql.django.type(models.Execution)
class Execution(gql.Node):
    project: Annotated["Project", lazy(".project")]
    project_version: Annotated["ProjectVersion", lazy(".project")]
    deployment: Optional[Annotated["Deployment", lazy(".deployment")]]
    created_at: auto
    updated_at: auto
    started_at: auto
    terminated_at: auto
    duration_millis: auto
    status: ExecutionStatus
    inputs: auto
    outputs: auto
    error: auto
    root: Optional["Execution"]
    parent: Optional["Execution"]
    descendants: list["Execution"]
    build: Optional[Statement]
    task: Optional[Statement]
    code: Optional[Statement]
    model: Optional[Statement]
    # trigger
    trigger_type: ExecutionTriggerType
    user: Optional[Annotated["User", lazy(".user")]]
    access_token: Optional[Annotated["AccessToken", lazy(".token")]]


async def _expand_filter(
    project_version_id: UUID,
    include_ancestor_versions: bool,
    build_ids: list[UUID] | None,
    code_ids: list[UUID] | None,
    task_ids: list[UUID] | None,
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
        expanded_symbol_ids = [*(build_ids or []), *(task_ids or []), *(code_ids or [])]
        # expand symbols using RefMapping.source_id/target_id up to ancestor_depth
        expanded_symbol_ids = await sync_to_async(models.RefMapping.objects.expand_target_ids)(
            expanded_symbol_ids, ancestor_depth
        )
    else:
        project_version_ids = [project_version_id]
        expanded_symbol_ids = [*(build_ids or []), *(task_ids or []), *(code_ids or [])]
    return expanded_symbol_ids, project_version_ids


@gql.type
class ExecutionQuery:
    @gql.connection(directives=[CanViewProject()])
    async def executions(
        self,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID] = None,
        include_ancestor_versions: bool = False,
        build_ids: list[GlobalID] | None = None,
        task_ids: list[GlobalID] | None = None,
        code_ids: list[GlobalID] | None = None,
        root_id: Optional[GlobalID] = None,
        root_id_null: bool = False,
    ) -> Iterable[Execution]:
        qs = models.Execution.objects.all()
        # :ExecutionsFilter
        project_version_id = to_uuid(project_version_id)
        build_ids = to_uuids(build_ids)
        task_ids = to_uuids(task_ids)
        code_ids = to_uuids(code_ids)
        # if filtering by a symbol and including multiple versions, expand into mappings
        expanded_symbol_ids, project_version_ids = await _expand_filter(
            project_version_id, include_ancestor_versions, build_ids, code_ids, task_ids
        )
        qs = qs.filter(project_id=project_id.node_id)
        if project_version_ids:
            qs = qs.filter(project_version_id__in=project_version_ids)
        if build_ids:
            qs = qs.filter(build_id__in=expanded_symbol_ids)
        if task_ids:
            qs = qs.filter(task_id__in=expanded_symbol_ids)
        if code_ids:
            qs = qs.filter(code_id__in=expanded_symbol_ids)
        if root_id is not None:
            qs = qs.filter(root_id=root_id.node_id)
        if root_id_null:
            qs = qs.filter(root_id__isnull=True)
        return qs


@gql.type
class ExecutionSubscription:
    @asafe_subscription
    async def executions_changed(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID],
        include_ancestor_versions: bool = False,
        build_ids: list[GlobalID] | None = None,
        task_ids: list[GlobalID] | None = None,
        code_ids: list[GlobalID] | None = None,
        root_id: Optional[GlobalID] = None,
        root_id_null: bool = False,
    ) -> AsyncGenerator[Execution, None]:
        project_id = UUID(project_id.node_id)
        project_version_id = UUID(project_version_id.node_id)
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        log = logger.bind(
            project_id=project_id,
            project_version_id=project_version_id,
            build_ids=build_ids,
            task_ids=task_ids,
            code_ids=code_ids,
            root_id=root_id,
            root_id_null=root_id_null,
            user=user,
        )
        try:
            await sync_to_async(check_can_view_project_by_id)(
                user, project_id=project_id, project_version_id=project_version_id
            )
        except PermissionDenied:
            log.debug("executions.subscribe_denied")
            return

        log.info("executions.subscribe")
        executions_sub = await subscribe(
            f"{NMessageType.EXECUTION_SAVED}.{project_version_id}", payload_t=ExecutionSavedPayload
        )

        # :ExecutionsFilter
        project_version_id = to_uuid(project_version_id)
        build_ids = to_uuids(build_ids)
        task_ids = to_uuids(task_ids)
        code_ids = to_uuids(code_ids)
        # If filtering by a symbol and including multiple versions, expand into their mappings.
        # We do this once before listening for performance and simplicity, though this means that new versions
        # will not be automatically included in the execution subscription. We could periodically re-check,
        # but that's a bit more complicated and not really worth it for now.
        expanded_symbol_ids, project_version_ids = await _expand_filter(
            project_version_id, include_ancestor_versions, build_ids, code_ids, task_ids
        )
        log.debug("executions.listen")
        while True:
            msg: NMessage[ExecutionSavedPayload] = await executions_sub.next_msg()
            for frame_data in msg.payload.frames:
                # :ExecutionsFilter
                other_build = build_ids and frame_data.build_id not in expanded_symbol_ids
                other_task = task_ids and frame_data.task_id not in expanded_symbol_ids
                other_code = code_ids and frame_data.code_id not in expanded_symbol_ids
                other_root = (
                    root_id is not None
                    and root_id.node_id != frame_data.root_id
                    or root_id_null is True
                    and frame_data.root_id is not None
                )
                if other_build or other_task or other_code or other_root:
                    # TODO @Performance: filter execution frames more precisely via NATS?
                    continue
                frame = mapper.rmap_execution_frame(frame_data)
                log.debug("executions.update", frame=frame)
                yield frame
