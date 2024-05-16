import contextvars
import re
from contextlib import asynccontextmanager

import psycopg
import psycopg_pool
import structlog
from psycopg.rows import dict_row
from psycopg_pool import AsyncConnectionPool

from bench.language import Store

# TODO :Robustness: figure out how to fix the psycopg pool warning
#  (what we're doing should be fine according to docs and the warning)
AsyncConnectionPool._warn_open_async = lambda *args, **kwargs: None  # type: ignore

logger = structlog.get_logger(__name__)
_connection_pools: dict[str, AsyncConnectionPool] = {}
_current_store: contextvars.ContextVar[Store | None] = contextvars.ContextVar(
    "current_store", default=None
)
_current_pg_crypto_key: contextvars.ContextVar[str | None] = contextvars.ContextVar(
    "current_pg_crypto_key", default=None
)


def current_pg_crypto_key() -> str:
    key = _current_pg_crypto_key.get()
    if key is None:
        store = _current_store.get()
        raise AssertionError(f"no active pg_crypto_key set for store {store!r}")
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
            timeout=5,
            reconnect_timeout=5,
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


def get_pg_connection_str(store: Store, database: str | None = None) -> str:
    # TODO :Security :Scalability: route store clients/hosts better :StoreRouting
    assert store.connection_uri, f"store {store!r} has no connection_url"
    if database is not None:
        return store.connection_uri.rsplit("/", 1)[0] + "/" + database
    else:
        return store.connection_uri


@asynccontextmanager
async def pg_cursor(connection_str: str, autocommit: bool = False):
    pool = await get_pg_connection_pool(connection_str)
    async with pool.connection() as conn:
        if conn.autocommit != autocommit:
            await conn.set_autocommit(autocommit)
        async with conn.cursor() as cur:
            yield cur


class _PgStoreConnection:
    __slots__ = ("store", "database", "autocommit", "_reset_token", "_conn", "_pool")

    def __init__(self, store: Store, database: str | None = None, autocommit: bool = False):
        self.store = store
        self.database = database
        self.autocommit = autocommit
        self._pool: AsyncConnectionPool | None = None
        self._conn: psycopg.AsyncConnection | None = None

    async def open(self) -> psycopg.AsyncCursor:
        connection_str = get_pg_connection_str(self.store, database=self.database)
        self._pool = await get_pg_connection_pool(connection_str)
        try:
            self._conn = await self._pool.getconn()
        except psycopg_pool.PoolTimeout as e:
            logger.error("pg_pool_timeout", store=self.store, pool=self._pool, exc_info=e)
            raise
        if self._conn.autocommit != self.autocommit:
            await self._conn.set_autocommit(self.autocommit)
        _current_store.set(self.store)
        _current_pg_crypto_key.set(self.store.bench.encryption_key)
        return self._conn.cursor()

    async def close(self) -> None:
        _current_store.set(None)
        if self._pool is not None and self._conn is not None:
            await self._pool.putconn(self._conn)

    async def __aenter__(self) -> psycopg.AsyncCursor:
        return await self.open()

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        await self.close()


@asynccontextmanager
async def pg_cursor_to_store(store: Store, autocommit: bool = False):
    async with _PgStoreConnection(store, autocommit=autocommit) as cur:
        yield cur


async def get_pg_store_connection(store: Store, autocommit: bool = False) -> _PgStoreConnection:
    return _PgStoreConnection(store, autocommit=autocommit)
