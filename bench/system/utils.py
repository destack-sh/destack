from contextlib import asynccontextmanager
from typing import Any, AsyncContextManager, Optional

import boto3
import psycopg
import structlog

from bench.language import Bench, Session, Store, StoreEngineType, StoreKind
from bench.language.const import USER_NODE_TYPES, NodeType, VERSION
from bench.language.query import PostgresEngine
from bench.language.resource import StoreCredential, StoreCredentialType, Region
from bench.sql.client import _PgStoreConnection
from bench.utils.func import bytetuple
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)

GLOBAL_PG_HOST = get_from_env("GLOBAL_PG_HOST", default=None)
GLOBAL_PG_NAME = get_from_env("GLOBAL_PG_NAME", default=None)
GLOBAL_PG_PORT = get_from_env("GLOBAL_PG_PORT", default=5432, type_cast=int)
GLOBAL_PG_USERNAME = get_from_env("GLOBAL_PG_USERNAME", default=None)
GLOBAL_PG_PASSWORD = get_from_env("GLOBAL_PG_PASSWORD", default=None)

USER_PG_HOST = get_from_env("USER_PG_HOST", optional=True)
USER_PG_NAME = get_from_env("USER_PG_NAME", optional=True)
USER_PG_PORT = get_from_env("USER_PG_PORT", default=5432, type_cast=int, optional=True)
USER_PG_USERNAME = get_from_env("USER_PG_USERNAME", optional=True)
USER_PG_PASSWORD = get_from_env("USER_PG_PASSWORD", optional=True)

GLOBAL_PG_CRYPTO_KEY = get_from_env("GLOBAL_PG_CRYPTO_KEY", default=None)
SYSTEM_BENCH = Bench(
    name="System", slug="system", region=Region.GLOBAL, encryption_key=GLOBAL_PG_CRYPTO_KEY
)
GLOBAL_STORE = Store(
    parent=SYSTEM_BENCH,
    name="Global",
    kind=StoreKind.RELATIONAL,
    engine=StoreEngineType.POSTGRES,
    version=VERSION,
    host=GLOBAL_PG_HOST,
    database=GLOBAL_PG_NAME,
    main_credential=StoreCredential(
        type=StoreCredentialType.ROOT,
        username=GLOBAL_PG_USERNAME,
        password=GLOBAL_PG_PASSWORD,
    ),
)
GLOBAL_POSTGRES_ENGINE = PostgresEngine(
    GLOBAL_STORE, scope=None, node_types=bytetuple(USER_NODE_TYPES.tuple + (NodeType.BENCH,))
)


@asynccontextmanager
async def global_pg_cursor(
    autocommit: bool = False,
) -> AsyncContextManager[psycopg.AsyncCursor[dict[str, Any]]]:
    async with _PgStoreConnection(GLOBAL_STORE, autocommit=autocommit) as cur:
        yield cur


@asynccontextmanager
async def global_session() -> AsyncContextManager[Session]:
    async with Session(
        parent=None, _engines=(GLOBAL_POSTGRES_ENGINE,), _fallback_engine=GLOBAL_POSTGRES_ENGINE
    ) as session:
        yield session


_s3_client: Optional["boto3.client"] = None


def get_s3_client() -> "boto3.client":
    global _s3_client
    if _s3_client is None:
        _s3_client = boto3.client(
            "s3",
            endpoint_url=get_from_env("AWS_ENDPOINT_URL"),
        )
    return _s3_client
