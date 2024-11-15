import pytest

from bench.language import Store
from bench.sql.client import pg_connection


@pytest.fixture
async def blank_cur(blank_store: Store):
    async with pg_connection(blank_store, autocommit=True) as conn:
        yield conn.cursor
