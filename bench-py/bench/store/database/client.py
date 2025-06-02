from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager

import asyncpg
import asyncpg.transaction

from bench.language import DatabaseInfo

# NOTE: we never expire/remove Pools since we assume only a few connections
_pool_by_url: dict[str, asyncpg.Pool] = {}


@asynccontextmanager
async def pg_connection(database: DatabaseInfo) -> AsyncGenerator[asyncpg.Connection, None]:
    """
    Context manager for an asyncpg.Connection.
    """

    assert database.sql_url, f"no sql_url for {database!r}"

    pool = _pool_by_url.get(database.sql_url)
    if pool is None:
        pool = await asyncpg.create_pool(database.sql_url)
        _pool_by_url[database.sql_url] = pool

    async with pool.acquire() as conn:
        yield conn


@asynccontextmanager
async def pg_transaction(
    database: DatabaseInfo,
) -> AsyncGenerator[tuple[asyncpg.Connection, asyncpg.transaction.Transaction], None]:
    """
    Context manager for an asyncpg.Transaction.
    """

    async with pg_connection(database) as conn:
        tx = conn.transaction()
        await tx.start()
        yield conn, tx
