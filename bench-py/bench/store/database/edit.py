import asyncpg

from bench.language import Change, ChangeResult


async def database_apply_changes(conn: asyncpg.Connection, change: Change) -> ChangeResult:
    raise NotImplementedError
