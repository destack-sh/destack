import psycopg

from bench.sql.core import ObjectKind
from bench.sql.graph import BENCH_TABLE_PREFIX, BUILTIN_GLOBAL_SCHEMA, BUILTIN_LOCAL_SCHEMA
from bench.sql.migration import (
    generate_sql_migration_ops,
    introspect_sql_schema,
    read_migrations_from_fs,
    sql_migrate,
)
from bench.utils.oracle import REAL_ORACLE


async def _do_test_stored_migrations(cur: psycopg.AsyncCursor, *, is_global: bool):
    # run all stored migrations
    stored_migrations = read_migrations_from_fs()
    await sql_migrate(cur, target=stored_migrations[-1].id, is_global=is_global, oracle=REAL_ORACLE)

    # diff again (should be empty now)
    current_schema = await introspect_sql_schema(
        cur, include_table_prefixes=(BENCH_TABLE_PREFIX,), exclude_table_prefixes=()
    )
    new_schema = BUILTIN_GLOBAL_SCHEMA if is_global else BUILTIN_LOCAL_SCHEMA
    current_ops = generate_sql_migration_ops(old_schema=current_schema, new_schema=new_schema)
    current_ops = [op for op in current_ops if op.object_kind != ObjectKind.EXTENSION]
    assert not current_ops, f"out of sync migrations, got {len(current_ops)} ops"


async def test_stored_migrations_global(blank_cur: psycopg.AsyncCursor):
    """Existing global migrations against a blank database."""
    await _do_test_stored_migrations(blank_cur, is_global=True)


async def test_stored_migrations_local(blank_cur: psycopg.AsyncCursor):
    """Existing local migrations against a blank database."""
    await _do_test_stored_migrations(blank_cur, is_global=False)
