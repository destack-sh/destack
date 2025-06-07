import time
from pathlib import Path
from typing import TYPE_CHECKING, Annotated, Optional

import structlog
import typer
from more_itertools import first
from rich import print
from rich.console import Console

from bench.language import REGION, VERSION, Area, Region
from bench.utils.oracle import REAL_ORACLE

from .utils import async_to_sync, parse_node_area, parse_region

if TYPE_CHECKING:
    from bench.language import Area, Region

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="migration management")
console = Console()


@app.command(help="generate SQL migrations")
@async_to_sync
async def make(
    area: Annotated[Area | None, typer.Option(parser=parse_node_area)] = None,
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    space: str = typer.Option(default="space", help="the space to use as local reference"),
    no_downgrade: bool = typer.Option(default=False, help="exclude downgrade operations"),
    dry_run: bool = typer.Option(default=False, help="only print, don't database"),
    overwrite: bool = typer.Option(default=False, help="overwrite existing migration for version"),
    from_scratch: bool = typer.Option(default=False, help="generate migration from scratch"),
):
    from bench.sharding import DATABASE_PROVIDER, get_global_database_from_env
    from bench.store.postgres import (
        BENCH_BUILTIN_TABLE_PREFIX,
        BENCH_CUSTOM_TABLE_PREFIX,
        BUILTIN_GLOBAL_SCHEMA,
        BUILTIN_MAIN_SCHEMA,
        Migration,
        add_migration_to_fs,
        generate_migration_code,
        generate_migration_ops,
        introspect_schema,
        pg_connection,
        read_migrations_from_fs,
        read_migrations_from_pg,
    )

    start = time.time()
    global_database = get_global_database_from_env()

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
    async with pg_connection(global_database) as conn:
        databased_migrations = await read_migrations_from_pg(conn)
    max_file_id = max(m.id for m in file_migrations) if file_migrations else 0
    max_databased_id = max(m.id for m in databased_migrations) if databased_migrations else 0
    if max_databased_id > max_file_id:
        raise RuntimeError(
            f"databased migrations are ahead of file migrations:\ndatabased={databased_migrations!r}\nfile={file_migrations!r}"
        )

    # diff main
    if area in (None, Area.MAIN_DATABASE):
        main_database = await DATABASE_PROVIDER.resolve_or_error(
            region or REGION, cell_name, external_name
        )
        async with pg_connection(main_database) as conn:
            old_main_schema = await introspect_schema(
                conn,
                include_table_prefixes=(BENCH_BUILTIN_TABLE_PREFIX,),
                exclude_table_prefixes=(BENCH_CUSTOM_TABLE_PREFIX,),
            )
        main_migration_ops = generate_migration_ops(
            old_schema=old_main_schema, new_schema=BUILTIN_MAIN_SCHEMA
        )
    else:
        main_migration_ops = []

    # diff global
    if area in (None, Area.GLOBAL_DATABASE):
        async with pg_connection(global_database) as conn:
            old_global_schema = await introspect_schema(
                conn,
                include_table_prefixes=(BENCH_BUILTIN_TABLE_PREFIX,),
                exclude_table_prefixes=(BENCH_CUSTOM_TABLE_PREFIX,),
            )
        global_migration_ops = generate_migration_ops(
            old_schema=old_global_schema, new_schema=BUILTIN_GLOBAL_SCHEMA
        )
    else:
        global_migration_ops = []

    # generate migration
    if not global_migration_ops and not main_migration_ops:
        logger.info("migrate.make.noop")
        return
    latest_migration = max(file_migrations, key=lambda m: m.id)
    new_migration = Migration(
        id=latest_migration.id + 1 if latest_migration is not None else 1,
        version=VERSION,
        has_global=bool(global_migration_ops),
        has_main=bool(main_migration_ops),
        applied_at=None,
    )
    migration_code = generate_migration_code(
        new_migration,
        global_ops=global_migration_ops,
        main_ops=main_migration_ops,
        exclude_inverse=no_downgrade,
        oracle=REAL_ORACLE,
    )
    if not dry_run:
        add_migration_to_fs(migration=new_migration, code=migration_code)
    else:
        print(migration_code)

    logger.info("migrate.make", duration=time.time() - start)


@app.command(help="apply SQL migrations")
@async_to_sync
async def apply(
    area: "Area" = typer.Option(  # noqa: B008
        parser=parse_node_area, help="the area to migrate"
    ),
    target: Optional[str] = typer.Option(
        default=None, help="the migration to migrate to [default=latest]"
    ),
    region: Optional["Region"] = typer.Option(  # noqa: B008
        default=REGION, help="the region to migrate [default=current]", parser=parse_region
    ),
    cell_name: Optional[str] = typer.Option(
        default=None, help="the cell to migrate, global otherwise"
    ),
    external_name: Optional[str] = typer.Option(
        default=None, help="the external name to migrate, global otherwise"
    ),
    dry_run: bool = typer.Option(default=False, help="only try, don't commit"),
):
    from bench.language import REGION, Area
    from bench.sharding import DATABASE_PROVIDER, get_global_database_from_env
    from bench.store.postgres import pg_transaction, sql_migrate

    start = time.time()

    # resolve databases to migrate
    if area == Area.GLOBAL_DATABASE:
        global_database = get_global_database_from_env()
        databases = [global_database]
    elif area == Area.MAIN_DATABASE:
        assert cell_name, "cell_name is required for main area"
        assert external_name, "external_name is required for main area"
        main_database = await DATABASE_PROVIDER.resolve_or_error(
            region or REGION, cell_name, external_name
        )
        databases = [main_database]
    else:
        raise RuntimeError(f"cannot migrate area: {area!r}")

    for database in databases:
        async with pg_transaction(database) as (conn, tx):
            await sql_migrate(conn=conn, target=target, area=area, oracle=REAL_ORACLE)
            if not dry_run:
                await tx.commit()
            else:
                await tx.rollback()

    logger.info("migrate", duration=time.time() - start)
