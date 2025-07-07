from collections.abc import AsyncIterator, Sequence
from typing import ClassVar, override

import opentelemetry.trace as trace
import structlog

from destack.language import (
    Event,
    Query,
    QueryResult,
    QueryUpdate,
    Store,
    StoreImplementation,
    StoreType,
)

from .core import MemoryContext, MemoryDatabase
from .edit import execute_events
from .query import execute_query

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


class MemoryStore(Store):
    """An in-memory Store."""

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
    async def commit(self, events: Sequence[Event]) -> Sequence[Event]:
        results: list[Event] = []
        for event in events:
            edits, cascaded_edits = execute_events(self.database, self.context, event)
            result = EventResult(
                id=event.id,
                status=EventStatus.COMPLETED,
                edits=list(edits),
                cascaded_edits=list(cascaded_edits),
            )
            results.append(result)
            logger.trace(
                "memory.commit.change",
                change=change,
                result=result,
                span="current",
            )
        logger.debug("memory.commit", changes=changes, results=results, span="current")
        return results

    @override
    async def subscribe(self, query: Query) -> AsyncIterator[QueryUpdate]:
        raise NotImplementedError
