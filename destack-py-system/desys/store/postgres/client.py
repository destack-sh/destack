from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager

import asyncpg
import asyncpg.transaction

from destack.language import Database, DatabaseInfo

# NOTE: we never expire/remove Pools since we assume only a few connections
_pool_by_url: dict[str, asyncpg.Pool] = {}


async def _get_pool(database: DatabaseInfo | Database) -> asyncpg.Pool:
    """Get a pool for a database."""

    assert database.connection_url, f"no connection_url for {database!r}"

    pool = _pool_by_url.get(database.connection_url)
    if pool is None:
        pool = await asyncpg.create_pool(
            database.connection_url,
            server_settings={"timezone": "UTC"},  # keep DB session in UTC
        )
        _pool_by_url[database.connection_url] = pool

    return pool


@asynccontextmanager
async def pg_connection(
    database: DatabaseInfo | Database,
) -> AsyncGenerator[asyncpg.Connection, None]:
    """
    Context manager for an asyncpg.Connection.
    """

    assert database.connection_url, f"no connection_url for {database!r}"

    pool = await _get_pool(database)
    async with pool.acquire() as conn:
        yield conn


@asynccontextmanager
async def pg_transaction(
    database: DatabaseInfo | Database,
) -> AsyncGenerator[tuple[asyncpg.Connection, asyncpg.transaction.Transaction], None]:
    """
    Context manager for an asyncpg.Transaction.
    """

    async with pg_connection(database) as conn:
        tx = conn.transaction()
        await tx.start()
        yield conn, tx
