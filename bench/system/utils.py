from contextlib import asynccontextmanager
import functools
from typing import Optional
from uuid import UUID

import boto3
import structlog
from botocore.config import Config
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Session, Bench, Store, StoreKind, StoreEngineType
from bench.language.const import ABOVE_SOURCE_NODE_TYPES
from bench.language.query import PostgresEngine
from bench.language.resource import StoreCredential, StoreCredentialType
from bench.proto.wire import AnyNodeData, AnyStructData, BenchHostStub
from bench.sql.client import pg_cursor_to_store
from bench.utils.func import uuid_to_str
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
SYSTEM_BENCH = Bench(name="system", slug="system", encryption_key=GLOBAL_PG_CRYPTO_KEY)
GLOBAL_STORE = Store(
    name="global",
    kind=StoreKind.RELATIONAL,
    engine=StoreEngineType.POSTGRES,
    parent=SYSTEM_BENCH,
    host=GLOBAL_PG_HOST,
    database=GLOBAL_PG_NAME,
    root_credential=StoreCredential(
        type=StoreCredentialType.ROOT,
        username=GLOBAL_PG_USERNAME,
        password=GLOBAL_PG_PASSWORD,
    ),
)
GLOBAL_POSTGRES_ENGINE = PostgresEngine(
    GLOBAL_STORE, scope=None, node_types=ABOVE_SOURCE_NODE_TYPES
)

global_pg_cursor = functools.partial(pg_cursor_to_store, store=GLOBAL_STORE)


@asynccontextmanager
async def global_session(read_only: bool = False, host: BenchHostStub | None = None) -> "Session":
    from bench.language.const import _active_session

    assert _active_session.get() is None, f"already in active session {_active_session.get()}"

    async with Session(parent=None, _engines=(GLOBAL_POSTGRES_ENGINE,), _host=host) as session:
        yield session


def validate_bench_data(
    data: AnyNodeData | AnyStructData,
    in_package: UUID | str | None = None,
) -> None:
    """Check that BenchData structs have valid data. Raises gRPC errors."""
    if data.metatype is None:
        raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing metatype")
    in_package = uuid_to_str(in_package)
    if in_package is not None and hasattr(data, "package_id") and data.package_id != in_package:
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT, f"wrong package_id: {data.package_id} != {in_package}"
        )
    # TODO @Robustness!: complete validate_bench_data?


def validate_bench_data_many(
    *data: AnyNodeData | AnyStructData, in_package: UUID | None = None
) -> None:
    for d in data:
        validate_bench_data(d, in_package=in_package)


_s3_client: Optional["boto3.client"] = None


def get_s3_client() -> "boto3.client":
    global _s3_client
    if _s3_client is None:
        _s3_client = boto3.client(
            "s3",
            endpoint_url=get_from_env("AWS_ENDPOINT_URL"),
            config=Config(s3={"addressing_style": "path"}, region_name=get_from_env("AWS_REGION")),
        )
    return _s3_client
