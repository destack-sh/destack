import asyncio
import secrets
from os import urandom
from typing import cast

import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Badge, Bench, Client, Server, User
from bench.language.access import Owner, Subject
from bench.language.const import NodeType
from bench.language.query import NodeNotFoundError
from bench.proto.wire import RpcMetadata
from bench.system.client import global_session
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


def generate_encryption_key(length: int = 32) -> str:
    """Encryption key for pgcrypto symmetric encryption."""
    return secrets.token_hex(length)


async def _get_client_from_metadata(metadata: RpcMetadata) -> Client | None:
    """Gets the authenticated client (if any)."""

    if metadata.client_id:
        client_id = to_uuid(metadata.client_id)
        client: Client = (
            await Client.include(User.email, Client.access_token)
            .ancestors(User, Server)
            .get(id=client_id)
        )
        if metadata.client_access_token != client.access_token:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid access token")
        return client
    else:
        return None


async def _get_badges_from_metadata(metadata: RpcMetadata) -> list[Badge]:
    """Gets the authenticated badge (if any)."""

    if metadata.badges:
        badges: list[Badge] = (
            await Badge.include(Badge.key, Badge.password)
            .filter(id__in=tuple(to_uuid(b.id) for b in metadata.badges))
            .tolist()
        )
        for actual_badge, expected_badge in zip(badges, metadata.badges):
            if actual_badge.key and actual_badge.key != expected_badge.key:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge key")
            if actual_badge.password and actual_badge.password != expected_badge.password:
                raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge password")
        return badges
    else:
        return []


async def get_subject_from_metadata(metadata: RpcMetadata) -> Subject:
    async with global_session():
        try:
            client = await _get_client_from_metadata(metadata)
        except NodeNotFoundError as e:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid client") from e
        try:
            badges = await _get_badges_from_metadata(metadata)
        except NodeNotFoundError as e:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge") from e
        if client is None:
            return Subject(is_authenticated=False, badges=badges)
        elif client.parent_type == NodeType.USER:
            # TODO :Broken: fetch all subject memberships/owned/roles
            #  (probably only on-demand to reduce latency)
            owned = [client.user]
            if client.user.main_bench_ptr:  # (we cheat a little and get only the main Bench)
                main_bench = await Bench.get(id=client.user.main_bench_ptr.id)
                owned.append(main_bench)
            return Subject(
                is_authenticated=True,
                is_staff=client.user.is_staff,
                client=client,
                user=client.user,
                badges=badges,
                owned=cast(list[Owner], owned),
            )
        else:
            raise ValueError(f"unexpected client: {client!r}")
