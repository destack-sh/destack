from collections.abc import Mapping, Sequence
from typing import override

from bench.language import (
    Change,
    ChangeResult,
    NodeArea,
    Query,
    QueryResult,
    Store,
)


class SplitStore(Store):
    """
    Split and route Queries and Changes to the appropriate Stores.
    Does not support atomic Changes across Stores (yet).
    """

    def __init__(self, store_by_area: Mapping[NodeArea, Store]):
        self.store_by_area = store_by_area

    def __str__(self):
        content_parts: list[str] = []
        for area, store in self.store_by_area.items():
            content_parts.append(f"{area.name}={store!s}")
        return ", ".join(content_parts)

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        raise NotImplementedError
