import typer

from bench.cli.utils import _async_to_sync_blocking

app = typer.Typer(short_help="run tests")


@app.command(help="runs in-code tests with pytest")
def pytest():
    # invoke pytest
    from pytest import main

    main(["."])


@app.command(help="runs the flotothemoon.tests package")
@_async_to_sync_blocking
async def integration():
    raise NotImplementedError


@app.command(help="run all tests")
def all():
    pytest()
    integration()
