from typing import Optional

import structlog
import typer

from bench.cli import proto, sql, version
from bench.cli.utils import _async_to_sync_blocking
from bench.language.node import Bench
from bench.server.session import detached_session

app = typer.Typer(short_help="broad language-level utilities (combine other stuff)")

logger = structlog.get_logger(__name__)


@app.command()
async def upgrade(
    revision: Optional[int] = typer.Argument(None, help="revision to force to"),
    no_downgrade: bool = False,
    overwrite: bool = False,
):
    if not overwrite or revision is not None:
        await version.bump(revision=revision)
    await proto.regen()
    await sql.regen()
    await sql.makemigrations(no_downgrade=no_downgrade, overwrite=overwrite)


@app.command()
async def migrate(target: str = None, bench: str = None, dry_run: bool = False):
    await sql.migrate(target=target, bench=bench, dry_run=dry_run)


@app.command(help="IPython shell with a global or bench-local session")
@_async_to_sync_blocking
async def shell(bench: str = None):
    """Open a Session shell."""
    if bench is not None:
        async with detached_session(read_only=True):
            bench: Bench = await Bench.get(slug=bench)
    logger.info("lang.shell", bench=bench)
    raise NotImplementedError("nocheckin: shell.session")
