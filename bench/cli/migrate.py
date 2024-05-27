import time
from pathlib import Path
from typing import Optional

import structlog
import typer
from more_itertools import first
from rich import print

from bench.cli.utils import async_to_sync_blocking
from bench.language import Bench, Environment, Store
from bench.language.const import VERSION, NodeType
from bench.language.query import NodeNotFoundError
from bench.sql.client import pg_cursor_to_store
from bench.sql.core import Schema
from bench.sql.engine import (
    GLOBAL_SCHEMA,
    LOCAL_SCHEMA,
    SqlUndefinedObjectError,
)
from bench.sql.migration import (
    Migration,
    add_migration_to_fs,
    delete_migrations_in_fs,
    delete_migrations_in_pg,
    generate_sql_migration_code,
    generate_sql_migration_ops,
    introspect_sql_schema,
    read_migrations_from_fs,
    read_migrations_from_pg,
)
from bench.sql.migration import sql_migrate as _migrate
from bench.system.core import GLOBAL_STORE, global_pg_cursor, global_session
from bench.utils.utils import format_python

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="migration management")


@app.command(help="generate global / local SQL migrations")
@async_to_sync_blocking
async def make(
    bench: str = typer.Option(default="bench", help="the bench to use as local reference"),
    no_downgrade: bool = typer.Option(default=False, help="exclude downgrade operations"),
    no_local: bool = typer.Option(default=False, help="exclude local operations"),
    dry_run: bool = typer.Option(default=False, help="only print, don't store"),
    overwrite: bool = typer.Option(default=False, help="overwrite existing migration for version"),
):
    start = time.time()

    # check existing migrations for inconsistencies
    file_migrations = read_migrations_from_fs()
    conflicting_migration = first((m for m in file_migrations if m.version == VERSION), None)
    if conflicting_migration:
        if overwrite:
            logger.info("makemigrations.overwrite", migration=conflicting_migration)
            assert conflicting_migration.path, f"{conflicting_migration!r} has no path"
            Path(conflicting_migration.path).unlink()
            file_migrations.remove(conflicting_migration)
        else:
            raise RuntimeError(
                f"existing migration for version {VERSION}: {conflicting_migration!r}"
            )
    async with global_pg_cursor() as cur:
        stored_migrations = await read_migrations_from_pg(cur)
    max_file_id = max(m.id for m in file_migrations) if file_migrations else 0
    max_stored_id = max(m.id for m in stored_migrations) if stored_migrations else 0
    if max_stored_id > max_file_id:
        raise RuntimeError(
            f"stored migrations are ahead of file migrations:\nstored={stored_migrations!r}\nfile={file_migrations!r}"
        )

    # diff local
    if not no_local:
        async with global_session():
            try:
                bench_node = (
                    await Bench.descendants(Environment, Store).select_all().get(slug=bench)
                )
                assert bench_node.main_environment, f"{bench!r} has no main environment"
                async with pg_cursor_to_store(bench_node.main_environment.store) as cur:
                    old_local_schema = await introspect_sql_schema(cur)
            except (NodeNotFoundError, SqlUndefinedObjectError):
                old_local_schema = Schema.blank()  # initial migration
        local_migration_ops = generate_sql_migration_ops(old_local_schema, LOCAL_SCHEMA)
    else:
        local_migration_ops = []

    # diff global
    async with global_pg_cursor() as cur:
        old_global_schema = await introspect_sql_schema(cur)
    global_migration_ops = generate_sql_migration_ops(old_global_schema, GLOBAL_SCHEMA)

    # generate migration
    if not global_migration_ops and not local_migration_ops:
        logger.info("makemigrations.noop")
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
    )
    if not dry_run:
        add_migration_to_fs(migration=new_migration, code=migration_code)
    else:
        print(migration_code)

    logger.info("makemigrations", duration=time.time() - start)


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
    start = time.time()

    # resolve local_pg_name (determine local/global migration)
    if bench is not None:
        async with global_session():
            if bench != "*":
                bench_node = (
                    await Bench.descendants(Environment, Store).select_all().get(slug=bench)
                )
                stores = tuple(e.store for e in bench_node.environments)
            else:
                benches = await Bench.descendants(Environment, Store).select_all().tolist()
                stores = tuple(e.store for b in benches for e in b.environments)
    else:
        stores = (GLOBAL_STORE,)

    for store in stores:
        async with pg_cursor_to_store(store) as cur:
            await _migrate(cur=cur, target=target, is_global=bench is None)
            if not dry_run:
                await cur.connection.commit()
            else:
                await cur.connection.rollback()

    logger.info("migrate", duration=time.time() - start)


@app.command(help="delete migrations")
@async_to_sync_blocking
async def clear(from_id: int, to_id: int):
    start = time.time()

    delete_migrations_in_fs(from_id, to_id)
    async with global_pg_cursor() as cur:
        await delete_migrations_in_pg(cur, from_id=from_id, to_id=to_id)
        await cur.connection.commit()
    async with global_session():
        benches = await Bench.tolist()
        for bench in benches:
            stores = tuple(e.store for e in bench.environments)
            for store in stores:
                async with pg_cursor_to_store(store) as cur:
                    await delete_migrations_in_pg(cur, from_id=from_id, to_id=to_id)
                    await cur.connection.commit()

    logger.info("clear_migrations", duration=time.time() - start)


@app.command()
@async_to_sync_blocking
async def introspect(bench: Optional[str] = None):  # type: ignore
    """Introspect the current schema of the Postgres instance."""
    start = time.perf_counter()

    if bench is not None:
        async with global_session():
            bench_node = await Bench.descendants(NodeType.ENVIRONMENT, NodeType.STORE).get(
                slug=bench
            )
            assert bench_node.main_environment, f"{bench!r} has no main environment"
        async with pg_cursor_to_store(bench_node.main_environment.store) as cur:
            schema = await introspect_sql_schema(
                cur, include_columns=True, include_indexes=True, include_constraints=True
            )
    else:
        async with global_pg_cursor() as cur:
            schema = await introspect_sql_schema(
                cur, include_columns=True, include_indexes=True, include_constraints=True
            )
            await cur.connection.rollback()

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
