import psycopg
import pytest
import structlog

from bench.language.setup import NODE_CLASSES
from bench.sql.client import get_pg_connection_str, pg_cursor
from bench.sql.engine import GLOBAL_TABLES, LOCAL_TABLES
from bench.sql.migration import (
    apply_migration_ops,
    generate_migration_ops,
    introspect_tables_from_pg,
    read_migrations_from_fs,
    migrate,
)
from bench.system.client import global_pg_cursor, GLOBAL_STORE

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
    current_tables = await introspect_tables_from_pg(blank_test_cur)
    new_tables = GLOBAL_TABLES if is_global else LOCAL_TABLES
    current_ops = generate_migration_ops(current_tables, new_tables)
    assert not current_ops, f"out of sync migrations, got {len(current_ops)} ops"


async def test_stored_migrations_global(blank_test_cur: psycopg.AsyncCursor):
    """Existing global migrations against a blank database."""
    await _do_test_stored_migrations(blank_test_cur, is_global=True)


async def test_stored_migrations_local(blank_test_cur: psycopg.AsyncCursor):
    """Existing local migrations against a blank database."""
    await _do_test_stored_migrations(blank_test_cur, is_global=False)


async def test_migrate_from_scratch(blank_test_cur: psycopg.AsyncCursor):
    """Regenerate new migrations against a blank database."""
    # init from blank
    blank_tables = await introspect_tables_from_pg(blank_test_cur)
    new_tables = [node.__table__ for node in NODE_CLASSES if node.__table__]
    blank_ops = generate_migration_ops(blank_tables, new_tables)
    await apply_migration_ops(blank_test_cur, blank_ops)
    await blank_test_cur.connection.commit()

    # diff again (should be empty now)
    current_tables = await introspect_tables_from_pg(blank_test_cur)
    current_ops = generate_migration_ops(current_tables, new_tables)
    assert not current_ops, f"broken migrations, got {len(current_ops)} ops"
