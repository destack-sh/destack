from collections.abc import Sequence
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
)

import structlog
from opentelemetry import trace

from destack.utils.uuid import UUID

from ..builtin import ACTIVE_SESSION, Entity, IsSubject, PropertyDeclaration, TypeCardinality
from ..common import (
    Change,
    ChangeResult,
    ChangeStatus,
    Edit,
    EditOperation,
    EditType,
    to_value,
)
from .graph import Supergraph
from .oracle import WORLD_ORACLE, Oracle

if TYPE_CHECKING:
    from destack.language import Edit, Origin, QueryConnection, Space, Store

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Session:
    """
    A managed Session for interacting with Destack.
    """

    __slots__ = (
        "_token",
        "changes",
        "closed_at",
        "connections",
        "edits",
        "oracle",
        "origin",
        "runtime",
        "space",
        "store",
        "subject",
        "supergraph",
    )

    def __init__(
        self,
        oracle: Oracle = WORLD_ORACLE,
        space: Optional["Space"] = None,
        origin: "Origin | None" = None,
        subject: IsSubject | None = None,
        store: "Store | None" = None,
    ):
        self.oracle: Oracle = oracle
        self.space: Space | None = space
        self.origin: Origin | None = origin
        self.subject: IsSubject | None = subject
        self.store: Store | None = store
        self.supergraph = Supergraph(self)

        # state/events tracking
        # nocheckin: track Events
        self.edits: list[Edit] = []
        self.changes: list[Change] = []

        # runtime
        self.connections: list[QueryConnection] = []
        self.closed_at: datetime | None = None
        self._token: Any | None = None

    def __str__(self) -> str:
        content_parts: list[str] = []
        if self.space:
            content_parts.append(f"destack={self.space.slug}")
        if self.subject is not None:
            content_parts.append(f"subject={self.subject!r}")
        if self.store is not None:
            content_parts.append(f"store={self.store!r}")
        if self.closed_at is not None:
            content_parts.append(f"closed_at={self.closed_at.isoformat()}")
        return ", ".join(content_parts)

    def __repr__(self) -> str:
        return f"<Session {self!s}>"

    async def open(self):
        """Opens the Session."""
        assert self.closed_at is None, f"{self!r} is already closed"
        assert self._token is None, f"{self!r} is already open"
        self._token = ACTIVE_SESSION.set(self)

    async def close(self):
        """Closes the Session."""
        assert self.closed_at is None, f"{self!r} is already closed"
        if self._token is not None:
            try:  # noqa: SIM105
                ACTIVE_SESSION.reset(self._token)
            except ValueError:
                pass  # token was created in a different context (during testing usually)
            self._token = None
        self.closed_at = self.oracle.utc()

    def create(self, node: Entity):
        """Creates a new Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.CREATE, node=node, value=to_value(node, node_as_value=True))
        self.edits.append(edit)
        node._is_new = False
        node._is_attached = True

    def upsert(self, node: Entity):
        """Creates or updates an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.UPSERT, node=node, value=to_value(node, node_as_value=True))
        self.edits.append(edit)
        node._is_new = False
        node._is_attached = True

    def update_set_property(self, node: Entity, prop: PropertyDeclaration, new_value: Any):
        """Set a Property on this Node (direct SET/CLEAR operations)."""
        old_value = getattr(node, prop.name)
        node_ptr = node.to_ref()
        prop_ptr = prop.to_ref()
        prop_type = prop.to_type()

        # undo
        if old_value is None or (prop.cardinality != TypeCardinality.SCALAR and not old_value):
            undo_operation = EditOperation.CLEAR
            old_value = None
        else:
            undo_operation = EditOperation.SET
            old_value = to_value(old_value, prop_type)

        # do
        if new_value is None or (prop.cardinality != TypeCardinality.SCALAR and not new_value):
            operation = EditOperation.CLEAR
            new_value = None
        else:
            operation = EditOperation.SET
            new_value = to_value(new_value, prop_type)

        undo_edit = Edit(
            type=EditType.UPDATE,
            node_ptr=node_ptr,
            attribute=prop_ptr,
            operation=undo_operation,
            value=old_value,
        )
        edit = Edit(
            type=EditType.UPDATE,
            node_ptr=node_ptr,
            attribute=prop_ptr,
            operation=operation,
            value=new_value,
            undo=undo_edit,
        )
        self.edits.append(edit)

    def update(self, node: Entity, edit: Edit):
        """Updates an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        self.edits.append(edit)

    def move(self, node: Entity, parent: Entity):
        """Moves an Entity to a new parent."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.MOVE, node=node, value=to_value(parent))
        self.edits.append(edit)

    def archive(self, node: Entity):
        """Archives an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        undo_edit = Edit(
            type=EditType.UNARCHIVE, node=node, value=to_value(node, node_as_value=True)
        )
        edit = Edit(type=EditType.ARCHIVE, node=node, undo=undo_edit)
        self.edits.append(edit)

    def unarchive(self, node: Entity):
        """Unarchives an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.UNARCHIVE, node=node)
        self.edits.append(edit)

    def delete(self, node: Entity):
        """Deletes an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        undo_edit = Edit(type=EditType.RESTORE, node=node, value=to_value(node, node_as_value=True))
        edit = Edit(type=EditType.DELETE, node=node, undo=undo_edit)
        self.edits.append(edit)

    def restore(self, node: Entity):
        """Restores a deleted Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.RESTORE, node=node)
        self.edits.append(edit)

    def flush(self):
        """Turn pending updates into Edits, and Edits into Changes."""
        # turn unassigned Edits into a Change
        if self.edits:
            change = Change(edits=self.edits, created_by=self.subject, origin=self.origin)
            self.edits = []
            self.changes.append(change)

    async def stage(self):
        """Stage pending Edits. Also stages pending Changes in the Store if possible."""
        assert self.closed_at is None, f"{self!r} is closed"
        self.flush()

    async def commit(self) -> Sequence[ChangeResult]:
        """
        Commits all Changes/Edits. Returns applied Changes.
        """
        assert self.closed_at is None, f"{self!r} is closed"
        assert self.store is not None, f"{self!r} has no Store"
        self.flush()
        changes = list(self.changes)
        self.changes = []
        results = await self.store.commit(changes)
        if any(result.status != ChangeStatus.COMPLETED for result in results):
            changes_by_id: dict[UUID, Change] = {change.id: change for change in changes}
            failed_changes = [
                changes_by_id[result.id]
                for result in results
                if result.status != ChangeStatus.COMPLETED
            ]
            raise RuntimeError(
                f"failed to commit {len(failed_changes)} Changes: {failed_changes!r}"
            )
        return results

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
