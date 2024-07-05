import pytest

from bench.language import Store
from bench.sql.client import pg_store_connection


@pytest.fixture()
async def blank_cur(blank_store: Store):
    async with pg_store_connection(blank_store, autocommit=True) as cur:
        yield cur
