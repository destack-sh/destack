import psycopg
import pytest
import structlog

from bench.language.node import NODE_CLASSES
from bench.sql.client import async_pg_cursor
from bench.sql.migration import (
    apply_migration_ops,
    generate_migration_ops,
    introspect_tables_from_pg,
)

logger = structlog.get_logger(__name__)


@pytest.fixture(scope="function")
async def blank_test_db(request: pytest.FixtureRequest):
    db_name = f"migrate_test_{request.function.__name__}"
    async with async_pg_cursor(autocommit=True) as cur:
        await cur.execute(f"DROP DATABASE IF EXISTS {db_name}")
        await cur.execute(f"CREATE DATABASE {db_name}")
        yield db_name


@pytest.fixture(scope="function")
async def blank_test_cur(blank_test_db: str) -> psycopg.AsyncCursor:
    async with async_pg_cursor(blank_test_db) as cur:
        yield cur
    await cur.connection.close()


async def test_current_migrate_from_scratch(blank_test_cur: psycopg.AsyncCursor):
    """Applies the currently stored migrations from scratch."""
    pass  # nocheckin implement when we have :FromScratchMigration


async def test_blank_migrate_from_scratch(blank_test_cur: psycopg.AsyncCursor):
    # init from blank
    blank_tables = await introspect_tables_from_pg(blank_test_cur)
    new_tables = [node.__table__ for node in NODE_CLASSES if node.__table__]
    blank_ops = generate_migration_ops(blank_tables, new_tables)
    await apply_migration_ops(blank_test_cur, blank_ops)
    await blank_test_cur.connection.commit()

    # diff again (should be empty now)
    current_tables = await introspect_tables_from_pg(blank_test_cur)
    current_ops = generate_migration_ops(current_tables, new_tables)
    assert not current_ops, f"expected blank migration, got {len(current_ops)} ops"
