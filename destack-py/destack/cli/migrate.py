import time
from typing import TYPE_CHECKING, Annotated, Optional

import structlog
import typer
from rich.console import Console

from destack.language import REGION, AreaType, Region
from destack.utils.oracle import REAL_ORACLE

from .utils import async_to_sync, parse_node_area, parse_region

if TYPE_CHECKING:
    from destack.language import AreaType, Region

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="migration management")
console = Console()


@app.command(help="generate SQL migrations")
@async_to_sync
async def make(
    area: Annotated[AreaType | None, typer.Option(parser=parse_node_area)] = None,
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
    area: "AreaType" = typer.Option(  # noqa: B008
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
    from destack.language import REGION, AreaType
    from destack.sharding import DATABASE_PROVIDER, get_global_database_from_env
    from destack.store.postgres import pg_transaction, sql_migrate

    start = time.time()

    # resolve databases to migrate
    if area == AreaType.GLOBAL_POSTGRES:
        global_database = get_global_database_from_env()
        databases = [global_database]
    elif area == AreaType.SPATIAL_POSTGRES:
        assert cell_name, "cell_name is required for spatial area"
        assert external_name, "external_name is required for spatial area"
        spatial_database = await DATABASE_PROVIDER.resolve_or_error(
            region or REGION, cell_name, external_name
        )
        databases = [spatial_database]
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
