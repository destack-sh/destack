from typing import Optional
from uuid import UUID

from bench.bench.core import Scope, Statement, StatementReference, StatementType, node
from bench.bench.issue import IssueType
from bench.bench.tag import HasTags


# common statements


@node(tracked=[])
class Blank(Statement):
    """A blank statement."""

    type: StatementType = StatementType.BLANK


@node(tracked=["text"])
class Text(Statement):
    """A comment that's not semantic/interpreted by default."""

    type: StatementType = StatementType.TEXT
    text: str | None = None


@node(tracked=["reference"])
class Reference(Statement, HasTags):
    """A reference to another statement."""

    type: StatementType = StatementType.REFERENCE
    reference: Statement | StatementReference = None

    def _interp(self, scope: Scope) -> None:
        if self.reference is None:
            pass
        elif not isinstance(self.reference, Statement):
            resolved = scope.lookup(self.reference)
            if resolved is None:
                self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
            else:
                self.reference = resolved

    @property
    def reference_id(self) -> Optional[UUID]:
        if isinstance(self.reference, Statement):
            return self.reference.id
        elif isinstance(self.reference, UUID):
            return self.reference
        else:
            return None


@node(tracked=[])
class Block(Statement, HasTags):
    """A named block of statements."""

    type: StatementType = StatementType.BLOCK
    description: str | None = None
