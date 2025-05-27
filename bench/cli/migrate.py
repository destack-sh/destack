import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING, Annotated, Optional

import structlog
import typer
from more_itertools import first
from rich import print
from rich.console import Console

from bench.language.core.const import REGION, NodeArea, Region
from bench.utils.oracle import REAL_ORACLE

from .utils import async_to_sync, parse_node_area, parse_region

if TYPE_CHECKING:
    from bench.language import NodeArea, Region

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="migration management")
console = Console()


@app.command(help="generate SQL migrations")
@async_to_sync
async def make(
    area: Annotated[NodeArea | None, typer.Option(parser=parse_node_area)] = None,
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    bench: str = typer.Option(default="bench", help="the bench to use as local reference"),
    no_downgrade: bool = typer.Option(default=False, help="exclude downgrade operations"),
    dry_run: bool = typer.Option(default=False, help="only print, don't database"),
    overwrite: bool = typer.Option(default=False, help="overwrite existing migration for version"),
    from_scratch: bool = typer.Option(default=False, help="generate migration from scratch"),
):
    from bench.language import VERSION, Bench, Database, NodeArea, Package
    from bench.sql import (
        BENCH_CUSTOM_NODE_PREFIX,
        BENCH_TABLE_PREFIX,
        BUILTIN_GLOBAL_SCHEMA,
        BUILTIN_LOCAL_SCHEMA,
        BUILTIN_REGIONAL_SCHEMA,
        Migration,
        SqlSchema,
        SqlUndefinedObjectError,
        add_migration_to_fs,
        generate_sql_migration_code,
        generate_sql_migration_ops,
        introspect_sql_schema,
        pg_connection,
        read_migrations_from_fs,
        read_migrations_from_pg,
    )
    from bench.system import (
        get_global_database_from_env,
        get_regional_database_from_env,
        global_session,
        pg_engine_from_database,
    )

    start = time.time()
    global_database = get_global_database_from_env()
    global_pg_engine = pg_engine_from_database(
        "pg-global", global_database, NodeArea.GLOBAL_POSTGRES
    )
    regional_database = get_regional_database_from_env(region)
    regional_pg_engine = pg_engine_from_database(
        f"pg-regional-{regional_database.region.name.lower()}",
        regional_database,
        NodeArea.REGIONAL_POSTGRES,
    )

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
        databased_migrations = await read_migrations_from_pg(conn.cursor)
    max_file_id = max(m.id for m in file_migrations) if file_migrations else 0
    max_databased_id = max(m.id for m in databased_migrations) if databased_migrations else 0
    if max_databased_id > max_file_id:
        raise RuntimeError(
            f"databased migrations are ahead of file migrations:\ndatabased={databased_migrations!r}\nfile={file_migrations!r}"
        )

    # diff local
    if area in (None, NodeArea.LOCAL_POSTGRES):
        if not from_scratch:
            try:
                async with global_session(
                    global_database, (global_pg_engine, regional_pg_engine), oracle=REAL_ORACLE
                ):
                    bench_node = await Bench.get(
                        where=Bench.property("slug").eq(bench),
                        Packages=Package.search(Databases=Database.search()),
                    ).execute_one()
                    assert bench_node.database, f"{bench!r} has no main database"
                    async with pg_connection(bench_node.database) as conn:
                        old_local_schema = await introspect_sql_schema(
                            conn.cursor,
                            include_table_prefixes=(BENCH_TABLE_PREFIX,),
                            exclude_table_prefixes=(BENCH_CUSTOM_NODE_PREFIX,),
                        )
            except SqlUndefinedObjectError as e:
                # missing from_scratch flag?
                console.print(f"[red]couldn't make migrations (missing --from-scratch?): {e}[/red]")
                sys.exit(-1)
        else:
            old_local_schema = SqlSchema.blank()
        local_migration_ops = generate_sql_migration_ops(
            old_schema=old_local_schema, new_schema=BUILTIN_LOCAL_SCHEMA
        )
    else:
        local_migration_ops = []

    # diff regional
    if area in (None, NodeArea.REGIONAL_POSTGRES):
        async with pg_connection(regional_database) as conn:
            old_regional_schema = await introspect_sql_schema(
                conn.cursor,
                include_table_prefixes=(BENCH_TABLE_PREFIX,),
                exclude_table_prefixes=(BENCH_CUSTOM_NODE_PREFIX,),
            )
        regional_migration_ops = generate_sql_migration_ops(
            old_schema=old_regional_schema, new_schema=BUILTIN_REGIONAL_SCHEMA
        )
    else:
        regional_migration_ops = []

    # diff global
    if area in (None, NodeArea.GLOBAL_POSTGRES):
        async with pg_connection(global_database) as conn:
            old_global_schema = await introspect_sql_schema(
                conn.cursor,
                include_table_prefixes=(BENCH_TABLE_PREFIX,),
                exclude_table_prefixes=(BENCH_CUSTOM_NODE_PREFIX,),
            )
        global_migration_ops = generate_sql_migration_ops(
            old_schema=old_global_schema, new_schema=BUILTIN_GLOBAL_SCHEMA
        )
    else:
        global_migration_ops = []

    # generate migration
    if not global_migration_ops and not local_migration_ops and not regional_migration_ops:
        logger.info("migrate.make.noop")
        return
    latest_migration = max(file_migrations, key=lambda m: m.id)
    new_migration = Migration(
        id=latest_migration.id + 1 if latest_migration is not None else 1,
        version=VERSION,
        has_global=bool(global_migration_ops),
        has_local=bool(local_migration_ops),
        has_regional=bool(regional_migration_ops),
        applied_at=None,
    )
    migration_code = generate_sql_migration_code(
        new_migration,
        global_ops=global_migration_ops,
        local_ops=local_migration_ops,
        regional_ops=regional_migration_ops,
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
    area: "NodeArea" = typer.Option(  # noqa: B008
        parser=parse_node_area, help="the area to migrate"
    ),
    target: Optional[str] = typer.Option(
        default=None, help="the migration to migrate to [default=latest]"
    ),
    region: Optional["Region"] = typer.Option(  # noqa: B008
        default=REGION, help="the region to migrate [default=current]", parser=parse_region
    ),
    bench: Optional[str] = typer.Option(
        default=None, help="the local bench to migrate, global otherwise"
    ),
    dry_run: bool = typer.Option(default=False, help="only try, don't commit"),
):
    from bench.language import REGION, NodeArea
    from bench.sql import pg_connection, sql_migrate
    from bench.system import get_global_database_from_env, get_regional_database_from_env

    start = time.time()
    global_database = get_global_database_from_env()
    regional_database = get_regional_database_from_env(region or REGION)

    # resolve databases to migrate
    if area == NodeArea.REGIONAL_POSTGRES:
        databases = [regional_database]
    elif area == NodeArea.GLOBAL_POSTGRES:
        databases = [global_database]
    else:
        raise RuntimeError(f"cannot migrate area: {area!r}")

    for database in databases:
        async with pg_connection(database) as conn:
            await sql_migrate(cur=conn.cursor, target=target, area=area, oracle=REAL_ORACLE)
            if not dry_run:
                await conn.commit()
            else:
                await conn.rollback()

    logger.info("migrate", duration=time.time() - start)
