from contextlib import contextmanager

# ruff: noqa: E402
from uuid import UUID

import grpclib
import pytest
import structlog

from bench.test.conftest import _setup_test_env

# NOTE: must run setup before importing from bench
_setup_test_env()

from opentelemetry import trace

from bench.language import VERSION, NodeReference, Store
from bench.language.bench import Bench
from bench.language.const import NodeType, Region
from bench.language.graph import NodeSuperGraph
from bench.language.validation import clean_name
from bench.sql.client import GLOBAL_PG_CRYPTO_KEY, get_pg_pool, pg_connection
from bench.sql.core import ALL_EXTENSIONS, Schema, Table
from bench.sql.graph import (
    BENCH_RECORD_TABLE_PREFIX,
    BENCH_TABLE_PREFIX,
    BUILTIN_GLOBAL_SCHEMA,
    BUILTIN_GLOBAL_TABLES,
    BUILTIN_LOCAL_TABLES,
    sqlstr,
)
from bench.sql.migration import (
    apply_sql_migration_ops,
    generate_sql_migration_ops,
    introspect_sql_schema,
)
from bench.system.utils.session import BEGINNING_OF_TIME, system_store_from_env
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def make_system_store(name: str):
    """Creates a system store for testing. Like global store in system/core."""

    host = get_from_env("GLOBAL_PG_HOST", description="Global Postgres host")
    username = get_from_env("GLOBAL_PG_USERNAME", description="Global Postgres username")
    password = get_from_env("GLOBAL_PG_PASSWORD", description="Global Postgres password")

    system_bench_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(root_ptr=system_bench_ptr)
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=Region.ZURICH,
        encryption_key=GLOBAL_PG_CRYPTO_KEY,
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    store = Store(
        parent=system_bench_stub,
        name=name,
        version=VERSION,
        external_name=name,
        connection_uri=f"postgresql://{username}:{password}@{host}/{name}",
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    return store


async def create_blank_test_db(store: Store):
    """Creates a blank postgres database"""
    async with pg_connection(system_store_from_env(), autocommit=True) as conn:
        await conn.execute(sqlstr(f'DROP DATABASE IF EXISTS "{store.external_name}"'))
        await conn.execute(sqlstr(f'CREATE DATABASE "{store.external_name}"'))


async def create_test_db(store: Store, schema: Schema):
    """Creates a postgres DB with one of our schemas"""
    await create_blank_test_db(store)
    async with pg_connection(store, autocommit=True) as conn:
        blank_schema = await introspect_sql_schema(
            conn.cursor,
            include_table_prefixes=(BENCH_TABLE_PREFIX,),
            exclude_table_prefixes=(BENCH_RECORD_TABLE_PREFIX,),
        )
        migration_ops = generate_sql_migration_ops(old_schema=blank_schema, new_schema=schema)
        await apply_sql_migration_ops(conn.cursor, migration_ops)
        await conn.commit()


async def delete_test_db(store: Store):
    """Deletes a postgres DB with one of our schemas"""
    await get_pg_pool(store).close()
    async with pg_connection(system_store_from_env(), autocommit=True) as conn:
        await conn.execute(sqlstr(f'DROP DATABASE IF EXISTS "{store.external_name}"'))


@pytest.fixture()
async def blank_store(request: pytest.FixtureRequest):
    """Gets the per test function blank store"""

    store = make_system_store(f"test-{clean_name(request.node.name)}")
    await create_blank_test_db(store)
    try:
        yield store
    finally:
        await delete_test_db(store)


@pytest.fixture()
async def global_store(request: pytest.FixtureRequest):
    """Gets the per test function global store"""

    store = make_system_store(f"test-{clean_name(request.node.name)}")
    await create_test_db(store, BUILTIN_GLOBAL_SCHEMA)
    try:
        yield store
    finally:
        await delete_test_db(store)


@pytest.fixture()
async def omni_store(request: pytest.FixtureRequest):
    """Gets the per test function global store"""

    ALL_TABLES: tuple[Table, ...] = (
        *BUILTIN_GLOBAL_TABLES,
        *(
            t
            for t in BUILTIN_LOCAL_TABLES
            if not any(t.name == g.name for g in BUILTIN_GLOBAL_TABLES)
        ),
    )
    OMNI_SCHEMA = Schema(ALL_EXTENSIONS, ALL_TABLES)

    store = make_system_store(f"test-{clean_name(request.node.name)}")
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
