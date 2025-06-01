from collections.abc import Sequence
from typing import override

from bench.language import Change, ChangeResult, Query, QueryResult, Store


class LocalStore(Store):
    """A simple in-memory Store.."""

    def __init__(self):
        pass

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        raise NotImplementedError

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError
