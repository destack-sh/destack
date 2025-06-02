import asyncpg

from bench.language import Query, QueryResult


async def execute_query(conn: asyncpg.Connection, query: Query) -> QueryResult:
    """Fetch the Query from the database."""
    raise NotImplementedError
