from collections.abc import Sequence
from typing import TYPE_CHECKING

from destack.language import (
    NodeDefinitionReference,
    NodeReference,
    NodeType,
    StoreDomain,
)
from destack.language.registry import NODE_CLASS_BY_TYPE, SUBDEFINITIONS_BY_NODE_TYPE

if TYPE_CHECKING:
    from .entity.core import MemoryEntityTable
    from .event.core import MemoryEventTable


class MemoryDatabase:
    """In-memory database of Nodes."""

    __slots__ = ("entity_tables", "event_tables")

    def __init__(self):
        from .entity.core import MemoryEntityTable
        from .event.core import MemoryEventTable

        self.entity_tables: dict[NodeType, MemoryEntityTable] = {}
        self.event_tables: dict[NodeType, MemoryEventTable] = {}

        # init tables
        for node_class in NODE_CLASS_BY_TYPE.values():
            if node_class.__definition__.is_abstract:
                continue
            elif node_class.__definition__.store_domain == StoreDomain.ENTITY:
                self.entity_tables[node_class.metatype] = MemoryEntityTable(
                    self, node_class.metatype
                )
            elif node_class.__definition__.store_domain == StoreDomain.EVENT:
                self.event_tables[node_class.metatype] = MemoryEventTable(self, node_class.metatype)
            else:
                raise ValueError(f"unknown store domain for {node_class.metatype}")

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
        return SUBDEFINITIONS_BY_NODE_TYPE[definition.node_type]

    def get_entity_table(
        self, definition: NodeDefinitionReference | NodeReference
    ) -> "MemoryEntityTable":
        """Get the Table for a NodeDefinition."""

        if isinstance(definition, NodeReference):
            node_type = definition.type
        else:
            node_type = definition.node_type
        if node_type not in self.database.entity_tables:
            raise ValueError(f"entity table for {node_type} not found in {self!s}")
        return self.database.entity_tables[node_type]

    def get_event_table(
        self, definition: NodeDefinitionReference | NodeReference
    ) -> "MemoryEventTable":
        """Get the Table for an EventDefinition."""

        if isinstance(definition, NodeReference):
            node_type = definition.type
        else:
            node_type = definition.node_type
        if node_type not in self.database.event_tables:
            raise ValueError(f"event table for {node_type} not found in {self!s}")
        return self.database.event_tables[node_type]
