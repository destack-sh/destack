import signal
import subprocess
from typing import TYPE_CHECKING, Annotated, Optional

import structlog
import typer

from bench.language.core.const import REGION, NodeArea, Region
from bench.utils.func import sanitize_connection_url
from bench.utils.oracle import REAL_ORACLE

from .utils import async_to_sync, parse_area, parse_region

if TYPE_CHECKING:
    from bench.language import NodeArea, Region

app = typer.Typer(short_help="postgres management")
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
@async_to_sync
async def shell(
    area: Annotated[NodeArea, typer.Option(parser=parse_area)],
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    bench: Optional[str] = None,
):  # type: ignore
    """Open a psql shell to either the global or a Bench-local database."""
    from bench.language import Bench, NodeArea, Package, Store
    from bench.system import (
        global_session,
        global_store_from_env,
        pg_engine_from_store,
        regional_store_from_env,
    )

    global_store = global_store_from_env()
    global_pg_engine = pg_engine_from_store("pg-global", global_store, NodeArea.GLOBAL)
    regional_store = regional_store_from_env(region)
    regional_pg_engine = pg_engine_from_store(
        f"pg-regional-{region.name.lower()}", regional_store, NodeArea.REGIONAL
    )

    if area == NodeArea.GLOBAL:
        store = global_store
    elif area == NodeArea.REGIONAL:
        store = regional_store
    elif area == NodeArea.LOCAL:
        assert bench is not None, "bench is required for local area"
        async with global_session(
            global_store, (global_pg_engine, regional_pg_engine), REAL_ORACLE
        ):
            bench_node = (
                await Bench.include_descendants(Package, Store).select_all().get(slug=bench)
            )
            assert bench_node.store is not None, f"{bench!r} has no main store"
            store = bench_node.store
    else:
        raise ValueError(f"invalid area: {area!r}")

    assert store.sql_url, f"store {store!r} has no connection_uri"
    logger.info(
        "shell.psql",
        area=area,
        bench=bench,
        store=store,
        sql_url=sanitize_connection_url(store.sql_url),
    )
    sigint_handler = signal.getsignal(signal.SIGINT)
    try:
        # allow SIGINT to pass to psql to abort queries
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        subprocess.run(["psql", store.sql_url], check=True)  # noqa: ASYNC221
    finally:
        signal.signal(signal.SIGINT, sigint_handler)
