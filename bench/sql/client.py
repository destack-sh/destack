import contextvars
import re
from contextlib import asynccontextmanager
from typing import Any, AsyncContextManager

import psycopg
import psycopg_pool
import structlog
from psycopg.rows import dict_row
from psycopg_pool import AsyncConnectionPool

from bench.language import Store, StoreEngineType

# TODO :Robustness: figure out how to fix the psycopg pool warning
#  (what we're doing should be fine according to docs and the warning)
AsyncConnectionPool._warn_open_async = lambda *args, **kwargs: None  # type: ignore

logger = structlog.get_logger(__name__)
_connection_pools: dict[str, AsyncConnectionPool] = {}
_current_pg_crypto_key: contextvars.ContextVar[str | None] = contextvars.ContextVar(
    "current_pg_crypto_key", default=None
)


def current_pg_crypto_key() -> str:
    key = _current_pg_crypto_key.get()
    assert key is not None, "no active pg_crypto_key set"
    return key


async def get_pg_connection_pool(connection_str: str) -> AsyncConnectionPool:
    if connection_str not in _connection_pools:
        assert isinstance(connection_str, str), f"connection_str {connection_str!r} is not a str"
        # parse out key parts for pool name
        match = _CONNECTION_STR_REGEX.match(connection_str)
        assert match, f"connection_str {connection_str!r} does not match expected format"
        pool = AsyncConnectionPool(
            connection_str,
            min_size=1,
            max_size=4,
            max_idle=60 * 60,
            timeout=2,
            reconnect_timeout=3,
            connection_class=psycopg.AsyncConnection,
            kwargs={"row_factory": dict_row},
            name=f"{match['username']}@{match['host']}/{match['database']}",
        )
        await pool.open()
        _connection_pools[connection_str] = pool
    return _connection_pools[connection_str]


_CONNECTION_STR_REGEX = re.compile(
    r"postgresql://(?P<username>[^:]+):(?P<password>[^@]+)@(?P<host>[^/]+)/(?P<database>.+)"
)


def get_pg_connection_str(store: Store, database: str = None) -> str:
    assert store.engine == StoreEngineType.POSTGRES, f"store {store!r} is not a postgres store"
    assert store.main_credential is not None, f"store {store!r} has no main_credential"
    connection_str = f"postgresql://{store.main_credential.username}:{store.main_credential.password}@{store.host}/{database or store.database}"
    return connection_str


@asynccontextmanager
async def pg_connection(
    local_pg_name: str | None = None, autocommit: bool = False
) -> AsyncContextManager[psycopg.AsyncConnection[dict[str, Any]]]:
    """Gets a psycopg cursor to the given database"""
    pool = await get_pg_connection_pool(local_pg_name)
    async with pool.connection() as conn:
        if conn.autocommit != autocommit:
            await conn.set_autocommit(autocommit)
        yield conn


@asynccontextmanager
async def pg_cursor(
    connection_str: str, autocommit: bool = False
) -> AsyncContextManager[psycopg.AsyncCursor[dict[str, Any]]]:
    pool = await get_pg_connection_pool(connection_str)
    async with pool.connection() as conn:
        if conn.autocommit != autocommit:
            await conn.set_autocommit(autocommit)
        async with conn.cursor() as cur:
            yield cur


class _PgStoreConnection:
    __slots__ = ("store", "autocommit", "_reset_token", "_conn", "_pool")

    def __init__(self, store: Store, autocommit: bool = False):
        assert store.parent.encryption_key, f"store {store!r} has no encryption_key"
        self.store = store
        self.autocommit = autocommit
        self._pool: AsyncConnectionPool | None = None
        self._conn: psycopg.AsyncConnection | None = None

    async def open(self) -> psycopg.AsyncCursor:
        connection_str = get_pg_connection_str(self.store)
        self._pool = await get_pg_connection_pool(connection_str)
        try:
            self._conn = await self._pool.getconn()
        except psycopg_pool.PoolTimeout as e:
            logger.error("pg_pool_timeout", store=self.store, pool=self._pool, exc_info=e)
            raise
        if self._conn.autocommit != self.autocommit:
            await self._conn.set_autocommit(self.autocommit)
        _current_pg_crypto_key.set(self.store.parent.encryption_key)
        return self._conn.cursor()

    async def close(self) -> None:
        if self._conn is not None:
            await self._pool.putconn(self._conn)

    async def __aenter__(self) -> psycopg.AsyncCursor:
        return await self.open()

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        await self.close()


@asynccontextmanager
async def pg_cursor_to_store(
    store: Store, autocommit: bool = False
) -> AsyncContextManager[psycopg.AsyncCursor[dict[str, Any]]]:
    async with _PgStoreConnection(store, autocommit=autocommit) as cur:
        yield cur


async def get_pg_store_connection(store: Store, autocommit: bool = False) -> _PgStoreConnection:
    return _PgStoreConnection(store, autocommit=autocommit)
