from collections.abc import Sequence
from typing import ClassVar, override

from destack.language import (
    Change,
    ChangeResult,
    Query,
    QueryResult,
    Store,
    StoreImplementation,
    StoreType,
)

# nocheckin: proper split committing/querying (store area_type in NodeReference?)
#  (including live/in-memory overrides)


class SplitStore(Store):
    """
    Split and route Queries and Changes to the appropriate Stores.
    Does not support atomic Changes across Stores (yet).
    """

    backend: ClassVar[StoreImplementation | None] = None

    def __init__(self, *stores: Store):
        self.stores: tuple[Store, ...] = stores
        self.store_by_type: dict[StoreType, Store] = {}
        for store in stores:
            for type in store.types:
                self.store_by_type[type] = store

    def __str__(self):
        content_parts: list[str] = []
        for type, store in self.store_by_type.items():
            content_parts.append(f"{type.name}={store!s}")
        return ", ".join(content_parts)

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        raise NotImplementedError
