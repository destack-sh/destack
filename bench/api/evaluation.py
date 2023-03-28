from typing import Annotated, Optional

from strawberry import auto, lazy
from strawberry.scalars import JSON
from strawberry_django_plus import gql

from bench import models
from bench.api.project import File, Project, ProjectVersion
from bench.api.statement import DatasetRecord, SimpleTypeNode, Statement

EvaluationKind = gql.enum(models.EvaluationKind)
EvaluationScope = gql.enum(models.EvaluationScope)


@gql.django.type(models.EvaluationResult)
class EvaluationResult(gql.Node):
    created_at: auto
    updated_at: auto
    project: Annotated["Project", lazy(".project")]
    project_version: Annotated["ProjectVersion", lazy(".project")]
    file: Annotated["File", lazy(".project")]
    kind: EvaluationKind
    scope: EvaluationScope
    statement: Annotated["Statement", lazy(".statement")]
    record: Annotated["DatasetRecord", lazy(".statement")]
    type_node: Annotated["SimpleTypeNode", lazy(".statement")]
    self_metrics: Optional[JSON]
    aggregated_metrics: JSON
