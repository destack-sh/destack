from collections.abc import AsyncIterator, Sequence
from typing import override

from bench.language import Change, ChangeResult, LiveStore, Query, QueryResult, QueryUpdate


class RemoteStore(LiveStore):
    """A store that fetches data from a remote source."""

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError

    @override
    async def subscribe(self, query: Query) -> AsyncIterator[QueryUpdate]:
        raise NotImplementedError

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        raise NotImplementedError
