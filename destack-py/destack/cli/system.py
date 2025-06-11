from typing import TYPE_CHECKING, Annotated

import structlog
import typer

from destack.language import REGION, AreaType, Region, Session

from .utils import async_to_sync, parse_region

if TYPE_CHECKING:
    pass

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="create builtin stuff (User, Destackes, etc.)")
@async_to_sync
async def bootstrap(
    region: Annotated[Region, typer.Option(parser=parse_region)], upsert: bool = False
):
    from destack.sharding import get_global_database_from_env
    from destack.store import PostgresStore
    from destack.supervisor import create_system_destackes

    global_database = get_global_database_from_env()
    store = PostgresStore(database=global_database, area=AreaType.GLOBAL_POSTGRES)
    async with Session(store=store) as session:
        await create_system_destackes(region=region, session=session, upsert=upsert)
        await session.commit()


@app.command(
    name="make-local-machine-runtime",
    help="gets or creates a local runtime Machine (and Client) for a Destack",
)
@async_to_sync
async def make_local_machine_runtime(
    destack_slug: str,
    title: str = "Local Runtime Machine",
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    local_machine_url: str = "http://localhost:60062",
):
    raise NotImplementedError
