import abc
from typing import TYPE_CHECKING, AsyncIterator, Sequence

if TYPE_CHECKING:
    from bench.language import Change, ChangeResult, Query, QueryResult, QueryUpdate


class Store(abc.ABC):
    """The backing Store for the Supergraph."""

    @abc.abstractmethod
    async def query(self, query: "Query") -> "QueryResult":
        """Query the Store."""
        ...

    @abc.abstractmethod
    async def commit(self, changes: Sequence["Change"]) -> Sequence["ChangeResult"]:
        """Commit a Transaction."""
        ...


class LiveStore(Store):
    """A Store that can live-update."""

    @abc.abstractmethod
    async def subscribe(self, query: "Query") -> AsyncIterator["QueryUpdate"]:
        """Subscribe to a Query in the Store."""
        ...


class OptimisticStore(LiveStore):
    """A Store that can optimistically apply Changes."""

    @abc.abstractmethod
    async def stage(self, changes: Sequence["Change"]) -> None:
        """Stage Changes locally."""
        ...
