# see https://observablehq.com/@dgreensp/implementing-fractional-indexing
# (licensed as CC-0)
# sync with fractional.ts in frontend

from typing import Optional, cast

from .declaration import declare_constant, declare_method
from .types import UInt32

_BASE_95_DIGITS = (
    "!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~"
)

FRACTIONAL_INTEGER_ZERO = "a0"
FRACTIONAL_INTEGER_MIN = "A00000000000000000000000000"
FRACTIONAL_INTEGER_MAX = "aZZZZZZZZZZZZZZZZZZZZZZZZZ"

declare_constant(301, FRACTIONAL_INTEGER_ZERO, name="FRACTIONAL_INTEGER_ZERO")
declare_constant(302, FRACTIONAL_INTEGER_MIN, name="FRACTIONAL_INTEGER_MIN")
declare_constant(303, FRACTIONAL_INTEGER_MAX, name="FRACTIONAL_INTEGER_MAX")


def _get_integer_length(head: str) -> int:
    if "a" <= head <= "z":
        return ord(head) - ord("a") + 2
    elif "A" <= head <= "Z":
        return ord("Z") - ord(head) + 2
    else:
        raise ValueError(f"invalid order key head: {head}")


def _validate_integer(int: str) -> None:
    if len(int) != _get_integer_length(int[0]):
        raise ValueError(f"invalid integer part of order key: {int}")


def _midpoint(a: str, b: Optional[str]) -> str:
    """
    Gets the midpoint between two strings, `a` and `b`, in the given `digits` base.
    `a` may be empty string, `b` is null or non-empty string.
    `a < b` lexicographically if `b` is non-null.
    No trailing zeros allowed.
    """
    if b is not None and a >= b:
        raise ValueError(f"{a} >= {b}")
    if (len(a) > 0 and a[-1] == "0") or (b is not None and b[-1] == "0"):
        raise ValueError("trailing zero")
    if b:
        # remove the longest common prefix.  pad `a` with 0s as we
        # go.  note that we don't need to pad `b`, because it can't
        # end before `a` while traversing the common prefix.
        n = 0
        while (a[n] if n < len(a) else "0") == b[n]:
            n += 1
        if n > 0:
            return b[:n] + _midpoint(a[n:], b[n:])
    # first digits (or lack of digit) are different
    digit_a = _BASE_95_DIGITS.index(a[0]) if a else 0
    digit_b = _BASE_95_DIGITS.index(b[0]) if b else len(_BASE_95_DIGITS)
    if digit_b - digit_a > 1:
        # use int(0.5 + ..) instead of round(..) because round(0.5) is 0 (??)
        mid_digit = int(0.5 + 0.5 * (digit_a + digit_b))
        return _BASE_95_DIGITS[mid_digit]
    else:
        # first digits are consecutive
        if b is not None and len(b) > 1:
            return b[0]
        else:
            # `b` is null or has length 1 (a single digit).
            # the first digit of `a` is the previous digit to `b`,
            # or 9 if `b` is null.
            # given, for example, midpoint('49', '5'), return
            # '4' + midpoint('9', null), which will become
            # '4' + '9' + midpoint('', null), which is '495'
            return _BASE_95_DIGITS[digit_a] + _midpoint(a[1:], None)


def increment_integer(x: str) -> Optional[str]:
    """
    Increments the given integer `x` in the given `digits` base.
    Returns `None` if the result is too large.
    """
    _validate_integer(x)
    head, *digs = x
    carry = True
    for i in range(len(digs) - 1, -1, -1):
        d = _BASE_95_DIGITS.index(digs[i]) + 1
        if d == len(_BASE_95_DIGITS):
            digs[i] = "0"
        else:
            digs[i] = _BASE_95_DIGITS[d]
            carry = False
            break
    if carry:
        if head == "Z":
            return "a0"
        if head == "z":
            return None
        h = chr(ord(head) + 1)
        if h > "a":
            digs.append("0")
        else:
            digs.pop()
        return h + "".join(digs)
    else:
        return head + "".join(digs)


@declare_method(301, is_implemented=True)
def decrement_integer(x: str) -> Optional[str]:
    """
    Decrements the given integer `x` in the given `digits` base.
    """
    _validate_integer(x)
    head, *digs = x
    borrow = True
    for i in range(len(digs) - 1, -1, -1):
        d = _BASE_95_DIGITS.index(digs[i]) - 1
        if d == -1:
            digs[i] = _BASE_95_DIGITS[-1]
        else:
            digs[i] = _BASE_95_DIGITS[d]
            borrow = False
    if borrow:
        if head == "a":
            return "Z" + _BASE_95_DIGITS[-1]
        if head == "A":
            return None
        h = chr(ord(head) - 1)
        if h < "Z":
            digs.append(_BASE_95_DIGITS[-1])
        else:
            digs.pop()
        return h + "".join(digs)
    else:
        return head + "".join(digs)


def _get_integer_part(key: str) -> str:
    """
    Gets the integer part of the given order key.
    """
    integer_part_length = _get_integer_length(key[0])
    if integer_part_length > len(key):
        raise ValueError(f"invalid order key: {key}")
    return key[:integer_part_length]


def _is_valid_order_key(key: str) -> bool:
    if key == FRACTIONAL_INTEGER_MIN:
        return False
    try:
        i = _get_integer_part(key)
    except ValueError:
        return False
    f = key[len(i) :]
    return not (len(f) > 0 and f[-1] == "0")


def _validate_order_key(key: str) -> None:
    if not _is_valid_order_key(key):
        raise ValueError(f"invalid order key: {key}")


@declare_method(302, is_implemented=True)
def get_order_key(a: Optional[str], b: Optional[str]) -> str:
    """
    Generates a key between the given keys `a` and `b` (inclusive) with logarithmic fraction growth.
    """
    if a is not None:
        _validate_order_key(a)
    if b is not None:
        _validate_order_key(b)
    if a is not None and b is not None and a >= b:
        raise ValueError(f"{a} >= {b}")
    if a is None:
        if b is None:
            return FRACTIONAL_INTEGER_ZERO
        ib = _get_integer_part(b)
        fb = b[len(ib) :]
        if ib == FRACTIONAL_INTEGER_MIN:
            return ib + _midpoint("", fb)
        return ib if ib < b else cast(str, decrement_integer(ib))
    if b is None:
        ia = _get_integer_part(a)
        fa = a[len(ia) :]
        i = increment_integer(ia)
        return i if i is not None else ia + _midpoint(fa, None)
    ia = _get_integer_part(a)
    fa = a[len(ia) :]
    ib = _get_integer_part(b)
    fb = b[len(ib) :]
    if ia == ib:
        return ia + _midpoint(fa, fb)
    i = cast(str, increment_integer(ia))
    return i if i < b else ia + _midpoint(fa, None)


@declare_method(303, is_implemented=True)
def get_order_keys(a: Optional[str], b: Optional[str], n: UInt32) -> list[str]:
    """
    Generates evenly spread n keys between the given keys `a` and `b` (inclusive).
    """
    if n == 0:
        return []
    if n == 1:
        return [get_order_key(a, b)]
    if b is None:
        c = get_order_key(a, b)
        result = [c]
        for _i in range(n - 1):
            c = get_order_key(c, b)
            result.append(c)
        return result
    if a is None:
        c = get_order_key(a, b)
        result = [c]
        for _i in range(n - 1):
            c = get_order_key(a, c)
            result.append(c)
        result.reverse()
        return result
    mid = n // 2
    c = get_order_key(a, b)
    return [*get_order_keys(a, c, mid), c, *get_order_keys(c, b, n - mid - 1)]
