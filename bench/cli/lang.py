import structlog
import typer

from bench.cli.utils import _async_to_sync_blocking, _check_is_consistent
from bench.language.node import Bench
from bench.system.utils import detached_session

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="check whether the current Bench state is properly migrated")
@_async_to_sync_blocking
async def check(check_db: bool = False):
    await _check_is_consistent(check_db=check_db)


@app.command(help="IPython shell with a global or bench-local session")
@_async_to_sync_blocking
async def shell(bench: str = None):
    """Open a Session shell."""
    if bench is not None:
        async with detached_session(read_only=True):
            bench: Bench = await Bench.get(slug=bench)
    logger.info("lang.shell", bench=bench)
    raise NotImplementedError("nocheckin: shell.session")
