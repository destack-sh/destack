import asyncio
import functools
import subprocess

import structlog

logger = structlog.get_logger(__name__)


def _async_to_sync_blocking(func=None):
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
                return asyncio.run(func(*args, **kwargs))

        return wrapped

    if func is None:
        return decorate
    else:
        return decorate(func)


def _shell(cmd: str, check=True, **kwargs):
    logger.debug("shell", cmd=cmd, check=check, **kwargs)
    subprocess.run(cmd, shell=True, check=check, **kwargs)
