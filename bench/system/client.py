from contextlib import asynccontextmanager

import structlog

from bench.language import Bench, Session, Store
from bench.language.bench import Region
from bench.language.connection import PostgresEngine
from bench.language.const import GLOBAL_NODE_TYPES, VERSION
from bench.language.session import CommitHook
from bench.proto.wire import GraphScope
from bench.sql.client import GLOBAL_PG_CRYPTO_KEY, _PgStoreConnection
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)

GLOBAL_PG_HOST = get_from_env("GLOBAL_PG_HOST")
GLOBAL_PG_NAME = get_from_env("GLOBAL_PG_NAME")
GLOBAL_PG_USERNAME = get_from_env("GLOBAL_PG_USERNAME")
GLOBAL_PG_PASSWORD = get_from_env("GLOBAL_PG_PASSWORD")

SYSTEM_BENCH_STUB = Bench(
    name="System (Stub)", slug="system", region=Region.GLOBAL, encryption_key=GLOBAL_PG_CRYPTO_KEY
)

GLOBAL_STORE = Store(
    parent=SYSTEM_BENCH_STUB,
    name="Global Store",
    version=VERSION,
    external_name=GLOBAL_PG_NAME,
    connection_uri=f"postgresql://{GLOBAL_PG_USERNAME}:{GLOBAL_PG_PASSWORD}@{GLOBAL_PG_HOST}/{GLOBAL_PG_NAME}",
)
GLOBAL_POSTGRES_ENGINE = PostgresEngine(
    store=GLOBAL_STORE, bench=SYSTEM_BENCH_STUB, scope=GraphScope(), node_types=GLOBAL_NODE_TYPES
)


@asynccontextmanager
async def global_pg_cursor(autocommit: bool = False):
    async with _PgStoreConnection(GLOBAL_STORE, SYSTEM_BENCH_STUB, autocommit=autocommit) as cur:
        yield cur


@asynccontextmanager
async def global_session(on_commit_hook: CommitHook | None = None):
    async with Session(
        parent=None,
        _default_scope=GraphScope(),
        _engines=(GLOBAL_POSTGRES_ENGINE,),
        _on_commit_hook=on_commit_hook,
    ) as session:
        yield session
