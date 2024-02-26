import asyncio
import functools
import subprocess
from typing import TYPE_CHECKING

import structlog

from bench.language import Bench, Environment, Store
from bench.language.setup import NODE_CLASSES
from bench.sql.client import pg_cursor_to_store
from bench.sql.engine import GLOBAL_TABLES, LOCAL_TABLES, NODE_TABLES, map_node_class_to_pg_table
from bench.system.client import global_pg_cursor, global_session

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


class InconsistencyError(RuntimeError):
    def __init__(self, msg: str):
        super().__init__(f"bench internal state is inconsistent: {msg}")


async def _check_is_consistent(*, check_db: bool, check_db_bench: str = "bench") -> None:
    """Checks whether the language constructs are in sync with the derived stuff."""
    from bench.language import VERSION as LANG_VERSION
    from bench.proto.wire import VERSION as PROTO_VERSION
    from bench.sql.migration import generate_migration_ops, introspect_tables_from_pg
    from bench.sql.schema import VERSION as SQL_VERSION

    log = logger.bind(version=LANG_VERSION)
    start = asyncio.get_running_loop().time()
    log.debug("lang.check_consistency")

    # check just versions
    log.debug("lang.check_consistency.versions", proto=PROTO_VERSION, sql=SQL_VERSION)
    if PROTO_VERSION != LANG_VERSION:
        raise InconsistencyError(f"proto version {PROTO_VERSION} != lang version {LANG_VERSION}")
    if SQL_VERSION != LANG_VERSION:
        raise InconsistencyError(f"sql version {SQL_VERSION} != lang version {LANG_VERSION}")

    # diff generated SQL schema vs current schema
    log.debug("lang.check_consistency.schema")
    new_tables = tuple(
        map_node_class_to_pg_table(cls)
        for cls in NODE_CLASSES
        if cls.__is_stored__ and not cls.__is_stored_custom__
    )
    migration_ops = generate_migration_ops(NODE_TABLES, new_tables)
    if migration_ops:
        logger.error("lang.check_consistency.schema.diff", diff=migration_ops)
        raise InconsistencyError(f"SQL schema is out of sync: {migration_ops!r}")

    # and diff DB state
    if check_db:
        log.debug("lang.check_consistency.db")

        # check global
        async with global_pg_cursor() as cur:
            old_global_tables = await introspect_tables_from_pg(cur)
        migration_ops = generate_migration_ops(old_global_tables, GLOBAL_TABLES)
        if migration_ops:
            raise InconsistencyError(f"global SQL schema is out of sync: {migration_ops!r}")

        # check local
        async with global_session():
            bench = await Bench.descendants(Environment, Store).get(slug=check_db_bench)
        async with pg_cursor_to_store(bench.main_environment.store) as cur:
            old_local_tables = await introspect_tables_from_pg(cur)
        migration_ops = generate_migration_ops(old_local_tables, LOCAL_TABLES)
        if migration_ops:
            raise InconsistencyError(f"local SQL schema is out of sync: {migration_ops!r}")

    log.debug("lang.check_consistency.done", duration=asyncio.get_running_loop().time() - start)
