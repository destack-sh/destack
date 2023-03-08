from typing import TYPE_CHECKING, Annotated, Iterable, Optional

from strawberry import auto, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.auth import CanViewProject
from bench.api.statement import Statement

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
    deployment: Annotated["Deployment", lazy(".deployment")]
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
    user: Annotated["User", lazy(".user")]
    access_token: Annotated["AccessToken", lazy(".token")]


@gql.django.type(models.ModelInference)
class ModelInference(gql.Node):
    model: Statement
    operation: auto
    settings_hash: auto
    input_hash: auto
    input: auto
    output: auto
    duration_ms: auto


@gql.type
class ExecutionQuery:
    @gql.connection(directives=[CanViewProject()])
    def executions(
        self,
        project_id: Optional[GlobalID] = None,
        project_version_id: Optional[GlobalID] = None,
        include_ancestor_versions: bool = False,
        build_id: Optional[GlobalID] = None,
        task_id: Optional[GlobalID] = None,
        code_id: Optional[GlobalID] = None,
        root_id: Optional[GlobalID] = None,
        root_id_null: bool = False,
    ) -> Iterable[Execution]:
        filtered = models.Execution.objects.all()

        # :ExecutionsFilter

        # if filtering by a symbol and including multiple versions, expand into mappings
        if include_ancestor_versions:
            # expand project_version_id and build/task/code to include all ancestor versions
            raise NotImplementedError  # TODO @Incomplete

        if project_id is not None:
            filtered = filtered.filter(project_id=project_id.node_id)
        if project_version_id is not None:
            filtered = filtered.filter(project_version_id=project_version_id.node_id)
        if build_id is not None:
            filtered = filtered.filter(build_id=build_id.node_id)
        if task_id is not None:
            filtered = filtered.filter(task_id=task_id.node_id)
        if code_id is not None:
            filtered = filtered.filter(code_id=code_id.node_id)

        if root_id is not None:
            filtered = filtered.filter(root_id=root_id.node_id)
        if root_id_null:
            filtered = filtered.filter(root_id__isnull=True)
        return filtered
