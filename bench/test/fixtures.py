import re
from contextlib import contextmanager
from urllib.parse import urlparse

import grpclib
import pytest
import structlog

from bench.language.core.graph import Supergraph
from bench.language.core.session import Session
from bench.sql.client import pg_connection
from bench.test.conftest import _setup_test_env

# ruff: noqa: E402
# NOTE: must run setup before importing from bench
_setup_test_env()

from opentelemetry import trace

from bench.language import (
    SYSTEM_ID,
    SYSTEM_SYSTEM_PACKAGE_ID,
    VERSION,
    Bench,
    BenchStatus,
    Database,
    Package,
    PackageType,
    Region,
)
from bench.sql import (
    ALL_EXTENSIONS,
    BENCH_CUSTOM_NODE_PREFIX,
    BENCH_TABLE_PREFIX,
    BUILTIN_GLOBAL_SCHEMA,
    BUILTIN_GLOBAL_TABLES,
    BUILTIN_LOCAL_TABLES,
    BUILTIN_REGIONAL_SCHEMA,
    BUILTIN_REGIONAL_TABLES,
    SqlSchema,
    apply_sql_migration_ops,
    generate_sql_migration_ops,
    introspect_sql_schema,
)
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def make_global_database(name: str):
    """Creates a global database for testing.."""
    from bench.system import BEGINNING_OF_TIME

    pg = get_from_env("GLOBAL_PG_URL", description="Global Postgres connection string")
    pg_url_parsed = urlparse(pg)
    pg_url = pg_url_parsed._replace(path=f"/{name}").geturl()

    system_session = Session(supergraph=Supergraph("system"))
    system_bench_stub = Bench(
        id=SYSTEM_ID,
        name="System",
        slug="system",
        region=Region.ZURICH,
        status=BenchStatus.ACTIVE,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    system_package_stub = Package(
        parent=system_bench_stub,
        type=PackageType.HOME,
        id=SYSTEM_SYSTEM_PACKAGE_ID,
        name="Home",
        slug="home",
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    database = Database(
        parent=system_package_stub,
        name=name,
        version=VERSION,
        external_name=name,
        sql_url=pg_url,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    return database


def make_regional_database(name: str):
    """Creates a regional database for testing."""
    from bench.system import BEGINNING_OF_TIME

    assert len(name) < 64, f"name must be less than 64 characters: {name!r}"
    pg = get_from_env("GLOBAL_PG_URL", description="Regional Postgres connection string")
    pg_url_parsed = urlparse(pg)
    pg_url = pg_url_parsed._replace(path=f"/{name}").geturl()

    system_session = Session(supergraph=Supergraph("system"))
    system_bench_stub = Bench(
        id=SYSTEM_ID,
        name="System",
        slug="system",
        region=Region.ZURICH,
        status=BenchStatus.ACTIVE,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    system_package_stub = Package(
        id=SYSTEM_SYSTEM_PACKAGE_ID,
        parent=system_bench_stub,
        type=PackageType.HOME,
        name="Home",
        slug="home",
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    database = Database(
        parent=system_package_stub,
        name=name,
        version=VERSION,
        external_name=name,
        sql_url=pg_url,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
        _session=system_session,
    )
    return database


async def create_blank_test_db(database: Database):
    """Creates a blank postgres database"""
    from bench.system import get_global_database_from_env

    async with pg_connection(get_global_database_from_env()) as conn:
        await conn.execute(f'DROP DATABASE IF EXISTS "{database.external_name}"')
        await conn.execute(f'CREATE DATABASE "{database.external_name}"')


async def create_test_db(database: Database, schema: SqlSchema):
    """Creates a postgres DB with one of our schemas"""
    await create_blank_test_db(database)
    async with pg_connection(database) as conn:
        old_schema = await introspect_sql_schema(
            conn,
            include_table_prefixes=(BENCH_TABLE_PREFIX,),
            exclude_table_prefixes=(BENCH_CUSTOM_NODE_PREFIX,),
        )
        migration_ops = generate_sql_migration_ops(old_schema=old_schema, new_schema=schema)
        await apply_sql_migration_ops(conn, migration_ops)


async def delete_test_db(database: Database):
    """Deletes a postgres DB with one of our schemas"""
    from bench.system import get_global_database_from_env

    async with pg_connection(get_global_database_from_env()) as conn:
        await conn.execute(f'DROP DATABASE IF EXISTS "{database.external_name}"')


def _clean_name(name: str) -> str:
    """Turn a name into a valid Python identifier"""
    return re.sub(r"[^a-zA-Z0-9]", "_", name)


@pytest.fixture
async def blank_database(request: pytest.FixtureRequest):
    """Gets the per test function blank database"""

    database = make_global_database(f"test-{_clean_name(request.node.name)[:32]}-blank")
    await create_blank_test_db(database)
    try:
        yield database
    finally:
        await delete_test_db(database)


@pytest.fixture
async def global_database(request: pytest.FixtureRequest):
    """Gets the per test function global database"""

    database = make_global_database(f"test-{_clean_name(request.node.name)[:32]}-global")
    await create_test_db(database, BUILTIN_GLOBAL_SCHEMA)
    try:
        yield database
    finally:
        await delete_test_db(database)


@pytest.fixture
async def regional_database(request: pytest.FixtureRequest):
    """Gets the per test function regional database"""

    database = make_regional_database(f"test-{_clean_name(request.node.name)[:32]}-regional")
    await create_test_db(database, BUILTIN_REGIONAL_SCHEMA)
    try:
        yield database
    finally:
        await delete_test_db(database)


@pytest.fixture
async def omni_database(request: pytest.FixtureRequest):
    """Gets the per test function global database"""

    ALL_TABLES = {}
    for table in (*BUILTIN_GLOBAL_TABLES, *BUILTIN_REGIONAL_TABLES, *BUILTIN_LOCAL_TABLES):
        ALL_TABLES[table.name] = table
    ALL_TABLES = tuple(ALL_TABLES.values())
    OMNI_SCHEMA = SqlSchema(ALL_EXTENSIONS, ALL_TABLES)

    database = make_global_database(f"test-{_clean_name(request.node.name)[:32]}-omni")
    await create_test_db(database, OMNI_SCHEMA)
    try:
        yield database
    finally:
        await delete_test_db(database)


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"
