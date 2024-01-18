import asyncio
import functools
import subprocess
from typing import TYPE_CHECKING

import structlog

from bench.language.node import NODE_CLASSES, Bench
from bench.server.utils import detached_session
from bench.sql.engine import map_node_class_to_pg_table, NODE_TABLES, GLOBAL_TABLES, LOCAL_TABLES

if TYPE_CHECKING:
    pass

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
    """Executes a shell command in a subprocess."""
    logger.debug("shell", cmd=cmd, check=check, **kwargs)
    subprocess.run(cmd, shell=True, check=check, **kwargs)


class OutOfSyncError(RuntimeError):
    def __init__(self, msg: str):
        super().__init__(f"out of sync, run 'bench lang upgrade': {msg}")


async def _check_is_consistent(*, check_db: bool, check_db_bench: str = "symbolx.bench") -> None:
    """Checks whether the language constructs are in sync with the derived stuff."""
    from bench.proto.wire import VERSION as PROTO_VERSION
    from bench.sql.schema import VERSION as SQL_VERSION
    from bench.language import VERSION as LANG_VERSION
    from bench.sql.migration import generate_migration_ops
    from bench.sql.migration import introspect_tables_from_pg
    from bench.sql.client import async_pg_cursor

    log = logger.bind(version=LANG_VERSION)
    start = asyncio.get_running_loop().time()
    log.debug("lang.check_consistency")

    # check just versions
    log.debug("lang.check_consistency.versions", proto=PROTO_VERSION, sql=SQL_VERSION)
    if PROTO_VERSION != LANG_VERSION:
        raise OutOfSyncError(f"proto version {PROTO_VERSION} != lang version {LANG_VERSION}")
    if SQL_VERSION != LANG_VERSION:
        raise OutOfSyncError(f"sql version {SQL_VERSION} != lang version {LANG_VERSION}")

    # diff generated SQL schema vs current schema
    log.debug("lang.check_consistency.schema")
    new_tables = tuple(
        map_node_class_to_pg_table(cls) for cls in NODE_CLASSES if not cls.__is_stored_custom__
    )
    migration_ops = generate_migration_ops(NODE_TABLES, new_tables)
    if migration_ops:
        raise OutOfSyncError(f"SQL schema is out of sync: {migration_ops!r}")

    # and diff DB state
    if check_db:
        log.debug("lang.check_consistency.db")

        # check global
        async with async_pg_cursor() as cur:
            old_global_tables = await introspect_tables_from_pg(cur)
        migration_ops = generate_migration_ops(old_global_tables, GLOBAL_TABLES)
        if migration_ops:
            raise OutOfSyncError(f"global SQL schema is out of sync: {migration_ops!r}")

        # check local
        async with detached_session(read_only=True):
            bench = await Bench.get(slug=check_db_bench)
        async with async_pg_cursor(local_pg_name=bench.pg_name) as cur:
            old_local_tables = await introspect_tables_from_pg(cur)
        migration_ops = generate_migration_ops(old_local_tables, LOCAL_TABLES)
        if migration_ops:
            raise OutOfSyncError(f"local SQL schema is out of sync: {migration_ops!r}")

    log.debug("lang.check_consistency.done", duration=asyncio.get_running_loop().time() - start)
