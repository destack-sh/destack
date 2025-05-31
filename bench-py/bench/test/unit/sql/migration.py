import asyncpg
from bench.language import NodeArea
from bench.sql import (
    BENCH_TABLE_PREFIX,
    BUILTIN_SCHEMA_BY_AREA,
    SqlObjectKind,
    generate_sql_migration_ops,
    introspect_sql_schema,
    read_migrations_from_fs,
    sql_migrate,
)
from bench.utils.oracle import REAL_ORACLE


async def _do_test_stored_migrations(conn: asyncpg.Connection, *, area: NodeArea):
    # run all stored migrations
    stored_migrations = read_migrations_from_fs()
    await sql_migrate(conn, target=stored_migrations[-1].id, area=area, oracle=REAL_ORACLE)

    # diff again (should be empty now)
    current_schema = await introspect_sql_schema(
        conn, include_table_prefixes=(BENCH_TABLE_PREFIX,), exclude_table_prefixes=()
    )
    new_schema = BUILTIN_SCHEMA_BY_AREA[area]
    current_ops = generate_sql_migration_ops(old_schema=current_schema, new_schema=new_schema)
    current_ops = [op for op in current_ops if op.object_kind != SqlObjectKind.EXTENSION]
    assert not current_ops, f"out of sync migrations, got {len(current_ops)} ops"


async def test_stored_migrations_global(blank_conn: asyncpg.Connection):
    """Existing global migrations against a blank table."""
    await _do_test_stored_migrations(blank_conn, area=NodeArea.GLOBAL_RELATIONAL)


async def test_stored_migrations_regional(blank_conn: asyncpg.Connection):
    """Existing regional migrations against a blank table."""
    await _do_test_stored_migrations(blank_conn, area=NodeArea.MAIN_RELATIONAL)


async def test_stored_migrations_local(blank_conn: asyncpg.Connection):
    """Existing local migrations against a blank table."""
    await _do_test_stored_migrations(blank_conn, area=NodeArea.CUSTOM_RELATIONAL)
