import psycopg
import pytest
import structlog

from bench.language.node import NODE_CLASSES
from bench.sql.client import async_pg_cursor
from bench.sql.migration import (
    introspect_tables_from_pg,
    generate_migration_ops,
    apply_migration_ops,
)

logger = structlog.get_logger(__name__)


@pytest.fixture(scope="module")
async def blank_test_db():
    async with async_pg_cursor(autocommit=True) as cur:
        await cur.execute("DROP DATABASE IF EXISTS migrate_test")
        await cur.execute("CREATE DATABASE migrate_test")


@pytest.fixture(scope="module")
async def test_cur(blank_test_db) -> psycopg.AsyncCursor:
    async with async_pg_cursor("migrate_test") as cur:
        yield cur


async def test_current_migrate_from_scratch(test_cur: psycopg.AsyncCursor):
    pass  # nocheckin implement when we have :FromScratchMigration


async def test_blank_migrate_from_scratch(test_cur: psycopg.AsyncCursor):
    # init from blank
    blank_tables = await introspect_tables_from_pg(test_cur)
    new_tables = [node.__table__ for node in NODE_CLASSES if node.__table__]
    blank_ops = generate_migration_ops(blank_tables, new_tables)
    await apply_migration_ops(test_cur, blank_ops)
    await test_cur.connection.commit()

    # diff again (should be no diff now)
    current_tables = await introspect_tables_from_pg(test_cur)
    current_ops = generate_migration_ops(current_tables, new_tables)
    assert not current_ops, f"expected blank migration, got {len(current_ops)} ops"
