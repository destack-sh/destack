import abc
from typing import TYPE_CHECKING, AsyncIterator, Sequence

if TYPE_CHECKING:
    from bench.language import Change, ChangeResult, Query, QueryResult, QueryUpdate


class Store(abc.ABC):
    """The read/write Store backing the Supergraph."""

    @abc.abstractmethod
    async def query(self, query: "Query") -> "QueryResult":
        """Query the Store."""
        ...

    @abc.abstractmethod
    async def commit(self, changes: Sequence["Change"] = ()) -> Sequence["ChangeResult"]:
        """Commit a Transaction (including any staged Changes)."""
        ...


class LiveStore(Store, abc.ABC):
    """A Store that can be subscribed to."""

    @abc.abstractmethod
    async def subscribe(self, query: "Query") -> AsyncIterator["QueryUpdate"]:
        """Subscribe to a Query in the Store."""
        ...


class OptimisticStore(LiveStore, abc.ABC):
    """A Store that can optimistically apply Changes."""

    @abc.abstractmethod
    async def stage(self, changes: Sequence["Change"]) -> None:
        """Stage Changes locally."""
        ...
