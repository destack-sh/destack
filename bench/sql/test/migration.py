from textwrap import indent

import psycopg
import pytest
import structlog

from bench.language import User, Handle
from bench.language.node import NODE_CLASSES
from bench.sql.client import async_pg_cursor
from bench.sql.migration import (
    introspect_tables_from_pg,
    generate_migration_ops,
    _render_migration_body,
    MigrationOp,
)
from bench.utils.utils import format_python

logger = structlog.get_logger(__name__)


@pytest.fixture(autouse=True, scope="module")
async def blank_test_db():
    async with async_pg_cursor(autocommit=True) as cur:
        await cur.execute("DROP DATABASE IF EXISTS test")
        await cur.execute("CREATE DATABASE test")


@pytest.fixture(scope="module")
async def test_cur() -> psycopg.AsyncCursor:
    async with async_pg_cursor("test") as cur:
        yield cur


async def test_current_migrate_from_scratch(test_cur: psycopg.AsyncCursor):
    pass  # nocheckin implement when we have :FromScratchMigration


async def apply_migration_ops(cur: psycopg.AsyncCursor, ops: list[MigrationOp]) -> None:
    """Directly apply the given migration ops (for testing)."""
    method_body = _render_migration_body(ops)
    method_body = format_python(method_body)
    logger.info("apply_migration_ops", method_body="\n" + method_body)

    # turn it into an async callable
    method = f"async def _apply_inline(cur):\n{indent(method_body, '    ')}"
    method_locals: dict[str, any] = {}
    exec(method, method_locals)
    _apply_inline = method_locals["_apply_inline"]
    await _apply_inline(cur)


async def test_blank_migrate_from_scratch(test_cur: psycopg.AsyncCursor):
    # init from blank
    blank_tables = await introspect_tables_from_pg(test_cur)
    # new_tables = [node.__table__ for node in NODE_CLASSES if node.__table__]
    new_tables = [Handle.__table__, User.__table__]
    blank_ops = generate_migration_ops(blank_tables, new_tables)
    await apply_migration_ops(test_cur, blank_ops)

    # diff again (should be no diff now)
    current_tables = await introspect_tables_from_pg(test_cur)
    current_ops = generate_migration_ops(current_tables, new_tables)
    assert not current_ops, f"expected blank migration, got {len(current_ops)} ops"
