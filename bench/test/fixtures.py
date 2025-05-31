import re
from contextlib import contextmanager
from urllib.parse import urlparse

import grpclib
import pytest
import structlog
from opentelemetry import trace

from bench.sql.client import pg_connection
from bench.test.conftest import _setup_test_env

# ruff: noqa: E402
# NOTE: must run setup before importing from bench
_setup_test_env()


from bench.language import REGION, DatabaseInfo, Tenancy
from bench.sql import (
    BENCH_CUSTOM_NODE_PREFIX,
    BENCH_TABLE_PREFIX,
    BUILTIN_GLOBAL_SCHEMA,
    BUILTIN_MAIN_SCHEMA,
    SqlSchema,
    apply_sql_migration_ops,
    generate_sql_migration_ops,
    introspect_sql_schema,
)
from bench.system import get_global_database_from_env
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def make_global_database(name: str) -> DatabaseInfo:
    """Creates a global database for testing.."""
    pg = get_from_env("GLOBAL_DATABASE_URL", description="Global Postgres connection string")
    pg_url_parsed = urlparse(pg)
    sql_url = pg_url_parsed._replace(path=f"/{name}").geturl()

    database = DatabaseInfo(
        region=REGION,
        cell_name=name,
        external_id=name,
        tenancy=Tenancy.DEDICATED,
        sql_url=sql_url,
    )
    return database


def make_bench_database(name: str) -> DatabaseInfo:
    """Creates a regional database for testing."""
    assert len(name) < 64, f"name must be less than 64 characters: {name!r}"
    pg = get_from_env("GLOBAL_DATABASE_URL", description="Regional Postgres connection string")
    pg_url_parsed = urlparse(pg)
    pg_url = pg_url_parsed._replace(path=f"/{name}").geturl()
    database = DatabaseInfo(
        region=REGION,
        cell_name="test-0",
        external_id=name,
        tenancy=Tenancy.DEDICATED,
        sql_url=pg_url,
    )
    return database


async def create_blank_test_db(database: DatabaseInfo):
    """Creates a blank postgres database"""
    async with pg_connection(get_global_database_from_env()) as conn:
        await conn.execute(f'DROP DATABASE IF EXISTS "{database.external_id}"')
        await conn.execute(f'CREATE DATABASE "{database.external_id}"')


async def create_test_db(database: DatabaseInfo, schema: SqlSchema):
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


async def delete_test_db(database: DatabaseInfo):
    """Deletes a postgres DB with one of our schemas"""
    async with pg_connection(get_global_database_from_env()) as conn:
        await conn.execute(f'DROP DATABASE IF EXISTS "{database.external_id}"')


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
async def main_database(request: pytest.FixtureRequest):
    """Gets the per test function regional database"""

    database = make_bench_database(f"test-{_clean_name(request.node.name)[:32]}-main")
    await create_test_db(database, BUILTIN_MAIN_SCHEMA)
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
