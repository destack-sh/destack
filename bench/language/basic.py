import enum
import itertools
from typing import Optional
from uuid import UUID

from bench.language.core import (
    ModuleVisitor,
    Scope,
    Statement,
    StatementReference,
    StatementType,
    node,
)
from bench.language.issue import IssueType

# common statements


@node(tracked=[])
class Blank(Statement):
    """A blank statement."""

    type: StatementType = StatementType.BLANK

    def _visit(self, visitor: ModuleVisitor) -> None:
        pass


@node
class HasText:
    """A comment that's not semantic/interpreted by default."""

    text: str | None = None

    def _visit(self, visitor: ModuleVisitor) -> None:
        pass


class TextHeadingLevel(enum.IntEnum):
    H1 = 1
    H2 = 2
    H3 = 3


@node(tracked=["text", "heading_level"])
class Text(HasText, Statement):
    heading_level: Optional[TextHeadingLevel] = None
    type: StatementType = StatementType.TEXT


# avoid circular import because Reference IsFlowNode
from bench.language.flow import IsFlowNode  # noqa: E402
from bench.language.tag import HasTags  # noqa: E402


@node(tracked=["reference"])
class Reference(Statement, HasTags, IsFlowNode):
    """A reference to another statement."""

    type: StatementType = StatementType.REFERENCE
    reference: Statement | StatementReference = None
    text: str | None = None

    def _clear(self) -> None:
        HasTags._clear(self)
        IsFlowNode._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasTags._interp(self, scope)
        IsFlowNode._interp(self, scope)
        resolved = None
        if not isinstance(self.reference, Statement):
            resolved = scope.lookup(self.reference)
        if resolved is None:
            self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
        else:
            self.reference = resolved

    def _visit(self, visitor: "ModuleVisitor") -> None:
        for n in itertools.chain(self.tags, self.triggers):
            visitor.visit(n)

    @property
    def reference_ck(self) -> Optional[UUID]:
        if isinstance(self.reference, Statement):
            return self.reference.ck
        elif isinstance(self.reference, UUID):
            return self.reference
        else:
            return None


# hard-coded, do not change ever :BenchUuidNamespace
BENCH_UUID_NAMESPACE = UUID("d822dab7-41ad-4706-a9c8-4379e15b2ed0")
