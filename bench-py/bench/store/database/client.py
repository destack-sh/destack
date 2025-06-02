from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager

import asyncpg
import asyncpg.transaction

from bench.language import DatabaseInfo


@asynccontextmanager
async def pg_connection(database: DatabaseInfo) -> AsyncGenerator[asyncpg.Connection, None]:
    """
    Context manager for an asyncpg.Connection.
    """

    # nocheckin: use asyncpg pool

    assert database.sql_url, f"no sql_url for {database!r}"
    conn = await asyncpg.connect(database.sql_url)
    try:
        yield conn
    finally:
        await conn.close()


@asynccontextmanager
async def pg_transaction(
    database: DatabaseInfo,
) -> AsyncGenerator[tuple[asyncpg.Connection, asyncpg.transaction.Transaction], None]:
    """
    Context manager for an asyncpg.Transaction.
    """

    assert database.sql_url, f"no sql_url for {database!r}"
    conn = await asyncpg.connect(database.sql_url)
    tx = conn.transaction()
    await tx.start()
    try:
        yield conn, tx
    except Exception:
        await tx.rollback()
        raise
    finally:
        await conn.close()
