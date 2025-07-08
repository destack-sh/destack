import abc
from collections.abc import AsyncGenerator, Sequence
from typing import TYPE_CHECKING, ClassVar, Optional

if TYPE_CHECKING:
    from destack.language import (
        EditEvent,
        Event,
        NodeType,
        Query,
        QueryResult,
        QueryUpdate,
        StoreImplementation,
        StoreKey,
    )


class Store(abc.ABC):
    """
    The read/write Store backing some part of the Supergraph.
    Some Stores only support a subset of Entities/Events (according to their StoreKeys).
    """

    implementation: ClassVar[Optional["StoreImplementation"]]
    keys: tuple["StoreKey", ...]
    node_types: tuple["NodeType", ...]

    @abc.abstractmethod
    async def query(self, query: "Query") -> "QueryResult":
        """
        Query the Store. Some Stores only support a subset of Queries.
        """
        ...


class EntityStore(Store):
    """
    A Store for Entities.
    """

    @abc.abstractmethod
    async def commit(self, events: Sequence["EditEvent"]) -> Sequence["EditEvent"]:
        """
        Commit the Events to the Store. Return the applied Events (including any cascaded Events).
        """
        ...


class EventStore(Store):
    """
    A Store for Events (technically a superset of EntityStore).
    """

    @abc.abstractmethod
    async def append(self, events: Sequence["Event"]) -> Sequence["Event"]:
        """
        Commit the Events to the Store. Return the applied Events (including any cascaded Events).
        """
        ...


class LiveStore(Store):
    """
    An EventStore that supports Query subscriptions.
    """

    @abc.abstractmethod
    async def subscribe(self, query: "Query") -> AsyncGenerator["QueryUpdate"]:
        """
        Subscribe to a Query in the Store.
        """
        ...
