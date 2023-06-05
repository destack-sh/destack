from typing import TYPE_CHECKING, Annotated, Optional

from strawberry_django_plus.relay import GlobalID
import structlog
from strawberry import lazy
from strawberry_django_plus import gql

from bench import language, models
from bench.language import wire

if TYPE_CHECKING:
    from bench.api.statement import Field, Statement
    from bench.api.project import File

logger = structlog.get_logger(__name__)

InterpScope = gql.enum(wire.InterpScope)
IssueKind = gql.enum(models.IssueKind)
IssueType = gql.enum(language.IssueType)


@gql.django.type(models.Issue)
class Issue(gql.Node):
    scope: InterpScope
    file: Optional[Annotated["File", lazy(".project")]]
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    kind: IssueKind
    type: IssueType
    message: Optional[str]


# doesn't exist in the DB (part of statement) but used for data sync
@gql.type
class InterpData:
    scope: InterpScope
    file_id: Optional[GlobalID]
    statement_id: Optional[GlobalID]
    issues: Optional[list[Issue]]
    resolved_fields: Optional[list[Annotated["Field", lazy(".statement")]]]
