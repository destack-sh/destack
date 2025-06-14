from collections.abc import Sequence
from typing import Any, assert_never

from fastuuid import UUID

from destack.language import (
    CustomEntityDefinition,
    Edit,
    NodeReference,
    NodeType,
    RelationReference,
    RelationType,
)
from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    NODE_TYPES_BY_TRAIT_TYPE,
    RELATION_REF_BY_CLASS,
)

# ruff: noqa: B903


class MemoryDatabase:
    """In-memory database of Nodes."""

    __slots__ = ("tables",)

    def __init__(self):
        self.tables: dict[tuple[NodeType, UUID | None], MemoryTable] = {}


class MemoryContext:
    """In-memory context of a Database during a 'transaction'."""

    __slots__ = ("database",)

    def __init__(self, database: "MemoryDatabase"):
        self.database = database

    def apply(self, edits: Sequence[Edit]) -> Sequence[Edit]:
        """Apply the Edits to the context. Returns the Edits that were applied."""
        ...

    def resolve_relation(self, relation: RelationReference) -> Sequence[RelationReference]:
        """Expand the specific Relations for a RelationReference."""
        if relation.type in (RelationType.BUILTIN_NODE, RelationType.CUSTOM_NODE):
            return (relation,)
        elif relation.type == RelationType.TRAIT:
            assert relation.trait_type is not None, f"no trait_type for {relation!r}"
            node_types = NODE_TYPES_BY_TRAIT_TYPE.get(relation.trait_type, ())
            return tuple(
                RELATION_REF_BY_CLASS[NODE_CLASS_BY_TYPE[node_type]] for node_type in node_types
            )
        else:
            assert_never(relation.type)

    def get_relation(self, relation: RelationReference | NodeReference) -> "MemoryTable":
        """Get the (single) Table for a node / relation. Doesn't work for multi-relations."""
        assert relation.node_type is not None, f"no node_type for {relation!r}"
        table_key = (relation.node_type, relation.definition_id)
        if table_key not in self.database.tables:
            self.database.tables[table_key] = MemoryTable(
                database=self.database, metatype=relation.node_type, definition=None
            )
        return self.database.tables[table_key]

    def copy(self) -> "MemoryContext":
        return MemoryContext(self.database)


class MemoryTable:
    """In-memory table of Nodes for some relation."""

    __slots__ = ("database", "definition", "definition_id", "metatype", "rows")

    def __init__(
        self,
        database: "MemoryDatabase",
        metatype: NodeType,
        definition: CustomEntityDefinition | None,
    ):
        self.database = database
        self.metatype = metatype
        self.definition = definition
        self.definition_id = definition.id if definition else None
        self.rows: dict[UUID, MemoryRow] = {}


class MemoryRow:
    """In-memory row of a Node."""

    __slots__ = ("id", "metatype", "parent_ptr", "ptr", "table", "value")

    def __init__(
        self,
        table: "MemoryTable",
        metatype: NodeType,
        id: UUID,
        ptr: NodeReference,
        parent_ptr: NodeReference | None,
        value: dict[str, Any],
    ):
        self.table = table
        self.metatype = metatype
        self.id = id
        self.ptr = ptr
        self.parent_ptr = parent_ptr
        self.value = value
