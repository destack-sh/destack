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

from ..builtin import ACTIVE_SESSION, IsSubject, Node, TypeCardinality
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
from .store import OptimisticStore

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
        "dirty",
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
        self.supergraph = Supergraph(self)
        self.oracle: Oracle = oracle
        self.space: Space | None = space
        self.origin: Origin | None = origin
        self.subject: IsSubject | None = subject
        self.store: Store | None = store

        # transaction (pending)
        self.dirty: dict[UUID, Node] = {}
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
        assert self._token is None, f"{self!r} is already open"
        self._token = ACTIVE_SESSION.set(self)

    async def close(self):
        """Closes the Session."""
        if self._token is not None:
            try:  # noqa: SIM105
                ACTIVE_SESSION.reset(self._token)
            except ValueError:
                pass  # token was created in a different context (during testing usually)
            self._token = None
        self.closed_at = self.oracle.utc()

    def create(self, node: Node):
        """Creates a new Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.CREATE, node=node, value=to_value(node, node_as_value=True))
        self.edits.append(edit)
        self.dirty[node.id] = node
        node._is_new = False
        node._is_attached = True

    def upsert(self, node: Node):
        """Creates or updates a Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.UPSERT, node=node, value=to_value(node, node_as_value=True))
        self.edits.append(edit)
        self.dirty[node.id] = node
        node._is_new = False
        node._is_attached = True

    def update(self, node: Node, edit: Edit):
        """Updates a Node."""
        self._flush_node(node)
        self.edits.append(edit)
        self.dirty[node.id] = node

    def move(self, node: Node, parent: Node):
        """Moves a Node to a new parent."""
        raise NotImplementedError

    def archive(self, node: Node):
        """Archives a Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        self._flush_node(node)
        undo_edit = Edit(
            type=EditType.UNARCHIVE, node=node, value=to_value(node, node_as_value=True)
        )
        edit = Edit(type=EditType.ARCHIVE, node=node, undo=undo_edit)
        self.edits.append(edit)
        self.dirty[node.id] = node

    def unarchive(self, node: Node):
        """Unarchives a Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        self._flush_node(node)
        edit = Edit(type=EditType.UNARCHIVE, node=node)
        self.edits.append(edit)
        self.dirty[node.id] = node

    def delete(self, node: Node):
        """Deletes a Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        self._flush_node(node)
        undo_edit = Edit(type=EditType.RESTORE, node=node, value=to_value(node, node_as_value=True))
        edit = Edit(type=EditType.DELETE, node=node, undo=undo_edit)
        self.edits.append(edit)
        self.dirty[node.id] = node

    def restore(self, node: Node):
        """Restores a deleted Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        self._flush_node(node)
        edit = Edit(type=EditType.RESTORE, node=node)
        self.edits.append(edit)
        self.dirty[node.id] = node

    def erase(self, node: Node):
        """Erases a Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        self._flush_node(node)
        undo_edit = Edit(type=EditType.CREATE, node=node, value=to_value(node, node_as_value=True))
        edit = Edit(type=EditType.ERASE, node=node, undo=undo_edit)
        self.edits.append(edit)
        self.dirty[node.id] = node

    def _flush_node(self, node: Node):
        """Turn a dirty Node into Edits."""
        if node._is_new:
            node._is_new = False
        elif node._dirty is not None:
            # turn dirty properties into Edits (basic SET/CLEAR operations)
            node_ptr = node.to_ref()
            for prop_name, prop_old_value in node._dirty.items():
                prop = node.__properties__[prop_name]
                prop_ptr = prop.to_ref()

                # undo
                if prop_old_value is None or (
                    prop.cardinality != TypeCardinality.SCALAR and not prop_old_value
                ):
                    undo_operation = EditOperation.CLEAR
                    old_value = None
                else:
                    undo_operation = EditOperation.SET
                    old_value = to_value(prop_old_value, prop.type)

                # do
                prop_new_value = getattr(node, prop_name)
                if prop_new_value is None or (
                    prop.cardinality != TypeCardinality.SCALAR and not prop_new_value
                ):
                    operation = EditOperation.CLEAR
                    new_value = None
                else:
                    operation = EditOperation.SET
                    new_value = to_value(prop_new_value, prop.type)

                undo_edit = Edit(
                    type=EditType.UPDATE,
                    node_ptr=node_ptr,
                    prop_ptr=prop_ptr,
                    operation=undo_operation,
                    value=old_value,
                )
                edit = Edit(
                    type=EditType.UPDATE,
                    node_ptr=node_ptr,
                    prop_ptr=prop_ptr,
                    operation=operation,
                    value=new_value,
                    undo=undo_edit,
                )
                self.edits.append(edit)

    def flush(self):
        """Turn pending updates into Edits, and Edits into Changes."""
        # flush dirty Nodes
        if self.dirty:
            for node in self.dirty.values():
                self._flush_node(node)
            self.dirty.clear()
        # turn unassigned Edits into a Change
        if self.edits:
            change = Change(edits=self.edits, created_by=self.subject, origin=self.origin)
            self.edits = []
            self.changes.append(change)

    async def stage(self):
        """Stage pending Edits. Also stages pending Changes in the Store if possible."""
        assert self.store is not None, f"{self!r} has no Store"
        self.flush()
        if isinstance(self.store, OptimisticStore):
            await self.store.stage(self.changes)

    async def commit(self) -> Sequence[ChangeResult]:
        """
        Commits all Changes/Edits. Returns applied Changes.
        """
        assert self.store is not None, f"{self!r} has no Store"
        self.flush()
        results = await self.store.commit(self.changes)
        if any(result.status != ChangeStatus.COMPLETED for result in results):
            changes_by_id: dict[UUID, Change] = {change.id: change for change in self.changes}
            failed_changes = [
                changes_by_id[result.id]
                for result in results
                if result.status != ChangeStatus.COMPLETED
            ]
            raise RuntimeError(
                f"failed to commit {len(failed_changes)} Changes: {failed_changes!r}"
            )
        self.changes = []
        return results

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
