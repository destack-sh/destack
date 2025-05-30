import signal
import subprocess
from typing import TYPE_CHECKING, Annotated, Optional

import structlog
import typer

from bench.language.core.const import REGION, NodeArea, Region
from bench.language.core.query import JoinType
from bench.utils.func import sanitize_connection_url
from bench.utils.oracle import REAL_ORACLE

from .utils import async_to_sync, parse_node_area, parse_region

if TYPE_CHECKING:
    from bench.language import NodeArea, Region

app = typer.Typer(short_help="postgres management")
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
@async_to_sync
async def shell(
    area: Annotated[NodeArea, typer.Option(parser=parse_node_area)],
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    bench: Optional[str] = None,
):  # type: ignore
    """Open a psql shell to either the global or a Bench-local database."""
    from bench.language import Bench, Database, NodeArea, Session, join
    from bench.system import (
        PostgresStore,
        get_global_database_from_env,
        get_main_database_from_env,
    )

    global_database = get_global_database_from_env()
    main_database = get_main_database_from_env(region)

    if area == NodeArea.GLOBAL_POSTGRES:
        database = global_database
    elif area == NodeArea.MAIN_POSTGRES:
        database = main_database
    elif area == NodeArea.CUSTOM_POSTGRES:
        assert bench is not None, "bench is required for local area"
        store = PostgresStore(
            {
                NodeArea.GLOBAL_POSTGRES: global_database,
                NodeArea.MAIN_POSTGRES: main_database,
            }
        )
        async with Session(store=store, oracle=REAL_ORACLE):
            bench_node = await Bench.get(
                where=Bench.property("slug").eq(bench),
                Database=Database.get(
                    join=join(
                        JoinType.LEFT, on=Database.property("id").eq(Bench.property("database"))
                    )
                ),
            ).execute_one()
            assert bench_node.database is not None, f"{bench!r} has no main database"
            database = bench_node.database
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
