import pytest

from bench.sql.client import async_pg_cursor


@pytest.fixture(autouse=True, scope="module")
async def make_test_db():
    async with async_pg_cursor(autocommit=True) as cur:
        await cur.execute("DROP DATABASE IF EXISTS test")
        await cur.execute("CREATE DATABASE test")


async def test_current_migrate_from_scratch():
    pass


async def test_blank_migrate_from_scratch():
    pass
