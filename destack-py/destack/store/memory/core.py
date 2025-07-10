from collections.abc import Sequence
from typing import TYPE_CHECKING

from destack.language import (
    NodeDefinitionReference,
    NodeReference,
    NodeType,
)
from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_REFERENCE_BY_CLASS,
)

if TYPE_CHECKING:
    from .entity.core import MemoryEntityTable
    from .event.core import MemoryEventTable


class MemoryDatabase:
    """In-memory database of Nodes."""

    __slots__ = ("entity_tables", "event_tables")

    def __init__(self):
        self.entity_tables: dict[NodeType, MemoryEntityTable] = {}
        self.event_tables: dict[NodeType, MemoryEventTable] = {}

    def __str__(self) -> str:
        num_entities = sum(len(table.rows) for table in self.entity_tables.values())
        num_events = sum(len(table.rows) for table in self.event_tables.values())
        return f"entities={num_entities}, events={num_events}, tables={len(self.entity_tables)}"

    def __repr__(self) -> str:
        return f"<MemoryDatabase {self!s}>"


class MemoryContext:
    """In-memory context of a Database during a 'transaction'."""

    __slots__ = ("database",)

    def __init__(self, database: "MemoryDatabase"):
        self.database = database

    def __str__(self) -> str:
        return f"entity_tables={len(self.database.entity_tables)}, event_tables={len(self.database.event_tables)}"

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

    def get_entity_table(
        self, definition: NodeDefinitionReference | NodeReference
    ) -> "MemoryEntityTable":
        """Get the Table for a NodeDefinition."""
        from .entity.core import MemoryEntityTable

        if isinstance(definition, NodeReference):
            node_type = definition.type
        else:
            node_type = definition.node_type
        if node_type not in self.database.entity_tables:
            self.database.entity_tables[node_type] = MemoryEntityTable(
                database=self.database, node_type=node_type
            )
        return self.database.entity_tables[node_type]

    def get_event_table(
        self, definition: NodeDefinitionReference | NodeReference
    ) -> "MemoryEventTable":
        """Get the Table for an EventDefinition."""
        from .event.core import MemoryEventTable

        if isinstance(definition, NodeReference):
            node_type = definition.type
        else:
            node_type = definition.node_type
        if node_type not in self.database.event_tables:
            self.database.event_tables[node_type] = MemoryEventTable(
                database=self.database, node_type=node_type
            )
        return self.database.event_tables[node_type]
