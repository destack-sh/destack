import asyncio
from os import urandom

import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Badge, Client, User
from bench.language.access import Subject
from bench.language.const import NodeType
from bench.language.link import NodeNotFoundError
from bench.proto.wire import RpcMetadata
from bench.system.utils import detached_session
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


async def _get_client_from_metadata(metadata: RpcMetadata) -> Client | None:
    """Gets the authenticated client (if any)."""

    if metadata.client_kind is None:
        return None

    client_id = to_uuid(metadata.client_id)
    client: Client = (
        await Client.include(User.email, Client.access_token)
        .ancestors(NodeType.USER, NodeType.SERVER)
        .get(id=client_id)
    )
    if client.access_token != metadata.client_access_token:
        raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid access token")
    return client


async def _get_badge_from_metadata(metadata: RpcMetadata) -> Badge | None:
    """Gets the authenticated badge (if any)."""

    if metadata.badge_link_token:
        badge: Badge = await Badge.include(Badge.link_password).get(
            link_token=metadata.badge_link_token
        )
        if badge is None:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge link token")
        if badge.link_password and metadata.badge_link_password != badge.link_password:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge link password")
        return badge
    elif metadata.badge_id:
        badge: Badge = await Badge.include(Badge.key_value).get(id=to_uuid(metadata.badge_id))
        if badge.key_value != metadata.badge_key_value:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid badge key value")
        return badge
    else:
        return None


async def get_subject_from_metadata(metadata: RpcMetadata) -> Subject:
    async with detached_session(read_only=True):
        try:
            client = await _get_client_from_metadata(metadata)
            badge = await _get_badge_from_metadata(metadata)
        except NodeNotFoundError as e:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid client or badge") from e
        if client is None:
            return Subject(is_authenticated=False, badge=badge)
        elif client.parent_type == NodeType.USER:
            # TODO @Broken: fetch subject memberships/owned/roles
            #  (probably only on-demand to reduce latency)
            return Subject(
                is_authenticated=True,
                is_staff=client.user.is_staff,
                client=client,
                user=client.user,
                badge=badge,
                owned=[client.user],
            )
        else:
            raise ValueError(f"unexpected client: {client!r}")
