from collections.abc import Sequence
from typing import override

from bench.language import (
    Change,
    ChangeResult,
    DatabaseInfo,
    EditOperation,
    EditType,
    Query,
    QueryResult,
    Store,
)


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

    def __init__(
        self,
        *,
        global_database: DatabaseInfo,
        bench_database: DatabaseInfo | None = None,
    ):
        self.global_database = global_database
        self.bench_database = bench_database

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        raise NotImplementedError
