import re
from collections.abc import AsyncGenerator
from contextlib import contextmanager
from urllib.parse import urlparse

import grpclib
import pytest
import structlog
from opentelemetry import trace

from destack.store import MemoryStore
from destack.test.conftest import _setup_test_env

# ruff: noqa: E402
# NOTE: must run setup before importing from destack
_setup_test_env()


from destack.language import (
    ACTIVE_SESSION,
    REGION,
    DatabaseInfo,
    DatabaseType,
    Session,
    StoreType,
    Tenancy,
)
from destack.utils.env import get_from_env
from desys.sharding import get_global_database_from_env
from desys.store.postgres import (
    DESTACK_BUILTIN_TABLE_PREFIX,
    DESTACK_CUSTOM_TABLE_PREFIX,
    PostgresSchema,
    PostgresStore,
    apply_migration_ops,
    generate_migration_ops,
    get_builtin_schema,
    introspect_schema,
    pg_connection,
)

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


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


async def create_blank_test_db(database: DatabaseInfo):
    """Creates a blank postgres database"""
    async with pg_connection(get_global_database_from_env()) as conn:
        await conn.execute(f'DROP DATABASE IF EXISTS "{database.external_name}"')
        await conn.execute(f'CREATE DATABASE "{database.external_name}"')


async def create_test_db(database: DatabaseInfo, schema: PostgresSchema):
    """Creates a postgres DB with one of our schemas"""
    await create_blank_test_db(database)
    async with pg_connection(database) as conn:
        old_schema = await introspect_schema(
            conn,
            include_table_prefixes=(DESTACK_BUILTIN_TABLE_PREFIX,),
            exclude_table_prefixes=(DESTACK_CUSTOM_TABLE_PREFIX,),
        )
        migration_ops = generate_migration_ops(old_schema=old_schema, new_schema=schema)
        await apply_migration_ops(conn, migration_ops)


async def delete_test_db(database: DatabaseInfo):
    """Deletes a postgres DB with one of our schemas"""
    async with pg_connection(get_global_database_from_env()) as conn:
        # terminate all connections to the database before dropping it
        await conn.execute(f"""
            SELECT pg_terminate_backend(pid)
            FROM pg_stat_activity
            WHERE datname = '{database.external_name}' AND pid <> pg_backend_pid()
        """)
        await conn.execute(f'DROP DATABASE IF EXISTS "{database.external_name}"')


def _clean_name(name: str) -> str:
    """Turn a name into a valid Python identifier"""
    return re.sub(r"[^a-zA-Z0-9]", "_", name)


@pytest.fixture
async def global_postgres_database(
    request: pytest.FixtureRequest,
) -> AsyncGenerator[DatabaseInfo, None]:
    """Gets the per test function global Database"""

    database = get_database(f"test-{_clean_name(request.node.name)[:32]}-global")
    schema = get_builtin_schema(StoreType.GLOBAL_ENTITY)
    await create_test_db(database, schema)
    try:
        yield database
    finally:
        await delete_test_db(database)


@pytest.fixture
async def spatial_postgres_database(
    request: pytest.FixtureRequest,
) -> AsyncGenerator[DatabaseInfo, None]:
    """Gets the per test function spatial Database"""

    database = get_database(f"test-{_clean_name(request.node.name)[:32]}-spatial")
    schema = get_builtin_schema(StoreType.SPATIAL_ENTITY)
    await create_test_db(database, schema)
    try:
        yield database
    finally:
        await delete_test_db(database)


@pytest.fixture
async def omni_postgres_database(
    request: pytest.FixtureRequest,
) -> AsyncGenerator[DatabaseInfo, None]:
    """Gets the per test function omni Database"""
    omni_schema = get_builtin_schema(*StoreType)
    database = get_database(f"test-{_clean_name(request.node.name)[:32]}-omni")
    await create_test_db(database, omni_schema)
    try:
        yield database
    finally:
        await delete_test_db(database)


# NOTE: we manually set session sync context since it's not propagated across pytest tasks (see above)
# https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-862817549 :PytestAsyncContext


@pytest.fixture
def postgres_store(omni_postgres_database: DatabaseInfo) -> PostgresStore:
    return PostgresStore(database=omni_postgres_database, types=tuple(StoreType))


@pytest.fixture
async def postgres_session_async(postgres_store: PostgresStore) -> AsyncGenerator[Session, None]:
    session = Session(store=postgres_store)
    await session.open()
    yield session
    await session.close()


@pytest.fixture
def postgres_session(postgres_session_async: Session):
    token = ACTIVE_SESSION.set(postgres_session_async)
    yield postgres_session_async
    ACTIVE_SESSION.reset(token)


@pytest.fixture
def memory_store() -> MemoryStore:
    return MemoryStore(types=tuple(StoreType))


@pytest.fixture  # :PytestAsyncContext
async def memory_session_async(memory_store: MemoryStore) -> AsyncGenerator[Session, None]:
    session = Session(store=memory_store)
    await session.open()
    yield session
    await session.close()


@pytest.fixture
def memory_session(memory_session_async: Session):
    token = ACTIVE_SESSION.set(memory_session_async)
    yield memory_session_async
    ACTIVE_SESSION.reset(token)


@pytest.fixture
def session(memory_session_async: Session):
    """Default Session is in-memory."""
    token = ACTIVE_SESSION.set(memory_session_async)
    yield memory_session_async
    ACTIVE_SESSION.reset(token)


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"
