from typing import TYPE_CHECKING, Annotated, Iterable, Optional
from uuid import UUID

from asgiref.sync import sync_to_async
from django.core.exceptions import ValidationError
from strawberry import auto, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.auth import CanViewProject
from bench.api.statement import Statement
from bench.api.util import to_uuid, to_uuids

if TYPE_CHECKING:
    from bench.api.deployment import Deployment
    from bench.api.project import Project, ProjectVersion
    from bench.api.token import AccessToken
    from bench.api.user import User

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


@gql.django.type(models.ModelInference)
class ModelInference(gql.Node):
    model: Statement
    operation: auto
    settings_hash: auto
    input_hash: auto
    input: auto
    output: auto
    duration_ms: auto


async def expand_project_version_ids(
    project_version_id: UUID,
    build_ids: list[UUID] | None,
    task_ids: list[UUID] | None,
    code_ids: list[UUID] | None,
    ancestor_depth: Optional[int],
):
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
    return project_version_ids, expanded_symbol_ids


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
        filtered = models.Execution.objects.all()

        # :ExecutionsFilter
        project_version_id = to_uuid(project_version_id)
        build_ids = to_uuids(build_ids)
        task_ids = to_uuids(task_ids)
        code_ids = to_uuids(code_ids)

        # if filtering by a symbol and including multiple versions, expand into mappings
        if include_ancestor_versions:
            if project_version_id is None:
                raise ValidationError(
                    "project_version_id must be specified if include_ancestor_versions"
                )
            project_version_ids, expanded_symbol_ids = await expand_project_version_ids(
                project_version_id, build_ids, task_ids, code_ids, ancestor_depth=8
            )
        else:
            project_version_ids = [project_version_id]
            expanded_symbol_ids = [*(build_ids or []), *(task_ids or []), *(code_ids or [])]

        if project_id is not None:
            filtered = filtered.filter(project_id=project_id.node_id)
        if project_version_ids:
            filtered = filtered.filter(project_version_id__in=project_version_ids)
        if build_ids:
            filtered = filtered.filter(build_id__in=expanded_symbol_ids)
        if task_ids:
            filtered = filtered.filter(task_id__in=expanded_symbol_ids)
        if code_ids:
            filtered = filtered.filter(code_id__in=expanded_symbol_ids)

        if root_id is not None:
            filtered = filtered.filter(root_id=root_id.node_id)
        if root_id_null:
            filtered = filtered.filter(root_id__isnull=True)
        return filtered
