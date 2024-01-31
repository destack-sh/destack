import asyncio
from os import urandom

import structlog
from grpclib import GRPCError, Status as GRPCStatus

from bench.language import Client, Worker, Badge
from bench.language.access import RequestSubject
from bench.proto.wire import RpcMetadata, ClientKind
from bench.utils.func import to_uuid

logger = structlog.get_logger(__name__)

PASSWORD_MIN_LENGTH = 8  # characters
PASSWORD_MAX_LENGTH = 128  # characters
SALT_LENGTH = 16  # bytes
SCRYPT_N = 2**15  # iterations count
SCRYPT_R = 8  # block size in bytes
SCRYPT_P = 1  # threads to use
SCRYPT_MAXMEM = 2**26  # max memory to use in bytes
SCRYPT_DKLEN = 32  # hash length in bytes
ACCESS_TOKEN_LENGTH = 32  # bytes


def generate_salt() -> bytes:
    """Generate a random salt."""
    return urandom(SALT_LENGTH)


def hash_password(password: str, salt: bytes) -> bytes:
    """Hash a password using scrypt."""
    from hashlib import scrypt

    assert len(salt) == SALT_LENGTH, f"invalid salt length: {len(salt)} != {SALT_LENGTH}"
    if PASSWORD_MIN_LENGTH < len(password) > PASSWORD_MAX_LENGTH:
        raise ValueError(
            f"password length ({len(password)}) is not in [{PASSWORD_MIN_LENGTH}, {PASSWORD_MAX_LENGTH}]"
        )

    return scrypt(
        password.encode("utf-8"),
        salt=salt,
        n=SCRYPT_N,
        r=SCRYPT_R,
        p=SCRYPT_P,
        maxmem=SCRYPT_MAXMEM,
        dklen=SCRYPT_DKLEN,
    )


MIN_CHECK_PASSWORD_DURATION = 0.005  # =5ms


async def check_password(password: str, salt: bytes, password_hash: bytes) -> bool:
    """Check if a password matches its hash. Adds a small random delay to prevent timing attacks."""
    loop = asyncio.get_running_loop()
    start = loop.time()
    result = hash_password(password, salt) == password_hash
    duration = loop.time() - start
    if duration < MIN_CHECK_PASSWORD_DURATION:
        await asyncio.sleep(MIN_CHECK_PASSWORD_DURATION - duration)
    return result


def generate_access_token() -> str:
    """Generate a random access token."""
    return urandom(ACCESS_TOKEN_LENGTH).hex()


async def _get_subject_from_metadata(metadata: RpcMetadata) -> Client | Worker | None:
    """Gets the authenticated client (if any)."""

    if metadata.client_kind is None:
        return None
    elif metadata.client_kind == ClientKind.USER:
        client = await Client.get(id=to_uuid(metadata.client_id))
        if client.access_token != metadata.access_token:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid access token")
        return client
    elif metadata.client_kind == ClientKind.WORKER:
        worker = await Worker.get(id=to_uuid(metadata.client_id))
        if worker.access_token != metadata.access_token:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid access token")
        return worker
    else:
        raise GRPCError(GRPCStatus.UNAUTHENTICATED, "unexpected client kind")


async def _get_badge_from_metadata(metadata: RpcMetadata) -> Badge | None:
    """Gets the authenticated badge (if any)."""

    if metadata.badge_link_token:
        badge: Badge = await Badge.options(include_sensitive=True).get(
            link_token=metadata.badge_link_token
        )
        if badge is None:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge link token")
        if badge.link_password and metadata.badge_link_password != badge.link_password:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge link password")
        return badge
    elif metadata.badge_id:
        badge: Badge = await Badge.options(include_sensitive=True).get(
            id=to_uuid(metadata.badge_id)
        )
        if badge.key_value != metadata.badge_key_value:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge key value")
        return badge
    else:
        return None


async def get_request_subject(metadata: RpcMetadata) -> RequestSubject:
    subject = await _get_subject_from_metadata(metadata)
    badge = await _get_badge_from_metadata(metadata)
    return RequestSubject.from_authentication(subject, badge)
