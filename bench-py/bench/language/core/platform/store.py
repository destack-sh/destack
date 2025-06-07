import abc
from collections.abc import AsyncIterator, Sequence
from typing import TYPE_CHECKING, ClassVar

if TYPE_CHECKING:
    from bench.language import (
        Change,
        ChangeResult,
        EditOperation,
        EditType,
        Query,
        QueryResult,
        QueryUpdate,
    )


class Store(abc.ABC):
    """
    The read/write Store backing (part of) the Supergraph.
    Could be a primary or secondary Storage from Databases or search or whatever.
    Some Stores only support a subset of Edits.
    """

    __supports_edit_types__: ClassVar[tuple["EditType", ...]]
    __supports_operations__: ClassVar[tuple["EditOperation", ...]]
    __supports_cascade__: ClassVar[bool]

    @abc.abstractmethod
    async def query(self, query: "Query") -> "QueryResult":
        """Query the Store."""
        ...

    @abc.abstractmethod
    async def commit(self, changes: Sequence["Change"]) -> Sequence["ChangeResult"]:
        """
        Commit the Changes as individual transactions.
        Returns the ChangeResults.
        """
        ...


class LiveStore(Store, abc.ABC):
    """
    A Store that can be subscribed to for Query updates.
    """

    @abc.abstractmethod
    async def subscribe(self, query: "Query") -> AsyncIterator["QueryUpdate"]:
        """Subscribe to a Query in the Store."""
        ...


class OptimisticStore(LiveStore, abc.ABC):
    """
    A Store that can optimistically apply Changes.
    To commit any Changes (incl. staged), pass the Changes to Store.commit.
    To unstage any Changes without committing, just call Store.unstage.
    """

    @abc.abstractmethod
    async def stage(self, changes: Sequence["Change"]) -> None:
        """Stage Changes locally. Idempotent for Changes by id."""
        ...

    @abc.abstractmethod
    async def unstage(self, changes: Sequence["Change"]) -> None:
        """Unstage Changes locally. Idempotent for Changes by id."""
        ...
