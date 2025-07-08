from collections.abc import AsyncGenerator, Sequence
from typing import ClassVar, override

from destack.language import (
    EditEvent,
    Event,
    EventStore,
    LiveStore,
    Query,
    QueryResult,
    QueryUpdate,
    Store,
    StoreImplementation,
    StoreType,
)
from destack.store.memory import MemoryEntityStore

# nocheckin: implement BufferedStore
#  (keep store_type/snapshot/... in Node & NodeReference instances?)
#  (including live/in-memory overrides with Snapshots?)


class BufferedStore(EventStore, LiveStore):
    """
    Route Queries and commits to underlying Stores, buffer certain Events in memory.
    Does not support atomic Events across Stores (yet).
    """

    implementation: ClassVar[StoreImplementation | None] = None  # no single implementation

    def __init__(self, *stores: Store):
        self.stores: tuple[Store, ...] = stores
        self.store_by_type: dict[StoreType, Store] = {}
        for store in stores:
            for store_type in store.types:
                if store_type in self.store_by_type:
                    raise ValueError(
                        f"already have a {store_type.name} Store: {self.store_by_type[store_type]!r} != {store!r}"
                    )
                self.store_by_type[store_type] = store
        self.buffer: MemoryEntityStore = MemoryEntityStore(types=tuple(self.store_by_type.keys()))

    def __str__(self):
        content_parts: list[str] = []
        for store_type, store in self.store_by_type.items():
            content_parts.append(f"{store_type.name}={store!s}")
        return ", ".join(content_parts)

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @override
    async def query(self, query: Query) -> QueryResult:
        result = await self.buffer.query(query)
        return result

    @override
    async def append(self, events: Sequence[Event]) -> Sequence[Event]:
        edit_events = [event for event in events if isinstance(event, EditEvent)]
        applied_events = await self.buffer.commit(edit_events)
        return applied_events

    @override
    async def subscribe(self, query: Query) -> AsyncGenerator[QueryUpdate]:
        raise NotImplementedError
        yield
