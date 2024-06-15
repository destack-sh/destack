import weakref
from uuid import UUID

import structlog
from cachetools import TTLCache
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import Client, Server, User
from bench.language.bench import Bench
from bench.language.query import NodeNotFoundError
from bench.utils.env import IS_DEV
from bench.utils.oracle import get_oracle
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

PASSWORD_MIN_LENGTH = 8  # characters
PASSWORD_MAX_LENGTH = 128  # characters
SALT_LENGTH = 16  # bytes
SCRYPT_N = 2**15  # iterations count
SCRYPT_R = 8  # block size in bytes
SCRYPT_P = 1  # threads to use
SCRYPT_MAXMEM = 2**26  # max memory to use in bytes
SCRYPT_DKLEN = 32  # hash length in bytes
ACCESS_TOKEN_LENGTH = 32  # bytes


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
    oracle = get_oracle()
    start = oracle.time()
    result = hash_password(password, salt) == password_hash
    duration = oracle.time() - start
    if duration < MIN_CHECK_PASSWORD_DURATION:
        await get_oracle().sleep(MIN_CHECK_PASSWORD_DURATION - duration)
    return result


CLIENT_CACHE_ENABLED = get_from_env(
    "CLIENT_CACHE_ENABLED",
    typ=bool,
    default=True,
    description="Whether to cache Clients in the access system",
)
_caches: weakref.WeakSet["ClientCache"] = weakref.WeakSet()


class ClientCache:
    def __init__(self):
        self._cache = TTLCache[UUID, Client](maxsize=10_000, ttl=60)
        _caches.add(self)

    async def get(self, client_id: UUID, client_access_token: str) -> Client:
        client = self._cache.get(client_id)
        if client is None:
            client = await do_get_client(client_id)
            self._cache[client_id] = client
        if client.access_token != client_access_token:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid access token")
        return client

    def purge(self, user: User) -> None:
        for key, client in list(self._cache.items()):
            if client.parent_id == user.id:
                self._cache.pop(key)


async def get_client(client_id: UUID, client_access_token: str):
    client = await do_get_client(client_id)
    if client.access_token != client_access_token:
        raise GRPCError(GRPCStatus.UNAUTHENTICATED, "invalid access token")
    return client


@tracer.start_as_current_span("access.get_client")
async def do_get_client(client_id: UUID) -> Client:
    try:
        client: Client = (
            await Client.include(User.email, Client.access_token)
            .ancestors(User, Server, Bench)
            .get(id=client_id)
        )
        client._untrack_rec()
        return client
    except NodeNotFoundError as e:
        raise GRPCError(GRPCStatus.UNAUTHENTICATED, str(e) if IS_DEV else "client not found") from e


def purge_client_caches(user: User) -> None:
    for cache in _caches:
        cache.purge(user)


# TODO :Security: invalidate client_cache on significant events (e.g. logout and such)
