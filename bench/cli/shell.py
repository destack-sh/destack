import structlog
import typer

from bench.cli.utils import _async_to_sync_blocking
from bench.language.node import Bench
from bench.server.session import detached_session

app = typer.Typer(short_help="convenience shell")

logger = structlog.get_logger(__name__)


@app.command()
@_async_to_sync_blocking
async def session(bench: str = None):
    """Open a Session shell."""
    if bench is not None:
        async with detached_session(read_only=True):
            bench: Bench = await Bench.get(slug=bench)
    logger.info("shell.session", bench=bench)
    raise NotImplementedError("nocheckin: shell.session")
