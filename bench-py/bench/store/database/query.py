import asyncpg

from bench.language import Query, QueryResult

from .core import DatabaseContext


async def execute_query(
    conn: asyncpg.Connection, ctx: DatabaseContext, query: Query
) -> QueryResult:
    """Fetch the Query from the database."""
    raise NotImplementedError
