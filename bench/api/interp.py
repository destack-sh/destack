from typing import TYPE_CHECKING, Annotated, Optional

import strawberry
import strawberry_django
import structlog
from django.db.models import QuerySet
from strawberry import lazy, relay

from bench import language as language
from bench import models
from bench.api.utils import ModuleNode

if TYPE_CHECKING:
    from bench.api.project import File
    from bench.api.statement import Field, Statement

logger = structlog.get_logger(__name__)

InterpScope = strawberry.enum(models.InterpScope)
IssueKind = strawberry.enum(models.IssueKind)
IssueType = strawberry.enum(language.IssueType)


@strawberry_django.type(models.Issue)
class Issue(relay.Node, ModuleNode):
    parent: Optional[ModuleNode]
    scope: InterpScope
    kind: IssueKind
    type: IssueType
    file: Optional[Annotated["File", lazy(".file")]]
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    message: Optional[str]


@strawberry_django.filter(models.Issue)
class IssueFilter:
    scope: InterpScope

    def filter(self, queryset: QuerySet[models.Issue]):
        if self.scope:
            queryset = queryset.filter(scope=self.scope)
        return queryset


@strawberry_django.type(models.ResolvedField)
class ResolvedField:
    # statement here is not actually optional but it needs to be to union with Issue
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    field: Annotated["Field", lazy(".statement")]
