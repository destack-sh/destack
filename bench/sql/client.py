import asyncio
from contextlib import asynccontextmanager
from typing import Any, ClassVar, cast

import psycopg
import structlog
from opentelemetry import trace
from psycopg.rows import dict_row
from psycopg.sql import SQL
from psycopg_pool import AsyncConnectionPool, PoolTimeout

from bench.language import Store
from bench.utils.func import sanitize_connection_uri
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# NOTE :Robustness: figure out how to fix the psycopg pool warning
#  (what we're doing should be fine according to docs _and_ according to the warning)
AsyncConnectionPool._warn_open_async = lambda *args, **kwargs: None  # type: ignore

GLOBAL_PG = get_from_env("GLOBAL_PG", description="Global Postgres connection string")
GLOBAL_PR_URL, GLOBAL_PG_CRYPTO_KEY = GLOBAL_PG.split("|", maxsplit=1)
PG_MAX_IDLE_TIMEOUT = get_from_env(
    "PG_MAX_IDLE_TIMEOUT",
    typ=int,
    default=60 * 60,
    description="Postgres max idle timeout in seconds",
)
PG_CONNECT_TIMEOUT = get_from_env(
    "PG_CONNECT_TIMEOUT", typ=int, default=5, description="Postgres connection timeout in seconds"
)
PG_RECONNECT_TIMEOUT = get_from_env(
    "PG_RECONNECT_TIMEOUT", typ=int, default=10, description="Postgres reconnect timeout in seconds"
)
PG_MIN_POOL_SIZE = get_from_env(
    "PG_MIN_POOL_SIZE", typ=int, default=4, description="Postgres min pool size"
)
PG_MAX_POOL_SIZE = get_from_env(
    "PG_MAX_POOL_SIZE", typ=int, default=8, description="Postgres max pool size"
)
PG_POOL_AUTOCLOSE = get_from_env(
    "PG_POOL_AUTOCLOSE",
    typ=bool,
    default=False,
    description="Whether to automatically close unused postgres pools",
)

# NOTE :Cleanup: we should probably gc unused pools after some time
_pools_by_store: dict[Store, "PostgresConnectionPool"] = {}


def get_pg_pool(store: Store) -> "PostgresConnectionPool":
    """Gets the connection pool for the given store."""
    if store not in _pools_by_store:
        _pools_by_store[store] = PostgresConnectionPool(
            store, min_size=PG_MIN_POOL_SIZE, max_size=PG_MAX_POOL_SIZE
        )
    return _pools_by_store[store]


def get_pg_pool_by_external_name(external_name: str) -> "PostgresConnectionPool | None":
    """Gets the connection pool for the given external name."""
    for store in _pools_by_store:
        if store.external_name == external_name:
            return _pools_by_store[store]
    return None


@asynccontextmanager
async def pg_connection(store: Store, *, owner: Any | None = None, autocommit: bool = False):
    """Opens a connection to the given store."""
    pool = get_pg_pool(store)
    if not pool.is_open:
        await pool.open()
    connection = await pool.acquire(
        owner=owner if owner is not None else store, autocommit=autocommit
    )
    try:
        yield connection
    finally:
        await pool.release(connection)


