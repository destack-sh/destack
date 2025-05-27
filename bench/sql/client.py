from contextlib import asynccontextmanager
from typing import AsyncGenerator

import asyncpg
import asyncpg.transaction

from bench.language import Database


@asynccontextmanager
async def pg_connection(database: Database) -> AsyncGenerator[asyncpg.Connection, None]:
    """
    Returns a context manager for a pg connection.
    """

    # nocheckin: use asyncpg pool

    assert database.sql_url, f"no sql_url for {database!r}"
    conn = await asyncpg.connect(database.sql_url)
    try:
        yield conn
    finally:
        await conn.close()


@asynccontextmanager
async def pg_tx(
    database: Database,
) -> AsyncGenerator[tuple[asyncpg.Connection, asyncpg.transaction.Transaction], None]:
    """
    Returns a context manager for a pg transaction.
    """

    assert database.sql_url, f"no sql_url for {database!r}"
    conn = await asyncpg.connect(database.sql_url)
    tx = await conn.transaction()
    try:
        yield conn, tx
    finally:
        await tx.rollback()
        await conn.close()
