import asyncio
import functools
import os
import subprocess
from typing import TYPE_CHECKING

import structlog
import typer
import uvloop
from opentelemetry import trace

from bench.language.const import Region

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def async_to_sync_blocking(func=None):
    """Automatically convert async functions to sync if not called in async context."""

    def decorate(func):
        # check that the func is async
        if not asyncio.iscoroutinefunction(func):
            raise TypeError(f"{func} is not a coroutine function")

        @functools.wraps(func)
        def wrapped(*args, **kwargs):
            # are we in an async context?
            try:
                asyncio.get_running_loop()
                is_in_loop = True
            except RuntimeError:
                is_in_loop = False
            if is_in_loop:
                return func(*args, **kwargs)
            else:
                return uvloop.run(func(*args, **kwargs))

        return wrapped

    if func is None:
        return decorate
    else:
        return decorate(func)


def run_shell_sync(cmd: str, check=True, **kwargs):
    """Executes a shell command in a subprocess."""
    cwd = os.getcwd()
    logger.trace("shell", cmd=cmd, cwd=cwd, check=check, **kwargs)
    subprocess.run(cmd, shell=True, check=check, **kwargs)


def parse_region(region: str | Region) -> Region:
    """Parse a Region from a string."""
    if isinstance(region, Region):
        return region
    region = region.upper()
    try:
        if region in Region.__members__:
            # try by name
            return Region[region]
        else:
            # try by value
            return Region(int(region))
    except ValueError as e:
        raise typer.BadParameter(
            f"invalid region: '{region.lower()}' (expected: {'|'.join(r.name.lower() for r in Region)})"
        ) from e
