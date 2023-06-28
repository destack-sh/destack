from typing import TYPE_CHECKING, Annotated, AsyncGenerator, Iterable, Optional
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied, ValidationError
from strawberry import auto, lazy
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.auth import CanViewProject, check_can_view_project_by_id
from bench.api.statement import Statement
from bench.api.utils import asafe_subscription, get_user_from_info, to_uuid, to_uuids
from bench.models import packer
from bench.msg.core import NMessage, subscribe
from bench.msg.messages import ExecutionSavedPayload, NMessageType

if TYPE_CHECKING:
    from bench.api.project import Project, ProjectVersion
    from bench.api.token import AccessToken
    from bench.api.user import User

logger = structlog.get_logger(__name__)

ExecutionStatus = gql.enum(models.ExecutionStatus)
ExecutionTriggerType = gql.enum(models.ExecutionTriggerType)


@gql.type
class PyFrame:
    filename: str
    lineno: int
    name: str
    line: str = None
    locals: Optional[JSON] = None

    def from_dict(self, data: dict) -> "PyFrame":
        return PyFrame(**data)


@gql.type
class RunError:
    """Wire-able representation of an exception."""

    type: str
    message: str
    statement: Optional[str]
    traceback: Optional[list[PyFrame]]

    @staticmethod
    def from_dict(data: dict) -> "RunError":
        if data["traceback"]:
            traceback = [PyFrame(**frame) for frame in data["traceback"]]
        else:
            traceback = None
        return RunError(
            type=data["type"],
            message=data["message"],
            statement=data.get("statement"),
            traceback=traceback,
        )


def get_error_nice(root: "Execution") -> Optional[RunError]:
    if root.error:
        return RunError.from_dict(data=root.error)
    else:
        return None


@gql.django.type(models.Execution)
class Execution(gql.Node):
    project: Annotated["Project", lazy(".project")]
    project_version: Annotated["ProjectVersion", lazy(".project")]
    created_at: auto
    updated_at: auto
    started_at: auto
    terminated_at: auto
    cached_generated_at: auto
    cached_duration: auto
    duration: auto
    status: ExecutionStatus
    inputs: auto
    outputs: auto
    error: auto
    error_nice: Optional[RunError] = gql.django.field(only=["error"], resolver=get_error_nice)
    metadata: auto
    root: Optional["Execution"]
    parent: Optional["Execution"]
    descendants: list["Execution"]
    runnable: Optional["Statement"]
    # trigger
    trigger_type: ExecutionTriggerType
    user: Optional[Annotated["User", lazy(".user")]]
    access_token: Optional[Annotated["AccessToken", lazy(".token")]]


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


@gql.type
class ExecutionQuery:
    @gql.django.connection(directives=[CanViewProject()])
    async def executions(
        self,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID] = None,
        include_ancestor_versions: bool = False,
        runnable_ids: list[GlobalID] | None = None,
        root_id_null: bool = False,
    ) -> Iterable[Execution]:
        qs = models.Execution.objects.all()
        # :ExecutionsFilter
        project_version_id = to_uuid(project_version_id)
        runnable_ids = to_uuids(runnable_ids)
        expanded_symbol_ids, project_version_ids = await _expand_filter(
            project_version_id, include_ancestor_versions, runnable_ids
        )
        qs = qs.filter(project_id=project_id.node_id)
        if project_version_ids:
            qs = qs.filter(project_version_id__in=project_version_ids)
        if runnable_ids:
            qs = qs.filter(runnable_id__in=expanded_symbol_ids)
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
        runnable_ids: list[GlobalID] | None = None,
        root_id: Optional[GlobalID] = None,
        root_id_null: bool = False,
    ) -> AsyncGenerator[Execution, None]:
        project_id = UUID(project_id.node_id)
        project_version_id = UUID(project_version_id.node_id)
        user = get_user_from_info(info)
        log = logger.bind(
            project_id=project_id,
            project_version_id=project_version_id,
            runnable_ids=runnable_ids,
            root_id=root_id,
            root_id_null=root_id_null,
            user=user,
        )
        try:
            await sync_to_async(check_can_view_project_by_id)(
                user, project_id=project_id, project_version_id=project_version_id
            )
        except PermissionDenied:
            log.debug("executions.subscribe_denied", exc_info=True)
            return

        log.info("executions.subscribe")
        executions_sub = await subscribe(
            f"{NMessageType.EXECUTION_SAVED}.{project_version_id}", payload_t=ExecutionSavedPayload
        )

        # :ExecutionsFilter
        project_version_id = to_uuid(project_version_id)
        runnable_ids = to_uuids(runnable_ids)
        expanded_ids, project_version_ids = await _expand_filter(
            project_version_id, include_ancestor_versions, runnable_ids
        )
        log.debug("executions.listen")
        while True:
            msg: NMessage[ExecutionSavedPayload] = await executions_sub.next_msg()
            for frame_data in msg.payload.frames:
                # :ExecutionsFilter
                other_runnable = (
                    frame_data.runnable_id is not None
                    and frame_data.runnable_id not in expanded_ids
                )
                other_root = (
                    root_id is not None
                    and root_id.node_id != frame_data.root_id
                    or root_id_null is True
                    and frame_data.root_id is not None
                )
                if other_runnable or other_root:
                    # TODO @Performance: filter execution frames more precisely via NATS?
                    continue
                frame = packer.unpack_data(frame_data)
                log.debug("executions.update", frame=frame)
                yield frame
