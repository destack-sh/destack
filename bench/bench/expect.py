from __future__ import annotations

from typing import Optional

from bench.bench.const import StatementType
from bench.bench.core import Statement, StatementReference, node


@node(tracked=["reference", "description"])
class Expectation(Statement):
    type: StatementType = StatementType.EXPECTATION
    reference: StatementReference | Statement | None = None
    description: Optional[str] = None
