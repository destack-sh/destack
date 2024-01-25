from contextlib import asynccontextmanager
from datetime import datetime
from typing import Optional
from uuid import UUID

import betterproto
import boto3
import structlog
from botocore.config import Config
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from multidict import MultiDict

from bench.language import Client, Worker, Session
from bench.proto.wire import AnyNodeData, AnyStructData, ClientKind, RpcMetadata
from bench.utils.func import to_uuid, uuid_to_str
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)


@asynccontextmanager
async def detached_session(commit: bool = False, read_only: bool = False) -> "Session":
    """Get a global session."""
    from bench.sql.client import async_pg_cursor
    from bench.language.const import _active_session

    assert not read_only or not commit, "read_only and commit are mutually exclusive"
    assert _active_session.get() is None, f"already in active session {_active_session.get()}"

    async with async_pg_cursor(local_pg_name=None) as global_pg_cursor:
        session = Session(parent=None, _global_pg_cursor=global_pg_cursor)
        _active_session.set(session)
        try:
            yield session
            if commit:
                await session.commit()
            elif session.has_regular_edits:
                if read_only:
                    raise RuntimeError(f"read_only session {session!r} has edits")
                logger.warning("session.discard", session=session)
        finally:
            session.closed_at = datetime.utcnow()  # pretend close to prevent further use
            _active_session.set(None)


def encode_metadata(metadata: RpcMetadata) -> dict:
    """Encodes RPC call metadata for gRPC/HTTP headers."""
    packed = metadata.to_dict(casing=betterproto.Casing.SNAKE, include_default_values=False)
    return {k.replace("_", "-"): str(v) for k, v in packed.items()}


def parse_metadata(metadata: MultiDict) -> RpcMetadata:
    """Parses RPC call metadata from gRPC/HTTP headers."""
    packed = {k.replace("-", "_"): v for k, v in metadata.items()}
    return RpcMetadata.from_dict(packed)


def validate_bench_data(
    data: AnyNodeData | AnyStructData,
    in_bench: UUID | str | None = None,
    in_package: UUID | str | None = None,
) -> None:
    """Check that the data structs have all the required fields. Raises gRPC errors."""
    in_bench = uuid_to_str(in_bench)
    in_package = uuid_to_str(in_package)
    if in_bench is not None and hasattr(data, "bench_id") and data.bench_id != in_bench:
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT, f"wrong bench_id: {data.bench_id} != {in_bench}"
        )
    if in_package is not None and hasattr(data, "package_id") and data.package_id != in_package:
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT, f"wrong package_id: {data.package_id} != {in_package}"
        )
    raise NotImplementedError("nocheckin: check_data")


def validate_bench_data_many(
    *data: AnyNodeData | AnyStructData, in_bench: UUID | None = None, in_package: UUID | None = None
) -> None:
    for d in data:
        validate_bench_data(d, in_bench=in_bench, in_package=in_package)


async def check_authenticated(metadata: RpcMetadata) -> Client | Worker:
    if metadata.client_kind == ClientKind.USER:
        client = await Client.get(id=to_uuid(metadata.client_id))
        if client.access_token != metadata.access_token:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "wrong access token")
        return client
    elif metadata.client_kind == ClientKind.WORKER:
        worker = await Worker.get(id=to_uuid(metadata.client_id))
        if worker.access_token != metadata.access_token:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "wrong access token")
        return worker
    else:
        raise GRPCError(GRPCStatus.UNAUTHENTICATED, "unexpected client kind")


async def check_authenticated_client(metadata: RpcMetadata) -> Client:
    client = await check_authenticated(metadata)
    if not isinstance(client, Client):
        raise GRPCError(GRPCStatus.UNAUTHENTICATED, "expected user client")
    return client


async def check_authenticated_worker(metadata: RpcMetadata) -> Worker:
    worker = await check_authenticated(metadata)
    if not isinstance(worker, Worker):
        raise GRPCError(GRPCStatus.UNAUTHENTICATED, "expected worker client")
    return worker


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
