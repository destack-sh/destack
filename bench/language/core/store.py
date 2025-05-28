import abc
from typing import TYPE_CHECKING, AsyncIterator, override

if TYPE_CHECKING:
    from bench.language import Edit, Query, QueryResult, QueryUpdate


class Store(abc.ABC):
    """The backing Store for the Supergraph."""

    @abc.abstractmethod
    async def query(self, query: "Query") -> "QueryResult":
        """Query the Store."""
        ...

    @abc.abstractmethod
    async def subscribe(self, query: "Query") -> AsyncIterator["QueryUpdate"]:
        """Subscribe to a Query in the Store."""
        ...

    @abc.abstractmethod
    async def stage(self, edits: list["Edit"]) -> list["Edit"]:
        """Stage Edits locally."""
        ...

    @abc.abstractmethod
    async def commit(self, edits: list["Edit"]) -> tuple[list["Edit"], list["Edit"]]:
        """Commit a Transaction."""
        ...


class NullStore(Store):
    """A Store that can't do anything."""

    @override
    async def query(self, query: "Query") -> "QueryResult":
        """Query the Store."""
        raise NotImplementedError

    @override
    async def subscribe(self, query: "Query") -> AsyncIterator["QueryUpdate"]:
        """Subscribe to a Query in the Store."""
        raise NotImplementedError

    @override
    async def stage(self, edits: list["Edit"]) -> list["Edit"]:
        """Stage Edits locally."""
        raise NotImplementedError

    @override
    async def commit(self, edits: list["Edit"]) -> tuple[list["Edit"], list["Edit"]]:
        """Commit a Transaction."""
        raise NotImplementedError
