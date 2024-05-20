import psycopg
import pytest
import structlog

from bench.sql.client import get_pg_connection_str, pg_cursor
from bench.sql.engine import GLOBAL_SCHEMA, LOCAL_SCHEMA
from bench.sql.migration import (
    generate_migration_ops,
    introspect_schema_from_pg,
    migrate,
    read_migrations_from_fs,
)
from bench.system.client import GLOBAL_STORE, global_pg_cursor

logger = structlog.get_logger(__name__)


@pytest.fixture(scope="function")
async def blank_test_db(request: pytest.FixtureRequest):
    db_name = f"migrate_test_{request.function.__name__}"
    async with global_pg_cursor(autocommit=True) as cur:
        await cur.execute(f"DROP DATABASE IF EXISTS {db_name}")  # type: ignore
        await cur.execute(f"CREATE DATABASE {db_name}")  # type: ignore
        yield db_name


@pytest.fixture(scope="function")
async def blank_test_cur(blank_test_db: str):
    connection_str = get_pg_connection_str(GLOBAL_STORE, blank_test_db)
    async with pg_cursor(connection_str) as cur:
        yield cur
    await cur.connection.close()


async def _do_test_stored_migrations(blank_test_cur: psycopg.AsyncCursor, *, is_global: bool):
    # run all stored migrations
    stored_migrations = read_migrations_from_fs()
    logger.info("migrate", migrations=stored_migrations)
    await migrate(blank_test_cur, target=stored_migrations[-1].id, is_global=is_global)

    # diff again (should be empty now)
    current_schema = await introspect_schema_from_pg(blank_test_cur)
    new_schema = GLOBAL_SCHEMA if is_global else LOCAL_SCHEMA
    current_ops = generate_migration_ops(current_schema, new_schema)
    assert not current_ops, f"out of sync migrations, got {len(current_ops)} ops"


async def test_stored_migrations_global(blank_test_cur: psycopg.AsyncCursor):
    """Existing global migrations against a blank database."""
    await _do_test_stored_migrations(blank_test_cur, is_global=True)


async def test_stored_migrations_local(blank_test_cur: psycopg.AsyncCursor):
    """Existing local migrations against a blank database."""
    await _do_test_stored_migrations(blank_test_cur, is_global=False)
