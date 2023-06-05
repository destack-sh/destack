from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Optional
from uuid import UUID

import pytz
import structlog
from strawberry import lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import language, models
from bench.api.util import to_global_id
from bench.language import wire
from bench.models import mapper

if TYPE_CHECKING:
    from bench.api.project import File
    from bench.api.statement import Field, Statement

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
    type: str
    message: Optional[str]


# doesn't exist in the DB (part of statement) but used for data sync
@gql.type
class InterpData:
    scope: InterpScope
    file_id: Optional[GlobalID]
    statement_id: Optional[GlobalID]
    issues: Optional[list[Issue]]
    resolved_fields: Optional[list[Annotated["Field", lazy(".statement")]]]


def rmap_interp_data(interp_data: wire.InterpData, module_id: UUID) -> InterpData:
    issues = (
        [mapper.wmap_issue(issue, module_id) for issue in interp_data.issues]
        if interp_data.issues is not None
        else None
    )
    resolved_fields = (
        [
            mapper.wmap_field(interp_data.statement_id, field)
            for field in interp_data.resolved_fields
        ]
        if interp_data.resolved_fields is not None
        else None
    )
    # set created/updated since they're not set by wmap
    now = datetime.utcnow().replace(tzinfo=pytz.utc)
    for field in resolved_fields or []:
        field.created_at = now
        field.updated_at = now

    file_id = to_global_id("File", interp_data.file_id) if interp_data.file_id else None
    statement_id = (
        to_global_id("Statement", interp_data.statement_id) if interp_data.statement_id else None
    )
    return InterpData(
        scope=interp_data.scope,
        file_id=file_id,
        statement_id=statement_id,
        issues=issues,
        resolved_fields=resolved_fields,
    )
