import secrets
import time
from typing import TYPE_CHECKING

try:
    if TYPE_CHECKING:
        from uuid import UUID, uuid4, uuid5
    else:
        from fastuuid import UUID, uuid4, uuid5
except ImportError:
    from uuid import UUID, uuid4, uuid5

_time_ns = time.time_ns
_token_bytes = secrets.token_bytes
_MASK62 = (1 << 62) - 1


def uuid7() -> UUID:
    """
    Generate a UUIDv7.
    See https://www.rfc-editor.org/rfc/rfc9562.html#name-uuid-version-7.
    """

    ts_ms = _time_ns() // 1_000_000  # 48-bit UNIX epoch ms (int, no float math)
    rnd = int.from_bytes(_token_bytes(10), "big")  # 80 random bits in one syscall
    rand_a = rnd >> 68  # top 12 bits
    rand_b = rnd & _MASK62  # bottom 62 bits
    # combine (with variant 10)
    value = (ts_ms << 80) | (0x7 << 76) | (rand_a << 64) | (0b10 << 62) | rand_b
    return UUID(int=value)


from .declaration import declare_method  # noqa: E402

uuid4 = declare_method(301, is_implemented=True, description="Generate a UUIDv4.")(uuid4)
uuid5 = declare_method(302, is_implemented=True, description="Generate a UUIDv5.")(uuid5)
uuid7 = declare_method(303, is_implemented=True, description="Generate a UUIDv7.")(uuid7)

__all__ = ["UUID", "uuid4", "uuid5", "uuid7"]