class PostgresConnectionPool:
    """
    A connection pool to a Store. Wraps an underlying psycopg connection pool.
    There should only ever be one AsyncPostgresPool per Store / connection URI at a time.
    """

    _pool_id: ClassVar[int] = 0

    def __init__(self, store: Store, min_size: int, max_size: int):
        self.store = store
        self.min_size = min_size
        self.max_size = max_size
        self.id = self._pool_id
        PostgresConnectionPool._pool_id += 1
        self._pool: AsyncConnectionPool | None = None
        self._pool_lock = asyncio.Lock()
        self._connections: list[PostgresConnection] = []
        assert store.connection_uri, f"store {store!r} has no connection_uri"
        self._connection_uri = store.connection_uri
        self._sanitized_connection_uri = sanitize_connection_uri(self._connection_uri)

    def __str__(self):
        return f"id={self.id}, uri={self._sanitized_connection_uri}, used={len(self._connections)}, pool={"<open>" if self._pool else '<closed>'}, store={self.store!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def is_open(self) -> bool:
        return self._pool is not None

    @property
    def is_used(self) -> bool:
        return len(self._connections) > 0

    async def _do_open(self):
        if self._pool is not None:
            return  # already open
        # open new pool
        self._pool = AsyncConnectionPool(
            self._connection_uri,
            min_size=self.min_size,
            max_size=self.max_size,
            max_idle=PG_MAX_IDLE_TIMEOUT,
            timeout=PG_CONNECT_TIMEOUT,
            reconnect_timeout=PG_RECONNECT_TIMEOUT,
            connection_class=psycopg.AsyncConnection,
            kwargs={"row_factory": dict_row},
            name=self._sanitized_connection_uri,
        )
        await self._pool.open()
        logger.trace("postgres.pool.open", pool=self, span="current")

    @tracer.start_as_current_span("postgres.pool.open")
    async def open(self):
        """Opens the connection pool (start allowing new connections)."""
        async with self._pool_lock:
            await self._do_open()

    async def _do_close(self):
        if self._pool is None:
            return  # already closed
        assert self._pool is not None, f"already closed: {self._pool!r}"
        await self._pool.close()
        # force close current connections
        for connection in self._connections:
            if connection._conn is not None:
                connection._conn.cancel()
                await connection._conn.close()
                await self._pool.putconn(connection._conn)
        self._connections.clear()
        self._pool = None
        logger.trace("postgres.pool.close", pool=self, span="current")

    @tracer.start_as_current_span("postgres.pool.close")
    async def close(self):
        """Closes the connection pool (stop allowing new connections)."""
        async with self._pool_lock:
            await self._do_close()

    @tracer.start_as_current_span("postgres.pool.acquire")
    async def acquire(self, owner: Any, autocommit: bool = False) -> "PostgresConnection":
        """Connects to the pool."""
        async with self._pool_lock:
            if not self.is_open:
                await self._do_open()
        assert self._pool is not None, f"pool not open: {self!r}"
        try:
            conn = await self._pool.getconn(PG_CONNECT_TIMEOUT)
        except PoolTimeout as e:
            logger.error(
                "postgres.pool.acquire.timeout",
                pool=self,
                error=e,
                connections=[c.id for c in self._connections],
                span="current",
            )
            raise
        connection = PostgresConnection(owner, self, conn)
        self._connections.append(connection)
        if autocommit != connection.autocommit:
            await conn.set_autocommit(autocommit)
        logger.trace("postgres.pool.acquire", pool=self, connection=connection, span="current")
        return connection

    @tracer.start_as_current_span("postgres.pool.release")
    async def release(self, connection: "PostgresConnection"):
        """Releases the connection back to the pool."""
        async with self._pool_lock:
            assert self._pool is not None, f"pool not open: {self!r}"
            assert connection.conn is not None, f"no connection for {connection!r}"
            await self._pool.putconn(connection.conn)
            self._connections.remove(connection)
            logger.trace("postgres.pool.release", pool=self, connection=connection, span="current")
            # auto close if no more connections
            if not self.is_used and PG_POOL_AUTOCLOSE:
                await self._do_close()

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()


class PostgresConnection:
    """A Postgres connection to a Store. Wraps an underlying psycopg connection."""

    _connection_id: ClassVar[int] = 0

    def __init__(
        self,
        owner: Any,
        pool: PostgresConnectionPool,
        conn: psycopg.AsyncConnection,
        autocommit: bool = False,
    ):
        self.id = self._connection_id
        self.owner = owner
        self.pool = pool
        PostgresConnection._connection_id += 1
        self._conn = conn
        self._cursor = conn.cursor()
        self.lock = asyncio.Lock()  # lock to acquire while using the connection
        self.autocommit = autocommit

    def __str__(self):
        return f"id={self.id}, pool={self.pool.id}, store={self.pool.store!r}, uri={self.pool._sanitized_connection_uri}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def conn(self) -> psycopg.AsyncConnection:
        assert self._conn is not None, f"no connection for {self!r}"
        return self._conn

    @property
    def cursor(self) -> psycopg.AsyncCursor:
        assert self._cursor is not None, f"no cursor for {self!r}"
        return self._cursor

    async def close(self):
        """Closes the connection (and release it back to the pool)."""
        assert self._conn is not None, f"no connection for {self!r}"
        await self.pool.release(self)
        self._conn = None
        self._cursor = None

    async def execute(self, query: str | SQL, *args):
        """Executes a query."""
        assert self._conn is not None, f"no connection for {self!r}"
        await self._conn.execute(cast(SQL, query), *args)

    async def commit(self):
        """Commits the current transaction."""
        assert self._conn is not None, f"no connection for {self!r}"
        await self._conn.commit()

    async def rollback(self):
        """Rolls back the current transaction."""
        assert self._conn is not None, f"no connection for {self!r}"
        await self._conn.rollback()
