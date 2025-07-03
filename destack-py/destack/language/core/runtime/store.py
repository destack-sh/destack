import abc
from collections.abc import AsyncIterator, Sequence
from typing import TYPE_CHECKING, ClassVar, Optional

from destack.language.registry import NODE_TYPES_BY_PRIMARY_STORE_TYPE

if TYPE_CHECKING:
    from destack.language import (
        Change,
        ChangeResult,
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
    Some Stores only support a subset of Edits.
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
        Returns a QueryResult.
        """
        ...

    @abc.abstractmethod
    async def commit(self, changes: Sequence["Change"]) -> Sequence["ChangeResult"]:
        """
        Commit the Changes as individual transactions (every Change is atomic by itself).
        Returns the ChangeResults per Change.
        """
        ...

    @abc.abstractmethod
    async def subscribe(self, query: "Query") -> AsyncIterator["QueryUpdate"]:
        """Subscribe to a Query in the Store."""
        ...
