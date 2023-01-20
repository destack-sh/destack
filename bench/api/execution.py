from typing import Optional

from strawberry import auto
from strawberry_django_plus import gql

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
    parent: Optional["Execution"]
    children: list["Execution"]
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
