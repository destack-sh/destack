import asyncio
import contextvars
import re
from contextlib import asynccontextmanager
from typing import ClassVar

import psycopg
import psycopg_pool
import structlog
from opentelemetry import trace
from psycopg.rows import dict_row
from psycopg_pool import AsyncConnectionPool

from bench.language import Bench, Store
from bench.language.const import SUB_PACKAGE_NODE_TYPES, NodeType, active_bench
from bench.sql.core import Table, TableObject
from bench.utils.func import sanitize_connection_uri
from bench.utils.utils import get_from_env

# NOTE :Robustness: figure out how to fix the psycopg pool warning
#  (what we're doing should be fine according to docs and the warning)
AsyncConnectionPool._warn_open_async = lambda *args, **kwargs: None  # type: ignore

GLOBAL_PG_CRYPTO_KEY = get_from_env(
    "GLOBAL_PG_CRYPTO_KEY", default=None, description="Symmetric key for PG crypto in global store"
)
PG_CONNECT_TIMEOUT = get_from_env(
    "PG_CONNECT_TIMEOUT", typ=int, default=10, description="Postgres connection timeout in seconds"
)
PG_RECONNECT_TIMEOUT = get_from_env(
    "PG_RECONNECT_TIMEOUT", typ=int, default=15, description="Postgres reconnect timeout in seconds"
)

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)
_connection_pools: dict[str, AsyncConnectionPool] = {}
_pool_lock = asyncio.Lock()
_force_pg_crypto_key: contextvars.ContextVar[str | None] = contextvars.ContextVar(
    "force_pg_crypto_key", default=None
)


def get_pg_crypto_key(object: TableObject | Table) -> str:
    """
    Gets the appropriate crypto key.
    NOTE :Architecture: reversing the crypto key from the table object feels backwards,
                        but we'll likely refactor the SQL engine sometime anyway.
    """
    if key := _force_pg_crypto_key.get():  # for testing.. :c
        return key

    from bench.sql.engine import NODE_TYPE_BY_TABLE_NAME

    table = object.table if isinstance(object, TableObject) else object
    node_type = NODE_TYPE_BY_TABLE_NAME.get(table.name)
    if node_type is None:  # :BlockTablePrefix
        if table.name.startswith("bench_record_"):
            node_type = NodeType.RECORD
        else:
            raise RuntimeError(f"unexpected table {table!r} (cannot determine node type)")

    if node_type in SUB_PACKAGE_NODE_TYPES:
        return active_bench().encryption_key
    else:
        assert GLOBAL_PG_CRYPTO_KEY, f"no global pg_crypto_key set for {object!r}"
        return GLOBAL_PG_CRYPTO_KEY


async def get_pg_connection_pool(connection_uri: str) -> AsyncConnectionPool:
    """Gets an open connection pool"""
    if connection_uri in _connection_pools:
        return _connection_pools[connection_uri]

    async with _pool_lock:
        # check if already open
        if connection_uri in _connection_pools:
            return _connection_pools[connection_uri]

        # parse out key parts for pool name
        sanitized_connection_uri = sanitize_connection_uri(connection_uri)
        match = _CONNECTION_STR_REGEX.match(connection_uri)
        assert match, f"connection_uri {connection_uri!r} does not match expected format"

        # open new pool
        pool = AsyncConnectionPool(
            connection_uri,
            min_size=2,
            max_size=10,
            max_idle=60 * 60,
            timeout=PG_CONNECT_TIMEOUT,
            reconnect_timeout=PG_RECONNECT_TIMEOUT,
            connection_class=psycopg.AsyncConnection,
            kwargs={"row_factory": dict_row},
            name=f"{match['username']}@{match['host']}/{match['database']}",
        )
        await pool.open()
        _connection_pools[connection_uri] = pool
        logger.trace("pg.pool.open", connection_uri=sanitized_connection_uri, pool=pool)
        return pool


