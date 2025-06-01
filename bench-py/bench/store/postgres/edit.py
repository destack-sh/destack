import asyncpg

from bench.language import Change, ChangeResult


async def apply_postgres_changes(conn: asyncpg.Connection, change: Change) -> ChangeResult:
    raise NotImplementedError
