import abc
from collections.abc import AsyncGenerator, Sequence
from typing import TYPE_CHECKING, ClassVar, Optional

from destack.language.registry import NODE_TYPES_BY_PRIMARY_STORE_TYPE

if TYPE_CHECKING:
    from destack.language import (
        EditEvent,
        Event,
        NodeType,
        Query,
        QueryResult,
        QueryUpdate,
        StoreImplementation,
        StoreType,
    )


class Store(abc.ABC):
    """
    The read/write Store backing (part of) the Supergraph.
    Some Stores only support a subset of Entities/Events.
    """

    implementation: ClassVar[Optional["StoreImplementation"]]

    def __init__(self, types: tuple["StoreType", ...]):
        self.types: tuple[StoreType, ...] = types
        self.node_types: tuple[NodeType, ...] = tuple(
            {
                node_type
                for store_type in types
                for node_type in NODE_TYPES_BY_PRIMARY_STORE_TYPE[store_type]
            }
        )

    @abc.abstractmethod
    async def query(self, query: "Query") -> "QueryResult":
        """
        Query the Store.
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


class LiveStore(EventStore):
    """
    An EventStore that supports Query subscriptions.
    """

    @abc.abstractmethod
    async def subscribe(self, query: "Query") -> AsyncGenerator["QueryUpdate"]:
        """
        Subscribe to a Query in the Store.
        """
        ...
