from typing import Iterable, Optional

from strawberry import auto
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.auth import CanViewProject
from bench.api.statement import Statement

ExecutionStatus = gql.enum(models.ExecutionStatus)


@gql.django.type(models.Execution)
class Execution(gql.Node):
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
        project_version_id: Optional[GlobalID] = None,
        build_id: Optional[GlobalID] = None,
        task_id: Optional[GlobalID] = None,
        code_id: Optional[GlobalID] = None,
        root_id: Optional[GlobalID] = None,
        root_id_null: bool = False,
    ) -> Iterable[Execution]:
        filtered = models.Execution.objects.all()
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
