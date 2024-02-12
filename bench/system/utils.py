from contextlib import asynccontextmanager
from datetime import datetime
from typing import Optional
from uuid import UUID

import boto3
import structlog
from botocore.config import Config
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Session
from bench.proto.wire import AnyNodeData, AnyStructData, BenchHostStub
from bench.utils.func import uuid_to_str
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)


@asynccontextmanager
async def global_session(read_only: bool = False, host: BenchHostStub | None = None) -> "Session":
    from bench.sql.client import async_pg_connection
    from bench.language.const import _active_session

    assert _active_session.get() is None, f"already in active session {_active_session.get()}"

    async with async_pg_connection(local_pg_name=None) as conn:
        session = Session(parent=None, _global_pg_connection=conn, _host=host)
        _active_session.set(session)
        try:
            yield session
            if session.has_edits:
                if read_only:
                    raise RuntimeError(f"read_only session {session!r} has edits")
                logger.warning("session.discard", session=session)
                await session.rollback()
        finally:
            session.closed_at = datetime.utcnow()  # pretend close to prevent further use
            _active_session.set(None)


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
