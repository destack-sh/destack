import structlog
import typer

from bench.cli import proto, sql
from bench.cli.utils import _async_to_sync_blocking, _check_is_consistent
from bench.language.node import Bench
from bench.server.utils import detached_session

app = typer.Typer(short_help="broad language-level utilities (combine other stuff)")

logger = structlog.get_logger(__name__)


@app.command(help="check whether the current Bench state is properly migrated")
@_async_to_sync_blocking
async def check(check_db: bool = False):
    await _check_is_consistent(check_db=check_db)


@app.command(help="upgrade all Bench language derived stuff (proto, sql, etc.)")
@_async_to_sync_blocking
async def upgrade(
    bench: str = typer.Option(default="symbolx.bench", help="the bench to use as local reference"),
    no_downgrade: bool = False,
    overwrite: bool = False,
):
    proto.regen()
    sql.regen()
    await sql.makemigrations(
        bench=bench, no_downgrade=no_downgrade, overwrite=overwrite, local_pg_name=None
    )


@app.command(help="migrate the Bench SQL databases")
@_async_to_sync_blocking
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
