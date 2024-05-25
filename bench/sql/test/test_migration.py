import random
import string

import psycopg
import pytest
import structlog

from bench.language.bench import Region
from bench.sql.client import get_pg_connection_uri, pg_cursor
from bench.sql.core import ObjectKind
from bench.sql.engine import GLOBAL_SCHEMA, LOCAL_SCHEMA
from bench.sql.migration import (
    generate_sql_migration_ops,
    introspect_sql_schema,
    read_migrations_from_fs,
    sql_migrate,
)
from bench.system.core import GLOBAL_STORE, global_pg_cursor
from bench.system.neon import NeonApiRemote
from bench.utils.env import ENVIRONMENT
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)


@pytest.fixture()
async def blank_global_test_db(request: pytest.FixtureRequest):
    db_name = f"migrate_test_{request.function.__name__}"
    async with global_pg_cursor(autocommit=True) as cur:
        await cur.execute(f"DROP DATABASE IF EXISTS {db_name}")  # type: ignore
        await cur.execute(f"CREATE DATABASE {db_name}")  # type: ignore
        yield db_name


@pytest.fixture()
async def blank_global_test_cur(blank_global_test_db: str):
    connection_uri = get_pg_connection_uri(GLOBAL_STORE, blank_global_test_db)
    async with pg_cursor(connection_uri) as cur:
        yield cur
    await cur.connection.close()


@pytest.fixture()
async def blank_local_test_db(request: pytest.FixtureRequest):
    # use actual Neon API (remote) because we don't have all the extensions locally
    neon_client = NeonApiRemote(
        url=get_from_env("NEON_BASE_URL"), api_key=get_from_env("NEON_API_KEY")
    )
    random_postfix = "".join(random.choices(string.ascii_lowercase, k=8))
    create_rep = await neon_client.create_project(
        name=f"{ENVIRONMENT}-{request.function.__name__}-{random_postfix}",
        region=Region.EUROPE_CENTRAL,
        pg_version=16,
    )
    yield create_rep.connection_uri
    await neon_client.delete_project(project_id=create_rep.project_id)


@pytest.fixture()
async def blank_local_test_cur(blank_local_test_db: str):
    async with pg_cursor(blank_local_test_db) as cur:
        yield cur
    await cur.connection.close()


async def _do_test_stored_migrations(blank_test_cur: psycopg.AsyncCursor, *, is_global: bool):
    # run all stored migrations
    stored_migrations = read_migrations_from_fs()
    logger.info("migrate", migrations=stored_migrations)
    await sql_migrate(blank_test_cur, target=stored_migrations[-1].id, is_global=is_global)

    # diff again (should be empty now)
    current_schema = await introspect_sql_schema(blank_test_cur)
    new_schema = GLOBAL_SCHEMA if is_global else LOCAL_SCHEMA
    current_ops = generate_sql_migration_ops(current_schema, new_schema)
    current_ops = [op for op in current_ops if op.object_kind != ObjectKind.EXTENSION]
    assert not current_ops, f"out of sync migrations, got {len(current_ops)} ops"


async def test_stored_migrations_global(blank_global_test_cur: psycopg.AsyncCursor):
    """Existing global migrations against a blank database."""
    await _do_test_stored_migrations(blank_global_test_cur, is_global=True)


async def test_stored_migrations_local(blank_local_test_cur: psycopg.AsyncCursor):
    """Existing local migrations against a blank database."""
    await _do_test_stored_migrations(blank_local_test_cur, is_global=False)
