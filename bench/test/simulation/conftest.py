# ruff: noqa: E402
from uuid import UUID

import pytest

from bench.test.conftest import _setup_test_env
from bench.test.simulation.oracle import SimulatedEventLoopPolicy

# NOTE: must run setup before importing from bench
_setup_test_env()


from bench.language import VERSION, NodeReference, Store
from bench.language.bench import Bench
from bench.language.const import NodeType, Region
from bench.language.graph import NodeSuperGraph
from bench.sql.client import GLOBAL_PG_CRYPTO_KEY, pg_store_connection
from bench.sql.core import Schema
from bench.sql.engine import sqlstr
from bench.sql.migration import (
    apply_sql_migration_ops,
    generate_sql_migration_ops,
    introspect_sql_schema,
)
from bench.system.core import BEGINNING_OF_TIME, global_pg_cursor
from bench.utils.utils import get_from_env


# NOTE: simulation tests must be run with one event loop per function to isolate
#  (and use our custom event loop for fast-forwarding support)
@pytest.fixture()
def event_loop_policy():
    return SimulatedEventLoopPolicy()


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
