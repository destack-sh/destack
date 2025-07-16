from collections.abc import Sequence
from typing import override

import opentelemetry.trace as trace
import structlog

from destack.language import (
    Event,
    EventStore,
    Query,
    QueryResult,
    StoreKey,
)
from destack.language.registry import get_node_types_for_stores

from ..core import MemoryContext, MemoryDatabase
from .append import execute_append
from .query import execute_query

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


class MemoryEventStore(EventStore):
    """An in-memory Store for Events."""

    def __init__(self, keys: tuple[StoreKey, ...], database: MemoryDatabase | None = None):
        self.keys = keys
        self.node_types = get_node_types_for_stores(keys)
        self.database = database or MemoryDatabase()
        self.context = MemoryContext(self.database)

    def __str__(self) -> str:
        num_nodes = sum(len(table.rows) for table in self.database.event_tables.values())
        return f"nodes={num_nodes}"

    def __repr__(self) -> str:
        return f"<MemoryEventStore {self!s}>"

    @override
    async def open(self) -> None:
        pass

    @override
    async def close(self) -> None:
        pass

    @override
    @tracer.start_as_current_span("memory.query")
    async def query(self, query: Query) -> QueryResult:
        result = execute_query(self.context, query)
        logger.debug("memory.query", query=query, result=result, span="current")
        return result

    @override
    @tracer.start_as_current_span("memory.append")
    async def append(self, events: Sequence[Event]) -> Sequence[Event]:
        return execute_append(self.context, events)
