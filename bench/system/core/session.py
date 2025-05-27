from datetime import datetime

from fastuuid import UUID

from bench.language import (
    REGION,
    VERSION,
    Bench,
    BenchStatus,
    Database,
    Node,
    Package,
    PackageType,
    Region,
    Supergraph,
)
from bench.utils.oracle import Oracle
from bench.utils.utils import get_from_env

BEGINNING_OF_TIME = datetime.fromisoformat("1970-01-01T00:00:00+00:00")


def make_system_database(region: Region, pg_url: str) -> Database:
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=region,
        status=BenchStatus.ACTIVE,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    system_package_stub = Package(
        parent=system_bench_stub,
        type=PackageType.HOME,
        id=UUID(int=1),
        name="Home",
        slug="home",
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    database = Database(
        parent=system_package_stub,
        name="Database",
        version=VERSION,
        sql_url=pg_url,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    return database


def get_global_database_from_env() -> Database:
    """Get the default global database configured in the environment"""
    pg = get_from_env("GLOBAL_PG_URL", description="Global Postgres connection string")
    pg_url = pg.split("|", maxsplit=1)[0]
    return make_system_database(REGION, pg_url)


def get_regional_database_from_env(region: Region = REGION) -> Database:
    """Get the default regional database configured in the environment"""
    from bench.system.core import DATABASE_MAP

    return DATABASE_MAP.get(region)


def global_session(
    node: Node | None,
    *,
    oracle: Oracle,
    supergraph: Supergraph | None = None,
    epoch: int | None = None,
    readonly: bool = False,
    split_read: bool = True,
):
    """Create a Session in a global database"""
    raise NotImplementedError
