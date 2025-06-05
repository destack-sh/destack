from typing import TYPE_CHECKING, Annotated

import structlog
import typer

from bench.language import REGION, Area, Region, Session

from .utils import async_to_sync, parse_region

if TYPE_CHECKING:
    pass

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="create builtin stuff (User, Benches, etc.)")
@async_to_sync
async def bootstrap(
    region: Annotated[Region, typer.Option(parser=parse_region)], upsert: bool = False
):
    from bench.sharding import get_global_database_from_env
    from bench.store import DatabaseStore
    from bench.supervisor import create_system_benches

    global_database = get_global_database_from_env()
    store = DatabaseStore(database=global_database, area=Area.GLOBAL_DATABASE)
    async with Session(store=store) as session:
        await create_system_benches(region=region, session=session, upsert=upsert)
        await session.commit()


@app.command(
    name="make-local-machine-runtime",
    help="gets or creates a local runtime Machine (and Client) for a Bench",
)
@async_to_sync
async def make_local_machine_runtime(
    bench_slug: str,
    title: str = "Local Runtime Machine",
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    local_machine_url: str = "http://localhost:60062",
):
    raise NotImplementedError
