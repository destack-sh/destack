from collections.abc import Sequence
from typing import ClassVar, override

import opentelemetry.trace as trace
import structlog

from destack.language import (
    EditEvent,
    EntityStore,
    Event,
    EventStore,
    NodeType,
    Query,
    QueryResult,
    StoreImplementation,
    StoreKey,
    to_value,
)
from destack.language.registry import get_node_types_for_stores

from .core import MemoryContext, MemoryDatabase, MemoryTable, VersionedNodeKey
from .edit import execute_edits
from .query import execute_query
from .wiring import pack_node_row

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


class MemoryEntityStore(EntityStore):
    """An in-memory Store for Entities."""

    implementation: ClassVar[StoreImplementation | None] = StoreImplementation.MEMORY

    def __init__(self, types: tuple[StoreKey, ...], database: MemoryDatabase | None = None):
        self.keys = types
        self.node_types = get_node_types_for_stores(types)
        self.database = database or MemoryDatabase()
        self.context = MemoryContext(self.database)

    def __str__(self) -> str:
        num_nodes = sum(len(table.rows) for table in self.database.tables.values())
        return f"nodes={num_nodes}, tables={len(self.database.tables)}"

    def __repr__(self) -> str:
        return f"<MemoryEntityStore {self!s}>"

    @override
    @tracer.start_as_current_span("memory.query")
    async def query(self, query: Query) -> QueryResult:
        result = execute_query(self.context, query)
        logger.debug("memory.query", query=query, result=result, span="current")
        return result

    @override
    @tracer.start_as_current_span("memory.commit")
    async def commit(self, events: Sequence[EditEvent]) -> Sequence[EditEvent]:
        edits, cascaded_edits = execute_edits(self.context, events)
        applied_edits = [*edits, *cascaded_edits]
        logger.trace(
            "memory.commit.edits",
            edits=len(edits),
            cascaded_edits=len(cascaded_edits),
            span="current",
        )
        return applied_edits


class MemoryEventStore(EventStore):
    """An in-memory Store for Events."""

    implementation: ClassVar[StoreImplementation | None] = StoreImplementation.MEMORY

    def __init__(self, types: tuple[StoreKey, ...], database: MemoryDatabase | None = None):
        self.keys = types
        self.node_types = get_node_types_for_stores(types)
        self.database = database or MemoryDatabase()
        self.context = MemoryContext(self.database)

    def __str__(self) -> str:
        num_nodes = sum(len(table.rows) for table in self.database.tables.values())
        return f"nodes={num_nodes}, tables={len(self.database.tables)}"

    def __repr__(self) -> str:
        return f"<MemoryEventStore {self!s}>"

    @override
    @tracer.start_as_current_span("memory.query")
    async def query(self, query: Query) -> QueryResult:
        result = execute_query(self.context, query)
        logger.debug("memory.query", query=query, result=result, span="current")
        return result

    @override
    @tracer.start_as_current_span("memory.append")
    async def append(self, events: Sequence[Event]) -> Sequence[Event]:
        for event in events:
            node_type = NodeType(event.metatype)
            if node_type not in self.database.tables:
                self.database.tables[node_type] = MemoryTable(
                    database=self.database, metatype=node_type
                )
            table = self.database.tables[node_type]
            event_value = to_value(event, node_as_value=True)
            row = pack_node_row(table, event_value)
            row_key = VersionedNodeKey(id=row.id, snapshot_id=row.snapshot_id)
            table.rows[row_key] = row
        return events
