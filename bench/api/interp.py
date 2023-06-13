from typing import TYPE_CHECKING, Annotated, Optional

import structlog
from strawberry import lazy
from strawberry_django_plus import gql

from bench import bench as language
from bench import models

if TYPE_CHECKING:
    from bench.api.project import File
    from bench.api.statement import Field, Statement

logger = structlog.get_logger(__name__)

InterpScope = gql.enum(models.InterpScope)
IssueKind = gql.enum(models.IssueKind)
IssueType = gql.enum(language.IssueType)


@gql.django.type(models.Issue)
class Issue(gql.Node):
    scope: InterpScope
    kind: IssueKind
    type: IssueType
    file: Optional[Annotated["File", lazy(".project")]]
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    message: Optional[str]


@gql.django.type(models.ResolvedField)
class ResolvedField(gql.Node):
    # statement here is not actually optional but it needs to be to union with Issue
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    field: Annotated["Field", lazy(".statement")]
