from collections.abc import Sequence
from typing import Self, override

import asyncpg
import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language import (
    Change,
    ChangeResult,
    ChangeStatus,
    CustomNodeDefinition,
    DatabaseInfo,
    Edit,
    EditOperation,
    EditType,
    NodeReference,
    NodeType,
    Query,
    QueryResult,
    Store,
)

from .client import pg_connection, pg_transaction
from .core import DatabaseContext, DatabaseTable
from .edit import execute_change
from .map import BENCH_CUSTOM_TABLE_PREFIX, BENCH_TABLE_PREFIX, BUILTIN_TABLE_BY_NAME
from .query import execute_query

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


class DatabaseStore(Store):
    """
    A Store backed by a Postgres Databases.
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
        self.ctx: DatabaseStoreContext | None = None

    def __str__(self) -> str:
        return f"database={self.database!r}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    async def _load_ctx(self, conn: asyncpg.Connection) -> "DatabaseStoreContext":
        ctx = DatabaseStoreContext(self)
        return ctx

    @override
    async def query(self, query: Query) -> QueryResult:
        async with pg_connection(self.database) as conn:
            if self.ctx is None:
                self.ctx = await self._load_ctx(conn)
            result = await execute_query(conn, self.ctx, query)
            logger.debug("database.query", query=query, result=result, span="current")
            return result

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        results: list[ChangeResult] = []
        async with pg_transaction(self.database) as (conn, tx):
            if self.ctx is None:
                self.ctx = await self._load_ctx(conn)

            # each change is its own atomic operation
            for change in changes:
                # duplicate context if we're mutating custom node definitions
                if any(
                    edit.node_type in (NodeType.CUSTOM_NODE_DEFINITION, NodeType.FIELD)
                    for edit in change.edits
                ):
                    local_ctx = self.ctx.copy()
                else:
                    local_ctx = self.ctx

                # apply
                try:
                    edits, cascaded_edits = await execute_change(conn, local_ctx, change)
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
                    self.ctx.apply(edits)
                except Exception as e:
                    logger.error("database.commit.error", change=change, exc_info=e, span="current")
                    result = ChangeResult(id=change.id, status=ChangeStatus.FAILED)
                results.append(result)
        return results


class DatabaseStoreContext(DatabaseContext):
    __slots__ = ("store", "tables_by_name")

    def __init__(self, store: DatabaseStore):
        self.store = store
        self.tables_by_name: dict[str, DatabaseTable] = {**BUILTIN_TABLE_BY_NAME}
        self.custom_node_definitions: dict[UUID, CustomNodeDefinition] = {}

    def copy(self) -> Self:
        raise NotImplementedError

    @override
    def get_table(self, key: str | NodeReference) -> DatabaseTable:
        # convert ptrs to table names
        if isinstance(key, NodeReference):
            if key.node_type != NodeType.CUSTOM_NODE_INSTANCE:
                key = f"{BENCH_TABLE_PREFIX}{key.node_type.name.lower()}"
            else:
                assert key.definition_id is not None, f"no definition_id for {key!r}"
                key = f"{BENCH_CUSTOM_TABLE_PREFIX}{key.definition_id}"
        # lookup
        table = self.tables_by_name.get(key)
        if table is None:
            raise LookupError(f"no table for {key!r} in {self.store!r}")
        return table

    @override
    def apply(self, edits: Sequence[Edit]):
        for edit in edits:
            if edit.node_type in (NodeType.CUSTOM_NODE_DEFINITION, NodeType.FIELD):
                pass  # optimistically update context copy
