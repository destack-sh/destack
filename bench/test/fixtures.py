from contextlib import contextmanager

# ruff: noqa: E402
from urllib.parse import urlparse
from uuid import UUID

import grpclib
import pytest
import structlog

from bench.test.conftest import _setup_test_env

# NOTE: must run setup before importing from bench
_setup_test_env()

from opentelemetry import trace

from bench.language import (
    VERSION,
    Bench,
    BenchStatus,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    Package,
    PackageType,
    Region,
    Store,
    clean_name,
)
from bench.sql import (
    ALL_EXTENSIONS,
    BENCH_RECORD_TABLE_PREFIX,
    BENCH_TABLE_PREFIX,
    BUILTIN_GLOBAL_SCHEMA,
    BUILTIN_GLOBAL_TABLES,
    BUILTIN_LOCAL_TABLES,
    BUILTIN_REGIONAL_SCHEMA,
    BUILTIN_REGIONAL_TABLES,
    Schema,
    apply_sql_migration_ops,
    generate_sql_migration_ops,
    get_pg_pool,
    introspect_sql_schema,
    pg_connection,
    sqlstr,
)
from bench.system import BEGINNING_OF_TIME, global_store_from_env
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def make_global_store(name: str):
    """Creates a global store for testing.."""

    pg = get_from_env("GLOBAL_PG", description="Global Postgres connection string")
    pg_url, pg_crypto_key = pg.split("|", maxsplit=1)
    pg_url_parsed = urlparse(pg_url)
    pg_url = pg_url_parsed._replace(path=f"/{name}").geturl()

    system_bench_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(name="Global", root_ptr=system_bench_ptr)
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=Region.ZURICH,
        encryption_key=pg_crypto_key,
        _supergraph=supergraph,
        status=BenchStatus.ACTIVATED,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    system_package_stub = Package(
        parent=system_bench_stub,
        type=PackageType.OPEN,
        id=UUID(int=1),
        name="Main",
        slug="main",
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    store = Store(
        parent=system_package_stub,
        name=name,
        version=VERSION,
        external_name=name,
        connection_uri=pg_url,
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    return store


def make_regional_store(name: str):
    """Creates a regional store for testing."""

    assert len(name) < 64, f"name must be less than 64 characters: {name!r}"
    pg = get_from_env("GLOBAL_PG", description="Regional Postgres connection string")
    pg_url, pg_crypto_key = pg.split("|", maxsplit=1)
    pg_url_parsed = urlparse(pg_url)
    pg_url = pg_url_parsed._replace(path=f"/{name}").geturl()

    system_bench_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(name="Regional", root_ptr=system_bench_ptr)
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=Region.ZURICH,
        encryption_key=pg_crypto_key,
        _supergraph=supergraph,
        status=BenchStatus.ACTIVATED,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    system_package_stub = Package(
        parent=system_bench_stub,
        type=PackageType.OPEN,
        id=UUID(int=1),
        name="Main",
        slug="main",
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    store = Store(
        parent=system_package_stub,
        name=name,
        version=VERSION,
        external_name=name,
        connection_uri=pg_url,
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    return store


async def create_blank_test_db(store: Store):
    """Creates a blank postgres database"""
    async with pg_connection(global_store_from_env(), owner=store, autocommit=True) as conn:
        await conn.execute(sqlstr(f'DROP DATABASE IF EXISTS "{store.external_name}"'))
        await conn.execute(sqlstr(f'CREATE DATABASE "{store.external_name}"'))


async def create_test_db(store: Store, schema: Schema):
    """Creates a postgres DB with one of our schemas"""
    await create_blank_test_db(store)
    async with pg_connection(store, owner=store, autocommit=True) as conn:
        old_schema = await introspect_sql_schema(
            conn.cursor,
            include_table_prefixes=(BENCH_TABLE_PREFIX,),
            exclude_table_prefixes=(BENCH_RECORD_TABLE_PREFIX,),
        )
        migration_ops = generate_sql_migration_ops(old_schema=old_schema, new_schema=schema)
        await apply_sql_migration_ops(conn.cursor, migration_ops)
        await conn.commit()


async def delete_test_db(store: Store):
    """Deletes a postgres DB with one of our schemas"""
    await get_pg_pool(store).close()
    async with pg_connection(global_store_from_env(), owner=store, autocommit=True) as conn:
        await conn.execute(sqlstr(f'DROP DATABASE IF EXISTS "{store.external_name}"'))


@pytest.fixture
async def blank_store(request: pytest.FixtureRequest):
    """Gets the per test function blank store"""

    store = make_global_store(f"test-{clean_name(request.node.name)[:32]}-blank")
    await create_blank_test_db(store)
    try:
        yield store
    finally:
        await delete_test_db(store)


@pytest.fixture
async def global_store(request: pytest.FixtureRequest):
    """Gets the per test function global store"""

    store = make_global_store(f"test-{clean_name(request.node.name)[:32]}-global")
    await create_test_db(store, BUILTIN_GLOBAL_SCHEMA)
    try:
        yield store
    finally:
        await delete_test_db(store)


@pytest.fixture
async def regional_store(request: pytest.FixtureRequest):
    """Gets the per test function regional store"""

    store = make_regional_store(f"test-{clean_name(request.node.name)[:32]}-regional")
    await create_test_db(store, BUILTIN_REGIONAL_SCHEMA)
    try:
        yield store
    finally:
        await delete_test_db(store)


@pytest.fixture
async def omni_store(request: pytest.FixtureRequest):
    """Gets the per test function global store"""

    ALL_TABLES = {}
    for table in (*BUILTIN_GLOBAL_TABLES, *BUILTIN_REGIONAL_TABLES, *BUILTIN_LOCAL_TABLES):
        ALL_TABLES[table.name] = table
    ALL_TABLES = tuple(ALL_TABLES.values())
    OMNI_SCHEMA = Schema(ALL_EXTENSIONS, ALL_TABLES)

    store = make_global_store(f"test-{clean_name(request.node.name)[:32]}-omni")
    await create_test_db(store, OMNI_SCHEMA)
    try:
        yield store
    finally:
        await delete_test_db(store)


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"
