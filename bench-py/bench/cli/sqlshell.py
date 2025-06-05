import signal
import subprocess
from typing import TYPE_CHECKING, Annotated, Optional

import structlog
import typer

from bench.language.core.const import REGION, Area, Region
from bench.utils.func import sanitize_connection_url

from .utils import async_to_sync, parse_node_area, parse_region

if TYPE_CHECKING:
    from bench.language import Area, Region

app = typer.Typer(short_help="postgres management")
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
@async_to_sync
async def shell(
    area: Annotated[Area, typer.Option(parser=parse_node_area)],
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    cell_name: Optional[str] = None,
    external_id: Optional[str] = None,
    bench: Optional[str] = None,
):  # type: ignore
    """Open a psql shell to either the global or a Bench-local database."""
    from bench.language import Area
    from bench.sharding import DATABASE_PROVIDER, get_global_database_from_env

    if area == Area.GLOBAL_DATABASE:
        database = get_global_database_from_env()
    elif area == Area.MAIN_DATABASE:
        assert cell_name is not None, "cell_name is required for main area"
        assert external_id is not None, "external_id is required for main area"
        database = await DATABASE_PROVIDER.resolve_or_error(region, cell_name, external_id)
    else:
        raise ValueError(f"invalid area: {area!r}")

    assert database.sql_url, f"database {database!r} has no connection_uri"
    logger.info(
        "shell.psql",
        area=area,
        bench=bench,
        database=database,
        sql_url=sanitize_connection_url(database.sql_url),
    )
    sigint_handler = signal.getsignal(signal.SIGINT)
    try:
        # allow SIGINT to pass to psql to abort queries
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        subprocess.run(["psql", database.sql_url], check=True)  # noqa: ASYNC221
    finally:
        signal.signal(signal.SIGINT, sigint_handler)
