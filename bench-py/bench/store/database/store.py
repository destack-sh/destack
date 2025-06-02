from collections.abc import Sequence
from typing import override

import structlog
from opentelemetry import trace

from bench.language import (
    Change,
    ChangeResult,
    ChangeStatus,
    DatabaseInfo,
    EditOperation,
    EditType,
    Query,
    QueryResult,
    Store,
)

from .client import pg_connection, pg_transaction
from .core import DatabaseContext
from .edit import execute_change
from .query import execute_query

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


class DatabaseStore(Store):
    """
    A Store backed by real Postgres Databases.
    """

    __supports_edit_types__ = (
        EditType.CREATE,
        EditType.UPSERT,
        EditType.UPDATE,
        EditType.DELETE,
        EditType.MOVE,
        EditType.ARCHIVE,
        EditType.UNARCHIVE,
        EditType.ERASE,
        EditType.RESTORE,
    )
    __supports_operations__ = (
        EditOperation.SET,
        EditOperation.CLEAR,
    )
    __supports_cascade__ = True

    __slots__ = ("ctx", "database")

    def __init__(self, database: DatabaseInfo):
        self.database = database
        self.ctx = DatabaseContext.from_builtin()

    def __str__(self) -> str:
        return f"database={self.database!r}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    @override
    async def query(self, query: Query) -> QueryResult:
        async with pg_connection(self.database) as conn:
            result = await execute_query(conn, query)
            logger.debug("database.query", query=query, result=result, span="current")
            return result

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        results: list[ChangeResult] = []
        async with pg_transaction(self.database) as (conn, tx):
            for change in changes:
                try:
                    edits, cascaded_edits = await execute_change(conn, self.ctx, change)
                    logger.debug(
                        "database.commit",
                        change=change,
                        edits=edits,
                        cascaded_edits=cascaded_edits,
                        span="current",
                    )
                    result = ChangeResult(
                        id=change.id,
                        status=ChangeStatus.COMPLETED,
                        edits=list(edits),
                        cascaded_edits=list(cascaded_edits),
                    )
                    await tx.commit()
                except Exception as e:
                    logger.error("database.commit.error", change=change, exc_info=e, span="current")
                    result = ChangeResult(id=change.id, status=ChangeStatus.FAILED)
                results.append(result)
        return results
