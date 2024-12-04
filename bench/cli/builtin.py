import structlog
import typer
from rich.console import Console

from bench.cli.utils import async_to_sync_blocking

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="migration management")
console = Console()


@async_to_sync_blocking
async def sync():
    from bench.language.builtin import BUILTINS
    from bench.runtime.base import BENCH_QUERY, PACKAGE_QUERY

    bench = await BENCH_QUERY.get(slug="bench")
    package = await PACKAGE_QUERY.get(bench.main_package_ptr)
    await patch_graph(package, BUILTINS)
