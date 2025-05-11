import pytest

from bench.language import Database
from bench.sql import pg_connection


@pytest.fixture
async def blank_cur(blank_store: Database):
    async with pg_connection(blank_store, owner=blank_store, autocommit=True) as conn:
        yield conn.cursor
