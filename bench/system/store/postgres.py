from typing import Mapping, Sequence, override

from bench.language import (
    Change,
    ChangeResult,
    Database,
    NodeArea,
    Query,
    QueryResult,
    Store,
)


class PostgresStore(Store):
    """
    A Store backed by real Postgres Databases.
    """

    def __init__(self, database_by_area: Mapping[NodeArea, Database]):
        self.database_by_area: dict[NodeArea, Database] = {**database_by_area}

    def add_database(self, area: NodeArea, database: Database) -> None:
        self.database_by_area[area] = database

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        raise NotImplementedError
