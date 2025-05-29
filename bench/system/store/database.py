from typing import Mapping, Sequence, override

from fastuuid import UUID

from bench.language import (
    BEGINNING_OF_TIME,
    REGION,
    VERSION,
    Bench,
    BenchStatus,
    Change,
    ChangeResult,
    Database,
    NodeArea,
    Package,
    PackageType,
    Query,
    QueryResult,
    Region,
    Session,
    Store,
)
from bench.utils.utils import get_from_env


class DatabaseStore(Store):
    """
    A Store backed by real Postgres Databases.
    """

    def __init__(self, database_by_area: Mapping[NodeArea, Database]):
        self.database_by_area = database_by_area

    @override
    async def query(self, query: Query) -> QueryResult:
        raise NotImplementedError

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        raise NotImplementedError


def make_system_database(region: Region, pg_url: str) -> Database:
    system_session = Session()
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=region,
        status=BenchStatus.ACTIVE,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    system_package_stub = Package(
        parent=system_bench_stub,
        type=PackageType.HOME,
        id=UUID(int=1),
        name="Home",
        slug="home",
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    database = Database(
        parent=system_package_stub,
        name="Database",
        version=VERSION,
        sql_url=pg_url,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    return database


def get_global_database_from_env() -> Database:
    """Get the default global database configured in the environment"""
    pg = get_from_env("GLOBAL_PG_URL", description="Global Postgres connection string")
    pg_url = pg.split("|", maxsplit=1)[0]
    return make_system_database(REGION, pg_url)


def get_regional_database_from_env(region: Region = REGION) -> Database:
    """Get the default regional database configured in the environment"""
    from .sharding import DATABASE_MAP

    return DATABASE_MAP.get(region)
