from collections import defaultdict
from typing import TYPE_CHECKING, Any, NamedTuple

from destack.language import (
    Materialization,
    NodeReference,
    NodeType,
)
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from ..core import MemoryDatabase


class VersionedNodeKey(NamedTuple):
    id: UUID
    snapshot_id: UUID | None


class MemoryEntityTable:
    """In-memory table of Entities for some NodeDefinition."""

    __slots__ = (
        "database",
        "node_type",
        "rows",
        "rows_by_parent",
        "rows_by_snapshot",
    )

    def __init__(
        self,
        database: "MemoryDatabase",
        node_type: NodeType,
    ):
        self.database = database
        self.node_type = node_type

        self.rows: dict[VersionedNodeKey, MemoryEntityRow] = {}
        self.rows_by_parent: dict[VersionedNodeKey, list[MemoryEntityRow]] = defaultdict(list)
        self.rows_by_snapshot: dict[UUID | None, dict[UUID, MemoryEntityRow]] = defaultdict(dict)

    def __str__(self) -> str:
        return f"node_type={self.node_type.name}, rows={len(self.rows)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"


class MemoryEntityRow:
    """In-memory row of a Node."""

    __slots__ = (
        "id",
        "materialization",
        "metatype",
        "parent_ptr",
        "ptr",
        "snapshot_id",
        "value",
    )

    def __init__(
        self,
        metatype: NodeType,
        id: UUID,
        snapshot_id: UUID | None,
        materialization: Materialization,
        ptr: NodeReference,
        parent_ptr: NodeReference | None,
        value: dict[str, Any],
    ):
        self.metatype = metatype
        self.id = id
        self.snapshot_id = snapshot_id
        self.materialization = materialization
        self.ptr = ptr
        self.parent_ptr = parent_ptr
        self.value = value

    def __str__(self) -> str:
        return f"node_type={self.metatype.name}, id={self.id}, value={len(self.value)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"
