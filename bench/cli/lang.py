import structlog
import typer

from bench.cli.utils import async_to_sync_blocking
from bench.language import Bench, Package
from bench.system.core import global_session

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="IPython shell with a global or Bench-local session")
@async_to_sync_blocking
async def shell(bench: str = None, package: str = None):  # type: ignore
    """Open a Session shell."""
    if bench is not None:
        async with global_session():
            bench: Bench = await Bench.get(slug=bench)
            if package is not None:
                package: Package = await Package.get(parent=bench, slug=package)
    else:
        if package is not None:
            raise ValueError(f"Package {package} must be relative to a Bench")

    logger.info("lang.shell", bench=bench, package=package)
    raise NotImplementedError("TODO :Incomplete: lang.shell")
