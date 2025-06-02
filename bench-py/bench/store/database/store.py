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

    def __init__(self, database: DatabaseInfo):
        self.database = database

    def __str__(self) -> str:
        return f"database={self.database!r}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        raise NotImplementedError
