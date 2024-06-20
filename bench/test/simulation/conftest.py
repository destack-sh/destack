# ruff: noqa: E402
from uuid import UUID

import pytest

from bench.test.conftest import setup_test_env
from bench.utils.oracle import REAL_ORACLE, Oracle

# NOTE: must run setup_test() before importing from bench
setup_test_env()


from bench.language import VERSION, Store
from bench.language.bench import Bench
from bench.language.const import NodeType, Region
from bench.language.expression import NodeReference
from bench.language.graph import NodeSuperGraph
from bench.language.session import Session
from bench.proto.wire import (
    GraphScope,
)
from bench.sql.client import GLOBAL_PG_CRYPTO_KEY, pg_store_connection
from bench.sql.core import Schema
from bench.sql.engine import GLOBAL_SCHEMA, sqlstr
from bench.sql.migration import (
    apply_sql_migration_ops,
    generate_sql_migration_ops,
    introspect_sql_schema,
)
from bench.system.core import BEGINNING_OF_TIME, global_pg_cursor, global_pg_engine_from_store
from bench.utils.utils import get_from_env

GLOBAL_PG_NAME = get_from_env("GLOBAL_PG_NAME", description="Global Postgres database name")


def make_global_store(name: str):
    """Creates a global store for testing. Like global store in system/core."""

    host = get_from_env("GLOBAL_PG_HOST", description="Global Postgres host")
    username = get_from_env("GLOBAL_PG_USERNAME", description="Global Postgres username")
    password = get_from_env("GLOBAL_PG_PASSWORD", description="Global Postgres password")

    system_bench_ptr = NodeReference(type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(root_ptr=system_bench_ptr)
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=Region.GLOBAL,
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
    # reset test database (connect to test since we can't drop active db)
    async with pg_store_connection(store, database="test", autocommit=True) as cur:
        await cur.execute(sqlstr(f'DROP DATABASE IF EXISTS "{store.external_name}"'))
        await cur.execute(sqlstr(f'CREATE DATABASE "{store.external_name}"'))


async def create_test_db(store: Store, schema: Schema):
    """Creates a postgres DB with one of our schemas"""
    await create_blank_test_db(store)

    async with global_pg_cursor(store, autocommit=True) as cur:
        blank_schema = await introspect_sql_schema(cur)
        migration_ops = generate_sql_migration_ops(blank_schema, schema)
        await apply_sql_migration_ops(cur, migration_ops)
        await cur.connection.commit()


@pytest.fixture()
async def blank_store(request: pytest.FixtureRequest):
    """Gets the per test function blank store"""

    store = make_global_store(f"test_{request.node.name}")
    await create_blank_test_db(store)
    return store


@pytest.fixture()
async def global_store(request: pytest.FixtureRequest):
    """Gets the per test function global store"""

    store = make_global_store(f"test_{request.node.name}")
    await create_test_db(store, GLOBAL_SCHEMA)
    return store


def create_global_session(global_store: Store, oracle: Oracle):
    """Gets direct access to a per test global engine"""

    global_pg_engine = global_pg_engine_from_store(global_store)
    session = Session(
        parent=None,
        _default_scope=GraphScope(),
        _engines=(global_pg_engine,),
        _epoch=0,
        _oracle=oracle,
        _supergraph=NodeSuperGraph(root_ptr=None),
    )
    return session


@pytest.fixture()
def global_real_session(global_store: Store):
    """
    Gets the per test function global real session.
    Unfortunately we can't set this session as the active session in context because
     pytest-asyncio does not propagate contextvars across async tests/fixtures.
    (see https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-1777004844)
    """

    return create_global_session(global_store, REAL_ORACLE)
