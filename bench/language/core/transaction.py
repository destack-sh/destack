import dataclasses
from typing import TYPE_CHECKING

import structlog
from fastuuid import UUID, uuid4
from opentelemetry import trace

from .node import Node

if TYPE_CHECKING:
    from bench.language import Edit, Session

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def new_edit_id() -> str:
    return str(uuid4())


@dataclasses.dataclass(slots=True)
class Transaction:
    """
    A Transaction is an atomic sequence of Edits.
    Edits may belong to Changes, which may be carried out over multiple Transactions.
    """

    id: UUID
    session: "Session"
    is_readonly: bool = dataclasses.field(default=False)

    dirty: dict[UUID, Node] = dataclasses.field(default_factory=dict)

    edits: list["Edit"] = dataclasses.field(default_factory=list)
    cascaded_edits: list["Edit"] = dataclasses.field(default_factory=list)

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

    def create(self, node: Node):
        """Creates a new Node."""
        raise NotImplementedError

    def upsert(self, node: Node):
        """Creates or updates a Node."""
        raise NotImplementedError

    def update(self, node: Node):
        """Updates an existing Node."""
        raise NotImplementedError

    def move(self, node: Node, parent: Node):
        """Moves a Node to a new parent."""
        raise NotImplementedError

    def archive(self, node: Node):
        """Archives a Node."""
        raise NotImplementedError

    def unarchive(self, node: Node):
        """Unarchives a Node."""
        raise NotImplementedError

    def delete(self, node: Node):
        """Deletes a Node."""
        raise NotImplementedError

    def restore(self, node: Node):
        """Restores a deleted Node."""
        raise NotImplementedError

    def erase(self, node: Node):
        """Erases a Node."""
        raise NotImplementedError

    @tracer.start_as_current_span("transaction.commit")
    async def commit(self) -> tuple[list["Edit"], list["Edit"]]:
        """Commits the transaction (flushing any pending edits)."""
        raise NotImplementedError
