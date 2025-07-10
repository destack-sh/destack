from collections.abc import Sequence
from typing import assert_never, cast, override

from destack.language import (
    EditEvent,
    EntityStore,
    Event,
    EventStore,
    Query,
    QueryResult,
    StoreDomain,
    StoreKey,
)
from destack.language.registry import get_node_types_for_stores

from .core import MemoryContext, MemoryDatabase
from .entity.store import MemoryEntityStore
from .event.store import MemoryEventStore


class MemoryStore(EntityStore, EventStore):
    """An in-memory Store for Entities and Events."""

    def __init__(self, keys: tuple[StoreKey, ...], database: MemoryDatabase | None = None):
        self.keys = keys
        self.node_types = get_node_types_for_stores(keys)
        self.database = database or MemoryDatabase()
        self.context = MemoryContext(self.database)

        self.entity_store = MemoryEntityStore(keys, self.database)
        self.event_store = MemoryEventStore(keys, self.database)

    def __str__(self) -> str:
        return f"entities={self.entity_store!s}, events={self.event_store!s}"

    def __repr__(self) -> str:
        return f"<MemoryStore {self!s}>"

    @override
    async def query(self, query: Query) -> QueryResult:
        if query.domain == StoreDomain.ENTITY:
            return await self.entity_store.query(query)
        elif query.domain == StoreDomain.EVENT:
            return await self.event_store.query(query)
        else:
            assert_never(query.domain)

    @override
    async def commit(self, events: Sequence[EditEvent]) -> Sequence[EditEvent]:
        applied_events = await self.append(events)
        return cast(Sequence[EditEvent], applied_events)

    @override
    async def append(self, events: Sequence[Event]) -> Sequence[Event]:
        applied_events = await self.event_store.append(events)
        edit_events = [event for event in applied_events if isinstance(event, EditEvent)]
        await self.entity_store.commit(edit_events)
        return applied_events
