import contextvars
import re
from contextlib import asynccontextmanager

import psycopg
import psycopg_pool
import structlog
from psycopg.rows import dict_row
from psycopg_pool import AsyncConnectionPool

from bench.language import Bench, Store
from bench.language.const import SUB_PACKAGE_NODE_TYPES, NodeType, active_bench
from bench.sql.core import Table, TableObject
from bench.utils.utils import get_from_env

# TODO :Robustness: figure out how to fix the psycopg pool warning
#  (what we're doing should be fine according to docs and the warning)
AsyncConnectionPool._warn_open_async = lambda *args, **kwargs: None  # type: ignore

GLOBAL_PG_CRYPTO_KEY = get_from_env("GLOBAL_PG_CRYPTO_KEY", default=None)
PG_CONNECT_TIMEOUT = get_from_env("PG_CONNECT_TIMEOUT", typ=int, default=10)
PG_RECONNECT_TIMEOUT = get_from_env("PG_RECONNECT_TIMEOUT", typ=int, default=20)

logger = structlog.get_logger(__name__)
_connection_pools: dict[str, AsyncConnectionPool] = {}
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
            timeout=PG_CONNECT_TIMEOUT,
            reconnect_timeout=PG_RECONNECT_TIMEOUT,
            connection_class=psycopg.AsyncConnection,
            kwargs={"row_factory": dict_row},
            name=f"{match['username']}@{match['host']}/{match['database']}",
        )
        await pool.open()
        _connection_pools[connection_str] = pool
    return _connection_pools[connection_str]


_CONNECTION_STR_REGEX = re.compile(
    r"postgresql://(?P<username>[^:]+)(:(?P<password>[^@]+))?@(?P<host>[^/]+)/(?P<database>.+)"
)


def get_pg_connection_str(store: Store, database: str | None = None) -> str:
    # TODO :Security :Scalability: route store clients/hosts better :StoreRouting
    assert store.connection_uri, f"store {store!r} has no connection_uri"
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
    __slots__ = ("store", "bench", "database", "autocommit", "_reset_token", "_conn", "_pool")

    def __init__(
        self, store: Store, bench: Bench, database: str | None = None, autocommit: bool = False
    ):
        self.store = store
        self.bench = bench
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
        return self._conn.cursor()

    async def close(self) -> None:
        if self._pool is not None and self._conn is not None:
            await self._pool.putconn(self._conn)

    async def __aenter__(self) -> psycopg.AsyncCursor:
        return await self.open()

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        await self.close()


@asynccontextmanager
async def pg_cursor_to_store(store: Store, bench: Bench | None = None, autocommit: bool = False):
    async with _PgStoreConnection(store, bench or store.bench, autocommit=autocommit) as cur:
        yield cur


async def get_pg_store_connection(
    store: Store, bench: Bench | None = None, autocommit: bool = False
) -> _PgStoreConnection:
    return _PgStoreConnection(store, bench or store.bench, autocommit=autocommit)
