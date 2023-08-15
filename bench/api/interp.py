from typing import TYPE_CHECKING, Annotated, Optional

import structlog
from django.db.models import QuerySet
from strawberry import lazy
from strawberry_django_plus import gql

from bench import language as language
from bench import models
from bench.api.utils import ModuleNode

if TYPE_CHECKING:
    from bench.api.project import File
    from bench.api.statement import Field, Statement

logger = structlog.get_logger(__name__)

InterpScope = gql.enum(models.InterpScope)
IssueKind = gql.enum(models.IssueKind)
IssueType = gql.enum(language.IssueType)


@gql.django.type(models.Issue)
class Issue(gql.Node, ModuleNode):
    parent: Optional[ModuleNode]
    scope: InterpScope
    kind: IssueKind
    type: IssueType
    file: Optional[Annotated["File", lazy(".file")]]
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    message: Optional[str]


@gql.django.filter(models.Issue)
class IssueFilter:
    scope: InterpScope

    def filter(self, queryset: QuerySet[models.Issue]):
        if self.scope:
            queryset = queryset.filter(scope=self.scope)
        return queryset


@gql.django.type(models.ResolvedField)
class ResolvedField(gql.Node):
    # statement here is not actually optional but it needs to be to union with Issue
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    field: Annotated["Field", lazy(".statement")]
