from contextlib import asynccontextmanager

import structlog

from bench.language import Bench, Session, Store, StoreEngineType, StoreKind
from bench.language.const import GLOBAL_NODE_TYPES, VERSION
from bench.language.query import PostgresEngine
from bench.language.resource import Region
from bench.sql.client import _PgStoreConnection
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)

GLOBAL_PG_HOST = get_from_env("GLOBAL_PG_HOST", default=None)
GLOBAL_PG_NAME = get_from_env("GLOBAL_PG_NAME", default=None)
GLOBAL_PG_USERNAME = get_from_env("GLOBAL_PG_USERNAME", default=None)
GLOBAL_PG_PASSWORD = get_from_env("GLOBAL_PG_PASSWORD", default=None)

USER_PG_HOST = get_from_env("USER_PG_HOST", optional=True)
USER_PG_USERNAME = get_from_env("USER_PG_USERNAME", optional=True)
USER_PG_PASSWORD = get_from_env("USER_PG_PASSWORD", optional=True)

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
    external_name=GLOBAL_PG_NAME,
    connection_uri=f"postgresql://{GLOBAL_PG_USERNAME}:{GLOBAL_PG_PASSWORD}@{GLOBAL_PG_HOST}/{GLOBAL_PG_NAME}",
)
GLOBAL_POSTGRES_ENGINE = PostgresEngine(GLOBAL_STORE, scope=None, node_types=GLOBAL_NODE_TYPES)


@asynccontextmanager
async def global_pg_cursor(autocommit: bool = False):
    async with _PgStoreConnection(GLOBAL_STORE, autocommit=autocommit) as cur:
        yield cur


@asynccontextmanager
async def global_session():
    async with Session(
        parent=None, _engines=(GLOBAL_POSTGRES_ENGINE,), _fallback_engine=GLOBAL_POSTGRES_ENGINE
    ) as session:
        yield session
