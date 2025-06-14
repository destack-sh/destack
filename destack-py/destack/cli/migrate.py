import time
from typing import TYPE_CHECKING, Annotated, Optional

import structlog
import typer
from rich.console import Console

from destack.language import REGION, Region, StoreType
from destack.utils.oracle import REAL_ORACLE

from .utils import async_to_sync, parse_region, parse_store_type

if TYPE_CHECKING:
    from destack.language import Region, StoreType

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="migration management")
console = Console()


@app.command(help="generate SQL migrations")
@async_to_sync
async def make(
    store_type: Annotated[StoreType | None, typer.Option(parser=parse_store_type)] = None,
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    space: str = typer.Option(default="space", help="the space to use as local reference"),
    no_downgrade: bool = typer.Option(default=False, help="exclude downgrade operations"),
    dry_run: bool = typer.Option(default=False, help="only print, don't database"),
    overwrite: bool = typer.Option(default=False, help="overwrite existing migration for version"),
    from_scratch: bool = typer.Option(default=False, help="generate migration from scratch"),
):
    raise NotImplementedError


@app.command(help="apply SQL migrations")
@async_to_sync
async def apply(
    store_type: "StoreType" = typer.Option(  # noqa: B008
        parser=parse_store_type, help="the store type to migrate"
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
    from destack.language import REGION, StoreType
    from destack.sharding import DATABASE_PROVIDER, get_global_database_from_env
    from destack.store.postgres import pg_transaction, postgres_migrate

    start = time.time()

    # resolve databases to migrate
    if store_type == StoreType.GLOBAL_ENTITY:
        global_database = get_global_database_from_env()
        databases = [global_database]
    elif store_type == StoreType.SPATIAL_ENTITY:
        assert cell_name, "cell_name is required for spatial stores"
        assert external_name, "external_name is required for spatial stores"
        spatial_database = await DATABASE_PROVIDER.resolve_or_error(
            region or REGION, cell_name, external_name
        )
        databases = [spatial_database]
    else:
        raise RuntimeError(f"cannot migrate store type: {store_type!r}")

    for database in databases:
        async with pg_transaction(database) as (conn, tx):
            await postgres_migrate(
                conn=conn, target=target, store_type=store_type, oracle=REAL_ORACLE
            )
            if not dry_run:
                await tx.commit()
            else:
                await tx.rollback()

    logger.info("migrate", duration=time.time() - start)
