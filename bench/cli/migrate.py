import sys
import time
from pathlib import Path
from typing import Optional

import structlog
import typer
from more_itertools import first
from rich import print
from rich.console import Console

from bench.cli.utils import async_to_sync_blocking
from bench.language import Bench, Store
from bench.language.const import VERSION, NodeType
from bench.sql.graph import BENCH_RECORD_TABLE_PREFIX, BENCH_TABLE_PREFIX
from bench.utils.oracle import REAL_ORACLE
from bench.utils.utils import format_python

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="migration management")
console = Console()

BENCH_QUERY = Bench.include_descendants(Store).select_all()


@app.command(help="generate global / local SQL migrations")
@async_to_sync_blocking
async def make(
    bench: str = typer.Option(default="bench", help="the bench to use as local reference"),
    no_downgrade: bool = typer.Option(default=False, help="exclude downgrade operations"),
    no_local: bool = typer.Option(default=False, help="exclude local operations"),
    dry_run: bool = typer.Option(default=False, help="only print, don't store"),
    overwrite: bool = typer.Option(default=False, help="overwrite existing migration for version"),
    from_scratch: bool = typer.Option(default=False, help="generate migration from scratch"),
):
    from bench.sql.client import pg_connection
    from bench.sql.core import Schema
    from bench.sql.engine import SqlUndefinedObjectError
    from bench.sql.graph import BUILTIN_GLOBAL_SCHEMA, BUILTIN_LOCAL_SCHEMA
    from bench.sql.migration import (
        Migration,
        add_migration_to_fs,
        generate_sql_migration_code,
        generate_sql_migration_ops,
        introspect_sql_schema,
        read_migrations_from_fs,
        read_migrations_from_pg,
    )
    from bench.system.utils.session import (
        global_session,
        pg_engine_from_store,
        system_store_from_env,
    )

    start = time.time()
    global_store = system_store_from_env()
    global_pg_engine = pg_engine_from_store(global_store)

    # check existing migrations for inconsistencies
    file_migrations = read_migrations_from_fs()
    conflicting_migration = first((m for m in file_migrations if m.version == VERSION), None)
    if conflicting_migration:
        if overwrite:
            logger.info("migrate.make.overwrite", migration=conflicting_migration)
            assert conflicting_migration.path, f"{conflicting_migration!r} has no path"
            Path(conflicting_migration.path).unlink()
            file_migrations.remove(conflicting_migration)
        else:
            raise RuntimeError(
                f"existing migration for version {VERSION}: {conflicting_migration!r}"
            )
    async with pg_connection(global_store) as conn:
        stored_migrations = await read_migrations_from_pg(conn.cursor)
    max_file_id = max(m.id for m in file_migrations) if file_migrations else 0
    max_stored_id = max(m.id for m in stored_migrations) if stored_migrations else 0
    if max_stored_id > max_file_id:
        raise RuntimeError(
            f"stored migrations are ahead of file migrations:\nstored={stored_migrations!r}\nfile={file_migrations!r}"
        )

    # diff local
    if not no_local:
        if not from_scratch:
            try:
                async with global_session(global_store, (global_pg_engine,), REAL_ORACLE):
                    bench_node = await BENCH_QUERY.get(slug=bench)
                    assert bench_node.main_store, f"{bench!r} has no main store"
                    async with pg_connection(bench_node.main_store) as conn:
                        old_local_schema = await introspect_sql_schema(
                            conn.cursor,
                            include_table_prefixes=(BENCH_TABLE_PREFIX,),
                            exclude_table_prefixes=(BENCH_RECORD_TABLE_PREFIX,),
                        )
            except SqlUndefinedObjectError as e:
                # missing from_scratch flag?
                console.print(f"[red]couldn't make migrations (missing --from-scratch?): {e}[/red]")
                sys.exit(-1)
        else:
            old_local_schema = Schema.blank()
        local_migration_ops = generate_sql_migration_ops(old_local_schema, BUILTIN_LOCAL_SCHEMA)
    else:
        local_migration_ops = []

    # diff global
    async with pg_connection(global_store) as conn:
        old_global_schema = await introspect_sql_schema(
            conn.cursor,
            include_table_prefixes=(BENCH_TABLE_PREFIX,),
            exclude_table_prefixes=(BENCH_RECORD_TABLE_PREFIX,),
        )
    global_migration_ops = generate_sql_migration_ops(old_global_schema, BUILTIN_GLOBAL_SCHEMA)

    # generate migration
    if not global_migration_ops and not local_migration_ops:
        logger.info("migrate.make.noop")
        return
    latest_migration = max(file_migrations, key=lambda m: m.id, default=None)
    new_migration = Migration(
        id=latest_migration.id + 1 if latest_migration is not None else 1,
        version=VERSION,
        has_global=bool(global_migration_ops),
        has_local=bool(local_migration_ops),
        applied_at=None,
    )
    migration_code = generate_sql_migration_code(
        new_migration,
        global_ops=global_migration_ops,
        local_ops=local_migration_ops,
        exclude_inverse=no_downgrade,
        oracle=REAL_ORACLE,
    )
    if not dry_run:
        add_migration_to_fs(migration=new_migration, code=migration_code)
    else:
        print(migration_code)

    logger.info("migrate.make", duration=time.time() - start)


