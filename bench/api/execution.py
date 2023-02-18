from typing import Iterable, Optional

from strawberry import UNSET, auto
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.statement import Statement


@gql.django.type(models.Execution)
class Execution(gql.Node):
    created_at: auto
    updated_at: auto
    started_at: auto
    terminated_at: auto
    duration_millis: auto
    status: auto
    inputs: auto
    outputs: auto
    error: auto
    root: Optional["Execution"]
    parent: Optional["Execution"]
    descendants: list["Execution"]
    code: Statement
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
    @gql.connection()
    def executions(
        self,
        project_version_id: Optional[GlobalID] = None,
        code_id: Optional[GlobalID] = None,
        root_id: Optional[GlobalID] = UNSET,
    ) -> Iterable[Execution]:
        filtered = models.Execution.objects.all()
        if code_id is not None:
            filtered = filtered.filter(code_id=code_id.node_id)
        if project_version_id is not None:
            filtered = filtered.filter(project_version_id=project_version_id.node_id)
        if root_id is not UNSET:
            if root_id is None:
                filtered = filtered.filter(root_id__isnull=True)
            else:
                filtered = filtered.filter(root_id=root_id.node_id)
        return filtered
