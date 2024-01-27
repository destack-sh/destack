import asyncio
from os import urandom

import structlog

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


MIN_CHECK_PASSWORD_DURATION = 0.005  # seconds


async def check_password(password: str, salt: bytes, password_hash: bytes) -> bool:
    """Check if a password matches its hash. Adds a small random delay to prevent timing attacks."""
    start = asyncio.get_running_loop().time()
    result = hash_password(password, salt) == password_hash
    duration = asyncio.get_running_loop().time() - start
    if duration < MIN_CHECK_PASSWORD_DURATION:
        await asyncio.sleep(MIN_CHECK_PASSWORD_DURATION - duration)
    return result


def generate_access_token() -> str:
    """Generate a random access token."""
    return urandom(ACCESS_TOKEN_LENGTH).hex()
