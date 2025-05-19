import dataclasses
from datetime import datetime
from typing import TYPE_CHECKING, Any, Callable, Sequence

import structlog
from fastuuid import UUID, uuid4
from opentelemetry import trace

from bench.pb2 import (
    EditData,
    EditOperationData,
)

from .const import EditType
from .node import Node

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
        return f"[id={self.id}] ({len(self._edits)} edits, {len(self._cascaded_edits)} cascaded, {len(self._pending_edit_events) + len(self._pending_edits)} pending)"

    def __repr__(self):
        return f"<Transaction {self}>"

    @property
    def has_edits(self) -> bool:
        return len(self._edits) > 0 or self.has_pending_edits

    @property
    def has_pending_edits(self) -> bool:
        return len(self._pending_edit_events) > 0 or len(self._pending_edits) > 0

    #
    # Transaction management
    #

    def record_edit_event(
        self,
        type: EditType,
        node: Node,
        *,
        now: datetime | None = None,
        operation: EditOperationData | None = None,
    ):
        """
        Records an edit event (which are later summed into actual edits).
        We try to be efficient and record minimal information quickly and only as needed.
        """

        raise NotImplementedError

    def _add_pending_edits(self, edits: Sequence[EditData]):
        """Adds full edits to the transaction directly."""
        self._pending_edits.extend(edits)

    def _track_edits(self, edits: Sequence[EditData]):
        """Tracks edits in our logical clock (local epoch)."""
        # assign local epoch if we have one
        epoch = self.session._local_epoch
        assert epoch is not None, f"no local epoch for {self.session!r}"
        for edit in edits:
            edit.epoch = epoch
            epoch += 1
        self.session._local_epoch = epoch

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
        self._edits = []
        self._cascaded_edits = []
        self._pending_edit_events = []
        self._touched_engine_ids.clear()
