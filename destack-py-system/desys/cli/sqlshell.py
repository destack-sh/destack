import signal
import subprocess
from typing import TYPE_CHECKING, Annotated, Optional

import structlog
import typer

from destack.cli.utils import async_to_sync, parse_region, parse_store_type
from destack.language.core import REGION, Region, StoreType
from destack.utils.func import sanitize_connection_url

if TYPE_CHECKING:
    from destack.language import Region, StoreType

app = typer.Typer(short_help="postgres management")
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
@async_to_sync
async def sqlshell(
    store_type: Annotated[StoreType, typer.Option(parser=parse_store_type)],
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    galaxy_name: Optional[str] = None,
    external_id: Optional[str] = None,
    space: Optional[str] = None,
):  # type: ignore
    """Open a psql shell to either the global or a Space-local database."""
    from destack.language import StoreType
    from desys.sharding import DATABASE_PROVIDER, get_global_database_from_env

    if store_type == StoreType.GLOBAL_ENTITY:
        database = get_global_database_from_env()
    elif store_type == StoreType.SPATIAL_ENTITY:
        assert galaxy_name is not None, "galaxy_name is required for main store_type"
        assert external_id is not None, "external_id is required for main store_type"
        database = await DATABASE_PROVIDER.resolve_or_error(region, galaxy_name, external_id)
    else:
        raise ValueError(f"invalid store_type: {store_type!r}")

    assert database.connection_url, f"database {database!r} has no connection_uri"
    logger.info(
        "shell.psql",
        store_type=store_type,
        space=space,
        database=database,
        sql_url=sanitize_connection_url(database.connection_url),
    )
    sigint_handler = signal.getsignal(signal.SIGINT)
    try:
        # allow SIGINT to pass to psql to abort queries
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        subprocess.run(["psql", database.connection_url], check=True)  # noqa: ASYNC221
    finally:
        signal.signal(signal.SIGINT, sigint_handler)
