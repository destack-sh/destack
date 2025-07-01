from collections import defaultdict
from collections.abc import Sequence
from typing import Any, assert_never

from destack.language import (
    CustomEntityDefinition,
    Edit,
    NodeDefinitionReference,
    NodeDefinitionType,
    NodeReference,
    NodeType,
)
from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_REFERENCE_BY_CLASS,
    NODE_TYPES_BY_TRAIT_TYPE,
)
from destack.utils.uuid import UUID


class MemoryDatabase:
    """In-memory database of Nodes."""

    __slots__ = ("tables",)

    def __init__(self):
        self.tables: dict[tuple[NodeType, UUID | None], MemoryTable] = {}

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

    def apply(self, edits: Sequence[Edit]) -> Sequence[Edit]:
        """Apply the Edits to the context. Returns the Edits that were applied."""
        ...

    def resolve(self, definition: NodeDefinitionReference) -> Sequence[NodeDefinitionReference]:
        """Expand the specific Definitions for a NodeDefinitionReference."""
        if definition.type == NodeDefinitionType.BUILTIN_NODE:
            assert definition.node_type is not None, f"no node_type for {definition!r}"
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
        elif definition.type == NodeDefinitionType.CUSTOM_NODE:
            raise NotImplementedError(f"cannot resolve {definition!r}")
        elif definition.type == NodeDefinitionType.BUILTIN_TRAIT:
            assert definition.trait_type is not None, f"no trait_type for {definition!r}"
            node_types = NODE_TYPES_BY_TRAIT_TYPE.get(definition.trait_type, ())
            return tuple(
                NODE_DEFINITION_REFERENCE_BY_CLASS[node_cls]
                for node_type in node_types
                if not (node_cls := NODE_CLASS_BY_TYPE[node_type]).__is_abstract__
            )
        elif definition.type == NodeDefinitionType.CUSTOM_TRAIT:
            raise NotImplementedError(f"cannot resolve {definition!r}")
        else:
            assert_never(definition.type)

    def get(self, definition: NodeDefinitionReference | NodeReference) -> "MemoryTable":
        """Get the (single) Table for a node / definition. Doesn't work for multi-definitions."""
        assert definition.node_type is not None, f"no node_type for {definition!r}"
        if isinstance(definition, NodeReference):
            if definition.node_type != NodeType.CUSTOM_ENTITY:
                table_key = (definition.node_type, None)
            else:
                table_key = (definition.node_type, definition.definition_id)
        else:
            if definition.node_type != NodeType.CUSTOM_ENTITY:
                table_key = (definition.node_type, None)
            else:
                table_key = (
                    definition.node_type,
                    definition.definition_ptr.id if definition.definition_ptr else None,
                )
        if table_key not in self.database.tables:
            self.database.tables[table_key] = MemoryTable(
                database=self.database, metatype=definition.node_type, definition=None
            )
        return self.database.tables[table_key]

    def copy(self) -> "MemoryContext":
        return MemoryContext(self.database)


class MemoryTable:
    """In-memory table of Nodes for some definition."""

    __slots__ = (
        "database",
        "definition",
        "definition_id",
        "node_type",
        "rows",
        "rows_by_parent_id",
    )

    def __init__(
        self,
        database: "MemoryDatabase",
        metatype: NodeType,
        definition: CustomEntityDefinition | None,
    ):
        self.database = database
        self.node_type = metatype
        self.definition = definition
        self.definition_id = definition.id if definition else None
        self.rows: dict[UUID, MemoryRow] = {}
        self.rows_by_parent_id: dict[UUID, list[MemoryRow]] = defaultdict(list)

    def __str__(self) -> str:
        return f"node_type={self.node_type.name}, definition_id={self.definition_id}, rows={len(self.rows)}"

    def __repr__(self) -> str:
        return f"<MemoryTable {self!s}>"


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

    def __str__(self) -> str:
        return f"node_type={self.metatype.name}, id={self.id}, value={len(self.value)}"

    def __repr__(self) -> str:
        return f"<MemoryRow {self!s}>"
