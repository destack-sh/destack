import asyncio
import functools
import os
import subprocess
from typing import TYPE_CHECKING

import structlog
import uvloop
from opentelemetry import trace

from bench.sql.graph import BENCH_RECORD_TABLE_PREFIX, BENCH_TABLE_PREFIX

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


class InconsistencyError(RuntimeError):
    def __init__(self, msg: str):
        super().__init__(f"bench internal state is inconsistent: {msg}")


@tracer.start_as_current_span("check_is_consistent")
async def check_is_consistent(*, check_db: bool) -> None:
    """Checks whether the language constructs are in sync with the derived stuff."""
    from bench.language import VERSION as LANG_VERSION
    from bench.language.setup import NODE_CLASSES
    from bench.proto.wire import VERSION as PROTO_VERSION
    from bench.sql.client import pg_connection
    from bench.sql.core import Schema
    from bench.sql.graph import GLOBAL_SCHEMA, NODE_TABLES, map_node_class_to_table
    from bench.sql.migration import generate_sql_migration_ops, introspect_sql_schema
    from bench.sql.schema import VERSION as SQL_VERSION
    from bench.system.utils.session import system_store_from_env

    log = logger.bind(version=LANG_VERSION)

    # check just versions
    if PROTO_VERSION != LANG_VERSION:
        raise InconsistencyError(f"proto version {PROTO_VERSION} != lang version {LANG_VERSION}")
    if SQL_VERSION != LANG_VERSION:
        raise InconsistencyError(f"sql version {SQL_VERSION} != lang version {LANG_VERSION}")

    # diff generated SQL schema vs current schema
    declared_tables = tuple(
        map_node_class_to_table(cls)
        for cls in NODE_CLASSES
        if cls.__is_stored__ and not cls.__is_stored_custom__
    )
    declared_schema = Schema(extensions=(), tables=declared_tables)
    actual_schema = Schema(extensions=(), tables=NODE_TABLES)
    migration_ops = generate_sql_migration_ops(actual_schema, declared_schema)
    if migration_ops:
        logger.error("check_is_consistent.schema.diff", diff=migration_ops)
        raise InconsistencyError(f"SQL schema is out of sync: {migration_ops!r}")

    # and diff DB state
    if check_db:
        # check global
        global_store = system_store_from_env()
        async with pg_connection(global_store) as conn:
            old_global_schema = await introspect_sql_schema(
                conn.cursor,
                include_table_prefixes=(BENCH_TABLE_PREFIX,),
                exclude_table_prefixes=(BENCH_RECORD_TABLE_PREFIX,),
            )
            await conn.rollback()
        migration_ops = generate_sql_migration_ops(old_global_schema, GLOBAL_SCHEMA)
        if migration_ops:
            raise InconsistencyError(f"global SQL schema is out of sync: {migration_ops!r}")

    log.debug("check_is_consistent", consistent=True, span="current")
