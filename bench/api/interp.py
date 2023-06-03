from typing import TYPE_CHECKING, Annotated, Optional
from uuid import UUID

import structlog
from strawberry import lazy
from strawberry_django_plus import gql

from bench import language, models

if TYPE_CHECKING:
    from bench.api.statement import Field, Statement

logger = structlog.get_logger(__name__)

IssueKind = gql.enum(models.IssueKind)
IssueType = gql.enum(language.IssueType)


@gql.django.type(models.Issue)
class Issue(gql.Node):
    kind: IssueKind
    type: IssueType
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    message: Optional[str]


# doesn't exist in the DB (part of statement) but used for data sync
@gql.type
class InterpData:
    statement_id: UUID
    issues: Optional[list[Issue]]
    resolved_fields: Optional[list[Annotated["Field", lazy(".statement")]]]
