import typer

from bench.cli.utils import _async_to_sync_blocking

app = typer.Typer(short_help="run tests")


@app.command(help="runs the tests package")
@_async_to_sync_blocking
async def integration():
    raise NotImplementedError
