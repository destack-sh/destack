import math
import struct
import unicodedata

import xxhash

_SEED = 0


def hash_string(value: str) -> int:
    """Hash a unicode string (decomposed/composed forms are collapsed, UTF-8 bytes)."""
    b = unicodedata.normalize("NFC", value).encode()
    return xxhash.xxh32_intdigest(b, _SEED)


def hash_bytes(value: bytes) -> int:
    """Hash a byte sequence."""
    return xxhash.xxh32_intdigest(value, _SEED)


def hash_int(value: int) -> int:
    """Hash an integer."""
    b = value.to_bytes(8, "little", signed=True)
    return xxhash.xxh32_intdigest(b, _SEED)


def hash_float(value: float) -> int:
    """Hash a float float."""
    bits = struct.pack(">d", math.nan if math.isnan(value) else value)
    return xxhash.xxh32_intdigest(bits, _SEED)


def hash_bool(value: bool) -> int:
    """Hash a boolean."""
    return int(value)
