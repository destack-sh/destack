import functools
from contextlib import asynccontextmanager
from typing import Any

import psycopg
from psycopg.rows import dict_row
from psycopg_pool import AsyncConnectionPool

from bench.utils.utils import get_from_env

GLOBAL_RO_USERNAME = "global-ro"
GLOBAL_RO_PASSWORD = "global-ro"

PG_HOST = get_from_env("LOCAL_PG_HOST", alt="USER_PG_HOST")
PG_NAME = get_from_env("LOCAL_PG_NAME", optional=True)
PG_PORT = get_from_env("LOCAL_PG_PORT", default=5432, type_cast=int, alt="USER_PG_PORT")
PG_USERNAME = get_from_env("LOCAL_PG_USERNAME", alt="USER_PG_USERNAME")
PG_PASSWORD = get_from_env("LOCAL_PG_PASSWORD", alt="USER_PG_PASSWORD")


@functools.cache
def _get_connection_str(pg_name: str) -> str:
    return f"postgresql://{PG_USERNAME}:{PG_PASSWORD}@{PG_HOST}:{PG_PORT}/{pg_name}"


_connection_pools: dict[str, AsyncConnectionPool] = {}


def _get_connection_pool(pg_name: str) -> AsyncConnectionPool:
    if pg_name not in _connection_pools:
        _connection_pools[pg_name] = AsyncConnectionPool(
            _get_connection_str(pg_name),
            min_size=1,
            max_size=5,
            max_idle=60 * 60,
            connection_class=psycopg.AsyncConnection,
            kwargs={"row_factory": dict_row},
        )
    return _connection_pools[pg_name]


@asynccontextmanager
async def async_pg_connection(
    pg_name: str, autocommit: bool = False
) -> psycopg.AsyncConnection[dict[str, Any]]:
    """Gets a psycopg (3) connection to the given database"""
    pool = _get_connection_pool(pg_name)
    async with pool.connection() as conn:
        if conn.autocommit != autocommit:
            await conn.set_autocommit(autocommit)
        yield conn


@asynccontextmanager
async def async_pg_cursor(
    pg_name: str, autocommit: bool = False
) -> psycopg.AsyncCursor[dict[str, Any]]:
    """Gets a psycopg (3) connection to the given database"""
    async with async_pg_connection(pg_name, autocommit=autocommit) as conn:
        async with conn.cursor() as cur:
            yield cur
