import structlog
import typer

from bench.cli.utils import _async_to_sync_blocking, _check_is_consistent
from bench.language import Bench, Package
from bench.system.utils import global_session

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="check whether the current Bench state is properly migrated")
@_async_to_sync_blocking
async def check(check_db: bool = False):
    await _check_is_consistent(check_db=check_db)


@app.command(help="IPython shell with a global or Bench-local session")
@_async_to_sync_blocking
async def shell(bench: str = None, package: str = None):
    """Open a Session shell."""
    if bench is not None:
        async with global_session(readonly=True):
            bench: Bench = await Bench.get(slug=bench)
            if package is not None:
                package = await Package.get(parent=bench, slug=package)
    else:
        if package is not None:
            raise ValueError(f"Package {package} must be relative to a Bench")

    logger.info("lang.shell", bench=bench, package=package)
    raise NotImplementedError("TODO :Incomplete: lang.shell")
