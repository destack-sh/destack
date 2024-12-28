from typing import Annotated

import structlog
import typer

from bench.cli.utils import async_to_sync_blocking, parse_region
from bench.language import Bench, Store
from bench.language.const import REGION, NodeArea, Region
from bench.system.utils.session import regional_store_from_env
from bench.utils.oracle import REAL_ORACLE

app = typer.Typer(short_help="postgres management")
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
@async_to_sync_blocking
async def shell(
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    bench: str = typer.Option(..., help="the Bench to open a shell in"),
):  # type: ignore
    """Open a runtime-like shell to a Bench."""
    from bench.system.utils.session import (
        global_session,
        global_store_from_env,
        pg_engine_from_store,
    )

    global_store = global_store_from_env()
    global_pg_engine = pg_engine_from_store("pg-global", global_store, NodeArea.GLOBAL)
    regional_store = regional_store_from_env(region)
    regional_pg_engine = pg_engine_from_store(
        f"pg-regional-{region.name.lower()}", regional_store, NodeArea.REGIONAL
    )

    async with global_session(global_store, (global_pg_engine, regional_pg_engine), REAL_ORACLE):
        bench_node = await Bench.include_descendants(Store).select_all().get(slug=bench)

    raise NotImplementedError(f"shell to {bench_node!r} not supported yet")