async def close_pg_connection_pool(connection_uri: str) -> None:
    """Closes a connection pool."""
    async with _pool_lock:
        pool = _connection_pools.get(connection_uri)
        if pool is not None:
            sanitized_connection_uri = sanitize_connection_uri(connection_uri)
            await pool.close()
            del _connection_pools[connection_uri]
            logger.trace("pg.pool.close", connection_uri=sanitized_connection_uri, pool=pool)


async def cycle_pg_connection_pool(connection_uri: str) -> None:
    """Cycles a connection pool (discarding all current connections and removing the pool)."""
    pool = _connection_pools.get(connection_uri)
    if pool is not None:
        for conn in pool._pool:
            if conn._pool is pool:
                await conn.close()
                await pool.putconn(conn)
    del _connection_pools[connection_uri]


_CONNECTION_STR_REGEX = re.compile(
    r"postgresql://(?P<username>[^:]+)(:(?P<password>[^@]+))?@(?P<host>[^/]+)/(?P<database>.+)"
)


def get_pg_connection_uri(store: Store, database: str | None = None) -> str:
    # TODO :Security :Scalability: route store clients/hosts better :StoreRouting
    assert store.connection_uri, f"store {store!r} has no connection_uri"
    if database is not None:
        return store.connection_uri.rsplit("/", 1)[0] + "/" + database
    else:
        return store.connection_uri


@asynccontextmanager
async def pg_cursor(connection_uri: str, autocommit: bool = False):
    pool = await get_pg_connection_pool(connection_uri)
    async with pool.connection() as conn:
        if conn.autocommit != autocommit:
            await conn.set_autocommit(autocommit)
        async with conn.cursor() as cur:
            yield cur


class AsyncPostgresConnection:
    """A Postgres connection to a Store. Wraps an underlying psycopg connection."""

    __slots__ = (
        "_conn",
        "_connection_uri",
        "_pool",
        "_reset_token",
        "_sanitized_connection_uri",
        "autocommit",
        "bench",
        "database",
        "id",
        "store",
    )

    _connection_id: ClassVar[int] = 0

    def __init__(
        self, store: Store, bench: Bench, database: str | None = None, autocommit: bool = False
    ):
        self.id = self._connection_id
        AsyncPostgresConnection._connection_id += 1
        self.store = store
        self.bench = bench
        self.database = database
        self.autocommit = autocommit
        self._pool: AsyncConnectionPool | None = None
        self._conn: psycopg.AsyncConnection | None = None
        self._connection_uri: str = get_pg_connection_uri(self.store, database=self.database)
        self._sanitized_connection_uri = sanitize_connection_uri(self._connection_uri)

    async def open(self) -> psycopg.AsyncCursor:
        self._pool = await get_pg_connection_pool(self._connection_uri)
        trace.get_current_span().set_attribute("pg_connection_uri", self._sanitized_connection_uri)
        log = logger.bind(id=self.id, store=self.store, pool=self._pool)
        try:
            self._conn = await self._pool.getconn()
            log.trace("pg.pool.acquire")
        except psycopg_pool.PoolTimeout as e:
            log.error("pg.pool.timeout", exc_info=e)
            raise
        if self._conn.autocommit != self.autocommit:
            await self._conn.set_autocommit(self.autocommit)
        return self._conn.cursor()

    async def close(self) -> None:
        if self._pool is not None and self._conn is not None:
            await self._pool.putconn(self._conn)
            logger.trace("pg.pool.release", id=self.id, store=self.store, pool=self._pool)
            self._conn = None
            # close pool if no longer in use
            if len(self._pool._pool) >= self._pool._nconns:  # ._pool are available conns
                await close_pg_connection_pool(self._connection_uri)

    async def __aenter__(self) -> psycopg.AsyncCursor:
        return await self.open()

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        await self.close()


def pg_store_connection(
    store: Store, bench: Bench | None = None, autocommit: bool = False, database: str | None = None
):
    return AsyncPostgresConnection(
        store, bench or store.bench, database=database, autocommit=autocommit
    )
