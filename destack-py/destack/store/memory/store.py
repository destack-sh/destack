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


class MemoryStore(Store):
    """A simple in-memory Store."""

    implementation: ClassVar[StoreImplementation | None] = StoreImplementation.MEMORY

    def __init__(self, types: tuple[StoreType, ...]):
        super().__init__(types)

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        raise NotImplementedError

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError
