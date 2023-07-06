from bench.bench.core import Statement, StatementType, node
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


@node(tracked=[])
class Block(Statement, HasTags):
    """A named block of statements."""

    type: StatementType = StatementType.BLOCK
