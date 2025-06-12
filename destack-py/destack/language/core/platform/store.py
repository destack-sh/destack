import abc
from collections.abc import AsyncIterator, Sequence
from typing import TYPE_CHECKING, ClassVar, Optional

if TYPE_CHECKING:
    from destack.language import (
        Change,
        ChangeResult,
        Query,
        QueryResult,
        QueryUpdate,
        StoreImplementation,
        StoreType,
        StoreZone,
    )


class Store(abc.ABC):
    """
    The read/write Store backing (part of) the Supergraph.
    Could be a primary or secondary Storage from Databases or search or whatever.
    Some Stores only support a subset of Edits.
    """

    implementation: ClassVar[Optional["StoreImplementation"]]

    def __init__(self, types: tuple["StoreType", ...]):
        self.types: tuple[StoreType, ...] = types
        self.zones: tuple[StoreZone, ...] = tuple({type.zone for type in types})

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
