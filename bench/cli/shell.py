import signal
import subprocess
from typing import Optional

import structlog
import typer

from bench.cli.utils import async_to_sync_blocking
from bench.language import Bench, Store
from bench.sql.client import get_pg_connection_uri
from bench.system.core import global_session, pg_engine_from_store, system_store_from_env
from bench.utils.func import sanitize_connection_uri
from bench.utils.oracle import REAL_ORACLE

app = typer.Typer(short_help="postgres management")
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
@async_to_sync_blocking
async def shell(bench: Optional[str] = None):  # type: ignore
    """Open a psql shell to either the global or a Bench-local database."""
    global_store = system_store_from_env()
    global_pg_engine = pg_engine_from_store(global_store)
    if bench is not None:
        async with global_session(global_store, (global_pg_engine,), REAL_ORACLE):
            bench_node = await Bench.descendants(Store).select_all().get(slug=bench)
            assert bench_node.main_store, f"{bench!r} has no main store"
            store = bench_node.main_store
    else:
        bench_node = None
        store = global_store
    connection_uri = get_pg_connection_uri(store)

    logger.info(
        "shell.psql",
        bench_node=bench_node,
        store=store,
        connection_uri=sanitize_connection_uri(connection_uri),
    )
    sigint_handler = signal.getsignal(signal.SIGINT)
    try:
        # allow SIGINT to pass to psql to abort queries
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        subprocess.run(["psql", connection_uri], check=True)  # noqa: ASYNC101
    finally:
        signal.signal(signal.SIGINT, sigint_handler)
