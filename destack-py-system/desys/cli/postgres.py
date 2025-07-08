import time
from typing import TYPE_CHECKING, Annotated, Optional

import structlog
import typer
from rich.console import Console

from destack.cli.utils import async_to_sync, parse_region, parse_store_key
from destack.language import REGION, WORLD_ORACLE, Region, StoreKey

if TYPE_CHECKING:
    from destack.language import Region, StoreKey

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="migration management")
console = Console()


@app.command(help="generate SQL migrations")
@async_to_sync
async def make(
    store_key: Annotated[StoreKey | None, typer.Option(parser=parse_store_key)] = None,
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
    store_key: "StoreKey" = typer.Option(  # noqa: B008
        parser=parse_store_key, help="the store key to migrate"
    ),
    target: Optional[str] = typer.Option(
        default=None, help="the migration to migrate to [default=latest]"
    ),
    region: Optional["Region"] = typer.Option(  # noqa: B008
        default=REGION, help="the region to migrate [default=current]", parser=parse_region
    ),
    galaxy_name: Optional[str] = typer.Option(
        default=None, help="the galaxy to migrate, global otherwise"
    ),
    external_name: Optional[str] = typer.Option(
        default=None, help="the external name to migrate, global otherwise"
    ),
    dry_run: bool = typer.Option(default=False, help="only try, don't commit"),
):
    from destack.language import REGION, StoreKey
    from desys.sharding import DATABASE_PROVIDER, get_global_database_from_env
    from desys.store.postgres import postgres_migrate, postgres_transaction

    start = time.time()

    # resolve databases to migrate
    if store_key == StoreKey.GLOBAL_ENTITY_PRIMARY:
        global_database = get_global_database_from_env()
        databases = [global_database]
    elif store_key == StoreKey.SPATIAL_ENTITY_PRIMARY:
        assert galaxy_name, "galaxy_name is required for spatial stores"
        assert external_name, "external_name is required for spatial stores"
        spatial_database = await DATABASE_PROVIDER.resolve_or_error(
            region or REGION, galaxy_name, external_name
        )
        databases = [spatial_database]
    else:
        raise RuntimeError(f"cannot migrate store key: {store_key!r}")

    for database in databases:
        async with postgres_transaction(database) as (conn, tx):
            await postgres_migrate(
                conn=conn, target=target, store_key=store_key, oracle=WORLD_ORACLE
            )
            if not dry_run:
                await tx.commit()
            else:
                await tx.rollback()

    logger.info("migrate", duration=time.time() - start)
