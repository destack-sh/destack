from datetime import datetime
from typing import TYPE_CHECKING, Any

from destack.language import (
    NodeReference,
    NodeType,
)
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from ..core import MemoryDatabase


class MemoryEventTable:
    """In-memory table of Events for some EventDefinition."""

    __slots__ = ("database", "node_type", "rows", "rows_sorted")

    def __init__(self, database: "MemoryDatabase", node_type: NodeType):
        self.database = database
        self.node_type = node_type
        self.rows: dict[UUID, MemoryEventRow] = {}
        self.rows_sorted: list[MemoryEventRow] = []  # sorted by created_at

    def __str__(self) -> str:
        return f"node_type={self.node_type.name}, rows={len(self.rows)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"


class MemoryEventRow:
    """In-memory row of an Event."""

    __slots__ = ("created_at", "id", "metatype", "ptr", "snapshot_id", "value")

    def __init__(
        self,
        id: UUID,
        snapshot_id: UUID | None,
        metatype: NodeType,
        ptr: NodeReference,
        created_at: datetime,
        value: dict[str, Any],
    ):
        self.id = id
        self.snapshot_id = snapshot_id
        self.metatype = metatype
        self.ptr = ptr
        self.created_at = created_at
        self.value = value

    def __str__(self) -> str:
        return f"node_type={self.metatype.name}, id={self.id}, value={len(self.value)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"
