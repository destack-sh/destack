BASE_58_DIGITS = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def base58_encode(data: bytes) -> str:
    """Encode data in base58."""
    n = int.from_bytes(data, "big")
    result = ""
    while n:
        n, r = divmod(n, 58)
        result = BASE_58_DIGITS[r] + result
    return result


def base58_decode(data: str) -> bytes:
    """Decode data from base58."""
    n = 0
    for c in data:
        n = n * 58 + BASE_58_DIGITS.index(c)
    return n.to_bytes((n.bit_length() + 7) // 8, "big")
