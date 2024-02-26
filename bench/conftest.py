from contextlib import contextmanager
import os
from typing import TYPE_CHECKING

import grpclib
import psycopg
import pytest
from pytest_asyncio import is_async_test

if TYPE_CHECKING:
    from bench.language.test.fabricator import Fabricator


def pytest_configure(config):
    os.environ["ENVIRONMENT"] = "test"
    from bench.utils.env import setup_dotenv

    setup_dotenv()

    from bench.utils.logging import configure_logging

    configure_logging()

    from bench.language.setup import _complete_bench_setup

    _complete_bench_setup()


def pytest_collection_modifyitems(items):
    pytest_asyncio_tests = (item for item in items if is_async_test(item))
    session_scope_marker = pytest.mark.asyncio(scope="session")
    for async_test in pytest_asyncio_tests:
        async_test.add_marker(session_scope_marker)


@pytest.fixture(autouse=True, scope="session")
async def prepared_test_db():
    from bench.language.setup import NODE_CLASSES
    from bench.sql.migration import (
        EXTENSIONS,
        apply_migration_ops,
        generate_migration_ops,
        introspect_tables_from_pg,
    )
    from bench.system.client import GLOBAL_STORE, GLOBAL_PG_NAME, global_pg_cursor
    from bench.sql.client import pg_cursor, get_pg_connection_str

    # ensure that default global_db_cursor points to test
    #  (means environment info was set up correctly)
    assert GLOBAL_PG_NAME == "test"

    # reset test database (connect to bench since we can't drop active db)
    #  (reconstruct default connection str here because GLOBAL_PG_NAME is different in test)
    default_connection_str = get_pg_connection_str(GLOBAL_STORE, "bench")
    async with pg_cursor(default_connection_str, autocommit=True) as cur:
        await cur.execute("DROP DATABASE IF EXISTS test")
        await cur.execute("CREATE DATABASE test")

    # migrate to current schema
    async with global_pg_cursor() as cur:
        for extension in EXTENSIONS:
            await cur.execute(f"CREATE EXTENSION IF NOT EXISTS {extension}")
        blank_tables = await introspect_tables_from_pg(cur)
        new_tables = [node.__table__ for node in NODE_CLASSES if node.__table__]
        blank_ops = generate_migration_ops(blank_tables, new_tables)
        await apply_migration_ops(cur, blank_ops)
        await cur.connection.commit()


@pytest.fixture(scope="function")
async def test_cur() -> psycopg.AsyncCursor:
    from bench.sql.client import pg_cursor
    from bench.sql.client import get_pg_connection_str
    from bench.system.client import GLOBAL_STORE

    async with pg_cursor(get_pg_connection_str(GLOBAL_STORE, "test")) as cur:
        yield cur


@pytest.fixture(scope="function")
async def fabricator() -> "Fabricator":
    from bench.language.test.fabricator import Fabricator

    yield Fabricator(seed=42)


@contextmanager
def raises_grpc_error(status: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    assert exc_info.value.status == status, f"expected {status}, got {exc_info!r}"
