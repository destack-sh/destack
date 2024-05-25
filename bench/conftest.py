import os
from contextlib import contextmanager
from typing import TYPE_CHECKING

import grpclib
import pytest
from pytest_asyncio import is_async_test

if TYPE_CHECKING:
    from bench.language import Bench


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
async def _prepared_test_db():
    from bench.sql.client import get_pg_connection_uri, pg_cursor
    from bench.sql.engine import GLOBAL_SCHEMA
    from bench.sql.migration import (
        EXTENSIONS,
        apply_sql_migration_ops,
        generate_sql_migration_ops,
        introspect_sql_schema,
    )
    from bench.system.core import GLOBAL_PG_NAME, GLOBAL_STORE, global_pg_cursor

    # ensure that default global_db_cursor points to test (means environment info was set up correctly)
    # if this fails, it's likely we mistakenly imported from bench.utils.env before our pytest_configure
    #  could override it, usually because of an innocent (cascading) import in our conftests.
    assert GLOBAL_PG_NAME == "test"

    # reset test database (connect to bench since we can't drop active db)
    #  (reconstruct default connection str here because GLOBAL_PG_NAME is different in test)
    default_connection_uri = get_pg_connection_uri(GLOBAL_STORE, "bench")
    async with pg_cursor(default_connection_uri, autocommit=True) as cur:
        await cur.execute("DROP DATABASE IF EXISTS test")
        await cur.execute("CREATE DATABASE test")

    # migrate to current global schema
    async with global_pg_cursor() as cur:
        for extension in EXTENSIONS:
            await cur.execute(f"CREATE EXTENSION IF NOT EXISTS {extension}")
        blank_schema = await introspect_sql_schema(cur)
        blank_ops = generate_sql_migration_ops(blank_schema, GLOBAL_SCHEMA)
        await apply_sql_migration_ops(cur, blank_ops)
        await cur.connection.commit()


@pytest.fixture()
async def test_cur():
    from bench.sql.client import get_pg_connection_uri, pg_cursor
    from bench.system.core import GLOBAL_STORE

    async with pg_cursor(get_pg_connection_uri(GLOBAL_STORE, "test")) as cur:
        yield cur


def global_session():
    from bench.language.session import Session
    from bench.system.core import GLOBAL_POSTGRES_ENGINE

    return Session(parent=None, _engines=(GLOBAL_POSTGRES_ENGINE,))


def bench_session(bench: "Bench"):
    from bench.language.session import Session
    from bench.system.core import GLOBAL_POSTGRES_ENGINE

    assert bench.main_branch is not None, f"{bench!r} has no main branch"
    return Session(parent=bench.main_branch.main_package, _engines=(GLOBAL_POSTGRES_ENGINE,))


@pytest.fixture()
def fabricator():
    from bench.language.test.fabricator import Fabricator

    return Fabricator(seed=42)


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"
