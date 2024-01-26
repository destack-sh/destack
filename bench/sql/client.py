from contextlib import asynccontextmanager
from typing import Any

import psycopg
from psycopg.rows import dict_row
from psycopg_pool import AsyncConnectionPool

from bench.utils.utils import get_from_env

UNIVERSAL_RO_USERNAME = "global-ro"
UNIVERSAL_RO_PASSWORD = "global-ro"

GLOBAL_PG_HOST = get_from_env("GLOBAL_PG_HOST", default=None)
GLOBAL_PG_NAME = get_from_env("GLOBAL_PG_NAME", default=None)
GLOBAL_PG_PORT = get_from_env("GLOBAL_PG_PORT", default=5432, type_cast=int)
GLOBAL_PG_USERNAME = get_from_env("GLOBAL_PG_USERNAME", default=None)
GLOBAL_PG_PASSWORD = get_from_env("GLOBAL_PG_PASSWORD", default=None)

LOCAL_PG_HOST = get_from_env("LOCAL_PG_HOST", alt="USER_PG_HOST")
LOCAL_PG_NAME = get_from_env("LOCAL_PG_NAME", optional=True)
LOCAL_PG_PORT = get_from_env("LOCAL_PG_PORT", default=5432, type_cast=int, alt="USER_PG_PORT")
LOCAL_PG_USERNAME = get_from_env("LOCAL_PG_USERNAME", alt="USER_PG_USERNAME")
LOCAL_PG_PASSWORD = get_from_env("LOCAL_PG_PASSWORD", alt="USER_PG_PASSWORD")

# TODO @Robustness: figure out how to fix the psycopg pool warning
#  (what we're doing should be fine according to docs and the warning)
AsyncConnectionPool._warn_open_async = lambda *args, **kwargs: None  # type: ignore


def _get_pg_connection_str(local_pg_name: str | None) -> str:
    if local_pg_name is None:
        # global database
        return f"postgresql://{GLOBAL_PG_USERNAME}:{GLOBAL_PG_PASSWORD}@{GLOBAL_PG_HOST}:{GLOBAL_PG_PORT}/{GLOBAL_PG_NAME}"
    else:
        # local database
        username, password = LOCAL_PG_USERNAME, LOCAL_PG_PASSWORD
        if LOCAL_PG_NAME and local_pg_name != LOCAL_PG_NAME:
            username, password = UNIVERSAL_RO_USERNAME, UNIVERSAL_RO_PASSWORD
        return f"postgresql://{username}:{password}@{LOCAL_PG_HOST}:{LOCAL_PG_PORT}/{local_pg_name}"


_connection_pools: dict[str, AsyncConnectionPool] = {}


async def get_pg_connection_pool(local_pg_name: str | None) -> AsyncConnectionPool:
    if local_pg_name not in _connection_pools:
        pool = AsyncConnectionPool(
            _get_pg_connection_str(local_pg_name),
            min_size=1,
            max_size=4,
            max_idle=60 * 60,
            reconnect_timeout=10,
            connection_class=psycopg.AsyncConnection,
            kwargs={"row_factory": dict_row},
        )
        await pool.open()
        _connection_pools[local_pg_name] = pool
    return _connection_pools[local_pg_name]


@asynccontextmanager
async def async_pg_connection(
    local_pg_name: str | None = None, autocommit: bool = False
) -> psycopg.AsyncConnection[dict[str, Any]]:
    """Gets a psycopg cursor to the given database"""
    pool = await get_pg_connection_pool(local_pg_name)
    async with pool.connection() as conn:
        if conn.autocommit != autocommit:
            await conn.set_autocommit(autocommit)
        yield conn


@asynccontextmanager
async def async_pg_cursor(
    local_pg_name: str | None = None, autocommit: bool = False
) -> psycopg.AsyncCursor[dict[str, Any]]:
    """Gets a psycopg cursor to the given database"""
    async with async_pg_connection(local_pg_name, autocommit=autocommit) as conn:
        async with conn.cursor() as cur:
            yield cur
