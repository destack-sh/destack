from contextlib import asynccontextmanager

import asyncpg

from bench.language import Database


@asynccontextmanager
async def pg_connection(database: Database, *, autocommit: bool = False):
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
