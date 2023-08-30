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
from bench.language.flow import IsFlowNode
from bench.language.issue import IssueType
from bench.language.tag import HasTags

# common statements


@node(tracked=[])
class Blank(Statement):
    """A blank statement."""

    type: StatementType = StatementType.BLANK

    def _visit(self, visitor: ModuleVisitor) -> None:
        pass


@node(tracked=["text"])
class Text(Statement):
    """A comment that's not semantic/interpreted by default."""

    type: StatementType = StatementType.TEXT
    text: str | None = None

    def _visit(self, visitor: ModuleVisitor) -> None:
        pass


@node(tracked=["reference"])
class Reference(Statement, HasTags, IsFlowNode):
    """A reference to another statement."""

    type: StatementType = StatementType.REFERENCE
    reference: Statement | StatementReference = None
    description: str | None = None

    def _clear(self) -> None:
        HasTags._clear(self)
        IsFlowNode._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasTags._interp(self, scope)
        IsFlowNode._interp(self, scope)
        resolved = None
        if resolved is not None and not isinstance(self.reference, Statement):
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


@node(tracked=[])
class Block(Statement, HasTags):
    """A named block of statements."""

    type: StatementType = StatementType.BLOCK
    description: str | None = None

    def _visit(self, visitor: ModuleVisitor) -> None:
        for n in itertools.chain(self.tags):
            visitor.visit(n)


@node(tracked=["description"])
class Expectation(Statement):  # not clear how this will evolve yet
    type: StatementType = StatementType.EXPECTATION
    description: Optional[str] = None

    def _visit(self, visitor: ModuleVisitor) -> None:
        pass


# hard-coded, do not change ever :BenchUuidNamespace
BENCH_UUID_NAMESPACE = UUID("d822dab7-41ad-4706-a9c8-4379e15b2ed0")
