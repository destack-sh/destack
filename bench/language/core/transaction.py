import dataclasses
from typing import TYPE_CHECKING, Any, Callable

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

    """All edits from this transaction (since the previous commit)."""
    _edits: list[EditData] = dataclasses.field(default_factory=list)
    _cascaded_edits: list[EditData] = dataclasses.field(default_factory=list)
    _touched_engine_ids: set[Any] = dataclasses.field(default_factory=set)

    def __str__(self):
        return f"[id={self.id}] ({len(self._edits)} edits, {len(self._cascaded_edits)} cascaded)"

    def __repr__(self):
        return f"<Transaction {self}>"

    @property
    def has_edits(self) -> bool:
        raise NotImplementedError

    @property
    def has_pending_edits(self) -> bool:
        raise NotImplementedError

    @tracer.start_as_current_span("transaction.flush")
    async def flush(
        self, filter: Callable[[EditData], bool] | None = None
    ) -> tuple[list[EditData], list[EditData]]:
        """Flushes any pending edits (without committing)."""
        raise NotImplementedError

    @tracer.start_as_current_span("transaction.commit")
    async def commit(self) -> tuple[list[EditData], list[EditData]]:
        """Commits the transaction (flushing any pending edits)."""
        raise NotImplementedError

    def reset(self):
        """Resets the transaction, any edits and connectors (without closing)."""
        raise NotImplementedError
