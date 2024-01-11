import re
import signal
import subprocess

import structlog
import typer

from bench.cli.utils import _async_to_sync_blocking
from bench.language.node import Bench
from bench.server.session import detached_session
from bench.sql.client import _get_connection_str, LOCAL_PG_HOST, LOCAL_PG_PORT

app = typer.Typer(short_help="convenience shells")

logger = structlog.get_logger(__name__)


@app.command()
@_async_to_sync_blocking
async def pg(bench: str = None):
    """Open a psql shell to either the global or a Bench-local database."""
    if bench is not None:
        async with detached_session(readonly=True):
            bench: Bench = await Bench.get(slug=bench)
        connection_str = f"postgresql://{bench.pg_username}:{bench.pg_password}@{LOCAL_PG_HOST}:{LOCAL_PG_PORT}/{bench.pg_name}"
    else:
        connection_str = _get_connection_str(local_pg_name=None)

    logger.info(
        "shell.psql", bench=bench, connection_str=re.sub(r":[^@]+@", ":*****@", connection_str)
    )
    sigint_handler = signal.getsignal(signal.SIGINT)
    try:
        # allow SIGINT to pass to psql to abort queries
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        subprocess.run(["psql", connection_str], check=True)
    finally:
        signal.signal(signal.SIGINT, sigint_handler)


@app.command()
@_async_to_sync_blocking
async def session(bench: str = None):
    """Open a Session shell."""
    if bench is not None:
        async with detached_session(readonly=True):
            bench: Bench = await Bench.get(slug=bench)
    logger.info("shell.session", bench=bench)
    raise NotImplementedError("nocheckin: shell.session")
