from __future__ import annotations

from strawberry import auto
from strawberry_django_plus import gql

from bench import models
from bench.api.symbol import Statement


@gql.django.type(models.ModelInference)
class ModelInference(gql.Node):
    model: Statement
    operation: auto
    settings_hash: auto
    input_hash: auto
    input: auto
    output: auto
    duration_ms: auto
