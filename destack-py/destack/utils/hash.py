import math
import struct
import unicodedata

import xxhash

_SEED = 0


def hash_string(value: str) -> int:
    # NFC to collapse composed/​decomposed forms, UTF-8 bytes
    b = unicodedata.normalize("NFC", value).encode()
    return xxhash.xxh64_intdigest(b, _SEED)


def hash_bytes(value: bytes) -> int:
    return xxhash.xxh64_intdigest(value, _SEED)


def hash_int(value: int) -> int:
    # fixed 64-bit two's-complement little-endian
    b = value.to_bytes(8, "little", signed=True)
    return xxhash.xxh64_intdigest(b, _SEED)


def hash_float(value: float) -> int:
    # IEEE-754 bits so -0.0 / NaN payloads stay distinct
    bits = struct.pack(">d", math.nan if math.isnan(value) else value)
    return xxhash.xxh64_intdigest(bits, _SEED)


def hash_bool(value: bool) -> int:
    return int(value)
