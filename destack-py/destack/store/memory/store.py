from collections.abc import Sequence
from typing import ClassVar, override

from destack.language import (
    Change,
    ChangeResult,
    ChangeStatus,
    Query,
    QueryResult,
    Store,
    StoreImplementation,
    StoreType,
)

from .core import MemoryContext, MemoryDatabase
from .edit import execute_change


class MemoryStore(Store):
    """A simple in-memory Store."""

    implementation: ClassVar[StoreImplementation | None] = StoreImplementation.MEMORY

    def __init__(self, types: tuple[StoreType, ...]):
        super().__init__(types)
        self.database = MemoryDatabase()
        self.context = MemoryContext(self.database)

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        results: list[ChangeResult] = []
        for change in changes:
            execute_change(self.database, self.context, change)
            results.append(ChangeResult(id=change.id, status=ChangeStatus.COMPLETED))
        return results

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError
