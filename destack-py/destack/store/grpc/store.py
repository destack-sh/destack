from collections.abc import AsyncIterator, Sequence
from typing import override

from destack.language import Change, ChangeResult, Query, QueryResult, QueryUpdate, Store


class GrpcStore(Store):
    """A Store that fetches data from a remote source via gRPC."""

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError

    @override
    async def subscribe(self, query: Query) -> AsyncIterator[QueryUpdate]:
        raise NotImplementedError

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        raise NotImplementedError
