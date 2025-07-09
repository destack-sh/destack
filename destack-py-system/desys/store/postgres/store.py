from collections.abc import Sequence
from typing import ClassVar, override

import structlog
from opentelemetry import trace

from destack.language import (
    DatabaseInfo,
    DatabaseType,
    EditEvent,
    EntityStore,
    Query,
    QueryResult,
    StoreImplementation,
    StoreKey,
)
from destack.language.registry import get_node_types_for_stores

from .client import postgres_connection
from .core import PostgresContext
from .edit import execute_edits
from .query import execute_query

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


class PostgresEntityStore(EntityStore):
    """
    A Store backed by a Postgres Database.
    """

    implementation: ClassVar[StoreImplementation | None] = StoreImplementation.POSTGRES

    __slots__ = ("context", "database")

    def __init__(self, database: DatabaseInfo, keys: tuple[StoreKey, ...]):
        self.keys = keys
        self.node_types = get_node_types_for_stores(keys)
        if database.type != DatabaseType.POSTGRES:
            raise ValueError(f"unexpected {database!r}")
        self.database = database
        self.context: PostgresContext = PostgresContext(keys)

    def __str__(self) -> str:
        return f"database={self.database!r}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    @override
    @tracer.start_as_current_span("postgres.query")
    async def query(self, query: Query) -> QueryResult:
        async with postgres_connection(self.database) as conn:
            result = await execute_query(conn, self.context, query)
        logger.debug("postgres.query", query=query, result=result, span="current")
        return result

    @override
    @tracer.start_as_current_span("postgres.commit")
    async def commit(self, events: Sequence[EditEvent]) -> Sequence[EditEvent]:
        async with postgres_connection(self.database) as conn:
            try:
                async with conn.transaction():
                    edits, cascaded_edits = await execute_edits(conn, self.context, events)
                    applied_edits = [*edits, *cascaded_edits]
            except Exception as e:
                logger.error(
                    "postgres.commit.error",
                    events=len(events),
                    exc_info=e,
                    span="current",
                )
                raise
        logger.debug(
            "postgres.commit",
            events=len(events),
            applied_edits=len(applied_edits),
            span="current",
        )
        return applied_edits
