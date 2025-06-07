from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager

import asyncpg
import asyncpg.transaction

from bench.language import DatabaseBase

# NOTE: we never expire/remove Pools since we assume only a few connections
_pool_by_url: dict[str, asyncpg.Pool] = {}


@asynccontextmanager
async def pg_connection(database: DatabaseBase) -> AsyncGenerator[asyncpg.Connection, None]:
    """
    Context manager for an asyncpg.Connection.
    """

    assert database.connection_url, f"no sql_url for {database!r}"

    pool = _pool_by_url.get(database.connection_url)
    if pool is None:
        pool = await asyncpg.create_pool(database.connection_url)
        _pool_by_url[database.connection_url] = pool

    async with pool.acquire() as conn:
        yield conn


@asynccontextmanager
async def pg_transaction(
    database: DatabaseBase,
) -> AsyncGenerator[tuple[asyncpg.Connection, asyncpg.transaction.Transaction], None]:
    """
    Context manager for an asyncpg.Transaction.
    """

    async with pg_connection(database) as conn:
        tx = conn.transaction()
        await tx.start()
        yield conn, tx
