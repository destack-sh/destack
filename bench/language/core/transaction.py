import dataclasses
from typing import TYPE_CHECKING

import structlog
from fastuuid import UUID, uuid4
from opentelemetry import trace

from bench.pb2 import EditData

if TYPE_CHECKING:
    from bench.language import Session

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def new_edit_id() -> str:
    return str(uuid4())


@dataclasses.dataclass(slots=True)
class Transaction:
    """
    A transaction is an atomic list of Edits (may be multiple sets of changes).
    """

    id: UUID
    session: "Session"
    is_readonly: bool = dataclasses.field(default=False)

    edits: list[EditData] = dataclasses.field(default_factory=list)
    cascaded_edits: list[EditData] = dataclasses.field(default_factory=list)

    def __str__(self):
        return f"[id={self.id}] ({len(self.edits)} edits, {len(self.cascaded_edits)} cascaded)"

    def __repr__(self):
        return f"<Transaction {self}>"

    @property
    def has_edits(self) -> bool:
        raise NotImplementedError

    @property
    def has_pending_edits(self) -> bool:
        raise NotImplementedError

    @tracer.start_as_current_span("transaction.commit")
    async def commit(self) -> tuple[list[EditData], list[EditData]]:
        """Commits the transaction (flushing any pending edits)."""
        raise NotImplementedError
