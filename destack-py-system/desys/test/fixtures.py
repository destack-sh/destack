import re
from collections.abc import AsyncGenerator
from contextlib import contextmanager
from urllib.parse import urlparse

import grpclib
import pytest
import pytest_asyncio
import structlog
from opentelemetry import trace

from destack.test.conftest import _setup_test_env

# ruff: noqa: E402
# NOTE: must run setup before importing from destack
_setup_test_env()

from destack.language import (
    ACTIVE_SESSION,
    REGION,
    DatabaseInfo,
    DatabaseType,
    Region,
    Session,
    Space,
    SpaceStatus,
    StoreKey,
    Tenancy,
)
from destack.store import MemoryEntityStore, MemoryStore
from destack.utils.env import get_from_env
from desys.store.postgres import (
    POSTGRES_BUILTIN_TABLE_PREFIX,
    PostgresEntityStore,
    PostgresSchema,
    apply_migration_ops,
    close_postgres_pool,
    generate_migration_ops,
    get_builtin_schema,
    introspect_schema,
    postgres_connection,
)

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

pytestmark = pytest.mark.asyncio(loop_scope="session")


def get_global_database_from_env() -> DatabaseInfo:
    """Get the default global database configured in the environment"""
    sql_url = get_from_env("GLOBAL_DATABASE_URL", description="Global database URL")
    return DatabaseInfo(
        type=DatabaseType.POSTGRES,
        connection_url=sql_url,
        region=Region.ZURICH,
        external_name="destack-global",
    )


def get_database(name: str) -> DatabaseInfo:
    """Creates a global database for testing.."""
    pg = get_from_env("GLOBAL_DATABASE_URL", description="Global Postgres connection string")
    pg_url_parsed = urlparse(pg)
    connection_url = pg_url_parsed._replace(path=f"/{name}").geturl()
    database = DatabaseInfo(
        type=DatabaseType.POSTGRES,
        region=REGION,
        galaxy_name="test-0",
        external_name=name,
        tenancy=Tenancy.DEDICATED,
        connection_url=connection_url,
    )
    return database


async def _create_blank_test_db(database: DatabaseInfo):
    """Creates a blank postgres database"""
    await close_postgres_pool(database)
    async with postgres_connection(get_global_database_from_env()) as conn:
        await conn.execute(f'DROP DATABASE IF EXISTS "{database.external_name}"')
        await conn.execute(f'CREATE DATABASE "{database.external_name}"')


@tracer.start_as_current_span("test.create_test_db")
async def _create_test_db(database: DatabaseInfo, schema: PostgresSchema):
    """Creates a postgres DB with one of our schemas"""
    await _create_blank_test_db(database)
    async with postgres_connection(database) as conn:
        old_schema = await introspect_schema(
            conn,
            include_table_prefixes=(POSTGRES_BUILTIN_TABLE_PREFIX,),
            exclude_table_prefixes=(),
        )
        migration_ops = generate_migration_ops(old_schema=old_schema, new_schema=schema)
        await apply_migration_ops(conn, migration_ops)
    logger.trace("test.create_test_db", database=database, span="current")


@tracer.start_as_current_span("test.delete_test_db")
async def _delete_test_db(database: DatabaseInfo):
    """Deletes a postgres DB with one of our schemas"""
    await close_postgres_pool(database)
    async with postgres_connection(get_global_database_from_env()) as conn:
        # terminate all connections to the database before dropping it
        await conn.execute(f"""
            SELECT pg_terminate_backend(pid)
            FROM pg_stat_activity
            WHERE datname = '{database.external_name}' AND pid <> pg_backend_pid()
        """)
        await conn.execute(f'DROP DATABASE IF EXISTS "{database.external_name}"')
    logger.trace("test.delete_test_db", database=database, span="current")


def _clean_name(name: str) -> str:
    """Turn a name into a valid Python identifier"""
    return re.sub(r"[^a-zA-Z0-9]", "_", name)


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def postgres_database(
    request: pytest.FixtureRequest,
) -> AsyncGenerator[DatabaseInfo, None]:
    """Gets the per test function omni Database"""
    omni_schema = get_builtin_schema(StoreKey.ENTITY_PRIMARY)
    database = get_database(f"test-{_clean_name(request.node.name)[:32]}-omni")
    await _create_test_db(database, omni_schema)
    try:
        yield database
    finally:
        await _delete_test_db(database)


@pytest.fixture
def postgres_store(postgres_database: DatabaseInfo) -> PostgresEntityStore:
    return PostgresEntityStore(database=postgres_database, keys=(StoreKey.ENTITY_PRIMARY,))


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def postgres_session(
    postgres_store: PostgresEntityStore,
) -> AsyncGenerator[Session, None]:
    session = Session(store=postgres_store)
    await session.open()
    ACTIVE_SESSION.set(session)
    yield session
    await session.close()


@pytest.fixture
def memory_store() -> MemoryStore:
    return MemoryStore(keys=tuple(StoreKey))


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def memory_session(memory_store: MemoryEntityStore) -> AsyncGenerator[Session, None]:
    session = Session(store=memory_store)
    await session.open()
    yield session
    await session.close()


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def session(memory_store: MemoryEntityStore):
    """Default Session is in-memory."""
    session = Session(store=memory_store)
    await session.open()
    yield session
    await session.close()


@pytest.fixture
def space(session: Session):
    space = Space(
        name="Test",
        slug="test",
        status=SpaceStatus.ACTIVE,
        region=REGION,
    )
    session.create(space)
    with space.active():
        yield space


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"
