from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from .node import Node
from .query import Query, QueryResult
from .session import Session

if TYPE_CHECKING:
    from bench.language import Store


class QueryConnection[RootT: "Node"]:
    """A connection to a Query and its result."""

    __slots__ = ("id", "is_live", "query", "result", "session", "store")

    def __init__(self, query: Query, store: "Store", session: "Session"):
        self.id: UUID = query.id
        self.query: Query = query
        self.is_live = query.is_live
        self.store: Store = store
        self.session: Session = session
        self.result: QueryResult | None = None

    async def execute(self) -> None:
        """Execute the Query."""
        self.result = await self.store.query(self.query)

    def to_one_or_none(self) -> Optional[RootT]:
        """Get the root (if any)."""
        raise NotImplementedError

    def to_one(self) -> RootT:
        """Get the root (error if none)."""
        raise NotImplementedError

    def to_list(self) -> list[RootT]:
        """Get the list of roots."""
        raise NotImplementedError

    def close(self) -> None:
        """Close the QueryConnection."""
        raise NotImplementedError

    async def wait_closed(self) -> None:
        """Wait for the QueryConnection to be closed."""
        raise NotImplementedError