@app.command(help="apply global OR local SQL migrations")
@async_to_sync_blocking
async def apply(
    target: Optional[str] = typer.Option(
        default=None, help="the migration to migrate to [default=latest]"
    ),
    bench: Optional[str] = typer.Option(
        default=None, help="the local bench to migrate, global otherwise"
    ),
    dry_run: bool = typer.Option(default=False, help="only try, don't commit"),
):
    from bench.sql.client import pg_connection
    from bench.sql.migration import sql_migrate as _migrate
    from bench.system.utils.session import (
        global_session,
        pg_engine_from_store,
        system_store_from_env,
    )

    start = time.time()
    global_store = system_store_from_env()
    global_pg_engine = pg_engine_from_store(global_store)

    # resolve local_pg_name (determine local/global migration)
    if bench is not None:
        async with global_session(global_store, (global_pg_engine,), REAL_ORACLE):
            if bench != "*":
                bench_node = await BENCH_QUERY.get(slug=bench)
                stores = (*bench_node.stores,)
            else:
                benches = await BENCH_QUERY.tolist()
                stores = tuple(store for bench in benches for store in bench.stores)
    else:
        stores = (global_store,)

    for store in stores:
        async with pg_connection(store) as conn:
            await _migrate(
                cur=conn.cursor, target=target, is_global=bench is None, oracle=REAL_ORACLE
            )
            if not dry_run:
                await conn.commit()
            else:
                await conn.rollback()

    logger.info("migrate", duration=time.time() - start)


@app.command()
@async_to_sync_blocking
async def introspect(bench: Optional[str] = None):  # type: ignore
    """Introspect the current schema of the Postgres instance."""

    from bench.sql.client import pg_connection
    from bench.sql.migration import introspect_sql_schema
    from bench.system.utils.session import (
        global_session,
        pg_engine_from_store,
        system_store_from_env,
    )

    start = time.perf_counter()
    global_store = system_store_from_env()
    global_pg_engine = pg_engine_from_store(global_store)

    if bench is not None:
        async with global_session(global_store, (global_pg_engine,), REAL_ORACLE):
            bench_node = await Bench.include_descendants(NodeType.STORE).get(slug=bench)
            assert bench_node.main_store, f"{bench!r} has no main environment"
        async with pg_connection(bench_node.main_store) as conn:
            schema = await introspect_sql_schema(
                conn.cursor,
                include_columns=True,
                include_indexes=True,
                include_constraints=True,
                include_table_prefixes=(BENCH_TABLE_PREFIX,),
                exclude_table_prefixes=(BENCH_RECORD_TABLE_PREFIX,),
            )
    else:
        async with pg_connection(global_store) as conn:
            schema = await introspect_sql_schema(
                conn.cursor,
                include_columns=True,
                include_indexes=True,
                include_constraints=True,
                include_table_prefixes=(BENCH_TABLE_PREFIX,),
                exclude_table_prefixes=(BENCH_RECORD_TABLE_PREFIX,),
            )
            await conn.rollback()

    # generate schema
    chunks: list[str] = []
    for table in schema.tables:
        const_name = f"{table.name}_TABLE".upper()
        if const_name.startswith("BENCH_"):
            const_name = const_name[6:]
        table_def = f"{const_name} = {table.source_repr()}"
        chunks.append(table_def)
    source = "\n\n".join(chunks)
    source = format_python(source)
    print(source)

    logger.info("sql.introspect", duration=time.perf_counter() - start)
