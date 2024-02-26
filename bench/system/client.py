from contextlib import asynccontextmanager
from typing import Any, AsyncContextManager

from opensearchpy import AsyncOpenSearch
import psycopg
import structlog

from bench.language import Bench, Session, Store, StoreEngineType, StoreKind
from bench.language.const import USER_NODE_TYPES, NodeType, VERSION
from bench.language.query import PostgresEngine
from bench.language.resource import ResourceCredential, Region
from bench.opensearch.client import os_client_to_store
from bench.sql.client import _PgStoreConnection
from bench.utils.func import bytetuple
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)

GLOBAL_PG_HOST = get_from_env("GLOBAL_PG_HOST", default=None)
GLOBAL_PG_NAME = get_from_env("GLOBAL_PG_NAME", default=None)
GLOBAL_PG_USERNAME = get_from_env("GLOBAL_PG_USERNAME", default=None)
GLOBAL_PG_PASSWORD = get_from_env("GLOBAL_PG_PASSWORD", default=None)

USER_PG_HOST = get_from_env("USER_PG_HOST", optional=True)
USER_PG_USERNAME = get_from_env("USER_PG_USERNAME", optional=True)
USER_PG_PASSWORD = get_from_env("USER_PG_PASSWORD", optional=True)

USER_OS_HOST = get_from_env("USER_OS_HOST")
USER_OS_USERNAME = get_from_env("USER_OS_USERNAME")
USER_OS_PASSWORD = get_from_env("USER_OS_PASSWORD")

GLOBAL_PG_CRYPTO_KEY = get_from_env("GLOBAL_PG_CRYPTO_KEY", default=None)
SYSTEM_BENCH_STUB = Bench(
    name="System (Stub)", slug="system", region=Region.GLOBAL, encryption_key=GLOBAL_PG_CRYPTO_KEY
)

GLOBAL_STORE = Store(
    parent=SYSTEM_BENCH_STUB,
    name="Global Store",
    kind=StoreKind.RELATIONAL,
    engine=StoreEngineType.POSTGRES,
    version=VERSION,
    host=GLOBAL_PG_HOST,
    database=GLOBAL_PG_NAME,
    main_credential=ResourceCredential(username=GLOBAL_PG_USERNAME, password=GLOBAL_PG_PASSWORD),
)
GLOBAL_POSTGRES_ENGINE = PostgresEngine(
    GLOBAL_STORE, scope=None, node_types=bytetuple(USER_NODE_TYPES.tuple + (NodeType.BENCH,))
)
# TODO :Security :Scalability: route user Store hosts better :StoreRouting
USER_STORE = Store(
    parent=SYSTEM_BENCH_STUB,
    name="User Store",
    kind=StoreKind.RELATIONAL,
    engine=StoreEngineType.POSTGRES,
    version=VERSION,
    host=USER_PG_HOST,
    database="postgres",  # technically there is no single user's database, so connect to default
    main_credential=ResourceCredential(username=USER_PG_USERNAME, password=USER_PG_PASSWORD),
)

USER_SEARCH = Store(
    parent=SYSTEM_BENCH_STUB,
    name="Global Store",
    kind=StoreKind.SEARCH,
    engine=StoreEngineType.OPENSEARCH,
    version=VERSION,
    host=USER_OS_HOST,
    main_credential=ResourceCredential(username=USER_OS_USERNAME, password=USER_OS_PASSWORD),
)


@asynccontextmanager
async def global_pg_cursor(
    autocommit: bool = False,
) -> AsyncContextManager[psycopg.AsyncCursor[dict[str, Any]]]:
    async with _PgStoreConnection(GLOBAL_STORE, autocommit=autocommit) as cur:
        yield cur


@asynccontextmanager
async def user_pg_cursor(
    database: str = None,
    autocommit: bool = False,
) -> AsyncContextManager[psycopg.AsyncCursor[dict[str, Any]]]:
    async with _PgStoreConnection(USER_STORE, database=database, autocommit=autocommit) as cur:
        yield cur


@asynccontextmanager
async def user_os_client() -> AsyncContextManager[AsyncOpenSearch]:
    async with os_client_to_store(USER_SEARCH) as client:
        yield client


@asynccontextmanager
async def global_session() -> AsyncContextManager[Session]:
    async with Session(
        parent=None, _engines=(GLOBAL_POSTGRES_ENGINE,), _fallback_engine=GLOBAL_POSTGRES_ENGINE
    ) as session:
        yield session
