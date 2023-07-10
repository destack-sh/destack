import abc
from typing import Any, AsyncIterator, Generic, Iterator, Optional, Self, TypeVar

from bench.bench.core import Module
from bench.bench.query import Query, Sort
from bench.utils.utils import DotList

#
# General Search interface
#

ElementDataT = TypeVar("ElementDataT")
ElementT = TypeVar("ElementT")


class Search(abc.ABC, Generic[ElementDataT, ElementT]):
    """A search over some module parts."""

    RESULT_BATCH_SIZE = 400

    def __init__(self, module: Module, query: Query, sort: list[Sort], limit: int):
        self.module = module
        self._query = query
        self._sort = sort
        self._limit = limit

    def filter(self, query: Query) -> "Self":
        raise NotImplementedError

    def sort(self, *sorts: Sort) -> "Self":
        raise NotImplementedError

    def limit(self, limit: int) -> "Self":
        raise NotImplementedError

    async def _do_search(
        self, after: list[Any] = None, limit: Optional[int] = None, count: bool = False
    ):
        raise NotImplementedError

    def _unpack_element_data(self, element_data: ElementDataT) -> ElementT:
        raise NotImplementedError

    def __iter__(self) -> Iterator[ElementT]:
        yield from self._iter(batched=False)

    def batched(self) -> Iterator[list[ElementT]]:
        yield from self._iter(batched=True)

    def _iter(self, batched: bool):
        after = None
        remaining_limit = self._limit
        while remaining_limit is None or remaining_limit > 0:
            rep = self.module.session.async_to_sync(self._do_search)(
                after=after, limit=remaining_limit
            )
            if len(rep.payload.elements) == 0:
                break
            elements = DotList() if batched else None
            for element_data in rep.payload.elements:
                element = self._unpack_element_data(element_data)
                if batched:
                    elements.append(element)
                else:
                    yield element
            if batched:
                yield elements
            after = rep.payload.end_cursor
            if remaining_limit is not None:
                remaining_limit -= len(rep.payload.elements)

    async def abatched(self) -> AsyncIterator[list[ElementT]]:
        async for batch in self._aiter(batched=True):
            yield batch

    async def __aiter__(self) -> AsyncIterator[ElementT]:
        """Iterates over the elements of the search result (batched)."""
        async for element in self._aiter(batched=False):
            yield element

    async def _aiter(self, batched: bool) -> AsyncIterator[ElementT]:
        from bench.bench import wire

        after = None
        remaining_limit = self._limit
        while remaining_limit is None or remaining_limit > 0:
            rep = await self._do_search(after=after, limit=remaining_limit)
            if len(rep.payload.elements) == 0:
                break
            elements = DotList() if batched else None
            for element_data in rep.payload.elements:
                element = self._unpack_element_data(element_data)
                if batched:
                    elements.append(element)
                else:
                    yield element
            if batched:
                yield elements
            after = rep.payload.end_cursor
            if remaining_limit is not None:
                remaining_limit -= len(rep.payload.elements)

    def __len__(self) -> int:
        return self.count()

    async def afirst(self) -> Optional[ElementT]:
        """Returns the first element of the search result."""
        async for element in self.limit(1):
            return element
        return None

    def first(self) -> Optional[ElementT]:
        """Returns the first element of the search result."""
        return self.module.session.async_to_sync(self.afirst)()

    async def atolist(self) -> list[ElementT]:
        """Returns the search result as a list."""
        return [element async for element in self]

    def tolist(self) -> list[ElementT]:
        """Returns the search result as a list."""
        return list(self)
