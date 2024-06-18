from contextlib import contextmanager

import grpclib
import pytest


@pytest.fixture(autouse=True, scope="session")
async def _prepared_test_db():
    from bench.sql.client import pg_store_connection
    from bench.sql.engine import GLOBAL_SCHEMA
    from bench.sql.migration import (
        EXTENSIONS,
        apply_sql_migration_ops,
        generate_sql_migration_ops,
        introspect_sql_schema,
    )
    from bench.system.core import GLOBAL_PG_NAME, GLOBAL_STORE, global_pg_cursor

    # ensure that default global_db_cursor points to a test DB
    # (check mistaken import from bench.utils.env before pytest_configure, see above)
    assert GLOBAL_PG_NAME.startswith("test")

    # reset test database (connect to bench since we can't drop active db)
    #  (reconstruct default connection str here because GLOBAL_PG_NAME is different in test)
    async with pg_store_connection(GLOBAL_STORE, database="bench", autocommit=True) as cur:
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
    from bench.sql.client import pg_store_connection
    from bench.system.core import GLOBAL_STORE

    async with pg_store_connection(GLOBAL_STORE, database="test") as cur:
        yield cur


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"
