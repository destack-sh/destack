from collections.abc import Sequence
from typing import ClassVar, override

import opentelemetry.trace as trace
import structlog

from destack.language import (
    EditEvent,
    EntityStore,
    Query,
    QueryResult,
    StoreImplementation,
    StoreType,
)

from .core import MemoryContext, MemoryDatabase
from .entity import execute_edits
from .query import execute_query

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


class MemoryEntityStore(EntityStore):
    """An in-memory Store for Entities."""

    implementation: ClassVar[StoreImplementation | None] = StoreImplementation.MEMORY

    def __init__(self, types: tuple[StoreType, ...]):
        super().__init__(types)
        self.database = MemoryDatabase()
        self.context = MemoryContext(self.database)

    def __str__(self) -> str:
        num_nodes = sum(len(table.rows) for table in self.database.tables.values())
        return f"nodes={num_nodes}, tables={len(self.database.tables)}"

    def __repr__(self) -> str:
        return f"<MemoryStore {self!s}>"

    @override
    @tracer.start_as_current_span("memory.query")
    async def query(self, query: Query) -> QueryResult:
        result = execute_query(self.context, query)
        logger.debug("memory.query", query=query, result=result, span="current")
        return result

    @override
    @tracer.start_as_current_span("memory.commit")
    async def commit(self, events: Sequence[EditEvent]) -> Sequence[EditEvent]:
        edits, cascaded_edits = execute_edits(self.database, self.context, events)
        applied_edits = [*edits, *cascaded_edits]
        logger.trace(
            "memory.commit.edits",
            edits=len(edits),
            cascaded_edits=len(cascaded_edits),
            span="current",
        )
        return applied_edits
