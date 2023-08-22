from typing import TYPE_CHECKING, Annotated, Optional

import strawberry
import strawberry_django
import structlog
from strawberry import lazy, relay

from bench import language as language
from bench import models
from bench.api.utils import ModuleNode

if TYPE_CHECKING:
    from bench.api.statement import Field, Statement

logger = structlog.get_logger(__name__)

IssueKind = strawberry.enum(models.IssueKind)
IssueType = strawberry.enum(language.IssueType)


@strawberry_django.type(models.Issue)
class Issue(relay.Node, ModuleNode):
    parent: Optional[ModuleNode]
    kind: IssueKind
    type: IssueType
    message: Optional[str]


@strawberry_django.type(models.ResolvedField)
class ResolvedField:
    # statement here is not actually optional but it needs to be to union with Issue
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    field: Annotated["Field", lazy(".statement")]
