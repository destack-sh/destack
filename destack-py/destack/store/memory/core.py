from collections import defaultdict
from collections.abc import Sequence
from typing import Any, NamedTuple

from destack.language import (
    Materialization,
    NodeDefinitionReference,
    NodeReference,
    NodeType,
)
from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_REFERENCE_BY_CLASS,
)
from destack.utils.uuid import UUID


class VersionedNodeKey(NamedTuple):
    id: UUID
    snapshot_id: UUID | None


class MemoryDatabase:
    """In-memory database of Nodes."""

    __slots__ = ("tables",)

    def __init__(self):
        self.tables: dict[NodeType, MemoryTable] = {}

    def __str__(self) -> str:
        num_nodes = sum(len(table.rows) for table in self.tables.values())
        return f"nodes={num_nodes}, tables={len(self.tables)}"

    def __repr__(self) -> str:
        return f"<MemoryDatabase {self!s}>"


class MemoryContext:
    """In-memory context of a Database during a 'transaction'."""

    __slots__ = ("database",)

    def __init__(self, database: "MemoryDatabase"):
        self.database = database

    def __str__(self) -> str:
        return f"tables={len(self.database.tables)}"

    def __repr__(self) -> str:
        return f"<MemoryContext {self!s}>"

    def resolve(self, definition: NodeDefinitionReference) -> Sequence[NodeDefinitionReference]:
        """Expand the (separately) stored definitions for a NodeDefinition."""
        node_cls = NODE_CLASS_BY_TYPE[definition.node_type]
        if not node_cls.__inherited_by__:
            return (definition,)
        subdefinitions: list[NodeDefinitionReference] = []
        if not node_cls.__is_abstract__:
            subdefinitions.append(definition)
        for subnode_type in node_cls.__inherited_by__:
            subnode_cls = NODE_CLASS_BY_TYPE[subnode_type]
            if not subnode_cls.__is_abstract__:
                subdefinitions.append(NODE_DEFINITION_REFERENCE_BY_CLASS[subnode_cls])
        return tuple(subdefinitions)

    def get(self, definition: NodeDefinitionReference | NodeReference) -> "MemoryTable":
        """Get the Table for a NodeDefinition."""
        if isinstance(definition, NodeReference):
            node_type = definition.type
        else:
            node_type = definition.node_type
        if node_type not in self.database.tables:
            self.database.tables[node_type] = MemoryTable(
                database=self.database, metatype=node_type
            )
        return self.database.tables[node_type]


class MemoryTable:
    """In-memory table of Nodes for some definition."""

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
        metatype: NodeType,
    ):
        self.database = database
        self.node_type = metatype

        self.rows: dict[VersionedNodeKey, MemoryRow] = {}
        self.rows_by_parent: dict[VersionedNodeKey, list[MemoryRow]] = defaultdict(list)
        self.rows_by_snapshot: dict[UUID | None, dict[UUID, MemoryRow]] = defaultdict(dict)

    def __str__(self) -> str:
        return f"node_type={self.node_type.name}, rows={len(self.rows)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"


class MemoryRow:
    """In-memory row of a Node."""

    __slots__ = (
        "id",
        "materialization",
        "metatype",
        "parent_ptr",
        "ptr",
        "snapshot_id",
        "table",
        "value",
    )

    def __init__(
        self,
        table: "MemoryTable",
        metatype: NodeType,
        id: UUID,
        snapshot_id: UUID | None,
        materialization: Materialization,
        ptr: NodeReference,
        parent_ptr: NodeReference | None,
        value: dict[str, Any],
    ):
        self.table = table
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
        return f"<MemoryRow {self!s}>"
